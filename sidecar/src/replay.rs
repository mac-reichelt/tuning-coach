//! Packet-capture replay.
//!
//! Loads a `.pcapng` (or legacy `.pcap`) capture of Forza UDP telemetry and
//! feeds the contained packets into the same `latest_telemetry_tx` watch
//! channel the live UDP listener uses, paced by the capture's own timestamps.
//!
//! This lets the sidecar drive the overlay/web view and every downstream
//! heuristic from a recorded session, with no live game running — useful for
//! validating the UI and for developing features against reproducible data.
//!
//! The parser is intentionally self-contained (no external pcap crate): it
//! walks pcapng Section Header / Interface Description / Enhanced Packet
//! blocks (and legacy pcap records), strips the link layer, extracts IPv4/UDP
//! payloads, and keeps every payload that [`parse_telemetry_packet`] accepts.
//! Payloads that are not valid Forza telemetry are skipped, so the same file
//! may contain unrelated traffic.

use std::{path::Path, sync::Arc, time::Duration};

use anyhow::Context;
use tokio::sync::{broadcast, watch};
use tracing::{info, warn};

use crate::telemetry::{parse_telemetry_packet, TelemetryPacket};

/// Upper bound on a single inter-packet sleep. Real Forza captures stream
/// continuously (~60 Hz) even while paused, so large gaps should not occur;
/// this is a safety net so a pathological capture can never make the replay
/// appear hung.
const MAX_PACKET_GAP: Duration = Duration::from_secs(30);

/// A parsed telemetry packet plus the delay to wait (at 1x speed) after the
/// previous packet before emitting it.
#[derive(Debug, Clone)]
pub struct ReplayPacket {
    pub delay: Duration,
    pub packet: TelemetryPacket,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Endian {
    Little,
    Big,
}

impl Endian {
    fn u16(self, b: &[u8]) -> u16 {
        let a = [b[0], b[1]];
        match self {
            Endian::Little => u16::from_le_bytes(a),
            Endian::Big => u16::from_be_bytes(a),
        }
    }

    fn u32(self, b: &[u8]) -> u32 {
        let a = [b[0], b[1], b[2], b[3]];
        match self {
            Endian::Little => u32::from_le_bytes(a),
            Endian::Big => u32::from_be_bytes(a),
        }
    }
}

/// Read a capture file from disk and extract every Forza telemetry packet,
/// preserving capture order and inter-packet timing.
pub fn load_replay_packets(path: &Path) -> anyhow::Result<Vec<ReplayPacket>> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("failed to read replay capture {}", path.display()))?;
    parse_capture(&bytes)
        .with_context(|| format!("failed to parse replay capture {}", path.display()))
}

/// Parse an in-memory capture (pcapng or legacy pcap) into replay packets.
fn parse_capture(bytes: &[u8]) -> anyhow::Result<Vec<ReplayPacket>> {
    let timed = if bytes.len() >= 4 && bytes[0..4] == [0x0A, 0x0D, 0x0D, 0x0A] {
        parse_pcapng(bytes)?
    } else if bytes.len() >= 4 && is_legacy_pcap_magic(&bytes[0..4]) {
        parse_legacy_pcap(bytes)?
    } else {
        anyhow::bail!("unrecognized capture format (expected pcapng or pcap magic)");
    };

    let mut out = Vec::with_capacity(timed.len());
    let mut prev_ts: Option<u64> = None;
    for (ts_ns, packet) in timed {
        let delay = match (prev_ts, ts_ns) {
            // First packet: emit immediately.
            (None, _) => Duration::ZERO,
            (Some(prev), Some(now)) if now >= prev => {
                Duration::from_nanos(now - prev).min(MAX_PACKET_GAP)
            }
            // Missing/backwards timestamp: assume ~60 Hz spacing.
            _ => Duration::from_micros(16_667),
        };
        if ts_ns.is_some() {
            prev_ts = ts_ns;
        }
        out.push(ReplayPacket { delay, packet });
    }
    Ok(out)
}

