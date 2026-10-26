//! The production USART receive queue, also exercised by host tests.
//!
//! Synchronisation belongs to firmware: it wraps this object in an interrupt
//! Mutex<RefCell<_>>. No reference into the queue escapes that critical section.

pub const RX_CAPACITY: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RxItem {
    pub byte: u8,
    /// A synthetic loss notice, not a byte to parse. Each flush gets one.
    pub corrupted: bool,
    /// Captured with the pop, carried through assembly and parsing, then
    /// checked with interrupts still masked when applying the command.
    pub generation: u32,
}

pub struct RxBuffer {
    bytes: [u8; RX_CAPACITY],
    head: u8,
    len: u8,
    discarding: bool,
    loss_generation: u32,
    notified_generation: u32,
}

impl Default for RxBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl RxBuffer {
    pub const fn new() -> Self {
        Self {
            bytes: [0; RX_CAPACITY],
            head: 0,
            len: 0,
            discarding: false,
            loss_generation: 0,
            notified_generation: 0,
        }
    }

    /// ISR entry, once for each UDR0 read (read UCSR0A *before* UDR0).
    /// `overrun` is DOR0: one or more frames were lost BEFORE this byte.
    /// `corrupt_frame` is FE0 or UPE0: this byte itself cannot be trusted.
    /// Both can be set, but one UDR0 read still produces only one loss notice.
    pub fn receive(&mut self, byte: u8, overrun: bool, corrupt_frame: bool) {
        // These flag reads have no side effects. Combining all three avoids
        // duplicating the AVR flush path for hardware errors versus overflow.
        if overrun | corrupt_frame | (self.len as usize == RX_CAPACITY) {
            // A plain global error bit is insufficient: the queue could hold
            // an older clean newline followed by the prefix of the bad line.
            // Clearing at that older newline could execute a truncated STEP.
            // Purge queued input conservatively and create one ordered error
            // boundary instead. Even a previously dequeued/parsed line must
            // match this generation at acceptance before it can execute.
            self.head = 0;
            self.len = 0;
            // Count EVERY flush, including a second fault before recovery LF.
            // Main drains one notice per generation, even without a newline.
            // Wrapping is intentional. Comparing generations assumes fewer
            // than 2^32 faults per byte/acceptance or undrained notice backlog
            // (over eight days at the UART's byte rate, versus millisecond TX).
            self.loss_generation = self.loss_generation.wrapping_add(1);
            self.discarding = true;
        }
        // FE/UPE taint even a byte that reads as LF, including when DOR is
        // also set. It must never masquerade as the recovery delimiter.
        if corrupt_frame {
            return;
        }
        if self.discarding {
            if byte == b'\n' {
                // An intact LF, including DOR-only or overflow on LF, ends
                // the damaged line. Queue it so main's assembler recovers too.
                self.discarding = false;
            } else if !overrun {
                // Keep filtering the rest of a damaged line in the ISR so a
                // long tail cannot fill the ring and manufacture more losses.
                return;
            }
            // A DOR-only byte is intact and follows the gap: deliver it even
            // mid-line. The ordered loss notice puts LineAssembler in discard
            // mode first, so this byte cannot execute a truncated command.
        }
        let tail = (self.head + self.len) & 31;
        self.bytes[tail as usize] = byte;
        self.len += 1;
    }

    pub fn pop(&mut self) -> Option<RxItem> {
        if self.notified_generation != self.loss_generation {
            self.notified_generation = self.notified_generation.wrapping_add(1);
            // A synthetic tainted byte starts assembler discard mode. It is
            // never parsed or stored; the real recovery newline follows later.
            return Some(RxItem {
                byte: 0,
                corrupted: true,
                generation: self.notified_generation,
            });
        }
        if self.len == 0 {
            return None;
        }
        let byte = self.bytes[self.head as usize];
        self.head = (self.head + 1) & 31;
        self.len -= 1;
        Some(RxItem {
            byte,
            corrupted: false,
            generation: self.loss_generation,
        })
    }

    /// The caller must hold the RX interrupt mutex for this ENTIRE operation,
    /// not just while copying the current generation. A flush between the
    /// comparison and controller/planner mutation would recreate the race.
    /// Keep the closure to a few state-word updates; prepare and print outside.
    /// A stale command is silently discarded: its loss notice owns ERR LINE.
    pub fn accept_if_current<T>(&self, generation: u32, accept: impl FnOnce() -> T) -> Option<T> {
        if generation == self.loss_generation {
            Some(accept())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod boundary_tests {
    use super::*;

    #[test]
    fn generation_wrap_still_rejects_stale_bytes_and_reports_each_loss() {
        let mut rx = RxBuffer::new();
        rx.loss_generation = u32::MAX - 1;
        rx.notified_generation = u32::MAX - 1;
        rx.receive(b'\n', false, false);
        let before_loss = rx.pop().unwrap();
        assert_eq!(rx.accept_if_current(before_loss.generation, || 7), Some(7));

        rx.receive(b'X', false, true);
        rx.receive(b'Y', false, true); // wraps to zero, both notices still pending
        assert_eq!(rx.accept_if_current(before_loss.generation, || 7), None);
        for generation in [u32::MAX, 0] {
            let notice = rx.pop().unwrap();
            assert!(notice.corrupted);
            assert_eq!(notice.generation, generation);
        }
        assert_eq!(rx.pop(), None);
        rx.receive(b'\n', false, false);
        let recovery = rx.pop().unwrap();
        assert!(
            !recovery.corrupted,
            "recovery LF must not duplicate a notice"
        );
        assert_eq!(recovery.generation, 0);
        assert_eq!(rx.accept_if_current(recovery.generation, || 7), Some(7));
    }
}
