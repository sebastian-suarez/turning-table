//! Fixed-point geometry. This module runs in main, with interrupts enabled.

use crate::{
    command::{MAX_PERIOD_MS, MAX_STOPS, MIN_PERIOD_MS, MIN_STOPS, ROT_LIMIT_TENTHS, STEP_LIMIT},
    Command, Controller, Error, Snapshot, State,
};

/// Measured in *hundredths* of a half-step per platter revolution. Keeping the
/// fraction avoids repeated-stop drift for gearboxes whose ratio is not whole.
///
/// Calibrated on 2026-10-26 (Phase 2 acceptance, section 3): this unit has the
/// exact 64:1 gearbox, 64 × 64 = 4096 half-steps per revolution. Ten commanded
/// revolutions at the provisional 407577 (the 63.68395:1 variant, 4075.77) ended
/// 17° short in five of six runs at 2, 4 and 10 ms per half-step; the 64:1
/// prediction is 200 half-steps short, 17.58°. A short result that does not
/// depend on the speed is the gearbox, not lost steps.
pub const RATIO_X100: u32 = 409_600;

/// All fields have already been range checked by Planner::prepare. Controller
/// rechecks busy/destination at application so a rejected move cannot mutate k.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreparedCommand {
    Move(i32),
    Next { steps: i32, k: u32 },
    Stops { stops: u16, origin: i32 },
    Zero,
    Speed(u8),
    Pos,
    Status,
}

impl PreparedCommand {
    pub const fn needs_idle(self) -> bool {
        !matches!(self, Self::Speed(_) | Self::Pos | Self::Status)
    }
}

pub struct Planner {
    ratio_x100: u32,
    stops: u16,
    k: u32,
    origin: i32,
}

impl Planner {
    pub const fn new(ratio_x100: u32, stops: u16) -> Option<Self> {
        if ratio_x100 == 0 || stops < MIN_STOPS || stops > MAX_STOPS {
            return None;
        }
        Some(Self {
            ratio_x100,
            stops,
            k: 0,
            origin: 0,
        })
    }

    /// Calibration changes the scale; STOPS and SCAN own grid rebasing.
    pub fn set_ratio(&mut self, ratio_x100: u32) {
        self.ratio_x100 = ratio_x100;
    }

    pub const fn ratio_x100(&self) -> u32 {
        self.ratio_x100
    }

    pub const fn stops(&self) -> u16 {
        self.stops
    }

    pub const fn k(&self) -> u32 {
        self.k
    }

    pub const fn origin(&self) -> i32 {
        self.origin
    }

    /// tenths/3600 is revolutions; ratio_x100/100 is half-steps/revolution.
    /// Multiply first in i64 and divide once by 360000: dividing in stages
    /// would lose fractional information before rounding. Even 36000*409600
    /// exceeds i32, which is why the intermediate is deliberately wider.
    pub fn degrees_to_halfsteps(&self, tenths: i32) -> Option<i32> {
        if !(-ROT_LIMIT_TENTHS..=ROT_LIMIT_TENTHS).contains(&tenths) {
            return None;
        }
        // Even 36000 * u32::MAX / 360000 fits i32 after rounding.
        Some(angle_position(tenths, self.ratio_x100))
    }

    /// Absolute ideal target relative to the planner origin. k never wraps
    /// after one revolution: rounding the cumulative target distributes the
    /// fractional half-step among *all* later revolutions as well as stops.
    fn target(&self, k: u32) -> Option<i64> {
        // Two u32 magnitudes always fit a u64 product. Reject its sign bit
        // before interpreting it as the required signed i64 intermediate.
        // This checks overflow without AVR's bulky generic mul-overflow helper.
        let product = wide_product(k, self.ratio_x100);
        let numerator = i64::try_from(product).ok()?;
        Some(round_half_away(numerator, u32::from(self.stops) * 100))
    }

    /// Preparation is read-only. A second NEXT while busy, or a rejected
    /// destination at a position limit, must leave the global stop count alone.
    /// main copies Snapshot under the mutex, then calls this outside the lock.
    pub fn prepare(&self, command: Command, motion: Snapshot) -> Result<PreparedCommand, Error> {
        if command.needs_idle() && motion.state != State::Idle {
            return Err(Error::Busy);
        }
        let prepared = match command {
            Command::Step(steps) => {
                if !(-STEP_LIMIT..=STEP_LIMIT).contains(&steps) {
                    return Err(Error::Range);
                }
                PreparedCommand::Move(steps)
            }
            Command::Rot(tenths) => {
                PreparedCommand::Move(self.degrees_to_halfsteps(tenths).ok_or(Error::Range)?)
            }
            Command::Next => {
                let k = self.k.checked_add(1).ok_or(Error::Range)?;
                // Stay on the original absolute grid after a manual STEP/ROT.
                // Work in i64 so origin + target and subtraction are checked
                // before narrowing; target()'s rounded result leaves ample
                // room for either i32 coordinate in this signed intermediate.
                let steps = i64::from(self.origin) + self.target(k).ok_or(Error::Range)?
                    - i64::from(motion.position);
                PreparedCommand::Next {
                    steps: i32::try_from(steps).map_err(|_| Error::Range)?,
                    k,
                }
            }
            Command::Stops(stops) => {
                if !(MIN_STOPS..=MAX_STOPS).contains(&stops) {
                    return Err(Error::Range);
                }
                PreparedCommand::Stops {
                    stops,
                    origin: motion.position,
                }
            }
            Command::Zero => PreparedCommand::Zero,
            Command::Speed(period) => {
                if !(MIN_PERIOD_MS..=MAX_PERIOD_MS).contains(&period) {
                    return Err(Error::Range);
                }
                PreparedCommand::Speed(period)
            }
            Command::Pos => PreparedCommand::Pos,
            Command::Status => PreparedCommand::Status,
            // The arm's absolute target and the Phase 3 admission table are
            // handled by main. Phase 2 preparation remains platter-only.
            _ => return Err(Error::Syntax),
        };
        if let PreparedCommand::Move(steps) | PreparedCommand::Next { steps, .. } = prepared {
            Controller::checked_destination(motion.position, steps)?;
        }
        Ok(prepared)
    }

