use std::{
    io::ErrorKind,
    time::{Duration, Instant},
};

use bytemuck::{Pod, Zeroable};
use tokio::{
    net::UdpSocket,
    sync::{broadcast, watch},
};
use tracing::{debug, info, warn};

pub const SLED_PACKET_LEN: usize = 232;
pub const DASH_PACKET_LEN: usize = 311;
pub const FM2023_DASH_PACKET_LEN: usize = 331;

/// Receive buffer sized well above any known Forza datagram.
///
/// On Windows, `recv_from` returns `WSAEMSGSIZE` (os error 10040) if the
/// incoming datagram exceeds the supplied buffer; it does **not** silently
/// truncate as Linux does.  Using a buffer larger than any plausible Forza
/// packet avoids that error and remains forward-compatible if a future game
/// revision adds more fields.
const UDP_RECV_BUFFER_LEN: usize = 2048;

#[derive(Debug, Clone, PartialEq)]
pub enum TelemetryPacket {
    Sled(SledPacket),
    Dash(DashPacket),
}

impl TelemetryPacket {
    fn variant_name(&self) -> &'static str {
        match self {
            Self::Sled(_) => "sled",
            Self::Dash(_) => "dash",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SledPacket {
    pub is_race_on: i32,
    pub timestamp_ms: u32,
    pub engine_max_rpm: f32,
    pub engine_idle_rpm: f32,
    pub current_engine_rpm: f32,
    pub acceleration_x: f32,
    pub acceleration_y: f32,
    pub acceleration_z: f32,
    pub velocity_x: f32,
    pub velocity_y: f32,
    pub velocity_z: f32,
    pub angular_velocity_x: f32,
    pub angular_velocity_y: f32,
    pub angular_velocity_z: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    pub normalized_suspension_travel_front_left: f32,
    pub normalized_suspension_travel_front_right: f32,
    pub normalized_suspension_travel_rear_left: f32,
    pub normalized_suspension_travel_rear_right: f32,
    pub tire_slip_ratio_front_left: f32,
    pub tire_slip_ratio_front_right: f32,
    pub tire_slip_ratio_rear_left: f32,
    pub tire_slip_ratio_rear_right: f32,
    pub wheel_rotation_speed_front_left: f32,
    pub wheel_rotation_speed_front_right: f32,
    pub wheel_rotation_speed_rear_left: f32,
    pub wheel_rotation_speed_rear_right: f32,
    pub wheel_on_rumble_strip_front_left: i32,
    pub wheel_on_rumble_strip_front_right: i32,
    pub wheel_on_rumble_strip_rear_left: i32,
    pub wheel_on_rumble_strip_rear_right: i32,
    pub wheel_in_puddle_depth_front_left: f32,
    pub wheel_in_puddle_depth_front_right: f32,
    pub wheel_in_puddle_depth_rear_left: f32,
    pub wheel_in_puddle_depth_rear_right: f32,
    pub surface_rumble_front_left: f32,
    pub surface_rumble_front_right: f32,
    pub surface_rumble_rear_left: f32,
    pub surface_rumble_rear_right: f32,
    pub tire_slip_angle_front_left: f32,
    pub tire_slip_angle_front_right: f32,
    pub tire_slip_angle_rear_left: f32,
    pub tire_slip_angle_rear_right: f32,
    pub tire_combined_slip_front_left: f32,
    pub tire_combined_slip_front_right: f32,
    pub tire_combined_slip_rear_left: f32,
    pub tire_combined_slip_rear_right: f32,
    pub suspension_travel_meters_front_left: f32,
    pub suspension_travel_meters_front_right: f32,
    pub suspension_travel_meters_rear_left: f32,
    pub suspension_travel_meters_rear_right: f32,
    pub car_ordinal: i32,
    pub car_class: i32,
    pub car_performance_index: i32,
    pub drivetrain_type: i32,
    pub num_cylinders: i32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DashPacket {
    pub sled: SledPacket,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub speed: f32,
    pub power: f32,
    pub torque: f32,
    pub tire_temp_front_left: f32,
    pub tire_temp_front_right: f32,
    pub tire_temp_rear_left: f32,
    pub tire_temp_rear_right: f32,
    pub boost: f32,
    pub fuel: f32,
    pub distance_traveled: f32,
    pub best_lap: f32,
    pub last_lap: f32,
    pub current_lap: f32,
    pub current_race_time: f32,
    pub lap_number: u16,
    pub race_position: u8,
    pub accel: u8,
    pub brake: u8,
    pub clutch: u8,
    pub hand_brake: u8,
    /// Raw gear byte from Forza: `0` = Reverse, `1..=10` = forward gears, `11` = Neutral.
    pub gear: u8,
    pub steer: i8,
    pub normalized_driving_line: i8,
    pub normalized_ai_brake_difference: i8,
    /// Tire wear fraction `[0.0, 1.0]`.  Present only in FM 2023 331-byte
    /// Dash packets; `None` for legacy 311-byte packets.
    pub tire_wear_front_left: Option<f32>,
    /// See [`DashPacket::tire_wear_front_left`].
    pub tire_wear_front_right: Option<f32>,
    /// See [`DashPacket::tire_wear_front_left`].
    pub tire_wear_rear_left: Option<f32>,
    /// See [`DashPacket::tire_wear_front_left`].
    pub tire_wear_rear_right: Option<f32>,
    /// Track identifier.  Present only in FM 2023 331-byte Dash packets;
    /// `None` for legacy 311-byte packets.
    pub track_ordinal: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseTelemetryError {
    InvalidLength(usize),
    NonFiniteFloat(&'static str),
}

impl std::fmt::Display for ParseTelemetryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidLength(length) => write!(f, "invalid telemetry packet length: {length}"),
            Self::NonFiniteFloat(field) => write!(f, "non-finite float in field {field}"),
        }
    }
}

impl std::error::Error for ParseTelemetryError {}

#[repr(C, packed)]
#[derive(Clone, Copy, Zeroable, Pod)]
struct RawSledPacket {
    is_race_on: i32,
    timestamp_ms: u32,
    engine_max_rpm: f32,
    engine_idle_rpm: f32,
    current_engine_rpm: f32,
    acceleration_x: f32,
    acceleration_y: f32,
    acceleration_z: f32,
    velocity_x: f32,
    velocity_y: f32,
    velocity_z: f32,
    angular_velocity_x: f32,
    angular_velocity_y: f32,
    angular_velocity_z: f32,
    yaw: f32,
    pitch: f32,
    roll: f32,
    normalized_suspension_travel_front_left: f32,
    normalized_suspension_travel_front_right: f32,
    normalized_suspension_travel_rear_left: f32,
    normalized_suspension_travel_rear_right: f32,
    tire_slip_ratio_front_left: f32,
    tire_slip_ratio_front_right: f32,
    tire_slip_ratio_rear_left: f32,
    tire_slip_ratio_rear_right: f32,
    wheel_rotation_speed_front_left: f32,
    wheel_rotation_speed_front_right: f32,
    wheel_rotation_speed_rear_left: f32,
    wheel_rotation_speed_rear_right: f32,
    wheel_on_rumble_strip_front_left: i32,
    wheel_on_rumble_strip_front_right: i32,
    wheel_on_rumble_strip_rear_left: i32,
    wheel_on_rumble_strip_rear_right: i32,
    wheel_in_puddle_depth_front_left: f32,
    wheel_in_puddle_depth_front_right: f32,
    wheel_in_puddle_depth_rear_left: f32,
    wheel_in_puddle_depth_rear_right: f32,
    surface_rumble_front_left: f32,
    surface_rumble_front_right: f32,
    surface_rumble_rear_left: f32,
    surface_rumble_rear_right: f32,
    tire_slip_angle_front_left: f32,
    tire_slip_angle_front_right: f32,
    tire_slip_angle_rear_left: f32,
    tire_slip_angle_rear_right: f32,
    tire_combined_slip_front_left: f32,
    tire_combined_slip_front_right: f32,
    tire_combined_slip_rear_left: f32,
    tire_combined_slip_rear_right: f32,
    suspension_travel_meters_front_left: f32,
    suspension_travel_meters_front_right: f32,
    suspension_travel_meters_rear_left: f32,
    suspension_travel_meters_rear_right: f32,
    car_ordinal: i32,
    car_class: i32,
    car_performance_index: i32,
    drivetrain_type: i32,
    num_cylinders: i32,
}

#[repr(C, packed)]
#[derive(Clone, Copy, Zeroable, Pod)]
struct RawDashPacket {
    sled: RawSledPacket,
    position_x: f32,
    position_y: f32,
    position_z: f32,
    speed: f32,
    power: f32,
    torque: f32,
    tire_temp_front_left: f32,
    tire_temp_front_right: f32,
    tire_temp_rear_left: f32,
    tire_temp_rear_right: f32,
    boost: f32,
    fuel: f32,
    distance_traveled: f32,
    best_lap: f32,
    last_lap: f32,
    current_lap: f32,
    current_race_time: f32,
    lap_number: u16,
    race_position: u8,
    accel: u8,
    brake: u8,
    clutch: u8,
    hand_brake: u8,
    gear: u8,
    steer: i8,
    normalized_driving_line: i8,
    normalized_ai_brake_difference: i8,
}

/// FM 2023 Dash packet — FM7 layout (`RawDashPacket`) followed by a 20-byte
/// trailer: four `f32` tire-wear fractions and one `i32` track ordinal.
///
/// Layout verified against a Wireshark capture of `127.0.0.1:5300` traffic
/// from Forza Motorsport (2023); see `sidecar/tests/fixtures/lap_validity/README.md`.
#[repr(C, packed)]
#[derive(Clone, Copy, Zeroable, Pod)]
struct RawFm2023DashPacket {
    dash: RawDashPacket,
    tire_wear_front_left: f32,
    tire_wear_front_right: f32,
    tire_wear_rear_left: f32,
    tire_wear_rear_right: f32,
    track_ordinal: i32,
}

pub fn parse_telemetry_packet(bytes: &[u8]) -> Result<TelemetryPacket, ParseTelemetryError> {
    match bytes.len() {
        SLED_PACKET_LEN => {
            let raw = bytemuck::try_from_bytes::<RawSledPacket>(bytes)
                .map_err(|_| ParseTelemetryError::InvalidLength(bytes.len()))?;
            let packet = SledPacket::from_raw(raw);
            validate_sled_packet(&packet)?;
            Ok(TelemetryPacket::Sled(packet))
        }
        DASH_PACKET_LEN => {
            let raw = bytemuck::try_from_bytes::<RawDashPacket>(bytes)
                .map_err(|_| ParseTelemetryError::InvalidLength(bytes.len()))?;
            let packet = DashPacket::from_raw(raw);
            validate_sled_packet(&packet.sled)?;
            validate_finite(packet.speed, "speed")?;
            Ok(TelemetryPacket::Dash(packet))
        }
        FM2023_DASH_PACKET_LEN => {
            let raw = bytemuck::try_from_bytes::<RawFm2023DashPacket>(bytes)
                .map_err(|_| ParseTelemetryError::InvalidLength(bytes.len()))?;
            let packet = DashPacket::from_fm2023_raw(raw);
            validate_sled_packet(&packet.sled)?;
            validate_finite(packet.speed, "speed")?;
            // Validate the FM 2023 trailer fields.
            if let Some(v) = packet.tire_wear_front_left {
                validate_finite(v, "tire_wear_front_left")?;
            }
            if let Some(v) = packet.tire_wear_front_right {
                validate_finite(v, "tire_wear_front_right")?;
            }
            if let Some(v) = packet.tire_wear_rear_left {
                validate_finite(v, "tire_wear_rear_left")?;
            }
            if let Some(v) = packet.tire_wear_rear_right {
                validate_finite(v, "tire_wear_rear_right")?;
            }
            Ok(TelemetryPacket::Dash(packet))
        }
        _ => Err(ParseTelemetryError::InvalidLength(bytes.len())),
    }
}

pub async fn udp_listener_loop(
    socket: UdpSocket,
    latest_packet_tx: watch::Sender<Option<TelemetryPacket>>,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> anyhow::Result<()> {
    let mut packet_buffer = [0u8; UDP_RECV_BUFFER_LEN];
    let mut packet_stats = PacketStats::new();
    let mut packet_rate_tick = tokio::time::interval(Duration::from_secs(60));
    packet_rate_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => {
                let drained_packets = drain_socket(&socket, &mut packet_buffer, &latest_packet_tx, &mut packet_stats);
                info!(drained_packets, "udp telemetry listener drained and shutting down");
                break;
            }
            _ = packet_rate_tick.tick() => {
                packet_stats.log_rate_if_started();
            }
            recv = socket.recv_from(&mut packet_buffer) => {
                match recv {
                    Ok((len, _peer)) => {
                        handle_packet(&packet_buffer[..len], &latest_packet_tx, &mut packet_stats);
                    }
                    Err(err) => warn!(%err, "failed to read udp telemetry packet"),
                }
            }
        }
    }

    Ok(())
}

fn drain_socket(
    socket: &UdpSocket,
    packet_buffer: &mut [u8; UDP_RECV_BUFFER_LEN],
    latest_packet_tx: &watch::Sender<Option<TelemetryPacket>>,
    packet_stats: &mut PacketStats,
) -> usize {
    let mut drained = 0usize;
    loop {
        match socket.try_recv_from(packet_buffer) {
            Ok((len, _peer)) => {
                drained += 1;
                handle_packet(&packet_buffer[..len], latest_packet_tx, packet_stats);
            }
            Err(err) if err.kind() == ErrorKind::WouldBlock => break,
            Err(err) => {
                warn!(%err, "failed while draining udp telemetry socket");
                break;
            }
        }
    }
    drained
}

fn handle_packet(
    packet_bytes: &[u8],
    latest_packet_tx: &watch::Sender<Option<TelemetryPacket>>,
    packet_stats: &mut PacketStats,
) {
    match parse_telemetry_packet(packet_bytes) {
        Ok(packet) => {
            packet_stats.record_valid_packet(&packet);
            let _ = latest_packet_tx.send_replace(Some(packet));
        }
        Err(ParseTelemetryError::InvalidLength(length)) => packet_stats.warn_invalid_length(length),
        Err(ParseTelemetryError::NonFiniteFloat(field)) => {
            debug!(
                field,
                "dropping packet due to non-finite float in validated field"
            );
        }
    }
}

struct PacketStats {
    total_packets: u64,
    interval_packets: u64,
    first_packet_logged: bool,
    next_invalid_length_warn_at: Instant,
}

impl PacketStats {
    fn new() -> Self {
        Self {
            total_packets: 0,
            interval_packets: 0,
            first_packet_logged: false,
            next_invalid_length_warn_at: Instant::now(),
        }
    }

