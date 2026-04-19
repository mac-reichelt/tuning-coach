#!/usr/bin/env python3
"""
Forza FM 2023 Dash UDP capture analyzer.

Reads a pcapng file containing Dash-format UDP packets (331 bytes) and
classifies the capture into:

  - segment: contiguous run of IsRaceOn=1 packets (player in control)
  - gap:     contiguous run of IsRaceOn=0 packets (cinematic, pause, rewind, etc.)

Gap classification rules (using deltas across the gap):
  - session_reset:   end-of-segment dist counter is "high", start-of-next is -1944 sentinel
  - rewind:          start-of-next dist < end-of-prev dist (player went backward)
  - cinematic:       end-of-prev pos teleports >50m AND dist counter increased
                     (game played a sequence in which the car kept driving)
  - pause:           teleport <5m AND dist delta ~0 (pure menu pause/unpause)
  - unknown:         anything else

In-segment events kept (strong-signal only):
  - contact:  AccelMag spike >5G (high-confidence wall/car contact)

Off-track / dirty-lap detection is INTENTIONALLY OMITTED. Surface rumble fires
on curbs and is too noisy for a reliable detector. Use lap-time invalidation
or a future tire-slip + position-against-track-edges detector instead.
"""

import json
import math
import os
import struct
import sys
from pathlib import Path

# ---- pcapng reader (stdlib only) ----

PCAPNG_BLOCK_EPB = 0x06
PCAPNG_BLOCK_IDB = 0x01

def iter_pcapng(path):
    """Yield (capture_idx, ts_ns, raw_frame_bytes) for every packet."""
    f = open(path, "rb")
    idx = -1
    ts_resol_ns_per_unit = 1  # default microseconds = 1000 ns/unit; updated when IDB seen
    while True:
        hdr = f.read(8)
        if len(hdr) < 8:
            break
        bt, blen = struct.unpack("<II", hdr)
        body = f.read(blen - 12)
        f.read(4)  # trailing block length
        if bt == PCAPNG_BLOCK_IDB and len(body) >= 8:
            opts = body[8:]
            o = 0
            while o + 4 <= len(opts):
                code, length = struct.unpack("<HH", opts[o : o + 4])
                o += 4
                data = opts[o : o + length]
                o = (o + length + 3) & ~3
                if code == 0:
                    break
                if code == 9 and length >= 1:
                    val = data[0]
                    # if_tsresol: high bit set = base 2; else base 10. value is exponent.
                    if val & 0x80:
                        ts_resol_ns_per_unit = int(1e9 / (2 ** (val & 0x7F)))
                    else:
                        ts_resol_ns_per_unit = int(1e9 / (10**val))
        elif bt == PCAPNG_BLOCK_EPB and len(body) >= 20:
            _iface, ts_h, ts_l, cap_len, _orig = struct.unpack("<IIIII", body[:20])
            ts_units = (ts_h << 32) | ts_l
            ts_ns = ts_units * ts_resol_ns_per_unit
            frame = body[20 : 20 + cap_len]
            idx += 1
            yield idx, ts_ns, frame


def extract_forza_payload(frame, want_dport=5300):
    """For NULL/loopback linktype 0, frame = 4-byte family + IPv4. Returns 331-byte payload or None."""
    if len(frame) < 4 + 20 + 8:
        return None
    ip = frame[4:]
    if ip[9] != 17:  # not UDP
        return None
    ihl = (ip[0] & 0x0F) * 4
    udp = ip[ihl:]
    if len(udp) < 8:
        return None
    dport = struct.unpack("!H", udp[2:4])[0]
    if dport != want_dport:
        return None
    payload = udp[8:]
    if len(payload) != 331:
        return None
    return payload


# ---- Forza Dash decoder (FM 2023 layout) ----

def decode_dash(p):
    return {
        "IsRaceOn": struct.unpack("<i", p[0:4])[0],
        "TimestampMs": struct.unpack("<I", p[4:8])[0],
        "AccelX": struct.unpack("<f", p[20:24])[0],
        "AccelY": struct.unpack("<f", p[24:28])[0],
        "AccelZ": struct.unpack("<f", p[28:32])[0],
        "CarOrdinal": struct.unpack("<i", p[212:216])[0],
        "PosX": struct.unpack("<f", p[232:236])[0],
        "PosY": struct.unpack("<f", p[236:240])[0],
        "PosZ": struct.unpack("<f", p[240:244])[0],
        "Speed": struct.unpack("<f", p[244:248])[0],
        "DistanceTraveled": struct.unpack("<f", p[280:284])[0],
        "BestLap": struct.unpack("<f", p[284:288])[0],
        "LastLap": struct.unpack("<f", p[288:292])[0],
        "CurrentLap": struct.unpack("<f", p[292:296])[0],
        "RaceTime": struct.unpack("<f", p[296:300])[0],
        "LapNum": struct.unpack("<H", p[300:302])[0],
        "TrackOrdinal": struct.unpack("<i", p[327:331])[0],
    }


# ---- Gap classifier ----