    /// Called by main only after Controller::apply accepted the same prepared
    /// command, in the same loss-generation-checked critical section. This
    /// only copies k/origin/stops; the ISR never performs the planner's i64 maths.
    pub fn commit(&mut self, command: PreparedCommand) {
        match command {
            PreparedCommand::Next { k, .. } => self.k = k,
            PreparedCommand::Stops { stops, origin } => {
                self.stops = stops;
                self.origin = origin;
                self.k = 0;
            }
            PreparedCommand::Zero => {
                self.origin = 0;
                self.k = 0;
            }
            _ => {}
        }
    }
}

/// Round a rational to the nearest integer, with exact halves away from zero.
/// All callers use 100 <= denominator <= 360000 (tests also use 3600000).
/// Dividing a signed i64 with Rust's generic AVR runtime would pull in several
/// KiB of helpers, exceeding the whole firmware's 8 KiB budget. Binary long
/// division below is the same exact integer operation using shifts/subtractions.
/// Four 16-bit partial products give an exact 32x32 -> 64 product without
/// pulling AVR's generic 64x64 multiply into flash. Each carry sum fits u32.
#[inline(never)]
fn wide_product(a: u32, b: u32) -> u64 {
    let (a0, a1) = (a & 0xffff, a >> 16);
    let (b0, b1) = (b & 0xffff, b >> 16);
    let low = a0 * b0;
    let middle = a1 * b0 + (low >> 16);
    let upper = middle >> 16;
    let middle = a0 * b1 + (middle & 0xffff);
    let high = a1 * b1 + upper + (middle >> 16);
    (u64::from(high) << 32) | u64::from((middle << 16) | (low & 0xffff))
}

/// Bounded degree conversion shared by both planners, homing and ring targets.
/// Callers limit tenths to +/-36000; the rounded result fits i32 at any u32 ratio.
#[inline(never)]
pub(crate) fn angle_position(tenths: i32, ratio: u32) -> i32 {
    let magnitude = wide_product(tenths.unsigned_abs(), ratio) as i64;
    // Rounding is sign-symmetric. Negate the bounded i32 result instead of
    // eight numerator bytes, which is smaller on AVR.
    let rounded = round_half_away(magnitude, 360_000) as i32;
    if tenths < 0 {
        -rounded
    } else {
        rounded
    }
}

/// It runs only in main with interrupts enabled, never in the timer ISR.
pub(crate) fn round_half_away(numerator: i64, denominator: u32) -> i64 {
    let negative = numerator < 0;
    let mut quotient = numerator.unsigned_abs();
    let mut remainder = 0_u32;
    for _ in 0..64 {
        // Shift one dividend bit into the remainder, starting with bit 63.
        // The processed dividend bits become quotient bits in the same word.
        // remainder < denominator before shifting; twice our bounded divisor
        // fits u32, so the shift cannot lose a significant bit.
        remainder = (remainder << 1) | ((quotient >> 63) as u32);
        quotient <<= 1;
        if remainder >= denominator {
            remainder -= denominator;
            quotient |= 1;
        }
    }
    // Adding half the divisor to the numerator could overflow. Comparing the
    // remainder instead preserves ties, even near i64's endpoints. Divisor is
    // at least 100, so the rounded quotient always fits a positive i64.
    let rounded = (quotient + u64::from(remainder * 2 >= denominator)) as i64;
    if negative {
        -rounded
    } else {
        rounded
    }
}

#[cfg(test)]
mod boundary_tests {
    use super::*;

    #[test]
    fn counter_and_multiplication_overflow_reject_without_mutation() {
        let idle = Controller::new().snapshot();
        let mut planner = Planner::new(RATIO_X100, 36).unwrap();
        planner.k = u32::MAX;
        assert_eq!(planner.prepare(Command::Next, idle), Err(Error::Range));
        assert_eq!(planner.k(), u32::MAX);

        let mut planner = Planner::new(u32::MAX, 1).unwrap();
        planner.k = u32::MAX - 1;
        assert_eq!(planner.prepare(Command::Next, idle), Err(Error::Range));
        assert_eq!(planner.k(), u32::MAX - 1);
    }
}