    fn record_valid_packet(&mut self, packet: &TelemetryPacket) {
        self.total_packets += 1;
        self.interval_packets += 1;
        if !self.first_packet_logged {
            self.first_packet_logged = true;
            info!(
                packet_variant = packet.variant_name(),
                "first telemetry packet received"
            );
        }
    }

    fn warn_invalid_length(&mut self, length: usize) {
        let now = Instant::now();
        if now >= self.next_invalid_length_warn_at {
            self.next_invalid_length_warn_at = now + Duration::from_secs(5);
            warn!(length, "dropping udp telemetry packet with invalid length");
        }
    }

    fn log_rate_if_started(&mut self) {
        if !self.first_packet_logged {
            return;
        }
        let packets_in_window = self.interval_packets;
        self.interval_packets = 0;
        info!(
            packets_in_window,
            packets_per_second = packets_in_window as f64 / 60.0,
            total_packets = self.total_packets,
            "telemetry packet rate"
        );
    }
}

macro_rules! read_unaligned_field {
    ($ptr:expr, $field:ident) => {
        // SAFETY: `$ptr` comes from a reference returned by `bytemuck::try_from_bytes` for a
        // `Pod` type, so the pointed-to bytes have a valid bit pattern for the destination type.
        // Packet structs are `repr(C, packed)`, so fields can be unaligned and must be read with
        // `read_unaligned`.
        unsafe { std::ptr::addr_of!((*$ptr).$field).read_unaligned() }
    };
}

impl SledPacket {
    fn from_raw(raw: &RawSledPacket) -> Self {
        let ptr = raw as *const RawSledPacket;
        Self {
            is_race_on: read_unaligned_field!(ptr, is_race_on),
            timestamp_ms: read_unaligned_field!(ptr, timestamp_ms),
            engine_max_rpm: read_unaligned_field!(ptr, engine_max_rpm),
            engine_idle_rpm: read_unaligned_field!(ptr, engine_idle_rpm),
            current_engine_rpm: read_unaligned_field!(ptr, current_engine_rpm),
            acceleration_x: read_unaligned_field!(ptr, acceleration_x),
            acceleration_y: read_unaligned_field!(ptr, acceleration_y),
            acceleration_z: read_unaligned_field!(ptr, acceleration_z),
            velocity_x: read_unaligned_field!(ptr, velocity_x),
            velocity_y: read_unaligned_field!(ptr, velocity_y),
            velocity_z: read_unaligned_field!(ptr, velocity_z),
            angular_velocity_x: read_unaligned_field!(ptr, angular_velocity_x),
            angular_velocity_y: read_unaligned_field!(ptr, angular_velocity_y),
            angular_velocity_z: read_unaligned_field!(ptr, angular_velocity_z),
            yaw: read_unaligned_field!(ptr, yaw),
            pitch: read_unaligned_field!(ptr, pitch),
            roll: read_unaligned_field!(ptr, roll),
            normalized_suspension_travel_front_left: read_unaligned_field!(
                ptr,
                normalized_suspension_travel_front_left
            ),
            normalized_suspension_travel_front_right: read_unaligned_field!(
                ptr,
                normalized_suspension_travel_front_right
            ),
            normalized_suspension_travel_rear_left: read_unaligned_field!(
                ptr,
                normalized_suspension_travel_rear_left
            ),
            normalized_suspension_travel_rear_right: read_unaligned_field!(
                ptr,
                normalized_suspension_travel_rear_right
            ),
            tire_slip_ratio_front_left: read_unaligned_field!(ptr, tire_slip_ratio_front_left),
            tire_slip_ratio_front_right: read_unaligned_field!(ptr, tire_slip_ratio_front_right),
            tire_slip_ratio_rear_left: read_unaligned_field!(ptr, tire_slip_ratio_rear_left),
            tire_slip_ratio_rear_right: read_unaligned_field!(ptr, tire_slip_ratio_rear_right),
            wheel_rotation_speed_front_left: read_unaligned_field!(
                ptr,
                wheel_rotation_speed_front_left
            ),
            wheel_rotation_speed_front_right: read_unaligned_field!(
                ptr,
                wheel_rotation_speed_front_right
            ),
            wheel_rotation_speed_rear_left: read_unaligned_field!(
                ptr,
                wheel_rotation_speed_rear_left
            ),
            wheel_rotation_speed_rear_right: read_unaligned_field!(
                ptr,
                wheel_rotation_speed_rear_right
            ),
            wheel_on_rumble_strip_front_left: read_unaligned_field!(
                ptr,
                wheel_on_rumble_strip_front_left
            ),
            wheel_on_rumble_strip_front_right: read_unaligned_field!(
                ptr,
                wheel_on_rumble_strip_front_right
            ),
            wheel_on_rumble_strip_rear_left: read_unaligned_field!(
                ptr,
                wheel_on_rumble_strip_rear_left
            ),
            wheel_on_rumble_strip_rear_right: read_unaligned_field!(
                ptr,
                wheel_on_rumble_strip_rear_right
            ),
            wheel_in_puddle_depth_front_left: read_unaligned_field!(
                ptr,
                wheel_in_puddle_depth_front_left
            ),
            wheel_in_puddle_depth_front_right: read_unaligned_field!(
                ptr,
                wheel_in_puddle_depth_front_right
            ),
            wheel_in_puddle_depth_rear_left: read_unaligned_field!(
                ptr,
                wheel_in_puddle_depth_rear_left
            ),
            wheel_in_puddle_depth_rear_right: read_unaligned_field!(
                ptr,
                wheel_in_puddle_depth_rear_right
            ),
            surface_rumble_front_left: read_unaligned_field!(ptr, surface_rumble_front_left),
            surface_rumble_front_right: read_unaligned_field!(ptr, surface_rumble_front_right),
            surface_rumble_rear_left: read_unaligned_field!(ptr, surface_rumble_rear_left),
            surface_rumble_rear_right: read_unaligned_field!(ptr, surface_rumble_rear_right),
            tire_slip_angle_front_left: read_unaligned_field!(ptr, tire_slip_angle_front_left),
            tire_slip_angle_front_right: read_unaligned_field!(ptr, tire_slip_angle_front_right),
            tire_slip_angle_rear_left: read_unaligned_field!(ptr, tire_slip_angle_rear_left),
            tire_slip_angle_rear_right: read_unaligned_field!(ptr, tire_slip_angle_rear_right),
            tire_combined_slip_front_left: read_unaligned_field!(
                ptr,
                tire_combined_slip_front_left
            ),
            tire_combined_slip_front_right: read_unaligned_field!(
                ptr,
                tire_combined_slip_front_right
            ),
            tire_combined_slip_rear_left: read_unaligned_field!(ptr, tire_combined_slip_rear_left),
            tire_combined_slip_rear_right: read_unaligned_field!(
                ptr,
                tire_combined_slip_rear_right
            ),
            suspension_travel_meters_front_left: read_unaligned_field!(
                ptr,
                suspension_travel_meters_front_left
            ),
            suspension_travel_meters_front_right: read_unaligned_field!(
                ptr,
                suspension_travel_meters_front_right
            ),
            suspension_travel_meters_rear_left: read_unaligned_field!(
                ptr,
                suspension_travel_meters_rear_left
            ),
            suspension_travel_meters_rear_right: read_unaligned_field!(
                ptr,
                suspension_travel_meters_rear_right
            ),
            car_ordinal: read_unaligned_field!(ptr, car_ordinal),
            car_class: read_unaligned_field!(ptr, car_class),
            car_performance_index: read_unaligned_field!(ptr, car_performance_index),
            drivetrain_type: read_unaligned_field!(ptr, drivetrain_type),
            num_cylinders: read_unaligned_field!(ptr, num_cylinders),
        }
    }
}

impl DashPacket {
    fn from_raw(raw: &RawDashPacket) -> Self {
        let ptr = raw as *const RawDashPacket;
        let raw_sled = read_unaligned_field!(ptr, sled);
        Self {
            sled: SledPacket::from_raw(&raw_sled),
            position_x: read_unaligned_field!(ptr, position_x),
            position_y: read_unaligned_field!(ptr, position_y),
            position_z: read_unaligned_field!(ptr, position_z),
            speed: read_unaligned_field!(ptr, speed),
            power: read_unaligned_field!(ptr, power),
            torque: read_unaligned_field!(ptr, torque),
            tire_temp_front_left: read_unaligned_field!(ptr, tire_temp_front_left),
            tire_temp_front_right: read_unaligned_field!(ptr, tire_temp_front_right),
            tire_temp_rear_left: read_unaligned_field!(ptr, tire_temp_rear_left),
            tire_temp_rear_right: read_unaligned_field!(ptr, tire_temp_rear_right),
            boost: read_unaligned_field!(ptr, boost),
            fuel: read_unaligned_field!(ptr, fuel),
            distance_traveled: read_unaligned_field!(ptr, distance_traveled),
            best_lap: read_unaligned_field!(ptr, best_lap),
            last_lap: read_unaligned_field!(ptr, last_lap),
            current_lap: read_unaligned_field!(ptr, current_lap),
            current_race_time: read_unaligned_field!(ptr, current_race_time),
            lap_number: read_unaligned_field!(ptr, lap_number),
            race_position: read_unaligned_field!(ptr, race_position),
            accel: read_unaligned_field!(ptr, accel),
            brake: read_unaligned_field!(ptr, brake),
            clutch: read_unaligned_field!(ptr, clutch),
            hand_brake: read_unaligned_field!(ptr, hand_brake),
            gear: read_unaligned_field!(ptr, gear),
            steer: read_unaligned_field!(ptr, steer),
            normalized_driving_line: read_unaligned_field!(ptr, normalized_driving_line),
            normalized_ai_brake_difference: read_unaligned_field!(
                ptr,
                normalized_ai_brake_difference
            ),
            // Legacy 311-byte packet: FM 2023 trailer fields not present.
            tire_wear_front_left: None,
            tire_wear_front_right: None,
            tire_wear_rear_left: None,
            tire_wear_rear_right: None,
            track_ordinal: None,
        }
    }