fn is_legacy_pcap_magic(m: &[u8]) -> bool {
    matches!(
        [m[0], m[1], m[2], m[3]],
        [0xA1, 0xB2, 0xC3, 0xD4] // LE, microseconds
            | [0xD4, 0xC3, 0xB2, 0xA1] // BE, microseconds
            | [0xA1, 0xB2, 0x3C, 0x4D] // LE, nanoseconds
            | [0x4D, 0x3C, 0xB2, 0xA1] // BE, nanoseconds
    )
}

/// Parsed packets carry an optional capture timestamp in nanoseconds.
type TimedPacket = (Option<u64>, TelemetryPacket);

fn parse_pcapng(bytes: &[u8]) -> anyhow::Result<Vec<TimedPacket>> {
    const SHB: [u8; 4] = [0x0A, 0x0D, 0x0D, 0x0A];
    let mut out = Vec::new();
    // Per-interface (linktype, nanoseconds-per-timestamp-unit).
    let mut interfaces: Vec<(u16, u64)> = Vec::new();
    let mut endian = Endian::Little;
    let mut pos = 0usize;

    while pos + 8 <= bytes.len() {
        // Section Header Blocks are self-describing for byte order; the block
        // type itself is a byte-order-invariant palindrome.
        if bytes[pos..pos + 4] == SHB {
            if pos + 12 > bytes.len() {
                break;
            }
            endian = if u32::from_le_bytes([
                bytes[pos + 8],
                bytes[pos + 9],
                bytes[pos + 10],
                bytes[pos + 11],
            ]) == 0x1A2B_3C4D
            {
                Endian::Little
            } else {
                Endian::Big
            };
            let total = endian.u32(&bytes[pos + 4..]) as usize;
            if total < 12 || pos + total > bytes.len() {
                break;
            }
            interfaces.clear();
            pos += total;
            continue;
        }

        let block_type = endian.u32(&bytes[pos..]);
        let total = endian.u32(&bytes[pos + 4..]) as usize;
        if total < 12 || pos + total > bytes.len() {
            break;
        }
        let body = &bytes[pos + 8..pos + total - 4];

        match block_type {
            0x0000_0001 => {
                // Interface Description Block: u16 linktype, u16 reserved, u32 snaplen, options.
                if body.len() >= 8 {
                    let linktype = endian.u16(&body[0..2]);
                    let resol = parse_if_tsresol(&body[8..], endian);
                    interfaces.push((linktype, resol));
                }
            }
            0x0000_0006 => {
                // Enhanced Packet Block.
                if body.len() >= 20 {
                    let iface = endian.u32(&body[0..4]) as usize;
                    let ts_high = endian.u32(&body[4..8]) as u64;
                    let ts_low = endian.u32(&body[8..12]) as u64;
                    let cap_len = endian.u32(&body[12..16]) as usize;
                    if let (Some(frame), Some(&(linktype, resol))) =
                        (body.get(20..20 + cap_len), interfaces.get(iface))
                    {
                        let ts_ns = ((ts_high << 32) | ts_low).saturating_mul(resol);
                        if let Some(packet) = forza_from_frame(linktype, frame) {
                            out.push((Some(ts_ns), packet));
                        }
                    }
                }
            }
            0x0000_0003 => {
                // Simple Packet Block: u32 original length, then packet (no timestamp).
                let linktype = interfaces.first().map(|&(lt, _)| lt).unwrap_or(0);
                if body.len() >= 4 {
                    let frame = &body[4..];
                    if let Some(packet) = forza_from_frame(linktype, frame) {
                        out.push((None, packet));
                    }
                }
            }
            _ => {}
        }

        pos += total;
    }

    Ok(out)
}

