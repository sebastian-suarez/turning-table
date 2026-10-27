//! A scan is at most four arm deltas interleaved with cumulative-grid turns.
use crate::{planner::angle_position, Error, ScanSettings, Settings, ARM_MAX_TENTHS};

pub const CLOCK_MARGIN_PERCENT: u32 = 1;
pub const START_ALLOWANCE_MS: u32 = 10_000;
pub const MOVE_OVERHEAD_TICKS: u32 = 171;

pub enum ScanMove {
    Arm(i32),
    Turn,
}

pub struct Scan {
    deltas: [i32; 4],
    rings: u8,
    index: u8,
    turn: bool,
    period: u8,
    photos: u16,
}

impl Scan {
    #[inline(never)]
    pub fn new(
        settings: &Settings,
        run: ScanSettings,
        speed_ms: u8,
        arm_position: i32,
    ) -> Result<Self, Error> {
        if !settings.valid()
            || !run.valid()
            || run.rings > settings.rings.len
            || !(1..=50).contains(&speed_ms)
        {
            return Err(Error::Range);
        }
        // The largest parser-accepted product is 1.8 billion, so this division
        // needs no new generic i64 divide helper on AVR.
        let period = divide(
            u32::from(run.photos) * u32::from(run.interval_ms) * 100 + settings.pratio_x100 / 2,
            settings.pratio_x100,
        );
        if !(1..=255).contains(&period) {
            return Err(Error::Range);
        }
        let mut scan = Self {
            deltas: [0; 4],
            rings: run.rings,
            index: 0,
            turn: false,
            period: period as u8,
            photos: 0,
        };
        let mut position = arm_position;
        let mut ticks = u32::from(run.rings) * MOVE_OVERHEAD_TICKS;
        for i in 0..usize::from(run.rings) {
            let tenths = i16::from(settings.rings.heights[i]) * 10;
            if !(settings.ahome_tenths..=ARM_MAX_TENTHS).contains(&tenths) {
                return Err(Error::Range);
            }
            let target = angle_position(i32::from(tenths), settings.aratio_x100);
            let delta = target.checked_sub(position).ok_or(Error::Range)?;
            // Validated ring angles put target in 0..=36111. checked_sub
            // already proves the delta fits, so adding it back reaches target.

            scan.deltas[i] = delta;
            if delta != 0 {
                // Even one 18-million-tick arm move exceeds the 300-photo
                // budget at the longest interval. This necessary bound also
                // proves the following 32-bit products and sums cannot wrap.
                if delta.unsigned_abs() > 18_000_000 {
                    return Err(Error::Range);
                }
                ticks += MOVE_OVERHEAD_TICKS + delta.unsigned_abs() * u32::from(speed_ms);
                if ticks > 18_000_000 {
                    return Err(Error::Range);
                }
            }
            position = target;
        }
        let steps = divide(u32::from(run.rings) * settings.pratio_x100 + 50, 100);
        ticks += steps * u32::from(scan.period);
        // At most 18M arm ticks + 4*10000*255 turn ticks; *101 fits u32.
        let elapsed = divide(ticks * (100 + CLOCK_MARGIN_PERCENT), 100);
        let photos = divide(elapsed + START_ALLOWANCE_MS, u32::from(run.interval_ms)) + 1;
        if photos > 300 {
            return Err(Error::Range);
        }
        scan.photos = photos as u16;
        Ok(scan)
    }

    pub const fn turn_period_ms(&self) -> u8 {
        self.period
    }
    pub const fn photos(&self) -> u32 {
        self.photos as u32
    }
}

impl Iterator for Scan {
    type Item = ScanMove;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.rings {
            return None;
        }
        if !self.turn {
            self.turn = true;
            let delta = self.deltas[usize::from(self.index)];
            if delta != 0 {
                return Some(ScanMove::Arm(delta));
            }
        }
        self.turn = false;
        self.index += 1;
        Some(ScanMove::Turn)
    }
}

// Divisors here are at most 1000000, so the shifted remainder fits u32.
// A fixed 32-bit long division avoids AVR's large generic divide runtime.
#[inline(never)]
fn divide(mut numerator: u32, denominator: u32) -> u32 {
    let mut remainder = 0_u32;
    for _ in 0..32 {
        remainder = (remainder << 1) | (numerator >> 31);
        numerator <<= 1;
        if remainder >= denominator {
            remainder -= denominator;
            numerator |= 1;
        }
    }
    numerator
}