    fn from_fm2023_raw(raw: &RawFm2023DashPacket) -> Self {
        let ptr = raw as *const RawFm2023DashPacket;
        let raw_dash = read_unaligned_field!(ptr, dash);
        let mut packet = Self::from_raw(&raw_dash);
        packet.tire_wear_front_left = Some(read_unaligned_field!(ptr, tire_wear_front_left));
        packet.tire_wear_front_right = Some(read_unaligned_field!(ptr, tire_wear_front_right));
        packet.tire_wear_rear_left = Some(read_unaligned_field!(ptr, tire_wear_rear_left));
        packet.tire_wear_rear_right = Some(read_unaligned_field!(ptr, tire_wear_rear_right));
        packet.track_ordinal = Some(read_unaligned_field!(ptr, track_ordinal));
        packet
    }
}

fn validate_sled_packet(packet: &SledPacket) -> Result<(), ParseTelemetryError> {
    validate_finite(packet.engine_max_rpm, "engine_max_rpm")?;
    validate_finite(packet.engine_idle_rpm, "engine_idle_rpm")?;
    validate_finite(packet.current_engine_rpm, "current_engine_rpm")
}

fn validate_finite(value: f32, field: &'static str) -> Result<(), ParseTelemetryError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(ParseTelemetryError::NonFiniteFloat(field))
    }
}

