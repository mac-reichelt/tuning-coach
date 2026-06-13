//! Symptom detectors.
//!
//! Two tiers, mirroring the user-facing timing model:
//!
//! * **Critical** — immediately-apparent, safety-relevant issues detected
//!   per-packet via [`CriticalState`] (bottoming, wheelspin, brake lockup).
//!   Emitted live during the lap.
//! * **Deferred** — balance / gearing / chassis-utilisation issues that need a
//!   whole lap (or a pause) of data, produced by [`analyze_lap`].
//!
//! Each detector follows the race-engineer pipeline: symptom → mechanism →
//! adjustment (preferred → fallback), with a confidence level and caveats.

use super::aggregate::{LapAggregate, Sample};
use super::thresholds;
use crate::recommendation::{
    AdjustmentPayload, RecommendationCategory, RecommendationConfidence, RecommendationUrgency,
};

/// An intermediate result the engine turns into a `RecommendationPayload`.
pub(super) struct Finding {
    pub category: RecommendationCategory,
    pub urgency: RecommendationUrgency,
    pub confidence: RecommendationConfidence,
    pub title: String,
    pub detected: String,
    pub cause: String,
    pub expected_outcome: String,
    pub adjustment: AdjustmentPayload,
    pub alternatives: Vec<AdjustmentPayload>,
    pub caveats: Vec<String>,
    /// Lower = surfaced first when more than `max_emit` deferred findings exist.
    pub priority: u8,
}

/// Build a directional adjustment. `from` is always `null` because the live
/// in-game setup values are not read yet (Phase 4); per ADR-0005 `to` is then a
/// signed delta in `unit` increments. The human-readable `summary` is the
/// source of truth the overlay renders.
fn adj(summary: &str, parameter: &str, to: f64, step: f64, unit: &str) -> AdjustmentPayload {
    AdjustmentPayload {
        summary: summary.to_string(),
        parameter: parameter.to_string(),
        from: None,
        to,
        step,
        unit: unit.to_string(),
    }
}

fn style_caveat() -> String {
    "Assumes a neutral/smooth driving style; style auto-detect is not yet active.".to_string()
}

// ── Critical detectors ──────────────────────────────────────────────────────

/// Counts consecutive "active" packets and fires once when a sustain window is
/// met, then re-arms only after the condition clears. This debounces single
/// noisy spikes and prevents continuous re-firing while the condition holds.
#[derive(Default)]
struct SustainLatch {
    count: u32,
    fired: bool,
}

impl SustainLatch {
    fn new() -> Self {
        Self {
            count: 0,
            fired: false,
        }
    }

    /// Returns `true` exactly once per sustained episode of `active`.
    fn update(&mut self, active: bool, needed: u32) -> bool {
        if !active {
            self.count = 0;
            self.fired = false;
            return false;
        }
        self.count += 1;
        if !self.fired && self.count >= needed {
            self.fired = true;
            return true;
        }
        false
    }
}

/// Per-packet critical-symptom state machine.
pub(super) struct CriticalState {
    bottom_front: SustainLatch,
    bottom_rear: SustainLatch,
    wheelspin: SustainLatch,
    lockup_front: SustainLatch,
    lockup_rear: SustainLatch,
}

impl CriticalState {
    pub(super) fn new() -> Self {
        Self {
            bottom_front: SustainLatch::new(),
            bottom_rear: SustainLatch::new(),
            wheelspin: SustainLatch::new(),
            lockup_front: SustainLatch::new(),
            lockup_rear: SustainLatch::new(),
        }
    }

