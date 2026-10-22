//! The production USART receive queue, also exercised by host tests.
//!
//! Synchronisation belongs to firmware: it wraps this object in an interrupt
//! Mutex<RefCell<_>>. No reference into the queue escapes that critical section.

pub const RX_CAPACITY: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RxItem {
    pub byte: u8,
    pub corrupted: bool,
}

pub struct RxBuffer {
    bytes: [u8; RX_CAPACITY],
    head: u8,
    len: u8,
    sticky_error: bool,
    discarding: bool,
    notify_error: bool,
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
            sticky_error: false,
            discarding: false,
            notify_error: false,
        }
    }

    /// ISR entry, once for each UDR0 read. A hardware error belongs to that
    /// specific byte (firmware must read UCSR0A *before* UDR0).
    pub fn receive(&mut self, byte: u8, hardware_error: bool) {
        if hardware_error || self.len as usize == RX_CAPACITY {
            // A plain global error bit is insufficient: the queue could hold
            // an older clean newline followed by the prefix of the bad line.
            // Clearing at that older newline could execute a truncated STEP.
            // Purge queued input conservatively and create one ordered error
            // boundary instead. Previously dispatched complete lines are safe.
            self.head = 0;
            self.len = 0;
            // Notify main even if a long burst has not sent its newline yet.
            // Repeated faults in the same discard episode share one notice.
            self.notify_error |= !self.sticky_error;
            self.sticky_error = true;
            self.discarding = true;
        }
        if self.discarding {
            if byte != b'\n' || hardware_error {
                return;
            }
            // This trustworthy delimiter is the first queued byte after the
            // purge, even if the original overflow byte was itself '\n'. It
            // is a recovery marker: main must taint its partial line before
            // consuming this newline. Later clean input queues *behind* it.
            self.discarding = false;
        }
        let tail = (self.head + self.len) & 31;
        self.bytes[tail as usize] = byte;
        self.len += 1;
    }

    pub fn pop(&mut self) -> Option<RxItem> {
        if self.notify_error {
            self.notify_error = false;
            // A synthetic tainted byte starts assembler discard mode. It is
            // never parsed or stored; the real recovery newline follows later.
            return Some(RxItem {
                byte: 0,
                corrupted: true,
            });
        }
        if self.len == 0 {
            return None;
        }
        let byte = self.bytes[self.head as usize];
        self.head = (self.head + 1) & 31;
        self.len -= 1;
        // While sticky_error is set, the first queued byte is necessarily the
        // recovery newline. Clearing here cannot accidentally clear an error
        // at a delimiter that preceded the fault. Queue, marker and clear are
        // one atomic operation under firmware's mutex.
        let corrupted = self.sticky_error;
        self.sticky_error = false;
        Some(RxItem { byte, corrupted })
    }

    pub const fn has_error(&self) -> bool {
        self.sticky_error
    }
}
