//! Heuristics engine — turns the live FM 2023 telemetry stream into tuning
//! recommendations.
//!
//! Design goals (from the user requirement + `race-engineer.agent.md`):
//!
//! * **Critical** issues that are immediately apparent and safety-relevant
//!   (bottoming, wheelspin, brake lockup) are surfaced **live** during the lap,
//!   debounced so they fire once per episode and at most once per category per
//!   [`thresholds::CRITICAL_COOLDOWN_MS`].
//! * **Deferred** issues (cornering balance, gearing, chassis utilisation) need
//!   a whole lap of data and are held until the lap **completes** or the session
//!   **pauses/finishes** — never interrupting a flying lap.
//!
//! The engine is pure and synchronous: feed it packets and session-state
//! transitions, get back `RecommendationPayload`s to emit. All channel wiring
//! lives in `main.rs::heuristics_loop`.

mod aggregate;
mod detectors;
mod thresholds;

use std::collections::HashMap;

use crate::recommendation::{RecommendationCategory, RecommendationPayload};
use crate::session_state::SessionState;
use crate::telemetry::DashPacket;

use aggregate::{LapAggregate, Sample};
use detectors::{analyze_lap, CriticalState, Finding};

/// Rolling state for one telemetry feed.
pub(crate) struct HeuristicsEngine {
    /// Active session row id (0 until a session starts).
    session_id: i64,
    /// Monotonic counter for unique recommendation ids.
    id_counter: u64,
    /// Last-seen lap number, for lap-boundary detection.
    lap_number: Option<u16>,
    /// Whether the in-progress lap is dirty (off-track / rewind / reset).
    current_lap_dirty: bool,
    /// Per-lap rolling statistics.
    aggregate: LapAggregate,
    /// Per-packet critical-symptom latches.
    critical: CriticalState,
    /// Last emit time per critical category, for cooldown.
    last_critical_emit_ms: HashMap<&'static str, u64>,
}

impl HeuristicsEngine {
    pub(crate) fn new() -> Self {
        Self {
            session_id: 0,
            id_counter: 0,
            lap_number: None,
            current_lap_dirty: false,
            aggregate: LapAggregate::new(),
            critical: CriticalState::new(),
            last_critical_emit_ms: HashMap::new(),
        }
    }

    /// Mark the in-progress lap dirty so its deferred analysis is discarded.
    pub(crate) fn mark_lap_dirty(&mut self) {
        self.current_lap_dirty = true;
    }

    /// Clear the dirty flag (e.g. a fresh clean lap was marked).
    pub(crate) fn mark_lap_clean(&mut self) {
        self.current_lap_dirty = false;
    }

    /// Process one telemetry packet.
    ///
    /// Returns critical recommendations to emit immediately, plus — when this
    /// packet crosses a lap boundary — the deferred recommendations for the lap
    /// that just completed.
    pub(crate) fn on_packet(
        &mut self,
        dash: &DashPacket,
        now_ms: u64,
    ) -> Vec<RecommendationPayload> {
        let mut out = Vec::new();

        // Lap-boundary detection: flush the completed lap, then reset.
        if let Some(prev) = self.lap_number {
            if dash.lap_number != prev {
                // Only analyse a genuine completed flying lap (prev ≥ 1 skips the
                // out-lap) that stayed clean.
                if dash.lap_number > prev && prev >= 1 {
                    out.extend(self.flush_deferred(dash, now_ms, thresholds::MAX_DEFERRED_PER_LAP));
                }
                self.aggregate = LapAggregate::new();
                self.current_lap_dirty = false;
            }
        }
        self.lap_number = Some(dash.lap_number);

        let sample = Sample::from_dash(dash);
        self.aggregate.push(&sample);

        for finding in self.critical.evaluate(&sample) {
            if let Some(rec) = self.maybe_emit_critical(finding, dash, now_ms) {
                out.push(rec);
            }
        }

        out
    }

