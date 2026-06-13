//! Per-packet signal extraction and per-lap aggregation.
//!
//! [`Sample`] derives the handful of normalised signals the detectors need from
//! a raw [`DashPacket`]; [`LapAggregate`] rolls them up across a lap (or a
//! partial lap, when flushed at a pause).

use super::thresholds;
use crate::telemetry::DashPacket;

/// Standard gravity, used to convert Forza's m/s² accelerations to g.
const G: f32 = 9.81;

/// Wheel index order used throughout: 0=FL, 1=FR, 2=RL, 3=RR.
const FL: usize = 0;
const FR: usize = 1;
const RL: usize = 2;
const RR: usize = 3;

/// Normalised per-packet signals derived from a [`DashPacket`].
pub(super) struct Sample {
    /// Lateral acceleration in g (Forza `AccelerationX`).
    pub lat_g: f32,
    /// Throttle fraction `0..1`.
    pub throttle: f32,
    /// Brake fraction `0..1`.
    pub brake: f32,
    /// Body speed in m/s.
    pub speed: f32,
    /// Current forward gear (`1..=10`; `0`=R, `11`=N).
    pub gear: u8,
    /// `CurrentEngineRpm / EngineMaxRpm`, clamped to `0..2`.
    pub rpm_frac: f32,
    /// `NormalizedSuspensionTravel` per corner (`0`=extended, `1`=compressed).
    pub nst: [f32; 4],
    /// `TireSlipRatio` per corner (+wheelspin / −lockup).
    pub slip_ratio: [f32; 4],
    /// `|TireSlipAngle|` per corner.
    pub slip_angle_abs: [f32; 4],
    /// `DrivetrainType` (0=FWD, 1=RWD, 2=AWD).
    pub drivetrain: i32,
}

impl Sample {
    pub(super) fn from_dash(d: &DashPacket) -> Self {
        let s = &d.sled;
        let rpm_frac = if s.engine_max_rpm > 1.0 {
            (s.current_engine_rpm / s.engine_max_rpm).clamp(0.0, 2.0)
        } else {
            0.0
        };
        Self {
            lat_g: s.acceleration_x / G,
            throttle: f32::from(d.accel) / 255.0,
            brake: f32::from(d.brake) / 255.0,
            speed: d.speed,
            gear: d.gear,
            rpm_frac,
            nst: [
                s.normalized_suspension_travel_front_left,
                s.normalized_suspension_travel_front_right,
                s.normalized_suspension_travel_rear_left,
                s.normalized_suspension_travel_rear_right,
            ],
            slip_ratio: [
                s.tire_slip_ratio_front_left,
                s.tire_slip_ratio_front_right,
                s.tire_slip_ratio_rear_left,
                s.tire_slip_ratio_rear_right,
            ],
            slip_angle_abs: [
                s.tire_slip_angle_front_left.abs(),
                s.tire_slip_angle_front_right.abs(),
                s.tire_slip_angle_rear_left.abs(),
                s.tire_slip_angle_rear_right.abs(),
            ],
            drivetrain: s.drivetrain_type,
        }
    }

    pub(super) fn front_slip_angle(&self) -> f32 {
        (self.slip_angle_abs[FL] + self.slip_angle_abs[FR]) * 0.5
    }

    pub(super) fn rear_slip_angle(&self) -> f32 {
        (self.slip_angle_abs[RL] + self.slip_angle_abs[RR]) * 0.5
    }

    pub(super) fn is_cornering(&self) -> bool {
        self.lat_g.abs() >= thresholds::CORNERING_MIN_LAT_G
            && self.speed >= thresholds::CORNERING_MIN_SPEED_MPS
            && self.brake < 0.5
    }

    /// Indices of the driven wheels for this drivetrain.
    pub(super) fn driven_wheels(&self) -> &'static [usize] {
        match self.drivetrain {
            0 => &[FL, FR],         // FWD
            1 => &[RL, RR],         // RWD
            _ => &[FL, FR, RL, RR], // AWD / unknown
        }
    }
}

/// Rolling per-lap statistics consumed by the deferred analysers.
#[derive(Default)]
pub(super) struct LapAggregate {
    pub samples: u32,
    pub driving_samples: u32,
    pub cornering_samples: u32,
    pub front_sa_sum: f64,
    pub rear_sa_sum: f64,
    pub front_near_bump: u32,
    pub rear_near_bump: u32,
    pub max_gear: u8,
    pub top_gear_throttle_samples: u32,
    pub max_rpm_frac_top_gear: f32,
    pub drivetrain: i32,
}

impl LapAggregate {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn push(&mut self, s: &Sample) {
        self.samples += 1;
        self.drivetrain = s.drivetrain;

        if (1..=10).contains(&s.gear) && s.gear > self.max_gear {
            self.max_gear = s.gear;
        }

        if s.speed >= thresholds::DRIVING_MIN_SPEED_MPS {
            self.driving_samples += 1;
            if s.nst[FL].max(s.nst[FR]) >= thresholds::NEAR_BUMP_NST {
                self.front_near_bump += 1;
            }
            if s.nst[RL].max(s.nst[RR]) >= thresholds::NEAR_BUMP_NST {
                self.rear_near_bump += 1;
            }
        }

        if s.is_cornering() {
            self.cornering_samples += 1;
            self.front_sa_sum += f64::from(s.front_slip_angle());
            self.rear_sa_sum += f64::from(s.rear_slip_angle());
        }

        // Top-gear, full-throttle, off-brake: gearing/redline assessment.
        if self.max_gear >= 1 && s.gear == self.max_gear && s.throttle >= 0.9 && s.brake < 0.1 {
            self.top_gear_throttle_samples += 1;
            if s.rpm_frac > self.max_rpm_frac_top_gear {
                self.max_rpm_frac_top_gear = s.rpm_frac;
            }
        }
    }
}
