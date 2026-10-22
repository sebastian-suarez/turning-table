//! Electrical phases of the 28BYJ-48, independent of GPIO register layout.

/// Read each nibble left to right as IN1, IN2, IN3, IN4. Thus bit 3 means
/// IN1, even though IN1 is wired to the *lowest* motor GPIO, PD2. Firmware
/// reverses the nibble when mapping it onto PD2..PD5; keeping this table in the
/// wiring diagram's order makes it easy to compare against the ULN2003 LEDs.
pub const HALF_STEPS: [u8; 8] = [
    0b1000, 0b1100, 0b0100, 0b0110, 0b0010, 0b0011, 0b0001, 0b1001,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Reverse,
}

/// The phase index is a physical property of the last emitted step. Releasing
/// coils or resetting the logical position must not reset this index: otherwise
/// re-energising could jump the rotor to a different phase without counting it.
#[derive(Clone, Copy, Debug, Default)]
pub struct Sequencer {
    index: u8,
}

impl Sequencer {
    pub const fn new() -> Self {
        Self { index: 0 }
    }

    pub fn pattern(&self) -> u8 {
        HALF_STEPS[self.index as usize]
    }

    pub const fn index(&self) -> u8 {
        self.index
    }

    pub fn advance(&mut self, direction: Direction) -> u8 {
        // Adding 7 modulo 8 is subtraction by 1 without unsigned underflow.
        // Adjacent entries, including the wrap, differ in exactly one coil.
        let increment = match direction {
            Direction::Forward => 1,
            Direction::Reverse => 7,
        };
        self.index = (self.index + increment) & 7;
        self.pattern()
    }
}
