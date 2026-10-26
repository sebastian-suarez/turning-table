//! Phase 3 acceptance suite, derived from `docs/specs/phase-3/spec.md` (v3, frozen 2026-10-26).
//!
//! This file is protected once `spec.sh ready` has recorded its hash: the implementation conforms to it, never
//! the other way round. A test that contradicts the spec is reported, not edited. Each test name starts with
//! the criterion id it checks (P1, S4, ...). P6, the Phase 2 suite, runs from `src/tests.rs` in the same
//! `cargo test`.
//!
//! It is an integration test, so it sees only the crate's public interface: the items listed in the spec's
//! "Core interface" section. Expected numbers come from the spec's formulas and worked examples; where a test
//! needs a formula over many inputs, it carries a small reference model of that formula, written from the spec.

use std::collections::HashSet;

use turntable_core::controller::DEFAULT_PERIOD_MS;
use turntable_core::{
    admit, button_action, load, save_plan, Activity, ButtonAction, Command, Controller, Debouncer,
    Error, Homing, HomingStep, PreparedCommand, Rings, Scan, ScanMove, ScanSettings, Settings,
    State, SETTINGS_BYTES,
};

// ---------------------------------------------------------------------------------------------------------
// Helpers shared by several groups
// ---------------------------------------------------------------------------------------------------------

fn parse(line: &str) -> Result<Command, Error> {
    Command::parse(line.as_bytes())
}

/// A ring list with its unused entries zero, as the EEPROM record requires.
fn rings(heights: &[u8]) -> Rings {
    let mut all = [0; 4];
    all[..heights.len()].copy_from_slice(heights);
    Rings {
        heights: all,
        len: heights.len() as u8,
    }
}

fn run(rings: u8, photos: u16, interval_ms: u16) -> ScanSettings {
    ScanSettings {
        rings,
        photos,
        interval_ms,
    }
}

fn with_rings(heights: &[u8]) -> Settings {
    Settings {
        rings: rings(heights),
        ..Settings::DEFAULT
    }
}

/// Field-by-field equality, so the suite does not require `PartialEq` on `Settings` itself.
fn same(a: &Settings, b: &Settings) -> bool {
    a.pratio_x100 == b.pratio_x100
        && a.aratio_x100 == b.aratio_x100
        && a.ahome_tenths == b.ahome_tenths
        && a.rings.len == b.rings.len
        && a.rings.heights == b.rings.heights
        && a.scan.rings == b.scan.rings
        && a.scan.photos == b.scan.photos
        && a.scan.interval_ms == b.scan.interval_ms
}

/// The spec's rounding: a rational rounded to the nearest integer, exact halves away from zero.
fn round_half_away(numerator: i128, denominator: i128) -> i128 {
    let quotient = numerator.abs() / denominator;
    let remainder = numerator.abs() % denominator;
    let rounded = quotient + i128::from(2 * remainder >= denominator);
    if numerator < 0 {
        -rounded
    } else {
        rounded
    }
}

/// Arm position (half-steps from 0°) of an elevation in tenths of a degree.
fn arm_position(tenths: i32, aratio_x100: u32) -> i32 {
    round_half_away(i128::from(tenths) * i128::from(aratio_x100), 360_000) as i32
}

// ---------------------------------------------------------------------------------------------------------
// P: parser
// ---------------------------------------------------------------------------------------------------------

#[test]
fn p1_arm_takes_one_decimal_between_0_and_90() {
    assert_eq!(parse("ARM 10"), Ok(Command::Arm(100)));
    assert_eq!(parse("arm 47.5"), Ok(Command::Arm(475)));
    assert_eq!(parse("ARM 0"), Ok(Command::Arm(0)));
    assert_eq!(parse("ARM 90"), Ok(Command::Arm(900)));
    for line in ["ARM 90.1", "ARM -0.1"] {
        assert_eq!(parse(line), Err(Error::Range), "{line}");
    }
    for line in ["ARM", "ARM 1.25", "ARM 1 2"] {
        assert_eq!(parse(line), Err(Error::Syntax), "{line}");
    }
}

#[test]
fn p2_bare_verbs_take_no_argument() {
    assert_eq!(parse("HOME"), Ok(Command::Home));
    assert_eq!(parse("STOP"), Ok(Command::Stop));
    assert_eq!(parse("CFG"), Ok(Command::Cfg));
    assert_eq!(parse("scan"), Ok(Command::Scan(None)));
    for line in ["HOME 1", "STOP 1", "CFG 1"] {
        assert_eq!(parse(line), Err(Error::Syntax), "{line}");
    }
    // STOP must not swallow the Phase 2 verb STOPS, nor the other way round.
    assert_eq!(parse("STOPS 36"), Ok(Command::Stops(36)));
}

#[test]
fn p2_new_errors_have_terse_names() {
    assert_eq!(Error::Home.as_str(), "HOME");
    assert_eq!(Error::Save.as_str(), "SAVE");
}

fn parsed_rings(line: &str) -> Vec<u8> {
    match parse(line) {
        Ok(Command::Rings(r)) => r.heights[..usize::from(r.len)].to_vec(),
        other => panic!("{line}: expected Rings, got {other:?}"),
    }
}

#[test]
fn p3_rings_keeps_one_to_four_whole_degrees_in_order() {
    assert_eq!(parsed_rings("RINGS 10"), [10]);
    assert_eq!(parsed_rings("RINGS 10 35 60"), [10, 35, 60]);
    assert_eq!(parsed_rings("RINGS 60 35 10 5"), [60, 35, 10, 5]);
    assert_eq!(parsed_rings("rings 0 90"), [0, 90]);
    for line in ["RINGS", "RINGS 1 2 3 4 5", "RINGS 10.5"] {
        assert_eq!(parse(line), Err(Error::Syntax), "{line}");
    }
    assert_eq!(parse("RINGS 91"), Err(Error::Range));
}

