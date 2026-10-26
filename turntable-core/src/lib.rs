//! Pure Phase 2 logic shared by the AVR firmware and its host acceptance tests.
//!
//! There is no heap, floating point or hardware dependency here. In particular,
//! the expensive fixed-point divisions belong to [`Planner`], called by main;
//! [`Controller::tick`] uses only small counters and 32-bit position updates.

#![no_std]

#[cfg(test)]
extern crate std;

use core::ops::DerefMut;

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

/// Accept one prepared command: the body of firmware's acceptance critical
/// section. The host tests call this same function, so they exercise the
/// production transaction instead of a copy of it.
///
/// The caller holds the interrupt mutex that guards `rx` and the controller
/// for the WHOLE call, from the generation comparison through the controller
/// and planner updates (see [`RxBuffer::accept_if_current`]). Only bounded word
/// comparisons and copies happen here: parse and prepare before, print after.
///
/// `motion` borrows the controller and runs only once the generation matched,
/// so a stale command never touches it. Firmware passes its RefCell borrow;
/// borrowing up front instead would put that borrow check ahead of the
/// generation comparison, which lengthens the AVR critical section.
///
/// `None` means the RX queue lost bytes after this command's line was dequeued.
/// Nothing is applied or committed, and the stale command gets no reply: the
/// queued loss notice already owns its one ERR LINE. Otherwise the controller's
/// verdict comes back together with the motion snapshot for the reply.
pub fn accept<M: DerefMut<Target = Controller>>(
    rx: &RxBuffer,
    motion: impl FnOnce() -> M,
    planner: &mut Planner,
    generation: u32,
    prepared: Result<PreparedCommand, Error>,
) -> Option<(Result<Output, Error>, Snapshot)> {
    rx.accept_if_current(generation, || {
        let mut motion = motion();
        let result = prepared.and_then(|prepared| {
            let output = motion.apply(prepared)?;
            // Commit before restoring interrupts: a loss cannot split the
            // controller acceptance from its matching planner update.
            planner.commit(prepared);
            Ok(output)
        });
        (result, motion.snapshot())
    })
}

#[cfg(test)]
mod tests;