DIST_SENTINEL = -1944.0
PAUSE_TELEPORT_M = 5.0
CINEMATIC_TELEPORT_M = 50.0
CINEMATIC_DIST_DELTA_M = 25.0  # game-time driving during the gap
CONTACT_G = 5.0


def classify_gap(prev_end, next_start):
    """prev_end and next_start are decoded packets (last of prev seg, first of next seg)."""
    dx = next_start["PosX"] - prev_end["PosX"]
    dy = next_start["PosY"] - prev_end["PosY"]
    dz = next_start["PosZ"] - prev_end["PosZ"]
    teleport_m = math.sqrt(dx * dx + dy * dy + dz * dz)
    dist_delta = next_start["DistanceTraveled"] - prev_end["DistanceTraveled"]

    # session_reset: dist counter resets to sentinel
    if abs(next_start["DistanceTraveled"] - DIST_SENTINEL) < 1.0:
        kind = "session_reset"
    # rewind: distance counter moved backward
    elif dist_delta < -10.0:
        kind = "rewind"
    # cinematic: significant teleport AND distance accumulated during the gap
    elif teleport_m > CINEMATIC_TELEPORT_M and dist_delta > CINEMATIC_DIST_DELTA_M:
        kind = "cinematic"
    # pause/unpause: nothing meaningful changed
    elif teleport_m < PAUSE_TELEPORT_M and abs(dist_delta) < 5.0:
        kind = "pause"
    else:
        kind = "unknown"

    return {
        "kind": kind,
        "teleport_m": round(teleport_m, 2),
        "dist_delta_m": round(dist_delta, 2),
    }


def detect_contacts(records):
    """Return list of contact events (peak G within each cluster, debounced 0.5s)."""
    out = []
    cluster = None  # (peak_idx, peak_g)
    last_above_idx = -10**9
    for idx, _ts, d in records:
        mag = math.sqrt(d["AccelX"] ** 2 + d["AccelY"] ** 2 + d["AccelZ"] ** 2) / 9.81
        if mag >= CONTACT_G:
            if cluster is None or idx - last_above_idx > 30:
                if cluster is not None:
                    out.append({"capture_idx": cluster[0], "magnitude_g": round(cluster[1], 2)})
                cluster = (idx, mag)
            elif mag > cluster[1]:
                cluster = (idx, mag)
            last_above_idx = idx
    if cluster is not None:
        out.append({"capture_idx": cluster[0], "magnitude_g": round(cluster[1], 2)})
    return out


# ---- main ----