fn parsed_scan(line: &str) -> (u8, u16, u16) {
    match parse(line) {
        Ok(Command::Scan(Some(s))) => (s.rings, s.photos, s.interval_ms),
        other => panic!("{line}: expected Scan(Some), got {other:?}"),
    }
}

#[test]
fn p4_scan_takes_rings_photos_and_interval() {
    assert_eq!(parsed_scan("SCAN 3 36 3000"), (3, 36, 3000));
    // These parse; whether they can run is Scan::new's decision (N2).
    assert_eq!(parsed_scan("SCAN 4 300 60000"), (4, 300, 60000));
    assert_eq!(parsed_scan("SCAN 1 1 100"), (1, 1, 100));
    for line in ["SCAN 3 36", "SCAN 1 2 3 4"] {
        assert_eq!(parse(line), Err(Error::Syntax), "{line}");
    }
    for line in [
        "SCAN 0 36 3000",
        "SCAN 5 36 3000",
        "SCAN 1 0 3000",
        "SCAN 1 301 3000",
        "SCAN 1 36 99",
        "SCAN 1 36 60001",
    ] {
        assert_eq!(parse(line), Err(Error::Range), "{line}");
    }
}

#[test]
fn p5_calibration_commands_and_their_bounds() {
    assert_eq!(parse("PRATIO 100000"), Ok(Command::PRatio(100_000)));
    assert_eq!(parse("PRATIO 1000000"), Ok(Command::PRatio(1_000_000)));
    assert_eq!(parse("ARATIO 100000"), Ok(Command::ARatio(100_000)));
    assert_eq!(parse("ARATIO 20000000"), Ok(Command::ARatio(20_000_000)));
    assert_eq!(parse("AHOME 0"), Ok(Command::AHome(0)));
    assert_eq!(parse("AHOME 5.5"), Ok(Command::AHome(55)));
    assert_eq!(parse("ahome 30"), Ok(Command::AHome(300)));
    for line in [
        "PRATIO 99999",
        "PRATIO 1000001",
        "ARATIO 99999",
        "ARATIO 20000001",
        "AHOME 30.1",
        "AHOME -0.1",
    ] {
        assert_eq!(parse(line), Err(Error::Range), "{line}");
    }
    // The clock correction was dropped in v2: TRIM is not a command.
    assert_eq!(parse("TRIM 0"), Err(Error::Syntax));
}

// ---------------------------------------------------------------------------------------------------------
// S: settings and EEPROM
// ---------------------------------------------------------------------------------------------------------

