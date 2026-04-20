use std::collections::{HashSet, VecDeque};

use serde::Serialize;

use crate::{storage::Storage, telemetry::DashPacket};

const DEFAULT_REWIND_BACKWARD_JUMP_M: f32 = 50.0;
const DEFAULT_SESSION_RESET_RACE_TIME_WINDOW_S: f32 = 2.0;
const DEFAULT_OFF_TRACK_WINDOW_MS: u32 = 500;
const DEFAULT_OFF_TRACK_MIN_WHEELS: u8 = 2;
const DEFAULT_SURFACE_RUMBLE_THRESHOLD: f32 = 0.35;
const DEFAULT_SURFACE_RUMBLE_WINDOW_PACKETS: usize = 5;
const DEFAULT_WHEEL_ON_RUMBLE_MIN: f32 = 0.1;
const DEFAULT_WALL_CONTACT_G_THRESHOLD: f32 = 10.0;
const DEFAULT_CORNER_CUT_SPEED_KPH_MIN: f32 = 30.0;
const DEFAULT_CORNER_CUT_COMBINED_SLIP_THRESHOLD: f32 = 1.0;
const STEER_INPUT_MAX_ABS: f32 = 127.0;
const DEFAULT_CORNER_CUT_MAX_ABS_STEER_NORM: f32 = 10.0 / STEER_INPUT_MAX_ABS;
const MAX_PACKET_GAP_MS_FOR_REWIND: u32 = 10_000;
const GRAVITY_MPS2: f32 = 9.81;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LapValidityConfig {
    pub rewind_backward_jump_m: f32,
    pub session_reset_race_time_window_s: f32,
    pub off_track_window_ms: u32,
    pub off_track_min_wheels: u8,
    pub surface_rumble_threshold: f32,
    pub surface_rumble_window_packets: usize,
    pub wall_contact_g_threshold: f32,
    pub corner_cut_speed_kph_min: f32,
    pub corner_cut_combined_slip_threshold: f32,
    pub corner_cut_max_abs_steer_norm: f32,
}

