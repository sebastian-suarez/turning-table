//! A motion is measured in 1 ms tick intervals, never in blocking delays.

use crate::{Direction, Error, PreparedCommand, Sequencer};

pub const POSITION_LIMIT: i32 = 1 << 30;
pub const ENERGISE_MS: u8 = 20;
pub const HOLD_MS: u8 = 50;
pub const SETTLE_MS: u8 = 100;
pub const DEFAULT_PERIOD_MS: u8 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Idle,
    Energise,
    Stepping,
    Hold,
    Release,
    Settle,
}

impl State {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::Energise => "Energise",
            Self::Stepping => "Stepping",
            Self::Hold => "Hold",
            Self::Release => "Release",
            Self::Settle => "Settle",
        }
    }
}

/// Copy this small snapshot while interrupts are disabled, then do planning
/// after re-enabling interrupts. AVR cannot read a changing i32 atomically.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub state: State,
    pub position: i32,
    /// Requested period for the next move; an active move has its own copy.
    pub period_ms: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Output {
    pub pattern: u8,
    /// A position-bearing event, not just a flag: ZERO may change position
    /// before main prints this completed move's asynchronous DONE reply.
    pub done: Option<i32>,
}

pub struct Controller {
    position: i32,
    remaining: u32,
    sequencer: Sequencer,
    state: State,
    ticks_left: u8,
    period_ms: u8,
    active_period_ms: u8,
    direction: Direction,
    pattern: u8,
}

impl Default for Controller {
    fn default() -> Self {
        Self::new()
    }
}

impl Controller {
    pub const fn new() -> Self {
        Self {
            position: 0,
            remaining: 0,
            sequencer: Sequencer::new(),
            state: State::Idle,
            ticks_left: 0,
            period_ms: DEFAULT_PERIOD_MS,
            active_period_ms: DEFAULT_PERIOD_MS,
            direction: Direction::Forward,
            pattern: 0,
        }
    }

    pub const fn snapshot(&self) -> Snapshot {
        Snapshot {
            state: self.state,
            position: self.position,
            period_ms: self.period_ms,
        }
    }

    pub const fn pattern(&self) -> u8 {
        self.pattern
    }

    pub const fn phase(&self) -> u8 {
        self.sequencer.index()
    }

    /// Check in i32, which is sufficient even on an 8-bit AVR: checked_add
    /// detects machine overflow before the stricter symmetric position limit.
    pub fn checked_destination(position: i32, steps: i32) -> Result<i32, Error> {
        let target = position.checked_add(steps).ok_or(Error::Range)?;
        if !(-POSITION_LIMIT..=POSITION_LIMIT).contains(&target) {
            return Err(Error::Range);
        }
        Ok(target)
    }

    /// Applying a prepared command is safe in a short critical section: all
    /// degree/stop multiplication and division has already happened in main.
    /// Planner state is committed separately, *only* if this succeeds.
    pub fn apply(&mut self, command: PreparedCommand) -> Result<Output, Error> {
        if command.needs_idle() && self.state != State::Idle {
            return Err(Error::Busy);
        }
        match command {
            PreparedCommand::Move(steps) | PreparedCommand::Next { steps, .. } => {
                return self.start(steps);
            }
            PreparedCommand::Zero => self.position = 0,
            PreparedCommand::Speed(period) => {
                if !(1..=50).contains(&period) {
                    return Err(Error::Range);
                }
                self.period_ms = period;
            }
            PreparedCommand::Stops { .. } | PreparedCommand::Pos | PreparedCommand::Status => {}
        }
        Ok(self.output(None))
    }

    /// Queue a move for the next timer boundary. We do not energise here:
    /// main might call this just before an interrupt, shortening a nominal
    /// 20 ms delay. The first tick applies the phase and starts the full delay.
    pub fn start(&mut self, steps: i32) -> Result<Output, Error> {
        if self.state != State::Idle {
            return Err(Error::Busy);
        }
        Self::checked_destination(self.position, steps)?;
        if steps == 0 {
            // No tick or coil transition is needed, but acceptance still owns
            // exactly one event. Returning it avoids an overwrite-prone latch.
            return Ok(self.output(Some(self.position)));
        }
        self.direction = if steps > 0 {
            Direction::Forward
        } else {
            Direction::Reverse
        };
        // unsigned_abs also handles i32::MIN without signed negation overflow;
        // no 64-bit arithmetic is needed anywhere in this controller.
        self.remaining = steps.unsigned_abs();
        self.active_period_ms = self.period_ms;
        self.state = State::Energise;
        // 21 is a sentinel for the first tick; subsequent ticks count 20..1.
        self.ticks_left = ENERGISE_MS + 1;
        Ok(self.output(None))
    }

    /// Called exactly once per millisecond. From the first energised output,
    /// first step = 20 + period, last step = 20 + count*period,
    /// release = last + 50, Done = release + 100 (all times in ms).
    pub fn tick(&mut self) -> Output {
        if self.state == State::Idle {
            return self.output(None);
        }
        self.ticks_left -= 1;
        match self.state {
            State::Energise if self.ticks_left == ENERGISE_MS => {
                self.pattern = self.sequencer.pattern();
            }
            State::Energise if self.ticks_left == 0 => {
                self.state = State::Stepping;
                self.ticks_left = self.active_period_ms;
            }
            State::Stepping if self.ticks_left == 0 => {
                self.pattern = self.sequencer.advance(self.direction);
                self.position += match self.direction {
                    Direction::Forward => 1,
                    Direction::Reverse => -1,
                };
                // Each intermediate position lies between the validated start
                // and destination; this increment cannot overflow the bounds.
                self.remaining -= 1;
                if self.remaining == 0 {
                    self.state = State::Hold;
                    self.ticks_left = HOLD_MS;
                } else {
                    self.ticks_left = self.active_period_ms;
                }
            }
            State::Hold if self.ticks_left == 0 => {
                self.pattern = 0;
                self.state = State::Release;
                self.ticks_left = SETTLE_MS;
            }
            State::Release | State::Settle => {
                // Release is the boundary at which outputs go low. Its tick
                // is part of the 100 ms settle interval, not an extra ms.
                if self.ticks_left == 0 {
                    self.state = State::Idle;
                    return self.output(Some(self.position));
                }
                self.state = State::Settle;
            }
            _ => {}
        }
        self.output(None)
    }

    fn output(&self, done: Option<i32>) -> Output {
        Output {
            pattern: self.pattern,
            done,
        }
    }
}