/// Parse the `if_tsresol` option (code 9) out of an IDB option block, returning
/// nanoseconds per timestamp unit. Defaults to 1000 (microsecond resolution).
fn parse_if_tsresol(opts: &[u8], endian: Endian) -> u64 {
    let mut o = 0usize;
    while o + 4 <= opts.len() {
        let code = endian.u16(&opts[o..o + 2]);
        let len = endian.u16(&opts[o + 2..o + 4]) as usize;
        o += 4;
        if code == 0 {
            break;
        }
        if code == 9 && len >= 1 && o < opts.len() {
            let val = opts[o];
            return if val & 0x80 != 0 {
                // base-2 exponent
                1_000_000_000u64 >> (val & 0x7F).min(63)
            } else {
                let exp = (val & 0x7F) as u32;
                1_000_000_000u64 / 10u64.pow(exp.min(18))
            }
            .max(1);
        }
        o = (o + len + 3) & !3;
    }
    1_000
}

fn parse_legacy_pcap(bytes: &[u8]) -> anyhow::Result<Vec<TimedPacket>> {
    if bytes.len() < 24 {
        anyhow::bail!("legacy pcap shorter than global header");
    }
    let endian = match [bytes[0], bytes[1], bytes[2], bytes[3]] {
        [0xA1, 0xB2, 0xC3, 0xD4] | [0xA1, 0xB2, 0x3C, 0x4D] => Endian::Little,
        _ => Endian::Big,
    };
    let nanosecond_ts = matches!(
        [bytes[0], bytes[1], bytes[2], bytes[3]],
        [0xA1, 0xB2, 0x3C, 0x4D] | [0x4D, 0x3C, 0xB2, 0xA1]
    );
    let linktype = endian.u32(&bytes[20..24]) as u16;

    let mut out = Vec::new();
    let mut pos = 24usize;
    while pos + 16 <= bytes.len() {
        let ts_sec = endian.u32(&bytes[pos..]) as u64;
        let ts_frac = endian.u32(&bytes[pos + 4..]) as u64;
        let incl_len = endian.u32(&bytes[pos + 8..]) as usize;
        pos += 16;
        let Some(frame) = bytes.get(pos..pos + incl_len) else {
            break;
        };
        let ts_ns = ts_sec * 1_000_000_000
            + if nanosecond_ts {
                ts_frac
            } else {
                ts_frac * 1_000
            };
        if let Some(packet) = forza_from_frame(linktype, frame) {
            out.push((Some(ts_ns), packet));
        }
        pos += incl_len;
    }
    Ok(out)
}

/// Strip the link layer for `linktype`, extract the IPv4/UDP payload, and parse
/// it as Forza telemetry. Returns `None` for any frame that is not valid Forza
/// telemetry.
fn forza_from_frame(linktype: u16, frame: &[u8]) -> Option<TelemetryPacket> {
    let ip = strip_link_layer(linktype, frame)?;
    let payload = udp_payload(ip)?;
    parse_telemetry_packet(payload).ok()
}

/// Return the IP portion of a captured frame for the given libpcap link type.
fn strip_link_layer(linktype: u16, frame: &[u8]) -> Option<&[u8]> {
    match linktype {
        // DLT_NULL / loopback: 4-byte address-family header.
        0 => frame.get(4..),
        // DLT_EN10MB: 14-byte Ethernet header; only IPv4 ethertype 0x0800.
        1 => {
            let ethertype = u16::from_be_bytes([*frame.get(12)?, *frame.get(13)?]);
            (ethertype == 0x0800).then(|| frame.get(14..))?
        }
        // DLT_RAW variants: IP directly.
        12 | 14 | 101 => Some(frame),
        // DLT_LINUX_SLL: 16-byte cooked header.
        113 => frame.get(16..),
        _ => None,
    }
}