    /// React to a session-state transition.
    ///
    /// On a new session, rolling state is reset. On pause/finish the current
    /// partial lap is flushed so the driver gets feedback at the pause screen.
    pub(crate) fn on_session_state(
        &mut self,
        to: SessionState,
        session_id: i64,
        dash: Option<&DashPacket>,
        now_ms: u64,
    ) -> Vec<RecommendationPayload> {
        if session_id != 0 && session_id != self.session_id {
            self.session_id = session_id;
            self.aggregate = LapAggregate::new();
            self.lap_number = None;
            self.current_lap_dirty = false;
            self.last_critical_emit_ms.clear();
        }

        match to {
            SessionState::Paused | SessionState::Finished => {
                let recs = match dash {
                    Some(dash) => {
                        self.flush_deferred(dash, now_ms, thresholds::MAX_DEFERRED_PER_PAUSE)
                    }
                    None => Vec::new(),
                };
                if matches!(to, SessionState::Finished) {
                    self.aggregate = LapAggregate::new();
                }
                recs
            }
            SessionState::Loading | SessionState::InRace => Vec::new(),
        }
    }

    fn maybe_emit_critical(
        &mut self,
        finding: Finding,
        dash: &DashPacket,
        now_ms: u64,
    ) -> Option<RecommendationPayload> {
        let key = category_key(&finding.category);
        let last = self.last_critical_emit_ms.get(key).copied();
        if let Some(last) = last {
            if now_ms.saturating_sub(last) < thresholds::CRITICAL_COOLDOWN_MS {
                return None;
            }
        }
        self.last_critical_emit_ms.insert(key, now_ms);
        Some(self.finalize(finding, dash, now_ms))
    }

    fn flush_deferred(
        &mut self,
        dash: &DashPacket,
        now_ms: u64,
        max: usize,
    ) -> Vec<RecommendationPayload> {
        if self.current_lap_dirty {
            return Vec::new();
        }
        let findings = analyze_lap(&self.aggregate, max);
        findings
            .into_iter()
            .map(|f| self.finalize(f, dash, now_ms))
            .collect()
    }

    fn finalize(&mut self, f: Finding, dash: &DashPacket, now_ms: u64) -> RecommendationPayload {
        self.id_counter += 1;
        RecommendationPayload {
            id: format!("R{now_ms:013}{:06}", self.id_counter),
            session_id: self.session_id.to_string(),
            lap_number: u32::from(dash.lap_number),
            category: f.category,
            title: f.title,
            detected: f.detected,
            cause: f.cause,
            adjustment: f.adjustment,
            expected_outcome: f.expected_outcome,
            confidence: f.confidence,
            caveats: f.caveats,
            alternatives: f.alternatives,
            driving_style_assumed: "smooth".to_string(),
            locked_fallback_used: false,
            corners: Vec::new(),
            needs_setup_form: false,
            tire_wear_max_at_emit: max_tire_wear(dash),
            urgency: f.urgency,
        }
    }
}

fn max_tire_wear(d: &DashPacket) -> f32 {
    [
        d.tire_wear_front_left,
        d.tire_wear_front_right,
        d.tire_wear_rear_left,
        d.tire_wear_rear_right,
    ]
    .into_iter()
    .flatten()
    .fold(0.0_f32, f32::max)
}

