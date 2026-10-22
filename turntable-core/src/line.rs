//! Assemble bounded lines while keeping a damaged line unexecutable.

use crate::Error;

pub const MAX_LINE_BYTES: usize = 24;

pub struct LineAssembler {
    bytes: [u8; MAX_LINE_BYTES],
    len: u8,
    discarding: bool,
}

impl Default for LineAssembler {
    fn default() -> Self {
        Self::new()
    }
}

impl LineAssembler {
    pub const fn new() -> Self {
        Self {
            bytes: [0; MAX_LINE_BYTES],
            len: 0,
            discarding: false,
        }
    }

    /// Returns one completed line or one error notification. CR is ignored;
    /// LF is the only resynchronisation boundary. The returned slice borrows
    /// our fixed buffer until the next push, so no line-sized copy is needed.
    ///
    /// Report an error as soon as it is detected, including a burst with no
    /// newline (the frozen acceptance case). Remain in discard mode afterwards
    /// and suppress repeat errors until LF, even if more errors are observed.
    pub fn push(&mut self, byte: u8, corrupted: bool) -> Option<Result<&[u8], Error>> {
        let new_error = corrupted && !self.discarding;
        self.discarding |= corrupted;
        if self.discarding {
            if byte == b'\n' {
                self.discarding = false;
                self.len = 0;
            }
            return if new_error {
                Some(Err(Error::Line))
            } else {
                None
            };
        }
        match byte {
            b'\r' => None,
            b'\n' => {
                let len = self.len as usize;
                self.len = 0;
                Some(Ok(&self.bytes[..len]))
            }
            _ if self.len as usize == MAX_LINE_BYTES => {
                self.discarding = true;
                Some(Err(Error::Line))
            }
            _ => {
                self.bytes[self.len as usize] = byte;
                self.len += 1;
                None
            }
        }
    }
}
