use serde::Serialize;

use crate::{storage::Storage, telemetry::DashPacket};

const DEFAULT_REWIND_BACKWARD_JUMP_M: f32 = 50.0;
const DEFAULT_SESSION_RESET_RACE_TIME_WINDOW_S: f32 = 2.0;
const MAX_PACKET_GAP_MS_FOR_REWIND: u32 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LapValidityConfig {
    pub rewind_backward_jump_m: f32,
    pub session_reset_race_time_window_s: f32,
}

impl Default for LapValidityConfig {
    fn default() -> Self {
        Self {
            rewind_backward_jump_m: DEFAULT_REWIND_BACKWARD_JUMP_M,
            session_reset_race_time_window_s: DEFAULT_SESSION_RESET_RACE_TIME_WINDOW_S,
        }
    }
}

impl LapValidityConfig {
    pub fn validate(&self) -> Result<(), String> {
        if !(self.rewind_backward_jump_m.is_finite() && self.rewind_backward_jump_m > 0.0) {
            return Err("rewind_backward_jump_m must be > 0".to_string());
        }
        if !(self.session_reset_race_time_window_s.is_finite()
            && self.session_reset_race_time_window_s > 0.0)
        {
            return Err("session_reset_race_time_window_s must be > 0".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum LapValidityEvent {
    LapRewindDetected {
        session_id: i64,
        lap_number: u16,
        approx_distance_rewound_m: f32,
        at_ms: u32,
    },
    SessionResetDetected {
        prior_session_id: i64,
        new_session_id: i64,
        at_ms: u32,
    },
}

#[derive(Debug, Default)]
enum SessionState {
    #[default]
    Idle,
    InRace,
}

#[derive(Debug)]
pub struct LapValidityDetector {
    config: LapValidityConfig,
    session_state: SessionState,
    current_session_id: Option<i64>,
    current_lap_number: Option<u16>,
    suppress_current_lap_analysis: bool,
    last_distance_traveled: Option<f32>,
    last_timestamp_ms: Option<u32>,
    last_current_race_time: Option<f32>,
}

impl LapValidityDetector {
    pub fn new(config: LapValidityConfig) -> Self {
        Self {
            config,
            session_state: SessionState::Idle,
            current_session_id: None,
            current_lap_number: None,
            suppress_current_lap_analysis: false,
            last_distance_traveled: None,
            last_timestamp_ms: None,
            last_current_race_time: None,
        }
    }

    pub fn suppress_current_lap_analysis(&self) -> bool {
        self.suppress_current_lap_analysis
    }

    pub fn process_packet(
        &mut self,
        packet: &DashPacket,
        storage: &Storage,
        sidecar_version: &str,
    ) -> Result<Vec<LapValidityEvent>, crate::storage::StorageError> {
        let _was_in_race = matches!(self.session_state, SessionState::InRace);
        if packet.sled.is_race_on != 1 {
            self.session_state = SessionState::Idle;
            self.last_distance_traveled = Some(packet.distance_traveled);
            self.last_timestamp_ms = Some(packet.sled.timestamp_ms);
            self.last_current_race_time = Some(packet.current_race_time);
            return Ok(Vec::new());
        }

        self.session_state = SessionState::InRace;
        let session_id = if let Some(existing) = self.current_session_id {
            existing
        } else {
            let created = storage.start_session(Some(i32::from(packet.sled.car_ordinal)), sidecar_version)?;
            self.current_session_id = Some(created);
            created
        };

        let mut events = Vec::new();
        if let Some(reset_event) = self.detect_session_reset(packet, storage, sidecar_version)? {
            events.push(reset_event);
        }

        let active_session_id = self.current_session_id.unwrap_or(session_id);
        if self.current_lap_number != Some(packet.lap_number) {
            storage.ensure_lap(
                active_session_id,
                packet.lap_number,
                packet.sled.timestamp_ms,
            )?;
            self.current_lap_number = Some(packet.lap_number);
            self.suppress_current_lap_analysis = false;
        }

        if let Some(rewind_event) = self.detect_rewind(packet, storage, active_session_id)? {
            events.push(rewind_event);
        }

        self.last_distance_traveled = Some(packet.distance_traveled);
        self.last_timestamp_ms = Some(packet.sled.timestamp_ms);
        self.last_current_race_time = Some(packet.current_race_time);
        Ok(events)
    }

    fn detect_session_reset(
        &mut self,
        packet: &DashPacket,
        storage: &Storage,
        sidecar_version: &str,
    ) -> Result<Option<LapValidityEvent>, crate::storage::StorageError> {
        let Some(previous_lap_number) = self.current_lap_number else {
            return Ok(None);
        };
        let Some(previous_race_time) = self.last_current_race_time else {
            return Ok(None);
        };
        let Some(prior_session_id) = self.current_session_id else {
            return Ok(None);
        };

        let lap_dropped_to_zero = previous_lap_number > 0 && packet.lap_number == 0;
        let race_time_reset = packet.current_race_time
            < self.config.session_reset_race_time_window_s
            && packet.current_race_time < previous_race_time;
        if !(lap_dropped_to_zero && race_time_reset) {
            return Ok(None);
        }

        storage.end_session(prior_session_id)?;
        let new_session_id = storage.start_session(Some(i32::from(packet.sled.car_ordinal)), sidecar_version)?;
        storage.ensure_lap(new_session_id, packet.lap_number, packet.sled.timestamp_ms)?;

        self.current_session_id = Some(new_session_id);
        self.current_lap_number = Some(packet.lap_number);
        self.suppress_current_lap_analysis = false;
        self.last_distance_traveled = None;

        Ok(Some(LapValidityEvent::SessionResetDetected {
            prior_session_id,
            new_session_id,
            at_ms: packet.sled.timestamp_ms,
        }))
    }

    fn detect_rewind(
        &mut self,
        packet: &DashPacket,
        storage: &Storage,
        session_id: i64,
    ) -> Result<Option<LapValidityEvent>, crate::storage::StorageError> {
        let (Some(previous_distance), Some(previous_timestamp)) =
            (self.last_distance_traveled, self.last_timestamp_ms)
        else {
            return Ok(None);
        };

        let packet_gap_ms = packet.sled.timestamp_ms.wrapping_sub(previous_timestamp);
        if packet_gap_ms > MAX_PACKET_GAP_MS_FOR_REWIND {
            return Ok(None);
        }

        let distance_delta = packet.distance_traveled - previous_distance;
        if distance_delta > -self.config.rewind_backward_jump_m {
            return Ok(None);
        }

        storage.mark_lap_rewind(session_id, packet.lap_number)?;
        self.suppress_current_lap_analysis = true;

        Ok(Some(LapValidityEvent::LapRewindDetected {
            session_id,
            lap_number: packet.lap_number,
            approx_distance_rewound_m: -distance_delta,
            at_ms: packet.sled.timestamp_ms,
        }))
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::{LapValidityConfig, LapValidityDetector, LapValidityEvent};
    use crate::{storage::Storage, telemetry::DashPacket, telemetry::SledPacket};

    #[test]
    fn detects_rewind_marks_lap_dirty_and_suppresses_lap_analysis() {
        let temp = TempDir::new().expect("temp dir");
        let storage = Storage::open(temp.path()).expect("storage");
        let mut detector = LapValidityDetector::new(LapValidityConfig::default());

        let first = dash_packet(1_000, 2, 350.0, 42.0, 42.0);
        let second = dash_packet(1_080, 2, 280.0, 43.0, 43.0);

        detector
            .process_packet(&first, &storage, "0.1.0")
            .expect("first packet");
        let events = detector
            .process_packet(&second, &storage, "0.1.0")
            .expect("second packet");

        assert_eq!(events.len(), 1);
        let LapValidityEvent::LapRewindDetected {
            session_id,
            lap_number,
            approx_distance_rewound_m,
            at_ms,
        } = &events[0]
        else {
            panic!("expected rewind event")
        };
        assert_eq!(*lap_number, 2);
        assert!(*approx_distance_rewound_m >= 70.0);
        assert_eq!(*at_ms, 1_080);
        let (valid, dirty_reason) = storage
            .read_lap_validity(*session_id, *lap_number)
            .expect("lap validity");
        assert!(!valid);
        assert_eq!(dirty_reason.as_deref(), Some("Rewind"));
        assert!(detector.suppress_current_lap_analysis());
    }

    #[test]
    fn does_not_emit_rewind_for_small_backward_motion() {
        let temp = TempDir::new().expect("temp dir");
        let storage = Storage::open(temp.path()).expect("storage");
        let mut detector = LapValidityDetector::new(LapValidityConfig::default());

        detector
            .process_packet(&dash_packet(1_000, 1, 500.0, 31.0, 31.0), &storage, "0.1.0")
            .expect("first");
        let events = detector
            .process_packet(&dash_packet(1_050, 1, 470.0, 31.5, 31.5), &storage, "0.1.0")
            .expect("second");

        assert!(events.is_empty());
        assert!(!detector.suppress_current_lap_analysis());
    }

    #[test]
    fn session_reset_emits_event_and_creates_new_session() {
        let temp = TempDir::new().expect("temp dir");
        let storage = Storage::open(temp.path()).expect("storage");
        let mut detector = LapValidityDetector::new(LapValidityConfig::default());

        detector
            .process_packet(
                &dash_packet(1_000, 3, 1_200.0, 95.0, 95.0),
                &storage,
                "0.1.0",
            )
            .expect("first");
        let events = detector
            .process_packet(&dash_packet(1_100, 0, 0.0, 96.0, 0.8), &storage, "0.1.0")
            .expect("reset");

        assert_eq!(events.len(), 1);
        let LapValidityEvent::SessionResetDetected {
            prior_session_id,
            new_session_id,
            at_ms,
        } = events[0]
        else {
            panic!("expected session reset event")
        };
        assert_ne!(prior_session_id, new_session_id);
        assert_eq!(at_ms, 1_100);
        assert_eq!(storage.count_sessions().expect("count sessions"), 2);
        assert!(storage
            .session_has_ended_at(prior_session_id)
            .expect("session close check"));
    }

    #[test]
    fn timestamp_wrap_does_not_trigger_false_rewind() {
        let temp = TempDir::new().expect("temp dir");
        let storage = Storage::open(temp.path()).expect("storage");
        let mut detector = LapValidityDetector::new(LapValidityConfig::default());

        detector
            .process_packet(
                &dash_packet(u32::MAX - 5, 4, 900.0, 12.0, 12.0),
                &storage,
                "0.1.0",
            )
            .expect("pre-wrap");
        let events = detector
            .process_packet(&dash_packet(5, 4, 870.0, 12.3, 12.3), &storage, "0.1.0")
            .expect("post-wrap");
        assert!(events.is_empty());
    }

    fn dash_packet(
        timestamp_ms: u32,
        lap_number: u16,
        distance_traveled: f32,
        current_lap: f32,
        current_race_time: f32,
    ) -> DashPacket {
        DashPacket {
            sled: SledPacket {
                is_race_on: 1,
                timestamp_ms,
                engine_max_rpm: 8_000.0,
                engine_idle_rpm: 900.0,
                current_engine_rpm: 2_500.0,
                acceleration_x: 0.0,
                acceleration_y: 0.0,
                acceleration_z: 0.0,
                velocity_x: 0.0,
                velocity_y: 0.0,
                velocity_z: 0.0,
                angular_velocity_x: 0.0,
                angular_velocity_y: 0.0,
                angular_velocity_z: 0.0,
                yaw: 0.0,
                pitch: 0.0,
                roll: 0.0,
                normalized_suspension_travel_front_left: 0.0,
                normalized_suspension_travel_front_right: 0.0,
                normalized_suspension_travel_rear_left: 0.0,
                normalized_suspension_travel_rear_right: 0.0,
                tire_slip_ratio_front_left: 0.0,
                tire_slip_ratio_front_right: 0.0,
                tire_slip_ratio_rear_left: 0.0,
                tire_slip_ratio_rear_right: 0.0,
                wheel_rotation_speed_front_left: 0.0,
                wheel_rotation_speed_front_right: 0.0,
                wheel_rotation_speed_rear_left: 0.0,
                wheel_rotation_speed_rear_right: 0.0,
                wheel_on_rumble_strip_front_left: 0,
                wheel_on_rumble_strip_front_right: 0,
                wheel_on_rumble_strip_rear_left: 0,
                wheel_on_rumble_strip_rear_right: 0,
                wheel_in_puddle_depth_front_left: 0.0,
                wheel_in_puddle_depth_front_right: 0.0,
                wheel_in_puddle_depth_rear_left: 0.0,
                wheel_in_puddle_depth_rear_right: 0.0,
                surface_rumble_front_left: 0.0,
                surface_rumble_front_right: 0.0,
                surface_rumble_rear_left: 0.0,
                surface_rumble_rear_right: 0.0,
                tire_slip_angle_front_left: 0.0,
                tire_slip_angle_front_right: 0.0,
                tire_slip_angle_rear_left: 0.0,
                tire_slip_angle_rear_right: 0.0,
                tire_combined_slip_front_left: 0.0,
                tire_combined_slip_front_right: 0.0,
                tire_combined_slip_rear_left: 0.0,
                tire_combined_slip_rear_right: 0.0,
                suspension_travel_meters_front_left: 0.0,
                suspension_travel_meters_front_right: 0.0,
                suspension_travel_meters_rear_left: 0.0,
                suspension_travel_meters_rear_right: 0.0,
                car_ordinal: 42,
                car_class: 6,
                car_performance_index: 700,
                drivetrain_type: 1,
                num_cylinders: 8,
            },
            position_x: 0.0,
            position_y: 0.0,
            position_z: 0.0,
            speed: 40.0,
            power: 0.0,
            torque: 0.0,
            tire_temp_front_left: 0.0,
            tire_temp_front_right: 0.0,
            tire_temp_rear_left: 0.0,
            tire_temp_rear_right: 0.0,
            boost: 0.0,
            fuel: 0.0,
            distance_traveled,
            best_lap: 0.0,
            last_lap: 0.0,
            current_lap,
            current_race_time,
            lap_number,
            race_position: 1,
            accel: 0,
            brake: 0,
            clutch: 0,
            hand_brake: 0,
            gear: 3,
            steer: 0,
            normalized_driving_line: 0,
            normalized_ai_brake_difference: 0,
        }
    }
}