    /// Advance every latch with this packet and return any findings that fired.
    pub(super) fn evaluate(&mut self, s: &Sample) -> Vec<Finding> {
        let mut out = Vec::new();
        let driving = s.speed >= thresholds::DRIVING_MIN_SPEED_MPS;

        // Bottoming — both corners of an axle pegged against the bump stop.
        let bf = driving
            && s.nst[0] >= thresholds::BOTTOMING_NST
            && s.nst[1] >= thresholds::BOTTOMING_NST;
        if self.bottom_front.update(bf, thresholds::BOTTOMING_SUSTAIN) {
            out.push(bottoming_finding(true, s.nst[0].max(s.nst[1])));
        }
        let br = driving
            && s.nst[2] >= thresholds::BOTTOMING_NST
            && s.nst[3] >= thresholds::BOTTOMING_NST;
        if self.bottom_rear.update(br, thresholds::BOTTOMING_SUSTAIN) {
            out.push(bottoming_finding(false, s.nst[2].max(s.nst[3])));
        }

        // Wheelspin — driven wheel spinning up under power.
        let driven_max = s
            .driven_wheels()
            .iter()
            .map(|&i| s.slip_ratio[i])
            .fold(f32::NEG_INFINITY, f32::max);
        let ws = driven_max >= thresholds::WHEELSPIN_SLIP_RATIO
            && s.throttle >= thresholds::WHEELSPIN_MIN_THROTTLE;
        if self.wheelspin.update(ws, thresholds::WHEELSPIN_SUSTAIN) {
            out.push(wheelspin_finding(s.drivetrain, driven_max, s.throttle));
        }

        // Brake lockup — wheel rotation collapsing under braking.
        let braking =
            s.brake >= thresholds::LOCKUP_MIN_BRAKE && s.speed >= thresholds::LOCKUP_MIN_SPEED_MPS;
        let lf = braking
            && (s.slip_ratio[0] <= thresholds::LOCKUP_SLIP_RATIO
                || s.slip_ratio[1] <= thresholds::LOCKUP_SLIP_RATIO);
        let lr = braking
            && (s.slip_ratio[2] <= thresholds::LOCKUP_SLIP_RATIO
                || s.slip_ratio[3] <= thresholds::LOCKUP_SLIP_RATIO);
        if self.lockup_front.update(lf, thresholds::LOCKUP_SUSTAIN) {
            out.push(lockup_finding(true, s.brake, s.speed));
        }
        if self.lockup_rear.update(lr, thresholds::LOCKUP_SUSTAIN) {
            out.push(lockup_finding(false, s.brake, s.speed));
        }

        out
    }
}

fn bottoming_finding(front: bool, nst: f32) -> Finding {
    let axle = if front { "front" } else { "rear" };
    let axle_cap = if front { "Front" } else { "Rear" };
    Finding {
        category: RecommendationCategory::RideHeight,
        urgency: RecommendationUrgency::Critical,
        confidence: RecommendationConfidence::High,
        title: format!("{axle_cap} suspension bottoming"),
        detected: format!(
            "{axle_cap} suspension travel pegged at ~{:.0}% (≥{:.0}%) while loaded.",
            nst * 100.0,
            thresholds::BOTTOMING_NST * 100.0,
        ),
        cause: format!(
            "Insufficient {axle} ride height / spring rate for the load — the chassis is hitting the bump stop."
        ),
        expected_outcome:
            "Stops the suspension slamming the bump stop and restores consistent contact-patch load."
                .to_string(),
        adjustment: adj(
            &format!("Raise {axle} ride height ~3 mm"),
            &format!("ride_height_{axle}"),
            3.0,
            1.0,
            "mm (Δ)",
        ),
        alternatives: vec![adj(
            &format!("Stiffen {axle} springs ~5%"),
            &format!("spring_rate_{axle}"),
            5.0,
            1.0,
            "% (Δ)",
        )],
        caveats: vec![
            "Raise ride height first, then stiffen springs/bump if it persists (research §3.3)."
                .to_string(),
            style_caveat(),
        ],
        priority: 0,
    }
}

fn wheelspin_finding(drivetrain: i32, slip: f32, throttle: f32) -> Finding {
    let mut caveats = vec![
        "Differential lock moves in 2% steps — use even numbers (research §2).".to_string(),
        style_caveat(),
    ];
    if drivetrain == 0 {
        caveats.push(
            "FWD car: this is front-wheel scrabble — also consider more front mechanical grip."
                .to_string(),
        );
    }
    Finding {
        category: RecommendationCategory::Differential,
        urgency: RecommendationUrgency::Critical,
        confidence: RecommendationConfidence::High,
        title: "Wheelspin on power".to_string(),
        detected: format!(
            "Driven-wheel slip ratio {slip:.2} (≥{:.2}) at {:.0}% throttle.",
            thresholds::WHEELSPIN_SLIP_RATIO,
            throttle * 100.0,
        ),
        cause: "Torque is exceeding driven-axle grip — accel diff lock too high or throttle applied too early."
            .to_string(),
        expected_outcome:
            "Reduces wheelspin off the corner, improving drive and rear tyre life.".to_string(),
        adjustment: adj(
            "Lower accel diff lock ~6%",
            "diff_accel_lock",
            -6.0,
            2.0,
            "% (Δ)",
        ),
        alternatives: vec![adj(
            "Feed the throttle in more progressively on exit (driver)",
            "throttle_application",
            0.0,
            0.0,
            "",
        )],
        caveats,
        priority: 0,
    }
}