#[cfg(test)]
mod tests {
    use std::{
        path::{Path, PathBuf},
        time::Duration,
    };

    use super::*;
    use proptest::{collection::vec, prelude::*};
    use tokio::sync::{broadcast, watch};

    #[test]
    fn raw_packet_layout_sizes_match_forza_spec() {
        assert_eq!(std::mem::size_of::<RawSledPacket>(), SLED_PACKET_LEN);
        assert_eq!(std::mem::size_of::<RawDashPacket>(), DASH_PACKET_LEN);
        assert_eq!(
            std::mem::size_of::<RawFm2023DashPacket>(),
            FM2023_DASH_PACKET_LEN
        );
    }

    #[test]
    fn parse_invalid_length_returns_error() {
        let err =
            parse_telemetry_packet(&[0u8; 200]).expect_err("parser should reject invalid len");
        assert_eq!(err, ParseTelemetryError::InvalidLength(200));
    }

    #[test]
    fn parse_sled_packet_returns_typed_data() {
        let mut raw = RawSledPacket::zeroed();
        raw.is_race_on = 1;
        raw.timestamp_ms = 1234;
        raw.engine_max_rpm = 8000.0;
        raw.engine_idle_rpm = 900.0;
        raw.current_engine_rpm = 3200.0;
        raw.car_ordinal = 12;
        raw.num_cylinders = 8;

        let parsed =
            parse_telemetry_packet(bytemuck::bytes_of(&raw)).expect("sled packet should parse");
        let TelemetryPacket::Sled(packet) = parsed else {
            panic!("expected sled packet")
        };

        assert_eq!(packet.is_race_on, 1);
        assert_eq!(packet.timestamp_ms, 1234);
        assert_eq!(packet.engine_max_rpm, 8000.0);
        assert_eq!(packet.current_engine_rpm, 3200.0);
        assert_eq!(packet.car_ordinal, 12);
        assert_eq!(packet.num_cylinders, 8);
    }