impl Default for LapValidityConfig {
    fn default() -> Self {
        Self {
            rewind_backward_jump_m: DEFAULT_REWIND_BACKWARD_JUMP_M,
            session_reset_race_time_window_s: DEFAULT_SESSION_RESET_RACE_TIME_WINDOW_S,
            off_track_window_ms: DEFAULT_OFF_TRACK_WINDOW_MS,
            off_track_min_wheels: DEFAULT_OFF_TRACK_MIN_WHEELS,
            surface_rumble_threshold: DEFAULT_SURFACE_RUMBLE_THRESHOLD,
            surface_rumble_window_packets: DEFAULT_SURFACE_RUMBLE_WINDOW_PACKETS,
            wall_contact_g_threshold: DEFAULT_WALL_CONTACT_G_THRESHOLD,
            corner_cut_speed_kph_min: DEFAULT_CORNER_CUT_SPEED_KPH_MIN,
            corner_cut_combined_slip_threshold: DEFAULT_CORNER_CUT_COMBINED_SLIP_THRESHOLD,
            corner_cut_max_abs_steer_norm: DEFAULT_CORNER_CUT_MAX_ABS_STEER_NORM,
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
        if self.off_track_window_ms == 0 {
            return Err("off_track_window_ms must be > 0".to_string());
        }
        if !(1..=4).contains(&self.off_track_min_wheels) {
            return Err("off_track_min_wheels must be between 1 and 4".to_string());
        }
        if !(self.surface_rumble_threshold.is_finite() && self.surface_rumble_threshold >= 0.0) {
            return Err("surface_rumble_threshold must be >= 0".to_string());
        }
        if self.surface_rumble_window_packets == 0 {
            return Err("surface_rumble_window_packets must be > 0".to_string());
        }
        if !(self.wall_contact_g_threshold.is_finite() && self.wall_contact_g_threshold > 0.0) {
            return Err("wall_contact_g_threshold must be > 0".to_string());
        }
        if !(self.corner_cut_speed_kph_min.is_finite() && self.corner_cut_speed_kph_min > 0.0) {
            return Err("corner_cut_speed_kph_min must be > 0".to_string());
        }
        if !(self.corner_cut_combined_slip_threshold.is_finite()
            && self.corner_cut_combined_slip_threshold > 0.0)
        {
            return Err("corner_cut_combined_slip_threshold must be > 0".to_string());
        }
        if !(self.corner_cut_max_abs_steer_norm.is_finite()
            && (0.0..=1.0).contains(&self.corner_cut_max_abs_steer_norm))
        {
            return Err("corner_cut_max_abs_steer_norm must be in [0, 1]".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Hash)]
pub enum DirtyReasonCode {
    #[serde(rename = "OffTrack")]
    OffTrack,
    #[serde(rename = "WallContact")]
    WallContact,
    #[serde(rename = "CornerCut")]
    CornerCut,
    #[serde(rename = "Rewind")]
    Rewind,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Hash)]
pub struct DirtyReason {
    pub code: DirtyReasonCode,
    pub best_effort: bool,
}

impl DirtyReason {
    fn off_track() -> Self {
        Self {
            code: DirtyReasonCode::OffTrack,
            best_effort: false,
        }
    }

    fn wall_contact() -> Self {
        Self {
            code: DirtyReasonCode::WallContact,
            best_effort: false,
        }
    }

    fn corner_cut() -> Self {
        Self {
            code: DirtyReasonCode::CornerCut,
            best_effort: true,
        }
    }

    fn as_db_reason(self) -> &'static str {
        match self.code {
            DirtyReasonCode::OffTrack => "OffTrack",
            DirtyReasonCode::WallContact => "WallContact",
            DirtyReasonCode::CornerCut => "CornerCut",
            DirtyReasonCode::Rewind => "Rewind",
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "event", rename_all = "snake_case")]
#[allow(clippy::enum_variant_names)]
pub enum LapValidityEvent {
    LapRewindDetected {
        session_id: i64,
        lap_number: u16,
        approx_distance_rewound_m: f32,
        at_ms: u32,
    },
    LapDirtyDetected {
        lap_id: i64,
        reason: DirtyReason,
        at_ms: u32,
        lap_number: u16,
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
    current_lap_dirty_reasons: HashSet<DirtyReasonCode>,
    pending_off_track_since_ms: Option<u32>,
    surface_rumble_window: VecDeque<f32>,
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
            current_lap_dirty_reasons: HashSet::new(),
            pending_off_track_since_ms: None,
            surface_rumble_window: VecDeque::new(),
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
        if packet.sled.is_race_on != 1 {
            self.session_state = SessionState::Idle;
            self.reset_transient_signals();
            self.last_distance_traveled = Some(packet.distance_traveled);
            self.last_timestamp_ms = Some(packet.sled.timestamp_ms);
            self.last_current_race_time = Some(packet.current_race_time);
            return Ok(Vec::new());
        }

        self.session_state = SessionState::InRace;
        let session_id = if let Some(existing) = self.current_session_id {
            existing
        } else {
            let created = storage.start_session(Some(packet.sled.car_ordinal), sidecar_version)?;
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
            self.current_lap_dirty_reasons.clear();
            self.reset_transient_signals();
        }

        if let Some(rewind_event) = self.detect_rewind(packet, storage, active_session_id)? {
            events.push(rewind_event);
        }

        if let Some(current_lap_number) = self.current_lap_number {
            let mut dirty_events =
                self.detect_dirty_reasons(packet, storage, active_session_id, current_lap_number)?;
            events.append(&mut dirty_events);
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
        let new_session_id =
            storage.start_session(Some(packet.sled.car_ordinal), sidecar_version)?;
        storage.ensure_lap(new_session_id, packet.lap_number, packet.sled.timestamp_ms)?;

        self.current_session_id = Some(new_session_id);
        self.current_lap_number = Some(packet.lap_number);
        self.suppress_current_lap_analysis = false;
        self.current_lap_dirty_reasons.clear();
        self.reset_transient_signals();
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
        self.current_lap_dirty_reasons
            .insert(DirtyReasonCode::Rewind);
        self.suppress_current_lap_analysis = true;

        Ok(Some(LapValidityEvent::LapRewindDetected {
            session_id,
            lap_number: packet.lap_number,
            approx_distance_rewound_m: -distance_delta,
            at_ms: packet.sled.timestamp_ms,
        }))
    }

    fn detect_dirty_reasons(
        &mut self,
        packet: &DashPacket,
        storage: &Storage,
        session_id: i64,
        lap_number: u16,
    ) -> Result<Vec<LapValidityEvent>, crate::storage::StorageError> {
        let mut reasons = Vec::new();

        if self.off_track_triggered(packet)
            && !self
                .current_lap_dirty_reasons
                .contains(&DirtyReasonCode::OffTrack)
        {
            reasons.push(DirtyReason::off_track());
        }

        if self.wall_contact_triggered(packet)
            && !self
                .current_lap_dirty_reasons
                .contains(&DirtyReasonCode::WallContact)
        {
            reasons.push(DirtyReason::wall_contact());
        }

        if self.corner_cut_triggered(packet)
            && !self
                .current_lap_dirty_reasons
                .contains(&DirtyReasonCode::CornerCut)
        {
            reasons.push(DirtyReason::corner_cut());
        }

        if reasons.is_empty() {
            return Ok(Vec::new());
        }

        self.suppress_current_lap_analysis = true;
        let mut events = Vec::with_capacity(reasons.len());
        for reason in reasons {
            let lap_id = storage.mark_lap_dirty(session_id, lap_number, reason.as_db_reason())?;
            self.current_lap_dirty_reasons.insert(reason.code);
            events.push(LapValidityEvent::LapDirtyDetected {
                lap_id,
                reason,
                at_ms: packet.sled.timestamp_ms,
                lap_number,
            });
        }

        Ok(events)
    }

    fn off_track_triggered(&mut self, packet: &DashPacket) -> bool {
        let wheel_count = [
            packet.sled.wheel_on_rumble_strip_front_left,
            packet.sled.wheel_on_rumble_strip_front_right,
            packet.sled.wheel_on_rumble_strip_rear_left,
            packet.sled.wheel_on_rumble_strip_rear_right,
        ]
        .into_iter()
        .filter(|value| (*value as f32) > DEFAULT_WHEEL_ON_RUMBLE_MIN)
        .count() as u8;

        let rumble_mean = [
            packet.sled.surface_rumble_front_left,
            packet.sled.surface_rumble_front_right,
            packet.sled.surface_rumble_rear_left,
            packet.sled.surface_rumble_rear_right,
        ]
        .into_iter()
        .map(|value| value.max(0.0))
        .sum::<f32>()
            / 4.0;

        self.surface_rumble_window.push_back(rumble_mean);
        while self.surface_rumble_window.len() > self.config.surface_rumble_window_packets {
            self.surface_rumble_window.pop_front();
        }
        let smoothed_rumble = if self.surface_rumble_window.is_empty() {
            0.0
        } else {
            self.surface_rumble_window.iter().sum::<f32>() / self.surface_rumble_window.len() as f32
        };

        let off_track_signal = wheel_count >= self.config.off_track_min_wheels
            || smoothed_rumble >= self.config.surface_rumble_threshold;

        if !off_track_signal {
            self.pending_off_track_since_ms = None;
            return false;
        }

        let since_ms = self
            .pending_off_track_since_ms
            .get_or_insert(packet.sled.timestamp_ms);
        packet.sled.timestamp_ms.wrapping_sub(*since_ms) >= self.config.off_track_window_ms
    }

    fn wall_contact_triggered(&self, packet: &DashPacket) -> bool {
        let threshold = self.config.wall_contact_g_threshold * GRAVITY_MPS2;
        [
            packet.sled.acceleration_x,
            packet.sled.acceleration_y,
            packet.sled.acceleration_z,
        ]
        .into_iter()
        .any(|axis| axis.abs() >= threshold)
    }

    fn corner_cut_triggered(&self, packet: &DashPacket) -> bool {
        if packet.speed * 3.6 <= self.config.corner_cut_speed_kph_min {
            return false;
        }

        let max_combined_slip = [
            packet.sled.tire_combined_slip_front_left,
            packet.sled.tire_combined_slip_front_right,
            packet.sled.tire_combined_slip_rear_left,
            packet.sled.tire_combined_slip_rear_right,
        ]
        .into_iter()
        .map(f32::abs)
        .fold(0.0, f32::max);
        if max_combined_slip < self.config.corner_cut_combined_slip_threshold {
            return false;
        }

        let steer_norm = (packet.steer as f32 / STEER_INPUT_MAX_ABS).abs();
        steer_norm <= self.config.corner_cut_max_abs_steer_norm
    }

    fn reset_transient_signals(&mut self) {
        self.pending_off_track_since_ms = None;
        self.surface_rumble_window.clear();
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::{DirtyReasonCode, LapValidityConfig, LapValidityDetector, LapValidityEvent};
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

    #[test]
    fn off_track_trigger_marks_lap_dirty_after_window() {
        let temp = TempDir::new().expect("temp dir");
        let storage = Storage::open(temp.path()).expect("storage");
        let mut detector = LapValidityDetector::new(LapValidityConfig::default());

        detector
            .process_packet(&off_track_packet(1_000, 1), &storage, "0.1.0")
            .expect("first");
        detector
            .process_packet(&off_track_packet(1_250, 1), &storage, "0.1.0")
            .expect("second");
        let events = detector
            .process_packet(&off_track_packet(1_550, 1), &storage, "0.1.0")
            .expect("third");

        assert_eq!(events.len(), 1);
        let LapValidityEvent::LapDirtyDetected {
            lap_id: _,
            reason,
            at_ms,
            lap_number,
        } = &events[0]
        else {
            panic!("expected dirty event")
        };
        assert_eq!(reason.code, DirtyReasonCode::OffTrack);
        assert!(!reason.best_effort);
        assert_eq!(*lap_number, 1);
        assert_eq!(*at_ms, 1_550);

        let session_id = first_session_id(&storage);
        let (valid, dirty_reason) = storage
            .read_lap_validity(session_id, 1)
            .expect("lap validity");
        assert!(!valid);
        assert_eq!(dirty_reason.as_deref(), Some("OffTrack"));
    }

    #[test]
    fn wall_contact_trigger_marks_lap_dirty() {
        let temp = TempDir::new().expect("temp dir");
        let storage = Storage::open(temp.path()).expect("storage");
        let mut detector = LapValidityDetector::new(LapValidityConfig::default());

        let mut packet = dash_packet(1_000, 1, 10.0, 1.0, 1.0);
        packet.sled.acceleration_x = 11.0 * 9.81;

        let events = detector
            .process_packet(&packet, &storage, "0.1.0")
            .expect("packet");
        assert_eq!(events.len(), 1);
        let LapValidityEvent::LapDirtyDetected { reason, .. } = &events[0] else {
            panic!("expected dirty event")
        };
        assert_eq!(reason.code, DirtyReasonCode::WallContact);
        assert!(!reason.best_effort);
    }

    #[test]
    fn corner_cut_trigger_marks_lap_dirty_as_best_effort() {
        let temp = TempDir::new().expect("temp dir");
        let storage = Storage::open(temp.path()).expect("storage");
        let mut detector = LapValidityDetector::new(LapValidityConfig::default());

        let mut packet = dash_packet(1_000, 1, 10.0, 1.0, 1.0);
        packet.speed = 40.0;
        packet.steer = 0;
        packet.sled.tire_combined_slip_front_left = 1.4;

        let events = detector
            .process_packet(&packet, &storage, "0.1.0")
            .expect("packet");
        assert_eq!(events.len(), 1);
        let LapValidityEvent::LapDirtyDetected { reason, .. } = &events[0] else {
            panic!("expected dirty event")
        };
        assert_eq!(reason.code, DirtyReasonCode::CornerCut);
        assert!(reason.best_effort);
    }

    #[test]
    fn dirty_reason_is_sticky_and_additional_reasons_are_appended() {
        let temp = TempDir::new().expect("temp dir");
        let storage = Storage::open(temp.path()).expect("storage");
        let mut detector = LapValidityDetector::new(LapValidityConfig::default());

        let mut wall_contact = dash_packet(1_000, 1, 10.0, 1.0, 1.0);
        wall_contact.sled.acceleration_y = 12.0 * 9.81;

        let mut corner_cut = dash_packet(1_200, 1, 20.0, 2.0, 2.0);
        corner_cut.speed = 45.0;
        corner_cut.sled.tire_combined_slip_front_right = 1.2;
        corner_cut.steer = 0;

        let first_events = detector
            .process_packet(&wall_contact, &storage, "0.1.0")
            .expect("wall contact");
        assert_eq!(first_events.len(), 1);

        let second_events = detector
            .process_packet(&corner_cut, &storage, "0.1.0")
            .expect("corner cut");
        assert_eq!(second_events.len(), 1);

        let session_id = first_session_id(&storage);
        let (valid, dirty_reason) = storage
            .read_lap_validity(session_id, 1)
            .expect("lap validity");
        assert!(!valid);
        assert_eq!(dirty_reason.as_deref(), Some("WallContact"));

        let dirty_reasons = storage
            .read_lap_dirty_reasons(session_id, 1)
            .expect("dirty reasons");
        assert_eq!(dirty_reasons, vec!["WallContact", "CornerCut"]);
    }

    #[test]
    fn short_surface_rumble_noise_is_debounced() {
        let temp = TempDir::new().expect("temp dir");
        let storage = Storage::open(temp.path()).expect("storage");
        let mut detector = LapValidityDetector::new(LapValidityConfig::default());

        detector
            .process_packet(&surface_rumble_packet(1_000, 1, 0.8), &storage, "0.1.0")
            .expect("first");
        detector
            .process_packet(&surface_rumble_packet(1_120, 1, 0.9), &storage, "0.1.0")
            .expect("second");
        let events = detector
            .process_packet(&surface_rumble_packet(1_170, 1, 0.0), &storage, "0.1.0")
            .expect("third");

        assert!(events.is_empty());
    }

    #[test]
    fn pause_resume_without_dirty_condition_emits_no_spurious_event() {
        let temp = TempDir::new().expect("temp dir");
        let storage = Storage::open(temp.path()).expect("storage");
        let mut detector = LapValidityDetector::new(LapValidityConfig::default());

        detector
            .process_packet(&dash_packet(1_000, 1, 10.0, 1.0, 1.0), &storage, "0.1.0")
            .expect("in race");

        let mut paused = dash_packet(1_100, 1, 10.0, 1.1, 1.1);
        paused.sled.is_race_on = 0;
        let pause_events = detector
            .process_packet(&paused, &storage, "0.1.0")
            .expect("pause");
        assert!(pause_events.is_empty());

        let resume_events = detector
            .process_packet(&dash_packet(1_200, 1, 12.0, 1.2, 1.2), &storage, "0.1.0")
            .expect("resume");
        assert!(resume_events.is_empty());
    }

    #[test]
    fn dirty_state_is_preserved_across_pause_resume() {
        let temp = TempDir::new().expect("temp dir");
        let storage = Storage::open(temp.path()).expect("storage");
        let mut detector = LapValidityDetector::new(LapValidityConfig::default());

        let mut contact = dash_packet(1_000, 1, 10.0, 1.0, 1.0);
        contact.sled.acceleration_z = 10.5 * 9.81;
        detector
            .process_packet(&contact, &storage, "0.1.0")
            .expect("contact");

        let mut paused = dash_packet(1_100, 1, 10.0, 1.1, 1.1);
        paused.sled.is_race_on = 0;
        detector
            .process_packet(&paused, &storage, "0.1.0")
            .expect("pause");

        let resume_events = detector
            .process_packet(&dash_packet(1_200, 1, 12.0, 1.2, 1.2), &storage, "0.1.0")
            .expect("resume");
        assert!(resume_events.is_empty());

        let session_id = first_session_id(&storage);
        let (valid, dirty_reason) = storage
            .read_lap_validity(session_id, 1)
            .expect("lap validity");
        assert!(!valid);
        assert_eq!(dirty_reason.as_deref(), Some("WallContact"));
    }

    #[test]
    fn dirty_lap_remains_invalid_when_lap_completes() {
        let temp = TempDir::new().expect("temp dir");
        let storage = Storage::open(temp.path()).expect("storage");
        let mut detector = LapValidityDetector::new(LapValidityConfig::default());

        let mut contact = dash_packet(1_000, 1, 10.0, 1.0, 1.0);
        contact.sled.acceleration_x = 11.0 * 9.81;
        detector
            .process_packet(&contact, &storage, "0.1.0")
            .expect("contact");

        detector
            .process_packet(&dash_packet(2_000, 2, 100.0, 0.2, 60.0), &storage, "0.1.0")
            .expect("lap boundary");

        let session_id = first_session_id(&storage);
        let (valid, dirty_reason) = storage
            .read_lap_validity(session_id, 1)
            .expect("lap 1 validity");
        assert!(!valid);
        assert_eq!(dirty_reason.as_deref(), Some("WallContact"));
    }

    fn off_track_packet(timestamp_ms: u32, lap_number: u16) -> DashPacket {
        let mut packet = dash_packet(timestamp_ms, lap_number, 10.0, 1.0, 1.0);
        packet.sled.wheel_on_rumble_strip_front_left = 1;
        packet.sled.wheel_on_rumble_strip_front_right = 1;
        packet
    }

    fn surface_rumble_packet(timestamp_ms: u32, lap_number: u16, rumble: f32) -> DashPacket {
        let mut packet = dash_packet(timestamp_ms, lap_number, 10.0, 1.0, 1.0);
        packet.sled.surface_rumble_front_left = rumble;
        packet.sled.surface_rumble_front_right = rumble;
        packet.sled.surface_rumble_rear_left = rumble;
        packet.sled.surface_rumble_rear_right = rumble;
        packet
    }

    fn first_session_id(storage: &Storage) -> i64 {
        assert_eq!(storage.count_sessions().expect("session count"), 1);
        storage.first_session_id().expect("first session id")
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