/// Stable snake_case key for cooldown bookkeeping (matches serde serialisation).
fn category_key(c: &RecommendationCategory) -> &'static str {
    match c {
        RecommendationCategory::Springs => "springs",
        RecommendationCategory::Damping => "damping",
        RecommendationCategory::AntiRoll => "anti_roll",
        RecommendationCategory::RideHeight => "ride_height",
        RecommendationCategory::Brakes => "brakes",
        RecommendationCategory::Tires => "tires",
        RecommendationCategory::Gearing => "gearing",
        RecommendationCategory::Alignment => "alignment",
        RecommendationCategory::Aero => "aero",
        RecommendationCategory::Differential => "differential",
        RecommendationCategory::Engine => "engine",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recommendation::{RecommendationConfidence, RecommendationUrgency};
    use crate::telemetry::DashPacket;

    /// A neutral "driving in a straight line" packet on lap 1, with all
    /// suspension corners at 40% travel (well off the bump stops).
    fn base_packet() -> DashPacket {
        DashPacket {
            sled: crate::telemetry::SledPacket {
                is_race_on: 1,
                car_ordinal: 100,
                drivetrain_type: 1, // RWD
                engine_max_rpm: 8000.0,
                current_engine_rpm: 4000.0,
                normalized_suspension_travel_front_left: 0.4,
                normalized_suspension_travel_front_right: 0.4,
                normalized_suspension_travel_rear_left: 0.4,
                normalized_suspension_travel_rear_right: 0.4,
                ..Default::default()
            },
            speed: 40.0,
            gear: 4,
            accel: 200,
            lap_number: 1,
            ..Default::default()
        }
    }

    fn feed(engine: &mut HeuristicsEngine, p: &DashPacket, n: u32) -> Vec<RecommendationPayload> {
        let mut out = Vec::new();
        for _ in 0..n {
            out.extend(engine.on_packet(p, 1_000));
        }
        out
    }

    #[test]
    fn front_bottoming_fires_after_sustain_window() {
        let mut engine = HeuristicsEngine::new();
        engine.on_session_state(SessionState::InRace, 1, None, 0);

        let mut p = base_packet();
        p.sled.normalized_suspension_travel_front_left = 1.0;
        p.sled.normalized_suspension_travel_front_right = 0.99;

        // Below the sustain window — nothing yet.
        let early = feed(&mut engine, &p, thresholds::BOTTOMING_SUSTAIN - 1);
        assert!(early.is_empty(), "should not fire before sustain window");

        // One more packet crosses the threshold.
        let fired = engine.on_packet(&p, 1_000);
        assert_eq!(fired.len(), 1);
        let rec = &fired[0];
        assert_eq!(rec.category, RecommendationCategory::RideHeight);
        assert_eq!(rec.urgency, RecommendationUrgency::Critical);
        assert_eq!(rec.confidence, RecommendationConfidence::High);
        assert!(rec.title.to_lowercase().contains("front"));
    }

    #[test]
    fn critical_respects_per_category_cooldown() {
        let mut engine = HeuristicsEngine::new();
        engine.on_session_state(SessionState::InRace, 1, None, 0);

        let mut p = base_packet();
        p.sled.normalized_suspension_travel_front_left = 1.0;
        p.sled.normalized_suspension_travel_front_right = 1.0;

        // Fire once.
        let mut fired = 0;
        for _ in 0..(thresholds::BOTTOMING_SUSTAIN + 5) {
            fired += engine.on_packet(&p, 1_000).len();
        }
        assert_eq!(fired, 1, "latch fires once per episode");

        // Clear the condition (re-arm the latch) then re-trigger within cooldown.
        let clear = base_packet();
        feed(&mut engine, &clear, 5);
        let mut refired = 0;
        for _ in 0..(thresholds::BOTTOMING_SUSTAIN + 5) {
            // 5s later — still inside the 20s cooldown.
            refired += engine.on_packet(&p, 6_000).len();
        }
        assert_eq!(refired, 0, "cooldown suppresses same-category re-fire");
    }

    #[test]
    fn wheelspin_fires_on_driven_axle_under_throttle() {
        let mut engine = HeuristicsEngine::new();
        engine.on_session_state(SessionState::InRace, 1, None, 0);

        let mut p = base_packet();
        p.accel = 255;
        p.speed = 20.0;
        // RWD → rear wheels spin.
        p.sled.tire_slip_ratio_rear_left = 0.45;
        p.sled.tire_slip_ratio_rear_right = 0.45;

        let recs = feed(&mut engine, &p, thresholds::WHEELSPIN_SUSTAIN + 1);
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].category, RecommendationCategory::Differential);
        assert_eq!(recs[0].urgency, RecommendationUrgency::Critical);
    }

    #[test]
    fn front_brake_lockup_recommends_rearward_bias() {
        let mut engine = HeuristicsEngine::new();
        engine.on_session_state(SessionState::InRace, 1, None, 0);

        let mut p = base_packet();
        p.accel = 0;
        p.brake = 220;
        p.speed = 50.0;
        p.sled.tire_slip_ratio_front_left = -0.4;

        let recs = feed(&mut engine, &p, thresholds::LOCKUP_SUSTAIN + 1);
        assert_eq!(recs.len(), 1);
        let rec = &recs[0];
        assert_eq!(rec.category, RecommendationCategory::Brakes);
        assert!(rec.adjustment.summary.contains("rearward"));
        assert!(rec.adjustment.to < 0.0, "front lockup shifts bias rearward");
    }

    #[test]
    fn understeer_is_deferred_until_lap_completes() {
        let mut engine = HeuristicsEngine::new();
        engine.on_session_state(SessionState::InRace, 1, None, 0);

        // A cornering packet with front slipping much more than rear.
        let mut corner = base_packet();
        corner.sled.acceleration_x = 12.0; // ~1.2 g lateral
        corner.sled.tire_slip_angle_front_left = 0.12;
        corner.sled.tire_slip_angle_front_right = 0.12;
        corner.sled.tire_slip_angle_rear_left = 0.05;
        corner.sled.tire_slip_angle_rear_right = 0.05;

        // Accumulate cornering data — nothing emitted mid-lap.
        let mid = feed(&mut engine, &corner, thresholds::MIN_CORNERING_SAMPLES + 20);
        assert!(mid.is_empty(), "balance findings must not fire mid-lap");

        // Cross into lap 2 → deferred analysis runs for the completed lap.
        let mut next_lap = corner.clone();
        next_lap.lap_number = 2;
        let recs = engine.on_packet(&next_lap, 2_000);
        assert!(
            recs.iter()
                .any(|r| r.category == RecommendationCategory::AntiRoll
                    && r.urgency == RecommendationUrgency::Deferred
                    && r.title.to_lowercase().contains("understeer")),
            "expected a deferred understeer recommendation at lap completion"
        );
    }

    #[test]
    fn dirty_lap_suppresses_deferred_analysis() {
        let mut engine = HeuristicsEngine::new();
        engine.on_session_state(SessionState::InRace, 1, None, 0);

        let mut corner = base_packet();
        corner.sled.acceleration_x = 12.0;
        corner.sled.tire_slip_angle_front_left = 0.12;
        corner.sled.tire_slip_angle_front_right = 0.12;
        corner.sled.tire_slip_angle_rear_left = 0.05;
        corner.sled.tire_slip_angle_rear_right = 0.05;
        feed(&mut engine, &corner, thresholds::MIN_CORNERING_SAMPLES + 20);

        engine.mark_lap_dirty();

        let mut next_lap = corner.clone();
        next_lap.lap_number = 2;
        let recs = engine.on_packet(&next_lap, 2_000);
        assert!(
            recs.is_empty(),
            "dirty lap must not produce recommendations"
        );
    }

    #[test]
    fn pause_flushes_partial_lap_deferred() {
        let mut engine = HeuristicsEngine::new();
        engine.on_session_state(SessionState::InRace, 1, None, 0);

        let mut corner = base_packet();
        corner.sled.acceleration_x = -12.0; // sign-independent
        corner.sled.tire_slip_angle_rear_left = 0.13;
        corner.sled.tire_slip_angle_rear_right = 0.13;
        corner.sled.tire_slip_angle_front_left = 0.05;
        corner.sled.tire_slip_angle_front_right = 0.05;
        feed(&mut engine, &corner, thresholds::MIN_CORNERING_SAMPLES + 20);

        let recs = engine.on_session_state(SessionState::Paused, 1, Some(&corner), 5_000);
        assert!(
            recs.iter()
                .any(|r| r.title.to_lowercase().contains("oversteer")),
            "pausing mid-lap should flush a deferred oversteer finding"
        );
    }

    #[test]
    fn gearing_under_rev_detected_at_lap_end() {
        let mut engine = HeuristicsEngine::new();
        engine.on_session_state(SessionState::InRace, 1, None, 0);

        let mut top = base_packet();
        top.gear = 6;
        top.accel = 255;
        top.brake = 0;
        top.sled.engine_max_rpm = 8000.0;
        top.sled.current_engine_rpm = 6000.0; // 75% of redline — under-geared
        feed(&mut engine, &top, thresholds::GEAR_MIN_SAMPLES + 10);

        let mut next_lap = top.clone();
        next_lap.lap_number = 2;
        let recs = engine.on_packet(&next_lap, 2_000);
        assert!(
            recs.iter()
                .any(|r| r.category == RecommendationCategory::Gearing
                    && r.urgency == RecommendationUrgency::Deferred),
            "expected a deferred gearing recommendation"
        );
    }

    #[test]
    fn out_lap_is_not_analysed() {
        let mut engine = HeuristicsEngine::new();
        engine.on_session_state(SessionState::InRace, 1, None, 0);

        // Lap 0 (out lap) with strong understeer signature.
        let mut corner = base_packet();
        corner.lap_number = 0;
        corner.sled.acceleration_x = 12.0;
        corner.sled.tire_slip_angle_front_left = 0.12;
        corner.sled.tire_slip_angle_front_right = 0.12;
        feed(&mut engine, &corner, thresholds::MIN_CORNERING_SAMPLES + 20);

        // Cross 0 → 1: the out lap must not be analysed.
        let mut lap1 = corner.clone();
        lap1.lap_number = 1;
        let recs = engine.on_packet(&lap1, 2_000);
        assert!(recs.is_empty(), "out lap (lap 0) must not be analysed");
    }
}
