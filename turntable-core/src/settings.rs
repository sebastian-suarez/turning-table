//! Two power-cut-safe EEPROM records. No hardware access belongs in the codec.

pub const SETTINGS_BYTES: usize = 24;
pub const ARM_MAX_TENTHS: i16 = 650;
pub const MAX_RINGS: u8 = 4;
pub const MAX_PHOTOS: u16 = 300;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rings {
    pub heights: [u8; 4],
    pub len: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScanSettings {
    pub rings: u8,
    pub photos: u16,
    pub interval_ms: u16,
}

impl ScanSettings {
    pub fn valid(&self) -> bool {
        (1..=MAX_RINGS).contains(&self.rings)
            && (1..=MAX_PHOTOS).contains(&self.photos)
            && (100..=60_000).contains(&self.interval_ms)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub pratio_x100: u32,
    pub aratio_x100: u32,
    pub ahome_tenths: i16,
    pub rings: Rings,
    pub scan: ScanSettings,
}

impl Settings {
    pub const DEFAULT: Self = Self {
        pratio_x100: 409_600,
        aratio_x100: 3_276_800,
        ahome_tenths: 50,
        rings: Rings {
            heights: [10, 35, 60, 0],
            len: 3,
        },
        scan: ScanSettings {
            rings: 3,
            photos: 36,
            interval_ms: 3000,
        },
    };

    pub fn valid(&self) -> bool {
        (100_000..=1_000_000).contains(&self.pratio_x100)
            && (100_000..=20_000_000).contains(&self.aratio_x100)
            && (0..=300).contains(&self.ahome_tenths)
            && (1..=MAX_RINGS).contains(&self.rings.len)
            && self.rings.heights.iter().enumerate().all(|(i, &h)| {
                if i < usize::from(self.rings.len) {
                    h <= 90
                } else {
                    h == 0
                }
            })
            && self.scan.valid()
    }

    // Encode fields explicitly: Rust's in-memory padding/layout is not an
    // EEPROM format. Byte 22 covers both the payload and generation at 23.
    pub fn encode(&self, generation: u8) -> [u8; SETTINGS_BYTES] {
        let mut r = [0; SETTINGS_BYTES];
        r[0] = 0x54;
        r[1] = 1;
        r[2..6].copy_from_slice(&self.pratio_x100.to_le_bytes());
        r[6..10].copy_from_slice(&self.aratio_x100.to_le_bytes());
        r[10..12].copy_from_slice(&self.ahome_tenths.to_le_bytes());
        r[12] = self.rings.len;
        r[13..17].copy_from_slice(&self.rings.heights);
        r[17] = self.scan.rings;
        r[18..20].copy_from_slice(&self.scan.photos.to_le_bytes());
        r[20..22].copy_from_slice(&self.scan.interval_ms.to_le_bytes());
        r[23] = generation;
        r[22] = crc(&r);
        r
    }

    pub fn decode(r: &[u8; SETTINGS_BYTES]) -> Option<(Self, u8)> {
        if r[0] != 0x54 || r[1] != 1 || r[22] != crc(r) {
            return None;
        }
        let settings = Self {
            pratio_x100: u32::from_le_bytes([r[2], r[3], r[4], r[5]]),
            aratio_x100: u32::from_le_bytes([r[6], r[7], r[8], r[9]]),
            ahome_tenths: i16::from_le_bytes([r[10], r[11]]),
            rings: Rings {
                len: r[12],
                heights: [r[13], r[14], r[15], r[16]],
            },
            scan: ScanSettings {
                rings: r[17],
                photos: u16::from_le_bytes([r[18], r[19]]),
                interval_ms: u16::from_le_bytes([r[20], r[21]]),
            },
        };
        settings.valid().then_some((settings, r[23]))
    }
}

fn crc(record: &[u8; SETTINGS_BYTES]) -> u8 {
    let mut crc = 0_u8;
    for &byte in record[..22].iter().chain(&record[23..]) {
        crc ^= byte;
        for _ in 0..8 {
            crc = if crc & 0x80 != 0 {
                (crc << 1) ^ 0x07
            } else {
                crc << 1
            };
        }
    }
    crc
}

pub struct Loaded {
    pub settings: Settings,
    pub slot: u8,
    pub generation: u8,
}

// Only adjacent generations establish recency. Equality or a larger gap is
// deliberately resolved to slot 0, including after wrapping 255 -> 0.
pub fn load(slot0: &[u8; 24], slot1: &[u8; 24]) -> Option<Loaded> {
    let a = Settings::decode(slot0);
    let b = Settings::decode(slot1);
    let (slot, (settings, generation)) = match (a, b) {
        (Some(a), Some(b)) if b.1 == a.1.wrapping_add(1) => (1, b),
        (Some(a), _) => (0, a),
        (None, Some(b)) => (1, b),
        (None, None) => return None,
    };
    Some(Loaded {
        settings,
        slot,
        generation,
    })
}

pub struct SavePlan {
    old: [u8; 24],
    new: [u8; 24],
    base: u16,
    offset: u8,
}

#[inline(never)]
pub fn save_plan(slot0: &[u8; 24], slot1: &[u8; 24], settings: &Settings) -> SavePlan {
    let current = load(slot0, slot1);
    let destination = current.as_ref().map_or(0, |l| 1 - l.slot);
    SavePlan {
        old: if destination == 0 { *slot0 } else { *slot1 },
        new: settings.encode(current.map_or(1, |l| l.generation.wrapping_add(1))),
        base: u16::from(destination) * 32,
        offset: 0,
    }
}

impl Iterator for SavePlan {
    type Item = (u16, u8);

    fn next(&mut self) -> Option<Self::Item> {
        while self.offset <= 24 {
            let offset = self.offset;
            self.offset += 1;
            // Invalidate BEFORE touching any payload, even if the old payload
            // happens to match. Publish only after CRC and generation are set.
            if offset == 0 {
                if self.old[0] == 0x54 {
                    return Some((self.base, 0));
                }
            } else if offset == 24 {
                return Some((self.base, 0x54));
            } else if self.old[usize::from(offset)] != self.new[usize::from(offset)] {
                return Some((self.base + u16::from(offset), self.new[usize::from(offset)]));
            }
        }
        None
    }
}