fn lockup_finding(front: bool, brake: f32, speed: f32) -> Finding {
    let (axle_cap, cause, outcome, to) = if front {
        (
            "Front",
            "Brake bias too far forward — the fronts are locking under braking.",
            "Stops front lockup and flat-spotting and shortens the braking distance.",
            -2.0,
        )
    } else {
        (
            "Rear",
            "Brake bias too far rearward — the rears are locking and stepping out.",
            "Stabilises the rear under braking and prevents snap rotation.",
            2.0,
        )
    };
    let direction = if front { "rearward" } else { "forward" };
    Finding {
        category: RecommendationCategory::Brakes,
        urgency: RecommendationUrgency::Critical,
        confidence: RecommendationConfidence::High,
        title: format!("{axle_cap} brake lockup"),
        detected: format!(
            "{axle_cap} wheel slip ratio ≤ {:.2} under {:.0}% brake at {:.0} km/h.",
            thresholds::LOCKUP_SLIP_RATIO,
            brake * 100.0,
            speed * 3.6,
        ),
        cause: cause.to_string(),
        expected_outcome: outcome.to_string(),
        adjustment: adj(
            &format!("Shift brake balance ~2% {direction}"),
            "brake_balance",
            to,
            1.0,
            "% front (Δ)",
        ),
        alternatives: vec![adj(
            "Lower overall brake pressure ~5%",
            "brake_pressure",
            -5.0,
            5.0,
            "% (Δ)",
        )],
        caveats: vec![
            "Lockup should only occur in the last 10–15% of pedal travel (research §3.8)."
                .to_string(),
            style_caveat(),
        ],
        priority: 0,
    }
}

// ── Deferred analysers ──────────────────────────────────────────────────────

/// Run every deferred analyser over a completed (or partial) lap, returning the
/// highest-priority `max_emit` findings. Caller guarantees the lap is clean.
pub(super) fn analyze_lap(agg: &LapAggregate, max_emit: usize) -> Vec<Finding> {
    let mut findings = Vec::new();

    // Cornering balance (understeer / oversteer).
    if agg.cornering_samples >= thresholds::MIN_CORNERING_SAMPLES {
        let cs = f64::from(agg.cornering_samples);
        let front = agg.front_sa_sum / cs;
        let rear = agg.rear_sa_sum / cs;
        let margin = front - rear;
        if margin >= thresholds::BALANCE_MARGIN_RAD {
            findings.push(understeer_finding(margin, agg.cornering_samples));
        } else if -margin >= thresholds::BALANCE_MARGIN_RAD {
            findings.push(oversteer_finding(-margin, agg.cornering_samples));
        }
    }

    // Gearing — under-revving in top gear on the straight.
    if agg.max_gear >= 2
        && agg.top_gear_throttle_samples >= thresholds::GEAR_MIN_SAMPLES
        && agg.max_rpm_frac_top_gear < thresholds::GEAR_UNDER_REV_FRAC
    {
        findings.push(gearing_finding(agg.max_rpm_frac_top_gear, agg.max_gear));
    }

    // Mild chassis bottoming (not severe enough for a live critical).
    if agg.driving_samples >= thresholds::MIN_DRIVING_SAMPLES {
        let ds = agg.driving_samples as f32;
        let front_frac = agg.front_near_bump as f32 / ds;
        let rear_frac = agg.rear_near_bump as f32 / ds;
        if front_frac >= thresholds::NEAR_BUMP_FRACTION {
            findings.push(mild_bottoming_finding(true, front_frac));
        } else if rear_frac >= thresholds::NEAR_BUMP_FRACTION {
            findings.push(mild_bottoming_finding(false, rear_frac));
        }
    }

    findings.sort_by_key(|f| f.priority);
    findings.truncate(max_emit);
    findings
}

fn understeer_finding(margin_rad: f64, samples: u32) -> Finding {
    let confidence = if margin_rad >= thresholds::BALANCE_STRONG_RAD {
        RecommendationConfidence::High
    } else {
        RecommendationConfidence::Medium
    };
    Finding {
        category: RecommendationCategory::AntiRoll,
        urgency: RecommendationUrgency::Deferred,
        confidence,
        title: "Mid-corner understeer".to_string(),
        detected: format!(
            "Front slip angle averaged {:.1}° more than rear across {samples} cornering samples this lap.",
            margin_rad.to_degrees(),
        ),
        cause: "The front axle reaches its grip limit before the rear — front-limited balance."
            .to_string(),
        expected_outcome:
            "Shifts lateral load transfer rearward so the front tyres keep more grip mid-corner."
                .to_string(),
        adjustment: adj(
            "Soften front ARB ~2 clicks",
            "anti_roll_front",
            -2.0,
            1.0,
            "clicks (Δ)",
        ),
        alternatives: vec![
            adj(
                "Stiffen rear ARB ~2 clicks",
                "anti_roll_rear",
                2.0,
                1.0,
                "clicks (Δ)",
            ),
            adj(
                "Lower front tyre pressure ~0.5 psi",
                "tire_pressure_front",
                -0.5,
                0.5,
                "psi (Δ)",
            ),
        ],
        caveats: vec![
            "Balance lever order: ARB → springs → pressure → camber (research §3).".to_string(),
            "Tyre pressure is not in the UDP stream — adjust by feel from the Heat page."
                .to_string(),
            style_caveat(),
        ],
        priority: 0,
    }
}