    #[test]
    fn parse_dash_packet_returns_typed_data() {
        let mut raw = RawDashPacket::zeroed();
        raw.sled.is_race_on = 1;
        raw.sled.timestamp_ms = 4321;
        raw.sled.engine_max_rpm = 9000.0;
        raw.sled.engine_idle_rpm = 1000.0;
        raw.sled.current_engine_rpm = 4500.0;
        raw.speed = 55.5;
        raw.lap_number = 7;
        raw.race_position = 3;
        raw.gear = 4;
        raw.steer = -12;

        let parsed =
            parse_telemetry_packet(bytemuck::bytes_of(&raw)).expect("dash packet should parse");
        let TelemetryPacket::Dash(packet) = parsed else {
            panic!("expected dash packet")
        };

        assert_eq!(packet.sled.timestamp_ms, 4321);
        assert_eq!(packet.speed, 55.5);
        assert_eq!(packet.lap_number, 7);
        assert_eq!(packet.race_position, 3);
        assert_eq!(packet.gear, 4);
        assert_eq!(packet.steer, -12);
        // Legacy 311-byte packets have no FM 2023 trailer.
        assert_eq!(packet.tire_wear_front_left, None);
        assert_eq!(packet.track_ordinal, None);
    }

    #[test]
    fn parse_fm2023_dash_packet_returns_tire_wear_and_track_ordinal() {
        let mut raw = RawFm2023DashPacket::zeroed();
        raw.dash.sled.is_race_on = 1;
        raw.dash.sled.engine_max_rpm = 9000.0;
        raw.dash.sled.engine_idle_rpm = 1000.0;
        raw.dash.sled.current_engine_rpm = 5000.0;
        raw.dash.speed = 33.0;
        raw.dash.lap_number = 2;
        raw.tire_wear_front_left = 0.92;
        raw.tire_wear_front_right = 0.91;
        raw.tire_wear_rear_left = 0.87;
        raw.tire_wear_rear_right = 0.89;
        raw.track_ordinal = 861;

        let bytes = bytemuck::bytes_of(&raw);
        assert_eq!(bytes.len(), FM2023_DASH_PACKET_LEN);

        let parsed = parse_telemetry_packet(bytes).expect("fm2023 dash packet should parse");
        let TelemetryPacket::Dash(packet) = parsed else {
            panic!("expected dash packet")
        };

        assert_eq!(packet.speed, 33.0);
        assert_eq!(packet.lap_number, 2);
        assert_eq!(packet.tire_wear_front_left, Some(0.92));
        assert_eq!(packet.tire_wear_front_right, Some(0.91));
        assert_eq!(packet.tire_wear_rear_left, Some(0.87));
        assert_eq!(packet.tire_wear_rear_right, Some(0.89));
        assert_eq!(packet.track_ordinal, Some(861));
    }

