//! Homing owns only its sequence; the timer ISR owns switch-triggered halts.
use crate::{planner::angle_position, Settings};

pub const HOME_PERIOD_MS: u8 = 4;
pub const HOME_TRAVEL_TENTHS: i32 = 800;
pub const HOME_BACKOFF_TENTHS: i32 = 50;
pub const SWITCH_DEBOUNCE_TICKS: u8 = 2;
pub const BUTTON_DEBOUNCE_TICKS: u8 = 20;

pub struct Homing {
    search: i32,
    home: i32,
    lifting: bool,
    stopped: bool,
}

pub enum HomingStep {
    Move {
        steps: i32,
        period_ms: u8,
        halt_on_switch: bool,
    },
    Homed(i32),
    Failed,
}

impl Homing {
    #[inline(never)]
    pub fn start(switch_closed: bool, settings: &Settings) -> (Self, HomingStep) {
        // Searches/backoffs are relative: the unhomed coordinate is arbitrary.
        // Only a switch-triggered, released-and-settled completion establishes
        // the absolute home coordinate; stopping must never establish it.
        let position = |tenths| angle_position(tenths, settings.aratio_x100);
        let homing = Self {
            search: -position(HOME_TRAVEL_TENTHS),
            home: position(i32::from(settings.ahome_tenths)),
            lifting: switch_closed,
            stopped: false,
        };
        let step = homing.movement(
            if switch_closed {
                position(HOME_BACKOFF_TENTHS)
            } else {
                homing.search
            },
            !switch_closed,
        );
        (homing, step)
    }

    fn movement(&self, steps: i32, halt_on_switch: bool) -> HomingStep {
        HomingStep::Move {
            steps,
            period_ms: HOME_PERIOD_MS,
            halt_on_switch,
        }
    }

    pub fn done(&mut self, switch_closed: bool, halted_by_switch: bool) -> HomingStep {
        if self.stopped {
            return HomingStep::Failed;
        }
        if self.lifting {
            self.lifting = false;
            if switch_closed {
                HomingStep::Failed
            } else {
                self.movement(self.search, true)
            }
        } else if halted_by_switch {
            HomingStep::Homed(self.home)
        } else {
            HomingStep::Failed
        }
    }

    pub fn stop(&mut self) {
        self.stopped = true;
    }
}
