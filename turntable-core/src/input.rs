//! Stable-sample inputs and the shared console/button admission table.
use crate::{Command, Error};

#[derive(Clone, Copy)]
pub struct Debouncer {
    ticks: u8,
    count: u8,
    closed: bool,
}

impl Debouncer {
    // Released at boot counts as an already stable release, so the first
    // press needs only ticks closed samples. Later releases also debounce.
    pub const fn new(ticks: u8) -> Self {
        Self {
            ticks,
            count: 0,
            closed: false,
        }
    }

    pub fn sample(&mut self, closed: bool) -> bool {
        if closed == self.closed {
            self.count = 0;
        } else {
            // A differing sample reaches the threshold and resets the count,
            // including threshold 255; count can never wrap before resetting.
            self.count += 1;
            if self.count >= self.ticks {
                self.closed = closed;
                self.count = 0;
                return closed;
            }
        }
        false
    }

    pub const fn closed(&self) -> bool {
        self.closed
    }
}

#[derive(Clone, Copy)]
pub struct Activity {
    pub platter_idle: bool,
    pub arm_idle: bool,
    pub scanning: bool,
    pub homing: bool,
    pub homed: bool,
}

pub fn admit(command: &Command, a: Activity) -> Result<(), Error> {
    match command {
        Command::Pos | Command::Status | Command::Cfg | Command::Stop => Ok(()),
        Command::Speed(_) if !a.scanning && !a.homing => Ok(()),
        _ if !a.platter_idle || !a.arm_idle || a.scanning || a.homing => Err(Error::Busy),
        Command::Arm(_) | Command::Scan(_) if !a.homed => Err(Error::Home),
        _ => Ok(()),
    }
}

pub enum ButtonAction {
    Stop,
    Home,
    Scan,
}

pub fn button_action(a: Activity) -> ButtonAction {
    if !a.platter_idle || !a.arm_idle || a.scanning || a.homing {
        ButtonAction::Stop
    } else if a.homed {
        ButtonAction::Scan
    } else {
        ButtonAction::Home
    }
}
