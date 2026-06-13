//! Tunable thresholds for the heuristics engine.
//!
//! Every constant is keyed to
//! `docs/research/fm2023-tunable-values-and-telemetry-optimization.md` (cited as
//! "research §N" below) and to the symptom→mechanism rules in
//! `.github/agents/race-engineer.agent.md`.
//!
//! Sample-rate assumption: Forza emits UDP at ~60 Hz (research §1.2). Sustain
//! windows below are expressed in packet counts; ~15 packets ≈ 0.25 s.

// ── Critical: suspension bottoming (research §3.3) ──────────────────────────

/// `NormalizedSuspensionTravel` value treated as bottoming. Research §3.3:
/// "reaching 1.0 = bottoming". Kept just below 1.0 to tolerate quantisation.
pub(super) const BOTTOMING_NST: f32 = 0.98;
/// Packets an axle must stay bottomed before a critical fires (~0.25 s).
pub(super) const BOTTOMING_SUSTAIN: u32 = 15;

// ── Critical: wheelspin (research §3.7) ─────────────────────────────────────

/// Driven-wheel slip ratio treated as severe wheelspin. Research §3.7 uses 0.15
/// for "wheelspin"; 0.30 is an unambiguous loss of traction worth interrupting.
pub(super) const WHEELSPIN_SLIP_RATIO: f32 = 0.30;
pub(super) const WHEELSPIN_SUSTAIN: u32 = 12;
/// Throttle (0..1) above which wheelspin is attributed to power-down.
pub(super) const WHEELSPIN_MIN_THROTTLE: f32 = 0.70;

// ── Critical: brake lockup (research §3.8) ──────────────────────────────────

/// Slip ratio (negative) treated as lockup. Research §3.8: wheel rotation → 0
/// under braking.
pub(super) const LOCKUP_SLIP_RATIO: f32 = -0.25;
pub(super) const LOCKUP_SUSTAIN: u32 = 10;
pub(super) const LOCKUP_MIN_BRAKE: f32 = 0.40;
pub(super) const LOCKUP_MIN_SPEED_MPS: f32 = 8.0;

// ── Shared: only analyse signals while actually driving ─────────────────────

/// Speed (m/s) above which a sample counts as "driving" (filters pit/garage).
pub(super) const DRIVING_MIN_SPEED_MPS: f32 = 5.0;

// ── Deferred: cornering balance (research §3.2/§3.4, race-engineer table) ────

/// Lateral g above which a sample counts as "mid-corner".
pub(super) const CORNERING_MIN_LAT_G: f32 = 0.60;
pub(super) const CORNERING_MIN_SPEED_MPS: f32 = 12.0;
/// Minimum cornering samples before a balance verdict is trusted.
pub(super) const MIN_CORNERING_SAMPLES: u32 = 40;
/// Front-minus-rear mean slip-angle margin (radians) for an understeer/oversteer
/// verdict. ~0.020 rad ≈ 1.1°. STRONG raises confidence from medium to high.
pub(super) const BALANCE_MARGIN_RAD: f64 = 0.020;
pub(super) const BALANCE_STRONG_RAD: f64 = 0.045;

// ── Deferred: gearing (research §3.6) ───────────────────────────────────────

/// Top-gear full-throttle samples required before judging the final drive.
pub(super) const GEAR_MIN_SAMPLES: u32 = 30;
/// Redline fraction below which the car is "under-geared" on the straight
/// (research §3.6: validate it just reaches redline; <90% = final drive too long).
pub(super) const GEAR_UNDER_REV_FRAC: f32 = 0.90;

// ── Deferred: mild bump-stop proximity (research §3.3, target 20–80% travel) ─

pub(super) const NEAR_BUMP_NST: f32 = 0.92;
pub(super) const NEAR_BUMP_FRACTION: f32 = 0.15;
/// Minimum driving samples before mild-bottoming analysis is trusted.
pub(super) const MIN_DRIVING_SAMPLES: u32 = 60;

// ── Emission control (race-engineer: never stack 5 recommendations) ─────────

/// Cooldown between two critical recommendations of the same category.
pub(super) const CRITICAL_COOLDOWN_MS: u64 = 20_000;
/// Max deferred recommendations emitted at a lap boundary.
pub(super) const MAX_DEFERRED_PER_LAP: usize = 2;
/// Max deferred recommendations emitted when the session pauses/finishes (more
/// time for the driver to read).
pub(super) const MAX_DEFERRED_PER_PAUSE: usize = 3;