fn oversteer_finding(margin_rad: f64, samples: u32) -> Finding {
    let confidence = if margin_rad >= thresholds::BALANCE_STRONG_RAD {
        RecommendationConfidence::High
    } else {
        RecommendationConfidence::Medium
    };
    Finding {
        category: RecommendationCategory::AntiRoll,
        urgency: RecommendationUrgency::Deferred,
        confidence,
        title: "Corner oversteer".to_string(),
        detected: format!(
            "Rear slip angle averaged {:.1}° more than front across {samples} cornering samples this lap.",
            margin_rad.to_degrees(),
        ),
        cause: "The rear axle reaches its grip limit before the front — rear-limited balance."
            .to_string(),
        expected_outcome:
            "Shifts lateral load transfer forward so the rear tyres keep more grip.".to_string(),
        adjustment: adj(
            "Soften rear ARB ~2 clicks",
            "anti_roll_rear",
            -2.0,
            1.0,
            "clicks (Δ)",
        ),
        alternatives: vec![
            adj(
                "Lower rear accel diff lock ~4%",
                "diff_accel_lock",
                -4.0,
                2.0,
                "% (Δ)",
            ),
            adj(
                "Add rear toe-in ~0.1°",
                "toe_rear",
                0.1,
                0.1,
                "° (Δ)",
            ),
        ],
        caveats: vec![
            "If it is power-on oversteer, prefer the diff/toe alternatives (research §3.4/§3.7)."
                .to_string(),
            style_caveat(),
        ],
        priority: 0,
    }
}

fn gearing_finding(frac: f32, max_gear: u8) -> Finding {
    Finding {
        category: RecommendationCategory::Gearing,
        urgency: RecommendationUrgency::Deferred,
        confidence: RecommendationConfidence::High,
        title: "Under-geared on the straight".to_string(),
        detected: format!(
            "Peaked at only {:.0}% of redline in top gear (gear {max_gear}) at full throttle this lap.",
            frac * 100.0,
        ),
        cause: "Final drive is too long — the car never reaches the rev limiter at the end of the straight."
            .to_string(),
        expected_outcome:
            "Brings the car to redline as it crosses the line at the straight's end, maximising acceleration."
                .to_string(),
        adjustment: adj(
            "Raise final drive ratio ~0.20",
            "final_drive",
            0.20,
            0.01,
            "ratio (Δ)",
        ),
        alternatives: Vec::new(),
        caveats: vec![
            "Aim to just touch the limiter at the end of the longest straight (research §3.6)."
                .to_string(),
            "Re-check gearing after any aero/drag change.".to_string(),
        ],
        priority: 1,
    }
}

fn mild_bottoming_finding(front: bool, frac: f32) -> Finding {
    let axle = if front { "front" } else { "rear" };
    let axle_cap = if front { "Front" } else { "Rear" };
    Finding {
        category: RecommendationCategory::RideHeight,
        urgency: RecommendationUrgency::Deferred,
        confidence: RecommendationConfidence::Medium,
        title: format!("{axle_cap} suspension running low on travel"),
        detected: format!(
            "{axle_cap} suspension within {:.0}% of the bump stop on {:.0}% of driving samples this lap.",
            (1.0 - thresholds::NEAR_BUMP_NST) * 100.0,
            frac * 100.0,
        ),
        cause: format!(
            "{axle_cap} ride height / spring rate is marginal for the load — little travel left for bumps and kerbs."
        ),
        expected_outcome: "Keeps the suspension inside its 20–80% travel window over bumps and kerbs."
            .to_string(),
        adjustment: adj(
            &format!("Raise {axle} ride height ~2 mm"),
            &format!("ride_height_{axle}"),
            2.0,
            1.0,
            "mm (Δ)",
        ),
        alternatives: vec![adj(
            &format!("Stiffen {axle} springs ~5%"),
            &format!("spring_rate_{axle}"),
            5.0,
            1.0,
            "% (Δ)",
        )],
        caveats: vec![
            "Target 20–80% suspension travel in normal driving (research §3.3).".to_string(),
            style_caveat(),
        ],
        priority: 2,
    }
}
