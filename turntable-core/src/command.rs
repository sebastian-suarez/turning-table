//! Strict, bounded ASCII parsing without allocating strings or using floats.

use crate::{line::MAX_LINE_BYTES, Error, Rings, ScanSettings};

// The console protocol's numeric limits (spec, "Console protocol"), each
// defined once. The parser answers ERR RANGE outside them; Planner and
// Controller check the same constants again, so a Command or PreparedCommand
// built in code cannot bypass them. Compile-time constants cost no RAM or flash.

/// `STEP n` accepts |n| up to this many half-steps.
pub const STEP_LIMIT: i32 = 1_000_000;
/// `ROT deg` accepts |deg| up to 3600.0 (ten turns), counted here in tenths.
pub const ROT_LIMIT_TENTHS: i32 = 36_000;
/// `STOPS n` accepts MIN_STOPS..=MAX_STOPS stops per revolution.
pub const MIN_STOPS: u16 = 1;
pub const MAX_STOPS: u16 = 720;
/// `SPEED ms` accepts MIN_PERIOD_MS..=MAX_PERIOD_MS ms (ticks) per half-step.
pub const MIN_PERIOD_MS: u8 = 1;
pub const MAX_PERIOD_MS: u8 = 50;

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
    Arm(i16),
    Home,
    Rings(Rings),
    Scan(Option<ScanSettings>),
    Stop,
    PRatio(u32),
    ARatio(u32),
    AHome(i16),
    Cfg,
}

impl Command {
    pub const fn needs_idle(self) -> bool {
        !matches!(
            self,
            Self::Pos | Self::Status | Self::Speed(_) | Self::Cfg | Self::Stop
        )
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
        let mut words: [&[u8]; 5] = [&[]; 5];
        let mut count = 0_u8;
        for word in line
            .split(|b| *b == b' ' || *b == b'\t')
            .filter(|w| !w.is_empty())
        {
            if usize::from(count) == words.len() {
                return Err(Error::Syntax);
            }
            words[usize::from(count)] = word;
            count += 1;
        }
        if count == 0 {
            return Err(Error::Syntax);
        }
        // One comparison loop and a shared arity check keep the AVR parser
        // bounded without expanding a separate argument parser for every verb.
        const VERBS: &[u8] = b"STEP\0ROT\0STOPS\0NEXT\0ZERO\0POS\0SPEED\0STATUS\0ARM\0HOME\0RINGS\0SCAN\0STOP\0PRATIO\0ARATIO\0AHOME\0CFG";
        let mut verb = 0_u8;
        for name in VERBS.split(|b| *b == 0) {
            if words[0].eq_ignore_ascii_case(name) {
                break;
            }
            verb += 1;
        }
        if verb == 17 {
            return Err(Error::Syntax);
        }
        if verb == 10 {
            if count == 1 {
                return Err(Error::Syntax);
            }
            let mut rings = Rings {
                heights: [0; 4],
                len: count - 1,
            };
            for (height, word) in rings.heights.iter_mut().zip(&words[1..usize::from(count)]) {
                *height = bounded(integer(word)?, 0, 90)? as u8;
            }
            return Ok(Self::Rings(rings));
        }
        if verb == 11 {
            return match count {
                1 => Ok(Self::Scan(None)),
                4 => Ok(Self::Scan(Some(ScanSettings {
                    rings: bounded(integer(words[1])?, 1, 4)? as u8,
                    photos: bounded(integer(words[2])?, 1, 300)? as u16,
                    interval_ms: bounded(integer(words[3])?, 100, 60_000)? as u16,
                }))),
                _ => Err(Error::Syntax),
            };
        }
        let numeric = matches!(verb, 0 | 1 | 2 | 6 | 8 | 13 | 14 | 15);
        if count != if numeric { 2 } else { 1 } {
            return Err(Error::Syntax);
        }
        if numeric {
            let argument = words[1];
            const BOUNDS: [[i32; 2]; 8] = [
                [-STEP_LIMIT, STEP_LIMIT],
                [-ROT_LIMIT_TENTHS, ROT_LIMIT_TENTHS],
                [1, 720],
                [1, 50],
                [0, 900],
                [100_000, 1_000_000],
                [100_000, 20_000_000],
                [0, 300],
            ];
            let index = match verb {
                0..=2 => verb,
                6 => 3,
                8 => 4,
                _ => verb - 8,
            };
            let [min, max] = BOUNDS[usize::from(index)];
            let value = bounded(
                if matches!(verb, 1 | 8 | 15) {
                    degrees(argument)?
                } else {
                    integer(argument)?
                },
                min,
                max,
            )?;
            return Ok(match verb {
                0 => Self::Step(value),
                1 => Self::Rot(value),
                2 => Self::Stops(value as u16),
                6 => Self::Speed(value as u8),
                8 => Self::Arm(value as i16),
                13 => Self::PRatio(value as u32),
                14 => Self::ARatio(value as u32),
                _ => Self::AHome(value as i16),
            });
        }
        Ok(match verb {
            3 => Self::Next,
            4 => Self::Zero,
            5 => Self::Pos,
            7 => Self::Status,
            9 => Self::Home,
            12 => Self::Stop,
            16 => Self::Cfg,
            _ => return Err(Error::Syntax),
        })
    }
}

#[inline(never)]
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

/// No protocol field exceeds ARATIO's 20 million. Reject larger magnitudes
/// here, before command-specific bounds. Checking after each digit is safe:
/// the preceding value is <=20M, so value*10+9 still fits u32 on AVR.
fn signed_digits(digits: &[u8], negative: bool) -> Result<i32, Error> {
    if digits.is_empty() || !digits.iter().all(u8::is_ascii_digit) {
        return Err(Error::Syntax);
    }
    let mut value = 0_u32;
    for &digit in digits {
        value = value * 10 + u32::from(digit - b'0');
        if value > 20_000_000 {
            return Err(Error::Range);
        }
    }
    let value = value as i32;
    Ok(if negative { -value } else { value })
}

fn integer(token: &[u8]) -> Result<i32, Error> {
    let (negative, digits) = sign(token);
    signed_digits(digits, negative)
}

fn degrees(token: &[u8]) -> Result<i32, Error> {
    let (negative, digits) = sign(token);
    // A valid fractional suffix is exactly ".digit". Anything else stays
    // in the whole token and is rejected by the existing decimal digit parser.
    let (whole, fraction) = if digits.len() >= 2
        && digits[digits.len() - 2] == b'.'
        && digits[digits.len() - 1].is_ascii_digit()
    {
        (
            &digits[..digits.len() - 2],
            i32::from(digits[digits.len() - 1] - b'0'),
        )
    } else {
        (digits, 0)
    };
    // Keep the same sign while combining whole degrees and the one decimal.
    // In particular "-0.1" must stay negative despite its zero whole part.
    let whole = signed_digits(whole, negative)?;
    // Bound first, then scale: 3600*10+9 fits comfortably in i32. The caller
    // enforces the final tenths limit, rejecting e.g. ROT 3600.1 as RANGE.
    if whole.unsigned_abs() > (ROT_LIMIT_TENTHS / 10) as u32 {
        return Err(Error::Range);
    }
    Ok(whole * 10 + if negative { -fraction } else { fraction })
}