/// Extract the UDP payload from an IPv4 packet, bounded by the UDP length field
/// so trailing capture padding does not corrupt the payload length.
fn udp_payload(ip: &[u8]) -> Option<&[u8]> {
    if ip.len() < 20 || ip[0] >> 4 != 4 || ip[9] != 17 {
        return None;
    }
    let ihl = (ip[0] & 0x0F) as usize * 4;
    if ihl < 20 {
        return None;
    }
    let udp = ip.get(ihl..)?;
    if udp.len() < 8 {
        return None;
    }
    let udp_len = u16::from_be_bytes([udp[4], udp[5]]) as usize;
    match udp_len.checked_sub(8) {
        Some(payload_len) if payload_len <= udp.len() - 8 => udp.get(8..8 + payload_len),
        // Fall back to the rest of the datagram if the length field is bogus.
        _ => udp.get(8..),
    }
}

/// Replay loop: emit each packet into `latest_packet_tx`, paced by the capture
/// timing (scaled by `speed`). Repeats while `looping` is set. Stops promptly
/// on shutdown. When not looping, broadcasts a shutdown once the capture is
/// exhausted so the process exits on its own.
pub async fn replay_loop(
    packets: Arc<Vec<ReplayPacket>>,
    latest_packet_tx: watch::Sender<Option<TelemetryPacket>>,
    shutdown_tx: broadcast::Sender<()>,
    looping: bool,
    speed: f32,
) -> anyhow::Result<()> {
    let mut shutdown_rx = shutdown_tx.subscribe();
    if packets.is_empty() {
        warn!("replay capture contained no Forza telemetry packets; nothing to replay");
        if !looping {
            let _ = shutdown_tx.send(());
        }
        return Ok(());
    }
    let speed = if speed > 0.0 { speed } else { 1.0 };
    info!(
        packet_count = packets.len(),
        looping, speed, "telemetry replay started"
    );

    let mut iteration: u64 = 0;
    loop {
        for replay_packet in packets.iter() {
            let delay = replay_packet.delay.div_f32(speed);
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    info!("telemetry replay shutting down");
                    return Ok(());
                }
                _ = tokio::time::sleep(delay) => {}
            }
            let _ = latest_packet_tx.send_replace(Some(replay_packet.packet.clone()));
        }
        iteration += 1;
        if !looping {
            info!("telemetry replay finished; signalling shutdown");
            let _ = shutdown_tx.send(());
            return Ok(());
        }
        info!(iteration, "telemetry replay looping");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/lap_validity/fm-training.pcapng")
    }

    #[test]
    fn loads_forza_packets_from_fixture_capture() {
        let packets = load_replay_packets(&fixture_path()).expect("load fixture");
        // README documents 30,048 Forza Dash packets in this capture.
        assert!(
            packets.len() > 30_000,
            "expected >30k packets, got {}",
            packets.len()
        );

        // Every packet must parse to telemetry; the capture is FM 2023 Dash.
        let dash_count = packets
            .iter()
            .filter(|p| matches!(p.packet, TelemetryPacket::Dash(_)))
            .count();
        assert_eq!(dash_count, packets.len());

        // Car ordinal 419 (2006 Audi RS4) appears in in-control packets.
        let has_expected_car = packets.iter().any(|p| match &p.packet {
            TelemetryPacket::Dash(d) => d.sled.car_ordinal == 419,
            _ => false,
        });
        assert!(has_expected_car, "expected car ordinal 419 in capture");
    }

    #[test]
    fn first_packet_has_zero_delay_and_gaps_are_bounded() {
        let packets = load_replay_packets(&fixture_path()).expect("load fixture");
        assert_eq!(packets[0].delay, Duration::ZERO);
        assert!(packets.iter().all(|p| p.delay <= MAX_PACKET_GAP));
    }

    #[test]
    fn rejects_unknown_capture_format() {
        let err = parse_capture(b"not a capture at all").unwrap_err();
        assert!(err.to_string().contains("unrecognized capture format"));
    }
}