/// CRC-8 exactly as the record table defines it: polynomial 0x07, initial value 0, no reflection, no final XOR.
fn crc8(bytes: impl IntoIterator<Item = u8>) -> u8 {
    let mut crc = 0_u8;
    for byte in bytes {
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

/// Lay out a record by the record table, without validating any field, then set its CRC.
/// S2 uses it to pin `encode`'s layout; S5 uses it to forge records whose only fault is one field.
fn forge(s: &Settings, magic: u8, version: u8, generation: u8) -> [u8; 24] {
    let mut r = [0_u8; 24];
    r[0] = magic;
    r[1] = version;
    r[2..6].copy_from_slice(&s.pratio_x100.to_le_bytes());
    r[6..10].copy_from_slice(&s.aratio_x100.to_le_bytes());
    r[10..12].copy_from_slice(&s.ahome_tenths.to_le_bytes());
    r[12] = s.rings.len;
    r[13..17].copy_from_slice(&s.rings.heights);
    r[17] = s.scan.rings;
    r[18..20].copy_from_slice(&s.scan.photos.to_le_bytes());
    r[20..22].copy_from_slice(&s.scan.interval_ms.to_le_bytes());
    r[23] = generation;
    r[22] = crc8(r[..22].iter().copied().chain([r[23]]));
    r
}

fn minimum_settings() -> Settings {
    Settings {
        pratio_x100: 100_000,
        aratio_x100: 100_000,
        ahome_tenths: 0,
        rings: rings(&[0]),
        scan: run(1, 1, 100),
    }
}

fn maximum_settings() -> Settings {
    Settings {
        pratio_x100: 1_000_000,
        aratio_x100: 20_000_000,
        ahome_tenths: 300,
        rings: rings(&[90, 90, 90, 90]),
        scan: run(4, 300, 60000),
    }
}

#[test]
fn s1_defaults_are_the_settings_table() {
    let d = Settings::DEFAULT;
    assert_eq!(d.pratio_x100, 409_600);
    assert_eq!(d.aratio_x100, 3_276_800);
    assert_eq!(d.ahome_tenths, 50);
    assert_eq!(d.rings.len, 3);
    assert_eq!(d.rings.heights, [10, 35, 60, 0]);
    assert_eq!(
        (d.scan.rings, d.scan.photos, d.scan.interval_ms),
        (3, 36, 3000)
    );
}

#[test]
fn s2_encode_follows_the_record_table_and_decode_inverts_it() {
    assert_eq!(SETTINGS_BYTES, 24);
    for s in [Settings::DEFAULT, minimum_settings(), maximum_settings()] {
        for generation in [0, 1, 255] {
            let bytes = s.encode(generation);
            assert_eq!(bytes, forge(&s, 0x54, 1, generation), "layout");
            let (back, g) = Settings::decode(&bytes).expect("a valid record decodes");
            assert!(same(&back, &s));
            assert_eq!(g, generation);
        }
    }
}

#[test]
fn s3_blank_and_zeroed_eeprom_are_not_records() {
    assert!(Settings::decode(&[0xFF; 24]).is_none());
    assert!(Settings::decode(&[0x00; 24]).is_none());
}

#[test]
fn s4_any_single_changed_byte_is_rejected() {
    for s in [Settings::DEFAULT, minimum_settings(), maximum_settings()] {
        let valid = s.encode(9);
        for position in 0..24 {
            for flip in [0x01, 0xFF] {
                let mut bytes = valid;
                bytes[position] ^= flip;
                assert!(
                    Settings::decode(&bytes).is_none(),
                    "byte {position} ^ {flip:#04x} was accepted"
                );
            }
        }
    }
}

#[test]
fn s5_a_correct_crc_does_not_rescue_an_out_of_range_field() {
    let mut cases: Vec<(&str, [u8; 24])> = vec![
        ("magic", forge(&Settings::DEFAULT, 0x55, 1, 3)),
        ("version", forge(&Settings::DEFAULT, 0x54, 2, 3)),
    ];
    let out_of_range: Vec<(&str, Settings)> = vec![
        (
            "pratio low",
            Settings {
                pratio_x100: 99_999,
                ..Settings::DEFAULT
            },
        ),
        (
            "pratio high",
            Settings {
                pratio_x100: 1_000_001,
                ..Settings::DEFAULT
            },
        ),
        (
            "aratio low",
            Settings {
                aratio_x100: 99_999,
                ..Settings::DEFAULT
            },
        ),
        (
            "aratio high",
            Settings {
                aratio_x100: 20_000_001,
                ..Settings::DEFAULT
            },
        ),
        (
            "ahome low",
            Settings {
                ahome_tenths: -1,
                ..Settings::DEFAULT
            },
        ),
        (
            "ahome high",
            Settings {
                ahome_tenths: 301,
                ..Settings::DEFAULT
            },
        ),
        (
            "ring count 0",
            Settings {
                rings: Rings {
                    heights: [0; 4],
                    len: 0,
                },
                ..Settings::DEFAULT
            },
        ),
        (
            "ring count 5",
            Settings {
                rings: Rings {
                    heights: [10; 4],
                    len: 5,
                },
                ..Settings::DEFAULT
            },
        ),
        (
            "height 91",
            Settings {
                rings: rings(&[10, 91]),
                ..Settings::DEFAULT
            },
        ),
        (
            "unused entry",
            Settings {
                rings: Rings {
                    heights: [10, 35, 60, 1],
                    len: 3,
                },
                ..Settings::DEFAULT
            },
        ),
        (
            "scan rings 0",
            Settings {
                scan: run(0, 36, 3000),
                ..Settings::DEFAULT
            },
        ),
        (
            "scan rings 5",
            Settings {
                scan: run(5, 36, 3000),
                ..Settings::DEFAULT
            },
        ),
        (
            "photos 0",
            Settings {
                scan: run(3, 0, 3000),
                ..Settings::DEFAULT
            },
        ),
        (
            "photos 301",
            Settings {
                scan: run(3, 301, 3000),
                ..Settings::DEFAULT
            },
        ),
        (
            "interval 99",
            Settings {
                scan: run(3, 36, 99),
                ..Settings::DEFAULT
            },
        ),
        (
            "interval 60001",
            Settings {
                scan: run(3, 36, 60001),
                ..Settings::DEFAULT
            },
        ),
    ];
    for (name, s) in &out_of_range {
        cases.push((name, forge(s, 0x54, 1, 3)));
    }
    for (name, bytes) in cases {
        assert!(Settings::decode(&bytes).is_none(), "{name} was accepted");
    }
}

/// The loaded settings, slot and generation as one comparable value.
type Key = (u32, u32, i16, u8, [u8; 4], u8, u16, u16, u8, u8);

fn key_of(s: &Settings, slot: u8, generation: u8) -> Key {
    (
        s.pratio_x100,
        s.aratio_x100,
        s.ahome_tenths,
        s.rings.len,
        s.rings.heights,
        s.scan.rings,
        s.scan.photos,
        s.scan.interval_ms,
        slot,
        generation,
    )
}

fn loaded(slot0: &[u8; 24], slot1: &[u8; 24]) -> Option<Key> {
    load(slot0, slot1).map(|l| key_of(&l.settings, l.slot, l.generation))
}

fn other_settings() -> Settings {
    Settings {
        pratio_x100: 407_577,
        rings: rings(&[15, 45]),
        scan: run(2, 24, 2000),
        ..Settings::DEFAULT
    }
}

#[test]
fn s6_load_picks_the_slot_one_generation_ahead() {
    let a = Settings::DEFAULT;
    let b = other_settings();
    let pick = |g0: u8, g1: u8| loaded(&a.encode(g0), &b.encode(g1));
    assert_eq!(pick(4, 5), Some(key_of(&b, 1, 5)));
    assert_eq!(pick(5, 4), Some(key_of(&a, 0, 5)));
    assert_eq!(pick(255, 0), Some(key_of(&b, 1, 0)));
    assert_eq!(pick(0, 255), Some(key_of(&a, 0, 0)));
    // Equal generations, or any other difference: slot 0.
    assert_eq!(pick(7, 7), Some(key_of(&a, 0, 7)));
    assert_eq!(pick(3, 9), Some(key_of(&a, 0, 3)));
    assert_eq!(pick(9, 3), Some(key_of(&a, 0, 9)));
    // One valid slot, either side.
    assert_eq!(loaded(&a.encode(8), &[0xFF; 24]), Some(key_of(&a, 0, 8)));
    assert_eq!(loaded(&[0xFF; 24], &b.encode(8)), Some(key_of(&b, 1, 8)));
    let mut corrupt = a.encode(4);
    corrupt[5] ^= 0x10;
    assert_eq!(loaded(&corrupt, &b.encode(3)), Some(key_of(&b, 1, 3)));
    // None valid.
    assert_eq!(loaded(&[0xFF; 24], &[0xFF; 24]), None);
    assert_eq!(loaded(&[0x00; 24], &[0xFF; 24]), None);
}

/// Two EEPROM slots: slot 0 at address 0, slot 1 at address 32.
type Eeprom = ([u8; 24], [u8; 24]);

fn write(e: &mut Eeprom, address: u16, byte: u8) {
    match address {
        0..=23 => e.0[usize::from(address)] = byte,
        32..=55 => e.1[usize::from(address) - 32] = byte,
        _ => panic!("save_plan wrote outside both slots, at address {address}"),
    }
}

fn plan(e: &Eeprom, s: &Settings) -> Vec<(u16, u8)> {
    save_plan(&e.0, &e.1, s).collect()
}

#[test]
fn s7_save_plan_clears_a_valid_destination_first_and_writes_its_magic_last() {
    let new = other_settings();
    // (start, destination base address, generation of the new record, whether the destination holds 0x54)
    let starts: Vec<(Eeprom, u16, u8, bool)> = vec![
        (
            (Settings::DEFAULT.encode(4), Settings::DEFAULT.encode(5)),
            0,
            6,
            true,
        ),
        (
            (Settings::DEFAULT.encode(5), Settings::DEFAULT.encode(4)),
            32,
            6,
            true,
        ),
        (
            (Settings::DEFAULT.encode(0), Settings::DEFAULT.encode(255)),
            32,
            1,
            true,
        ),
        (
            (Settings::DEFAULT.encode(255), Settings::DEFAULT.encode(0)),
            0,
            1,
            true,
        ),
        ((Settings::DEFAULT.encode(1), [0xFF; 24]), 32, 2, false),
        (([0xFF; 24], [0xFF; 24]), 0, 1, false),
    ];
    for (start, base, generation, valid_destination) in starts {
        let writes = plan(&start, &new);
        assert!(writes.len() <= 25, "{} writes", writes.len());
        assert!(
            writes.iter().all(|&(a, _)| (base..base + 24).contains(&a)),
            "outside the destination"
        );
        let (first, last) = (writes[0], writes[writes.len() - 1]);
        assert_eq!(last, (base, 0x54), "the magic byte goes last");
        let middle = if valid_destination {
            assert_eq!(first, (base, 0x00), "a valid destination is cleared first");
            &writes[1..writes.len() - 1]
        } else {
            assert_ne!(first.0, base, "a destination without 0x54 is not cleared");
            &writes[..writes.len() - 1]
        };
        assert!(
            middle.windows(2).all(|w| w[0].0 < w[1].0),
            "offsets 1..23 in address order"
        );
        assert!(middle.iter().all(|&(a, _)| a > base && a < base + 24));
        let mut after = start;
        for &(a, b) in &writes {
            write(&mut after, a, b);
        }
        let destination = if base == 0 { after.0 } else { after.1 };
        assert_eq!(
            destination,
            new.encode(generation),
            "the slot holds exactly the new record"
        );
        assert_eq!(
            loaded(&after.0, &after.1),
            Some(key_of(&new, (base / 32) as u8, generation))
        );
    }
}

/// S8: every cut position and every listed torn value, for up to three saves in a row, from each start state.
/// States reached twice (for example a torn byte that happened to get its intended value) are explored once:
/// identical bytes have identical futures, so this stays exhaustive.
fn explore(e: Eeprom, saves: &[Settings], seen: &mut HashSet<(Eeprom, usize)>, checked: &mut u64) {
    let Some((new, rest)) = saves.split_first() else {
        return;
    };
    if !seen.insert((e, saves.len())) {
        return;
    }
    let before = loaded(&e.0, &e.1);
    let writes = plan(&e, new);
    let mut full = e;
    for &(a, b) in &writes {
        write(&mut full, a, b);
    }
    let after = loaded(&full.0, &full.1);
    let after_settings = load(&full.0, &full.1)
        .expect("a completed save loads")
        .settings;
    assert!(
        same(&after_settings, new),
        "a completed save loads the new settings"
    );

    for cut in 0..=writes.len() {
        let mut torn_values: Vec<Option<u8>> = Vec::new();
        if cut == writes.len() {
            torn_values.push(None);
        } else {
            let intended = writes[cut].1;
            for v in [0x00, 0xFF, intended, intended ^ 0x5A, 0x54] {
                if !torn_values.contains(&Some(v)) {
                    torn_values.push(Some(v));
                }
            }
        }
        for torn in torn_values {
            let mut state = e;
            for &(a, b) in &writes[..cut] {
                write(&mut state, a, b);
            }
            if let Some(v) = torn {
                write(&mut state, writes[cut].0, v);
            }
            let completes = cut == writes.len() || (cut + 1 == writes.len() && torn == Some(0x54));
            let expected = if completes { after } else { before };
            assert_eq!(
                loaded(&state.0, &state.1),
                expected,
                "save of {} cut after {cut} of {} writes, torn value {torn:?}",
                new.pratio_x100,
                writes.len()
            );
            *checked += 1;
            explore(state, rest, seen, checked);
        }
    }
}

#[test]
fn s8_interrupted_saves_leave_the_old_or_the_new_settings() {
    let saves = [
        Settings {
            pratio_x100: 378_759,
            aratio_x100: 3_260_616,
            ahome_tenths: 53,
            rings: rings(&[10, 40]),
            scan: run(2, 48, 2500),
        },
        Settings {
            pratio_x100: 383_999,
            rings: rings(&[5, 65, 5, 65]),
            scan: run(4, 70, 3000),
            ..Settings::DEFAULT
        },
        Settings {
            pratio_x100: 400_000,
            aratio_x100: 3_000_000,
            ahome_tenths: 120,
            rings: rings(&[20]),
            scan: run(1, 36, 3000),
        },
    ];
    let previous = other_settings();
    let starts: Vec<Eeprom> = vec![
        (Settings::DEFAULT.encode(1), previous.encode(2)),
        (Settings::DEFAULT.encode(2), previous.encode(1)),
        (Settings::DEFAULT.encode(255), previous.encode(0)),
        (Settings::DEFAULT.encode(0), previous.encode(255)),
        ([0xFF; 24], [0xFF; 24]),
    ];
    let mut checked = 0;
    for start in starts {
        let mut seen = HashSet::new();
        explore(start, &saves, &mut seen, &mut checked);
    }
    assert!(checked > 10_000, "only {checked} states checked");
}

// ---------------------------------------------------------------------------------------------------------
// C: controller halt and moves at a given period
// ---------------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Tick {
    pattern: u8,
    done: Option<i32>,
    position: i32,
}

fn tick(c: &mut Controller) -> Tick {
    let output = c.tick();
    Tick {
        pattern: output.pattern,
        done: output.done,
        position: c.snapshot().position,
    }
}

fn ticks(c: &mut Controller, n: usize) -> Vec<Tick> {
    (0..n).map(|_| tick(c)).collect()
}

fn set_speed(c: &mut Controller, period: u8) {
    c.apply(PreparedCommand::Speed(period))
        .expect("SPEED while idle");
}

/// Ticks (1-based, counted from the first tick after the start) at which the position changed.
fn step_ticks(trace: &[Tick], start_position: i32) -> Vec<usize> {
    let mut previous = start_position;
    let mut steps = Vec::new();
    for (i, t) in trace.iter().enumerate() {
        if t.position != previous {
            assert_eq!(
                (t.position - previous).abs(),
                1,
                "one half-step per tick at most"
            );
            steps.push(i + 1);
            previous = t.position;
        }
    }
    steps
}

fn done_ticks(trace: &[Tick]) -> Vec<(usize, i32)> {
    trace
        .iter()
        .enumerate()
        .filter_map(|(i, t)| t.done.map(|p| (i + 1, p)))
        .collect()
}

/// After a halt between two ticks: no half-step, the last pattern until the 50th tick, all-low from it,
/// and exactly one Done, on the 150th tick, carrying the position reached.
fn assert_halt_timing(after: &[Tick], last_pattern: u8, position: i32) {
    for (i, t) in after.iter().enumerate() {
        let n = i + 1;
        assert_eq!(t.position, position, "half-step after the halt, tick {n}");
        let expected = if n < 50 { last_pattern } else { 0 };
        assert_eq!(t.pattern, expected, "pattern on tick {n} after the halt");
    }
    assert_eq!(done_ticks(after), vec![(150, position)]);
}

#[test]
fn c1_halt_while_stepping_stops_at_once_and_releases_on_the_usual_schedule() {
    for period in [1_u8, 2, 50] {
        for direction in [1_i32, -1] {
            for offset in 0..period.min(3) {
                let mut c = Controller::new();
                set_speed(&mut c, period);
                c.start(direction * 40).unwrap();
                // Run to the 5th half-step, then `offset` more ticks into the next period.
                let mut trace = Vec::new();
                while step_ticks(&trace, 0).len() < 5 {
                    trace.push(tick(&mut c));
                }
                trace.extend(ticks(&mut c, usize::from(offset)));
                let at_halt = *trace.last().unwrap();
                assert_eq!(c.snapshot().state, State::Stepping);
                c.halt();
                let after = ticks(&mut c, 400);
                assert_halt_timing(&after, at_halt.pattern, direction * 5);

                // The phase is retained: the next move energises the same coils, then steps on from them.
                c.start(direction).unwrap();
                let next = ticks(&mut c, 30);
                assert_eq!(
                    next[0].pattern, at_halt.pattern,
                    "energise resumes the retained phase"
                );
            }
        }
    }
}

#[test]
fn c2_halt_while_energising_emits_no_half_step() {
    for ticks_before in [0_usize, 1, 5, 19] {
        let mut c = Controller::new();
        c.start(10).unwrap();
        let before = ticks(&mut c, ticks_before);
        assert_eq!(c.snapshot().state, State::Energise);
        let last_pattern = before.last().map_or(c.pattern(), |t| t.pattern);
        c.halt();
        let after = ticks(&mut c, 400);
        assert_halt_timing(&after, last_pattern, 0);
        assert!(step_ticks(&after, 0).is_empty());
    }
}

/// Run a 3-step move, optionally halting right after the first tick that leaves the controller in `state`.
fn traced_move(halt_in: Option<State>) -> Vec<Tick> {
    let mut c = Controller::new();
    c.start(3).unwrap();
    let mut trace = Vec::new();
    let mut halted = false;
    for _ in 0..600 {
        trace.push(tick(&mut c));
        if !halted && Some(c.snapshot().state) == halt_in {
            c.halt();
            halted = true;
        }
    }
    if let Some(state) = halt_in {
        assert!(halted, "the move never reached {state:?}");
    }
    trace
}

#[test]
fn c3_halt_after_the_last_step_or_when_idle_changes_nothing() {
    let reference = traced_move(None);
    for state in [State::Hold, State::Release, State::Settle, State::Idle] {
        assert_eq!(traced_move(Some(state)), reference, "halt in {state:?}");
    }
    // Halting a controller that never moved is harmless too.
    let mut c = Controller::new();
    c.halt();
    assert!(ticks(&mut c, 200)
        .iter()
        .all(|t| t.pattern == 0 && t.done.is_none() && t.position == 0));
}

#[test]
fn c4_start_with_period_steps_at_its_own_period_and_leaves_speed_alone() {
    for period in [1_u8, 26, 53, 255] {
        let mut c = Controller::new();
        set_speed(&mut c, 5);
        c.start_with_period(6, period).unwrap();
        let trace = ticks(&mut c, 21 + 6 * usize::from(period) + 200);
        // Phase 2's rule: from the first energised tick (tick 1), the first half-step comes 20 ticks plus one
        // period later, then one per period; release 50 ticks after the last, Done 150 ticks after it.
        let p = usize::from(period);
        let expected: Vec<usize> = (0..6).map(|k| 21 + p + k * p).collect();
        assert_eq!(step_ticks(&trace, 0), expected, "period {period}");
        let last = expected[5];
        assert_eq!(done_ticks(&trace), vec![(last + 150, 6)]);
        assert_eq!(c.snapshot().period_ms, 5, "SPEED unchanged");

        // A later ordinary start uses the SPEED period again.
        c.start(2).unwrap();
        let later = ticks(&mut c, 300);
        assert_eq!(step_ticks(&later, 6), vec![21 + 5, 21 + 10]);
    }

    let mut c = Controller::new();
    assert_eq!(c.start_with_period(1, 0).err(), Some(Error::Range));
    assert_eq!(c.snapshot().period_ms, DEFAULT_PERIOD_MS);
    c.start_with_period(10, 3).unwrap();
    assert_eq!(c.start_with_period(1, 3).err(), Some(Error::Busy));
    ticks(&mut c, 5);
    assert_eq!(c.start_with_period(1, 3).err(), Some(Error::Busy));
}

// ---------------------------------------------------------------------------------------------------------
// D: debouncing, admission and the button
// ---------------------------------------------------------------------------------------------------------

fn presses(d: &mut Debouncer, samples: impl IntoIterator<Item = bool>) -> usize {
    samples
        .into_iter()
        .filter(|&closed| d.sample(closed))
        .count()
}

#[test]
fn d1_button_press_needs_twenty_stable_samples() {
    // A fresh debouncer starts as released: 20 closed samples are a press.
    let mut d = Debouncer::new(20);
    assert_eq!(presses(&mut d, [true; 19]), 0);
    assert!(d.sample(true), "the 20th closed sample registers the press");
    assert_eq!(presses(&mut d, [true; 1000]), 0, "holding repeats nothing");

    // After a release, a new press needs 20 open then 20 closed samples.
    assert_eq!(presses(&mut d, [false; 19]), 0);
    assert_eq!(
        presses(&mut d, [true; 20]),
        0,
        "19 open samples were not a release"
    );
    assert_eq!(presses(&mut d, [false; 20]), 0);
    assert_eq!(presses(&mut d, [true; 19]), 0);
    assert!(d.sample(true));

    // Bouncing contacts register nothing.
    let mut d = Debouncer::new(20);
    assert_eq!(presses(&mut d, [false; 20]), 0);
    assert_eq!(presses(&mut d, (0..1000).map(|i| i % 2 == 0)), 0);
    assert_eq!(presses(&mut d, (0..1000).map(|i| i % 7 != 0)), 0);
}

#[test]
fn d2_home_switch_level_follows_two_stable_samples() {
    let mut d = Debouncer::new(2);
    assert!(!d.closed());
    d.sample(true);
    assert!(!d.closed(), "one closed sample is not enough");
    d.sample(true);
    assert!(d.closed(), "closed on the 2nd consecutive closed sample");
    d.sample(false);
    assert!(d.closed(), "one open sample is not enough");
    d.sample(false);
    assert!(!d.closed(), "open on the 2nd consecutive open sample");
    d.sample(true);
    d.sample(false);
    d.sample(true);
    assert!(!d.closed(), "alternating samples keep the level");
}

/// The five `Activity` flags as bits 0..4: platter idle, arm idle, scanning, homing, homed.
/// Rebuilt for every call, so the suite needs no `Copy` or `Debug` on `Activity`.
fn activity(bits: u8) -> Activity {
    Activity {
        platter_idle: bits & 1 != 0,
        arm_idle: bits & 2 != 0,
        scanning: bits & 4 != 0,
        homing: bits & 8 != 0,
        homed: bits & 16 != 0,
    }
}

fn describe(bits: u8) -> String {
    let flag = |bit: u8, name: &str| {
        if bits & bit != 0 {
            format!("{name} ")
        } else {
            String::new()
        }
    };
    format!(
        "{}{}{}{}{}",
        flag(1, "platter_idle"),
        flag(2, "arm_idle"),
        flag(4, "scanning"),
        flag(8, "homing"),
        flag(16, "homed")
    )
}

#[test]
fn d3_button_stops_motion_homes_or_starts_the_saved_scan() {
    for bits in 0..32 {
        let a = activity(bits);
        let moving = !a.platter_idle || !a.arm_idle || a.scanning || a.homing;
        let homed = a.homed;
        let action = button_action(a);
        let ok = if moving {
            matches!(action, ButtonAction::Stop)
        } else if !homed {
            matches!(action, ButtonAction::Home)
        } else {
            matches!(action, ButtonAction::Scan)
        };
        assert!(ok, "wrong action for: {}", describe(bits));
    }
}

#[test]
fn d4_admission_follows_the_spec_table() {
    let commands = [
        Command::Step(1),
        Command::Rot(10),
        Command::Stops(36),
        Command::Next,
        Command::Zero,
        Command::Pos,
        Command::Speed(2),
        Command::Status,
        Command::Arm(100),
        Command::Home,
        Command::Rings(rings(&[10])),
        Command::Scan(None),
        Command::Scan(Some(run(1, 36, 3000))),
        Command::Stop,
        Command::PRatio(409_600),
        Command::ARatio(3_276_800),
        Command::AHome(50),
        Command::Cfg,
    ];
    for command in commands {
        for bits in 0..32 {
            let a = activity(bits);
            let busy_all = !a.platter_idle || !a.arm_idle || a.scanning || a.homing;
            let expected = match command {
                Command::Pos | Command::Status | Command::Cfg | Command::Stop => Ok(()),
                Command::Speed(_) if a.scanning || a.homing => Err(Error::Busy),
                Command::Speed(_) => Ok(()),
                _ if busy_all => Err(Error::Busy),
                Command::Arm(_) | Command::Scan(_) if !a.homed => Err(Error::Home),
                _ => Ok(()),
            };
            assert_eq!(
                admit(&command, a),
                expected,
                "{command:?} with {}",
                describe(bits)
            );
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// H: homing
// ---------------------------------------------------------------------------------------------------------

fn is_move(step: &HomingStep, steps: i32, halt_on_switch: bool) -> bool {
    matches!(*step, HomingStep::Move { steps: s, period_ms: 4, halt_on_switch: h } if s == steps && h == halt_on_switch)
}

#[test]
fn h1_open_switch_starts_the_search_down() {
    let (_, step) = Homing::start(false, &Settings::DEFAULT);
    assert!(is_move(&step, -7282, true));
}

#[test]
fn h2_closed_switch_lifts_first_then_searches_or_fails() {
    let (mut h, step) = Homing::start(true, &Settings::DEFAULT);
    assert!(is_move(&step, 455, false));
    assert!(is_move(&h.done(false, false), -7282, true));

    let (mut h, _) = Homing::start(true, &Settings::DEFAULT);
    assert!(
        matches!(h.done(true, false), HomingStep::Failed),
        "still closed after the lift"
    );
}

#[test]
fn h3_switch_halt_homes_at_the_home_angle() {
    // (aratio, ahome tenths, search steps, lift steps, home position): round(tenths × aratio / 360000).
    let cases = [
        (3_276_800, 50, -7282, 455, 455),
        (3_260_616, 53, -7246, 453, 480),
        (100_000, 0, -222, 14, 0),
        (20_000_000, 300, -44444, 2778, 16667),
    ];
    for (aratio_x100, ahome_tenths, search, lift, home) in cases {
        let settings = Settings {
            aratio_x100,
            ahome_tenths,
            ..Settings::DEFAULT
        };
        assert_eq!(home, arm_position(i32::from(ahome_tenths), aratio_x100));
        for switch_closed in [false, true] {
            let (mut h, step) = Homing::start(false, &settings);
            assert!(
                is_move(&step, search, true),
                "search for aratio {aratio_x100}"
            );
            assert!(matches!(h.done(switch_closed, true), HomingStep::Homed(p) if p == home));
        }
        let (mut h, step) = Homing::start(true, &settings);
        assert!(is_move(&step, lift, false), "lift for aratio {aratio_x100}");
        assert!(is_move(&h.done(false, false), search, true));
        assert!(matches!(h.done(true, true), HomingStep::Homed(p) if p == home));
    }
}

#[test]
fn h4_search_without_a_switch_halt_fails() {
    for switch_closed in [false, true] {
        let (mut h, _) = Homing::start(false, &Settings::DEFAULT);
        assert!(matches!(h.done(switch_closed, false), HomingStep::Failed));
    }
}

#[test]
fn h5_stop_fails_homing_in_either_phase() {
    let (mut h, _) = Homing::start(true, &Settings::DEFAULT);
    h.stop();
    assert!(
        matches!(h.done(false, false), HomingStep::Failed),
        "stopped during the lift"
    );

    for (switch_closed, halted) in [(false, false), (true, true), (false, true)] {
        let (mut h, _) = Homing::start(false, &Settings::DEFAULT);
        h.stop();
        assert!(
            matches!(h.done(switch_closed, halted), HomingStep::Failed),
            "stopped during the search"
        );
    }
}

// ---------------------------------------------------------------------------------------------------------
// N: scan sequence
// ---------------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Move {
    Arm(i32),
    Turn,
}

/// The sequence, turn period and photo estimate the spec's "Scan sequence" section defines.
fn model(
    s: &Settings,
    r: ScanSettings,
    speed_ms: u8,
    arm: i32,
) -> Result<(Vec<Move>, u8, u32), Error> {
    const OVERHEAD: i128 = 171;
    if r.rings > s.rings.len {
        return Err(Error::Range);
    }
    let heights = &s.rings.heights[..usize::from(r.rings)];
    if heights
        .iter()
        .any(|&h| i32::from(h) * 10 < i32::from(s.ahome_tenths) || i32::from(h) * 10 > 650)
    {
        return Err(Error::Range);
    }
    let period = round_half_away(
        i128::from(r.photos) * i128::from(r.interval_ms) * 100,
        i128::from(s.pratio_x100),
    );
    if !(1..=255).contains(&period) {
        return Err(Error::Range);
    }
    let mut moves = Vec::new();
    let mut position = arm;
    let mut ticks: i128 = 0;
    for &h in heights {
        let target = arm_position(i32::from(h) * 10, s.aratio_x100);
        if target != position {
            moves.push(Move::Arm(target - position));
            ticks +=
                OVERHEAD + i128::from((target - position).unsigned_abs()) * i128::from(speed_ms);
            position = target;
        }
        moves.push(Move::Turn);
    }
    let rings = i128::from(r.rings);
    ticks += rings * OVERHEAD + round_half_away(rings * i128::from(s.pratio_x100), 100) * period;
    let photos = (ticks * 101 / 100 + 10_000) / i128::from(r.interval_ms) + 1;
    if photos > 300 {
        return Err(Error::Range);
    }
    Ok((moves, period as u8, photos as u32))
}

fn actual(
    s: &Settings,
    r: ScanSettings,
    speed_ms: u8,
    arm: i32,
) -> Result<(Vec<Move>, u8, u32), Error> {
    let mut scan = Scan::new(s, r, speed_ms, arm)?;
    let (period, photos) = (scan.turn_period_ms(), scan.photos());
    // `Scan` may implement `Iterator` (which clippy prefers for a `next` method) or have its own `next`:
    // `from_fn` calls `next` explicitly and works with both. Nine is one more than four rings can need.
    let moves: Vec<Move> = std::iter::from_fn(|| scan.next())
        .take(9)
        .map(|m| match m {
            ScanMove::Arm(steps) => Move::Arm(steps),
            ScanMove::Turn => Move::Turn,
        })
        .collect();
    assert!(moves.len() <= 8, "more moves than four rings can need");
    Ok((moves, period, photos))
}

#[test]
fn n1_worked_example() {
    use Move::{Arm, Turn};
    let d = Settings::DEFAULT;
    assert_eq!(
        actual(&d, run(3, 36, 3000), 2, 455),
        Ok((
            vec![Arm(455), Turn, Arm(2276), Turn, Arm(2275), Turn],
            26,
            115
        ))
    );
    assert_eq!(
        actual(&d, run(1, 36, 3000), 2, 455),
        Ok((vec![Arm(455), Turn], 26, 40))
    );
    // The test's model agrees with the worked example, so the grids below check the same formulas.
    assert_eq!(
        model(&d, run(3, 36, 3000), 2, 455),
        actual(&d, run(3, 36, 3000), 2, 455)
    );
}

#[test]
fn n2_turn_period_is_photos_times_interval_over_the_ratio() {
    let s = with_rings(&[10]);
    assert_eq!(
        actual(&s, run(1, 1, 100), 2, 455).err(),
        Some(Error::Range),
        "period 0"
    );
    assert_eq!(
        actual(&s, run(1, 300, 60000), 2, 455).err(),
        Some(Error::Range),
        "period 4395"
    );
    for pratio_x100 in [100_000, 407_577, 409_600, 1_000_000] {
        for photos in [1, 2, 36, 72, 300] {
            for interval in [100, 1000, 3000, 9999, 60000] {
                let s = Settings {
                    pratio_x100,
                    ..with_rings(&[10])
                };
                let r = run(1, photos, interval);
                assert_eq!(
                    actual(&s, r, 2, 455),
                    model(&s, r, 2, 455),
                    "{pratio_x100} {photos} {interval}"
                );
            }
        }
    }
}

#[test]
fn n3_arm_moves_go_exactly_to_each_ring() {
    use Move::{Arm, Turn};
    let d = Settings::DEFAULT;
    // Already at the first ring (10° = 910): no positioning move.
    assert_eq!(actual(&d, run(1, 36, 3000), 2, 910).unwrap().0, vec![Turn]);
    // Equal consecutive heights: no arm move between their turns.
    let same_height = with_rings(&[10, 10, 35]);
    assert_eq!(
        actual(&same_height, run(3, 36, 3000), 2, 455).unwrap().0,
        vec![Arm(455), Turn, Turn, Arm(2276), Turn]
    );
    // A descending list moves the arm down.
    let descending = with_rings(&[60, 35, 10]);
    assert_eq!(
        actual(&descending, run(3, 36, 3000), 2, 455).unwrap().0,
        vec![Arm(5006), Turn, Arm(-2275), Turn, Arm(-2276), Turn]
    );
    // Each move is the target minus the previous position, from any start and arm ratio.
    for aratio_x100 in [100_000, 3_260_616, 3_276_800, 20_000_000] {
        for arm in [-100, 0, 455, 3000, 6000] {
            let s = Settings {
                aratio_x100,
                ..with_rings(&[20, 50, 30, 65])
            };
            let r = run(4, 10, 1000);
            assert_eq!(
                actual(&s, r, 2, arm),
                model(&s, r, 2, arm),
                "{aratio_x100} from {arm}"
            );
        }
    }
}

#[test]
fn n4_photo_estimate_and_its_300_limit() {
    let d = Settings::DEFAULT;
    assert_eq!(
        actual(&d, run(3, 96, 3000), 2, 455).map(|m| (m.1, m.2)),
        Ok((70, 297))
    );
    assert_eq!(
        actual(&d, run(3, 97, 3000), 2, 455).err(),
        Some(Error::Range),
        "estimate 301"
    );
    let zigzag = with_rings(&[5, 65, 5, 65]);
    assert_eq!(
        actual(&zigzag, run(4, 70, 3000), 2, 455).map(|m| m.2),
        Ok(297)
    );
    assert_eq!(
        actual(&zigzag, run(4, 71, 3000), 2, 455).err(),
        Some(Error::Range),
        "estimate 302"
    );
    for speed in [1, 2, 4] {
        for pratio_x100 in [407_577, 409_600] {
            for arm in [455, 910, 3000] {
                for (photos, interval) in
                    [(36, 3000), (24, 5000), (72, 1500), (90, 3000), (100, 2000)]
                {
                    let s = Settings {
                        pratio_x100,
                        ..Settings::DEFAULT
                    };
                    let r = run(3, photos, interval);
                    assert_eq!(
                        actual(&s, r, speed, arm),
                        model(&s, r, speed, arm),
                        "speed {speed}, pratio {pratio_x100}, arm {arm}, {photos} × {interval}"
                    );
                }
            }
        }
    }
}

#[test]
fn n5_rings_must_exist_and_lie_within_the_arm_travel() {
    let r1 = run(1, 36, 3000);
    let two = with_rings(&[10, 35]);
    assert_eq!(
        actual(&two, run(3, 36, 3000), 2, 455).err(),
        Some(Error::Range),
        "3 rings of 2"
    );
    assert_eq!(
        actual(&with_rings(&[4]), r1, 2, 455).err(),
        Some(Error::Range),
        "below the home angle"
    );
    assert!(
        actual(&with_rings(&[5]), r1, 2, 455).is_ok(),
        "exactly the home angle"
    );
    assert!(
        actual(&with_rings(&[65]), r1, 2, 455).is_ok(),
        "exactly 65°"
    );
    assert_eq!(
        actual(&with_rings(&[66]), r1, 2, 455).err(),
        Some(Error::Range),
        "above 65°"
    );
    // Only the first `rings` heights are checked.
    let low_second = with_rings(&[10, 4]);
    assert!(actual(&low_second, r1, 2, 455).is_ok());
    assert_eq!(
        actual(&low_second, run(2, 36, 3000), 2, 455).err(),
        Some(Error::Range)
    );
    // The home angle is a setting: raising it moves the lower limit.
    let raised = Settings {
        ahome_tenths: 120,
        ..Settings::DEFAULT
    };
    assert_eq!(
        actual(&raised, r1, 2, 1092).err(),
        Some(Error::Range),
        "10° below a 12° home"
    );
}
