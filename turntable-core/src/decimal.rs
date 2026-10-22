//! Small decimal conversion for the console's ufmt adapter.
//!
//! AVR's generic 32-bit decimal division pulls in a large compiler runtime.
//! Repeated subtraction of descending powers of ten needs at most 90
//! subtractions and a 40-byte table instead; no division or allocation.

#[derive(Clone, Copy)]
pub struct Decimal {
    magnitude: u32,
    negative: bool,
}

impl Decimal {
    pub const fn unsigned(value: u32) -> Self {
        Self {
            magnitude: value,
            negative: false,
        }
    }

    pub const fn signed(value: i32) -> Self {
        Self {
            magnitude: value.unsigned_abs(),
            negative: value < 0,
        }
    }

    /// Ten digits plus a possible minus sign is enough for every i32/u32.
    pub fn digits(self, buffer: &mut [u8; 11]) -> &str {
        let mut len = 0;
        if self.negative {
            buffer[len] = b'-';
            len += 1;
        }
        let mut remaining = self.magnitude;
        let mut started = false;
        for divisor in [
            1_000_000_000,
            100_000_000,
            10_000_000,
            1_000_000,
            100_000,
            10_000,
            1_000,
            100,
            10,
            1,
        ] {
            let mut digit = 0;
            while remaining >= divisor {
                remaining -= divisor;
                digit += 1;
            }
            if digit != 0 || started || divisor == 1 {
                buffer[len] = b'0' + digit;
                len += 1;
                started = true;
            }
        }
        // SAFETY: the only written bytes are '-' and ASCII '0'..'9', and
        // len counts exactly those initialised bytes. All are valid UTF-8.
        unsafe { core::str::from_utf8_unchecked(&buffer[..len]) }
    }
}
