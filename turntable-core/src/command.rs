//! Strict, bounded ASCII parsing without allocating strings or using floats.

use crate::{line::MAX_LINE_BYTES, Error};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Step(i32),
    /// Signed tenths of a degree: -7.5 degrees is represented by -75.
    Rot(i32),
    Stops(u16),
    Next,
    Zero,
    Pos,
    Speed(u8),
    Status,
}

impl Command {
    pub const fn needs_idle(self) -> bool {
        !matches!(self, Self::Pos | Self::Status | Self::Speed(_))
    }

    // On AVR, keep parser temporaries out of the main loop's RX/event frame.
    // Sharing this call avoids register spills and saves flash, with no heap.
    #[inline(never)]
    pub fn parse(line: &[u8]) -> Result<Self, Error> {
        // Normally LineAssembler has already ignored CR. Accept a terminal CR
        // here too, so parsing a single CRLF console line works on its own.
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if line.len() > MAX_LINE_BYTES {
            return Err(Error::Line);
        }
        if !line.is_ascii() {
            return Err(Error::Syntax);
        }
        let mut words = line
            .split(|b| *b == b' ' || *b == b'\t')
            .filter(|word| !word.is_empty());
        let verb = words.next().ok_or(Error::Syntax)?;
        let argument = words.next();
        if words.next().is_some() {
            return Err(Error::Syntax);
        }

        if verb.eq_ignore_ascii_case(b"STEP") {
            let n = integer(argument.ok_or(Error::Syntax)?)?;
            return bounded(n, -1_000_000, 1_000_000).map(Self::Step);
        }
        if verb.eq_ignore_ascii_case(b"ROT") {
            let tenths = degrees(argument.ok_or(Error::Syntax)?)?;
            return bounded(tenths, -36_000, 36_000).map(Self::Rot);
        }
        if verb.eq_ignore_ascii_case(b"STOPS") {
            let n = integer(argument.ok_or(Error::Syntax)?)?;
            return bounded(n, 1, 720).map(|n| Self::Stops(n as u16));
        }
        if verb.eq_ignore_ascii_case(b"SPEED") {
            let n = integer(argument.ok_or(Error::Syntax)?)?;
            return bounded(n, 1, 50).map(|n| Self::Speed(n as u8));
        }
        if argument.is_some() {
            return Err(Error::Syntax);
        }
        if verb.eq_ignore_ascii_case(b"NEXT") {
            Ok(Self::Next)
        } else if verb.eq_ignore_ascii_case(b"ZERO") {
            Ok(Self::Zero)
        } else if verb.eq_ignore_ascii_case(b"POS") {
            Ok(Self::Pos)
        } else if verb.eq_ignore_ascii_case(b"STATUS") {
            Ok(Self::Status)
        } else {
            Err(Error::Syntax)
        }
    }
}

fn bounded(value: i32, min: i32, max: i32) -> Result<i32, Error> {
    if (min..=max).contains(&value) {
        Ok(value)
    } else {
        Err(Error::Range)
    }
}

fn sign(token: &[u8]) -> (bool, &[u8]) {
    match token.first() {
        Some(b'-') => (true, &token[1..]),
        Some(b'+') => (false, &token[1..]),
        _ => (false, token),
    }
}

/// Accumulate an unsigned magnitude so the magnitude of i32::MIN fits too.
/// Test the bound *before* multiplying by ten. This is checked arithmetic, but
/// avoids the large generic checked-multiply runtime helper on an 8-bit AVR.
fn signed_digits(digits: &[u8], negative: bool) -> Result<i32, Error> {
    if digits.is_empty() || !digits.iter().all(u8::is_ascii_digit) {
        return Err(Error::Syntax);
    }
    let mut value = 0_u32;
    for &digit in digits {
        let digit = u32::from(digit - b'0');
        // 2147483647 / 10 = 214748364 with final digit at most 7 (8
        // for a negative value). These comparisons prove the next operation
        // fits u32 and the final signed result fits i32.
        let last_digit = if negative { 8 } else { 7 };
        if value > 214_748_364 || (value == 214_748_364 && digit > last_digit) {
            return Err(Error::Range);
        }
        value = value * 10 + digit;
    }
    if negative {
        // Wrapping unsigned subtraction represents -2147483648 correctly;
        // signed negation of that magnitude would itself overflow.
        Ok(0_u32.wrapping_sub(value) as i32)
    } else {
        Ok(value as i32)
    }
}

fn integer(token: &[u8]) -> Result<i32, Error> {
    let (negative, digits) = sign(token);
    signed_digits(digits, negative)
}

fn degrees(token: &[u8]) -> Result<i32, Error> {
    let (negative, digits) = sign(token);
    let mut parts = digits.split(|b| *b == b'.');
    let whole = parts.next().ok_or(Error::Syntax)?;
    let fraction = match parts.next() {
        None => 0,
        Some([digit]) if digit.is_ascii_digit() => i32::from(*digit - b'0'),
        _ => return Err(Error::Syntax),
    };
    if parts.next().is_some() {
        return Err(Error::Syntax);
    }
    // Keep the same sign while combining whole degrees and the one decimal.
    // In particular "-0.1" must stay negative despite its zero whole part.
    let whole = signed_digits(whole, negative)?;
    // Bound first, then scale: 3600*10+9 fits comfortably in i32. The caller
    // enforces the final tenths limit, rejecting e.g. ROT 3600.1 as RANGE.
    bounded(whole, -3600, 3600)?;
    Ok(whole * 10 + if negative { -fraction } else { fraction })
}
