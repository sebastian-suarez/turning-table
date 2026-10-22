//! Pure Phase 2 logic shared by the AVR firmware and its host acceptance tests.
//!
//! There is no heap, floating point or hardware dependency here. In particular,
//! the expensive fixed-point divisions belong to [`Planner`], called by main;
//! [`Controller::tick`] uses only small counters and 32-bit position updates.

#![no_std]

#[cfg(test)]
extern crate std;

pub mod command;
pub mod controller;
pub mod decimal;
pub mod line;
pub mod planner;
pub mod rx;
pub mod sequencer;

pub use command::Command;
pub use controller::{Controller, Output, Snapshot, State};
pub use line::LineAssembler;
pub use planner::{Planner, PreparedCommand, RATIO_X100};
pub use rx::{RxBuffer, RxItem};
pub use sequencer::{Direction, Sequencer};

/// These terse names are also the complete protocol error vocabulary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Busy,
    Range,
    Syntax,
    Line,
}

impl Error {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Busy => "BUSY",
            Self::Range => "RANGE",
            Self::Syntax => "SYNTAX",
            Self::Line => "LINE",
        }
    }
}

#[cfg(test)]
mod tests;