def analyze(pcapng_path):
    records = []  # all Forza packets, decoded: (capture_idx, ts_ns, decoded)
    for cap_idx, ts_ns, frame in iter_pcapng(pcapng_path):
        payload = extract_forza_payload(frame, want_dport=5300)
        if payload is None:
            continue
        records.append((cap_idx, ts_ns, decode_dash(payload)))

    # Re-index packets within the dst-port-5300 stream (this is the per-stream index
    # that scenarios.json will reference, NOT the master capture index).
    forza_idx_by_cap_idx = {cap: i for i, (cap, _ts, _d) in enumerate(records)}
    records = [(i, ts, d) for i, (_cap, ts, d) in enumerate(records)]

    if not records:
        raise SystemExit("no Forza Dash packets found")

    # IsRaceOn=0 packets have zeroed fields; pull metadata from a real packet.
    first_real = next(r for r in records if r[2]["IsRaceOn"])
    car = first_real[2]["CarOrdinal"]
    track = max((r[2]["TrackOrdinal"] for r in records), default=0)

    # Find segments (IsRaceOn=1 runs) and gaps (IsRaceOn=0 runs) over the
    # range from the first IsRaceOn=1 packet to the last IsRaceOn=1 packet.
    first_in = next((i for i, r in enumerate(records) if r[2]["IsRaceOn"]), None)
    last_in = next((i for i in range(len(records) - 1, -1, -1) if records[i][2]["IsRaceOn"]), None)
    if first_in is None:
        raise SystemExit("no IsRaceOn=1 packets")

    segments = []
    cur = []
    for r in records[first_in : last_in + 1]:
        if r[2]["IsRaceOn"]:
            cur.append(r)
        elif cur:
            segments.append(cur)
            cur = []
    if cur:
        segments.append(cur)

    # Build a single timeline of segments + gaps in capture order
    timeline = []
    for i, seg in enumerate(segments):
        # lap structuring inside the segment
        laps = []
        cur_lap_num = seg[0][2]["LapNum"]
        lap_start = seg[0]
        for r in seg[1:]:
            if r[2]["LapNum"] != cur_lap_num:
                laps.append(
                    {
                        "lap_num": int(cur_lap_num),
                        "first_idx": lap_start[0],
                        "last_idx": r[0] - 1,
                        "duration_s": round((r[1] - lap_start[1]) / 1e9, 3),
                    }
                )
                cur_lap_num = r[2]["LapNum"]
                lap_start = r
        laps.append(
            {
                "lap_num": int(cur_lap_num),
                "first_idx": lap_start[0],
                "last_idx": seg[-1][0],
                "duration_s": round((seg[-1][1] - lap_start[1]) / 1e9, 3),
            }
        )

        timeline.append(
            {
                "type": "segment",
                "id": f"seg{i:02d}",
                "first_idx": seg[0][0],
                "last_idx": seg[-1][0],
                "packet_count": len(seg),
                "duration_s": round((seg[-1][1] - seg[0][1]) / 1e9, 3),
                "lap_nums": sorted({l["lap_num"] for l in laps}),
                "laps": laps,
                "start_pos": [round(seg[0][2]["PosX"], 2), round(seg[0][2]["PosY"], 2), round(seg[0][2]["PosZ"], 2)],
                "end_pos": [round(seg[-1][2]["PosX"], 2), round(seg[-1][2]["PosY"], 2), round(seg[-1][2]["PosZ"], 2)],
                "start_dist_m": round(seg[0][2]["DistanceTraveled"], 2),
                "end_dist_m": round(seg[-1][2]["DistanceTraveled"], 2),
                "max_speed_m_s": round(max(r[2]["Speed"] for r in seg), 2),
            }
        )
        if i + 1 < len(segments):
            nxt = segments[i + 1]
            cls = classify_gap(seg[-1][2], nxt[0][2])
            timeline.append(
                {
                    "type": "gap",
                    "id": f"gap{i:02d}",
                    "after_segment": f"seg{i:02d}",
                    "before_segment": f"seg{i+1:02d}",
                    "first_idx": seg[-1][0] + 1,
                    "last_idx": nxt[0][0] - 1,
                    "duration_s": round((records[nxt[0][0]][1] - records[seg[-1][0]][1]) / 1e9, 3),
                    "classification": cls["kind"],
                    "teleport_m": cls["teleport_m"],
                    "dist_delta_m": cls["dist_delta_m"],
                }
            )

    contacts = detect_contacts(records)

    # Tag each contact with its enclosing segment (if any)
    for c in contacts:
        for entry in timeline:
            if entry["type"] == "segment" and entry["first_idx"] <= c["capture_idx"] <= entry["last_idx"]:
                c["segment_id"] = entry["id"]
                # find lap
                for l in entry["laps"]:
                    if l["first_idx"] <= c["capture_idx"] <= l["last_idx"]:
                        c["lap_num"] = l["lap_num"]
                        break
                break

    return {
        "capture": {
            "source_filename": Path(pcapng_path).name,
            "udp_dst_port": 5300,
            "packet_format": "forza_motorsport_dash_v_fm2023",
            "packet_size_bytes": 331,
            "total_forza_packets": len(records),
            "trim_first_idx": records[first_in][0],
            "trim_last_idx": records[last_in][0],
            "car_ordinal": int(car),
            "track_ordinal": int(track),
            "car_human": "2006 Audi RS4 (ord 419)",
            "track_human": "Brands Hatch Indy Circuit (ord 861)",
        },
        "gap_classification_rules": {
            "session_reset": "next_segment.DistanceTraveled == -1944.0 sentinel",
            "rewind": "DistanceTraveled decreased by >10m across the gap",
            "cinematic": "position teleport >50m AND DistanceTraveled increased >25m (game drove the car during the gap)",
            "pause": "position teleport <5m AND |DistanceTraveled delta| <5m",
            "unknown": "did not match any rule (investigate)",
        },
        "in_segment_event_rules": {
            "contact": f"sqrt(AccelX^2 + AccelY^2 + AccelZ^2) / 9.81 >= {CONTACT_G}G, debounced by 0.5s",
        },
        "timeline": timeline,
        "contacts": contacts,
    }


def main():
    src = sys.argv[1] if len(sys.argv) > 1 else os.path.expanduser("~/fm-training.pcapng")
    out_dir = Path(os.path.expanduser("~/fm-analysis"))
    out_dir.mkdir(exist_ok=True)
    print(f"reading {src} ...")
    report = analyze(src)
    out = out_dir / "analysis-report.json"
    out.write_text(json.dumps(report, indent=2))
    print(f"saved {out}")
    # Brief summary
    cap = report["capture"]
    print(f"\n  car ord={cap['car_ordinal']}  track ord={cap['track_ordinal']}")
    print(f"  forza packets: {cap['total_forza_packets']}  trim range: {cap['trim_first_idx']}..{cap['trim_last_idx']}")
    print(f"\n  timeline:")
    for e in report["timeline"]:
        if e["type"] == "segment":
            laps = ",".join(str(n) for n in e["lap_nums"])
            print(f"    {e['id']}  pkts {e['first_idx']:>5}..{e['last_idx']:<5} ({e['packet_count']:>5} pkts, {e['duration_s']:>6.1f}s)  LapNums=[{laps}]  maxSpd={e['max_speed_m_s']:.1f} m/s")
        else:
            print(f"    {e['id']}                         ({e['duration_s']:>6.1f}s)  {e['classification']}  teleport={e['teleport_m']}m dist_delta={e['dist_delta_m']}m")
    print(f"\n  contacts: {len(report['contacts'])}")
    for c in report["contacts"]:
        print(f"    pkt {c['capture_idx']:>5}  {c['magnitude_g']:>5.1f}G  {c.get('segment_id','?')}/lap{c.get('lap_num','?')}")


if __name__ == "__main__":
    main()