    #[test]
    fn parse_non_finite_rpm_is_rejected() {
        let mut raw = RawSledPacket::zeroed();
        raw.engine_max_rpm = f32::INFINITY;
        raw.engine_idle_rpm = 900.0;
        raw.current_engine_rpm = 2000.0;

        let err = parse_telemetry_packet(bytemuck::bytes_of(&raw))
            .expect_err("non-finite validated fields should be rejected");
        assert_eq!(err, ParseTelemetryError::NonFiniteFloat("engine_max_rpm"));
    }

    proptest! {
        #[test]
        fn parser_never_panics_on_random_bytes(bytes in vec(any::<u8>(), 0..600)) {
            let _ = parse_telemetry_packet(&bytes);
        }
    }

    #[test]
    fn fixture_packets_match_snapshots() {
        let fixture_dir = fixture_dir();
        for fixture_name in [
            "dash_packet_01.bin",
            "dash_packet_02.bin",
            "sled_packet_01.bin",
        ] {
            let packet_bytes =
                std::fs::read(fixture_dir.join(fixture_name)).expect("fixture should be readable");
            let parsed = parse_telemetry_packet(&packet_bytes).expect("fixture should parse");
            insta::assert_debug_snapshot!(fixture_name, parsed);
        }
    }

    #[tokio::test]
    async fn udp_listener_publishes_latest_packet_on_watch_channel() {
        let receiver_socket = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("receiver socket should bind");
        let receiver_addr = receiver_socket.local_addr().expect("receiver addr");
        let sender_socket = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("sender socket should bind");

        let (latest_packet_tx, mut latest_packet_rx) = watch::channel(None);
        let (shutdown_tx, _) = broadcast::channel(4);
        let listener_task = tokio::spawn(udp_listener_loop(
            receiver_socket,
            latest_packet_tx,
            shutdown_tx.subscribe(),
        ));

        let packet =
            std::fs::read(fixture_dir().join("dash_packet_01.bin")).expect("fixture should load");
        sender_socket
            .send_to(&packet, receiver_addr)
            .await
            .expect("packet should send");

        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                latest_packet_rx
                    .changed()
                    .await
                    .expect("watch channel should stay open");
                if latest_packet_rx.borrow().is_some() {
                    break;
                }
            }
        })
        .await
        .expect("listener should publish packet");

        let _ = shutdown_tx.send(());
        listener_task
            .await
            .expect("listener task should join")
            .expect("listener should exit cleanly");
    }

    fn fixture_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
    }
}
