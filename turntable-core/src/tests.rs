//! Frozen Phase 2 acceptance cases, exercising the same objects used on AVR.
//! Only this cfg(test) module uses std, for assertions and test collections.

use std::{format, string::String, vec, vec::Vec};

use crate::{
    controller::{ENERGISE_MS, HOLD_MS, POSITION_LIMIT, SETTLE_MS},
    planner::round_half_away,
    sequencer::HALF_STEPS,
    Command, Controller, Direction, Error, LineAssembler, Output, Planner, PreparedCommand,
    RxBuffer, Sequencer, Snapshot, State, RATIO_X100,
};

fn planner(ratio: u32, stops: u16) -> Planner {
    Planner::new(ratio, stops).unwrap()
}

/// This is the firmware's prepare / atomic apply / commit transaction. Planning
/// is separate so firmware never puts the i64 operations inside its mutex.
fn execute(
    planner: &mut Planner,
    controller: &mut Controller,
    command: Command,
) -> Result<Output, Error> {
    let prepared = planner.prepare(command, controller.snapshot())?;
    let output = controller.apply(prepared)?;
    planner.commit(prepared);
    Ok(output)
}

fn finish(controller: &mut Controller) -> i32 {
    let mut completed = None;
    // Bounds the test so a broken state machine fails instead of hanging.
    for _ in 0..1_000_000 {
        let output = controller.tick();
        if output.done.is_some() {
            assert!(completed.is_none(), "duplicate Done");
            completed = output.done;
            break;
        }
    }
    let completed = completed.expect("move did not finish");
    for _ in 0..200 {
        assert_eq!(controller.tick().done, None);
    }
    completed
}

fn canonical(command: Command) -> String {
    match command {
        Command::Step(n) => format!("STEP {n}"),
        Command::Rot(tenths) => {
            let sign = if tenths < 0 { "-" } else { "" };
            let magnitude = tenths.unsigned_abs();
            format!("ROT {sign}{}.{}", magnitude / 10, magnitude % 10)
        }
        Command::Stops(n) => format!("STOPS {n}"),
        Command::Next => String::from("NEXT"),
        Command::Zero => String::from("ZERO"),
        Command::Pos => String::from("POS"),
        Command::Speed(n) => format!("SPEED {n}"),
        Command::Status => String::from("STATUS"),
    }
}

#[test]
fn console_decimal_conversion_covers_signs_boundaries_and_sampled_u32_values() {
    use crate::decimal::Decimal;
    let mut buffer = [0; 11];
    for value in [
        i32::MIN,
        -1_073_741_824,
        -1_000_000,
        -10,
        -1,
        0,
        1,
        10,
        1_000_000,
        1_073_741_824,
        i32::MAX,
    ] {
        assert_eq!(
            Decimal::signed(value).digits(&mut buffer),
            format!("{value}")
        );
    }
    for value in [
        0,
        1,
        9,
        10,
        99,
        100,
        999,
        1000,
        65535,
        65536,
        999_999_999,
        1_000_000_000,
        u32::MAX,
    ] {
        assert_eq!(
            Decimal::unsigned(value).digits(&mut buffer),
            format!("{value}")
        );
    }
    let mut value = 1_u32;
    for _ in 0..10_000 {
        value = value.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        assert_eq!(
            Decimal::unsigned(value).digits(&mut buffer),
            format!("{value}")
        );
    }
}

#[test]
fn parser_every_command_round_trips_and_accepts_required_spellings() {
    let cases: &[(&[u8], Command)] = &[
        (b"STEP 5", Command::Step(5)),
        (b"step +5", Command::Step(5)),
        (b"STEP -1000000", Command::Step(-1_000_000)),
        (b"STEP 1000000", Command::Step(1_000_000)),
        (b"STEP -0", Command::Step(0)),
        (b"rot -7.5", Command::Rot(-75)),
        (b"ROT 10", Command::Rot(100)),
        (b"ROT +0.1", Command::Rot(1)),
        (b"ROT -0.1", Command::Rot(-1)),
        (b"ROT 3600", Command::Rot(36_000)),
        (b"ROT -3600", Command::Rot(-36_000)),
        (b"ROT 3600.0", Command::Rot(36_000)),
        (b"STOPS +1", Command::Stops(1)),
        (b"STOPS 720", Command::Stops(720)),
        (b"NEXT", Command::Next),
        (b"zero", Command::Zero),
        (b"POS\r", Command::Pos),
        (b"SPEED 1", Command::Speed(1)),
        (b"SPEED 50", Command::Speed(50)),
        (b"sTaTuS", Command::Status),
        (b" \tSTEP\t+0005 \r", Command::Step(5)),
    ];
    for &(line, expected) in cases {
        let parsed = Command::parse(line).unwrap();
        assert_eq!(parsed, expected, "{line:?}");
        assert_eq!(Command::parse(canonical(parsed).as_bytes()), Ok(expected));
    }
}

#[test]
fn parser_rejects_unknown_missing_extra_and_non_numeric_arguments() {
    let cases: &[&[u8]] = &[
        b"",
        b" \t",
        b"HELP",
        b"ARM 10",
        b"STEPS 1",
        b"STEP",
        b"ROT",
        b"STOPS",
        b"SPEED",
        b"STEP 1 2",
        b"ROT 1 2",
        b"STOPS 1 2",
        b"SPEED 1 2",
        b"NEXT 1",
        b"ZERO 1",
        b"POS 1",
        b"STATUS 1",
        b"STEP x",
        b"ROT x",
        b"STOPS x",
        b"SPEED x",
        b"STEP 1.0",
        b"STOPS 2.0",
        b"SPEED 1.0",
        b"STEP +",
        b"STEP --1",
        b"STEP +-1",
        b"ROT -",
        b"ROT .5",
        b"ROT 1.",
        b"ROT 1.00",
        b"ROT 1.2.3",
        b"ROT 1e1",
        b"STEP 0x10",
        b"STEP 1\0",
        b"P\xffS",
    ];
    for line in cases {
        assert_eq!(Command::parse(line), Err(Error::Syntax), "{line:?}");
    }
}

#[test]
fn parser_rejects_each_just_outside_range_and_oversized_numbers() {
    for line in [
        "STEP 1000001",
        "STEP -1000001",
        "ROT 3600.1",
        "ROT -3600.1",
        "STOPS 0",
        "STOPS -1",
        "STOPS 721",
        "SPEED 0",
        "SPEED -1",
        "SPEED 51",
        "STEP 2147483648",
        "STEP -2147483648",
        "STEP -2147483649",
        "STEP 9999999999999999999",
        "ROT 2147483647",
        "ROT -2147483648",
    ] {
        assert_eq!(Command::parse(line.as_bytes()), Err(Error::Range), "{line}");
    }
    // Length includes spaces and digits, not the ignored CR or line delimiter.
    assert_eq!(
        Command::parse(b"STEP                   0"),
        Ok(Command::Step(0))
    );
    assert_eq!(
        Command::parse(b"STEP                    0"),
        Err(Error::Line)
    );
    assert_eq!(Command::parse(&[b'X'; 25]), Err(Error::Line));
}

#[test]
fn degrees_both_ratios_all_frozen_values_signs_and_bounds() {
    for (ratio, expected) in [
        (407_577, [4076, 1019, 113, 40_758]),
        (409_600, [4096, 1024, 114, 40_960]),
    ] {
        let planner = planner(ratio, 36);
        for (tenths, steps) in [3600, 900, 100, 36_000].into_iter().zip(expected) {
            assert_eq!(planner.degrees_to_halfsteps(tenths), Some(steps));
            assert_eq!(planner.degrees_to_halfsteps(-tenths), Some(-steps));
        }
        assert_eq!(planner.degrees_to_halfsteps(0), Some(0));
        for value in [36_001, -36_001, i32::MIN, i32::MAX] {
            assert_eq!(planner.degrees_to_halfsteps(value), None);
        }
        // Exhaustively check sign symmetry over the supported tenths domain.
        for tenths in 0..=36_000 {
            assert_eq!(
                planner.degrees_to_halfsteps(-tenths),
                planner.degrees_to_halfsteps(tenths).map(|v| -v)
            );
        }
    }
}

#[test]
fn degrees_rounding_is_half_away_including_exact_positive_and_negative_ties() {
    for (numerator, denominator, expected) in [
        (0, 100, 0),
        (49, 100, 0),
        (50, 100, 1),
        (51, 100, 1),
        (150, 100, 2),
        (-49, 100, 0),
        (-50, 100, -1),
        (-51, 100, -1),
        (-150, 100, -2),
    ] {
        assert_eq!(round_half_away(numerator, denominator), expected);
    }
    // Neither production ratio produces an exact half-step tie inside the
    // permitted integer-tenths domain. A synthetic 1800-step/rev ratio makes
    // 0.1 degree exactly 0.5 steps and exercises the *public* conversion path.
    let planner = planner(180_000, 1);
    assert_eq!(planner.degrees_to_halfsteps(1), Some(1));
    assert_eq!(planner.degrees_to_halfsteps(-1), Some(-1));
    assert_eq!(planner.degrees_to_halfsteps(3), Some(2));
    assert_eq!(planner.degrees_to_halfsteps(-3), Some(-2));
    // The spec's 0.05-degree example is not representable by the public API.
    // Check its rational value directly using the production rounding helper;
    // for these ratios it is about 0.57 steps, not an exact tie.
    for ratio in [407_577_i64, 409_600] {
        assert_eq!(round_half_away(5 * ratio, 3_600_000), 1);
        assert_eq!(round_half_away(-5 * ratio, 3_600_000), -1);
    }
}

#[test]
fn small_long_division_matches_native_host_arithmetic_at_full_i64_width() {
    // Independent host oracle: i128 can add the half-divisor even at i64's
    // endpoints. Exercise high bits that the ordinary 10-turn cases do not use.
    for denominator in [100_u32, 72_000, 360_000, 3_600_000] {
        let reference = |numerator: i64| {
            let n = i128::from(numerator);
            let d = i128::from(denominator);
            let magnitude = (n.abs() + d / 2) / d;
            if n < 0 {
                -magnitude as i64
            } else {
                magnitude as i64
            }
        };
        for numerator in [i64::MIN, i64::MIN + 1, -1, 0, 1, i64::MAX - 1, i64::MAX] {
            assert_eq!(
                round_half_away(numerator, denominator),
                reference(numerator)
            );
        }
        let mut bits = 1_u64;
        for _ in 0..5000 {
            bits = bits.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            let numerator = bits as i64;
            assert_eq!(
                round_half_away(numerator, denominator),
                reference(numerator)
            );
        }
    }
}

#[test]
fn planner_ten_revolutions_close_cumulatively_for_all_stop_counts_and_ratios() {
    for ratio in [407_577, 409_600] {
        for stops in [1, 2, 3, 5, 8, 24, 36, 72, 360, 720] {
            let mut planner = planner(ratio, stops);
            let mut position = 0_i32;
            let mut first_revolution = 0;
            for k in 1..=10 * u32::from(stops) {
                let action = planner
                    .prepare(
                        Command::Next,
                        Snapshot {
                            state: State::Idle,
                            position,
                            period_ms: 2,
                        },
                    )
                    .unwrap();
                let PreparedCommand::Next { steps, k: next_k } = action else {
                    panic!("NEXT did not prepare a move");
                };
                assert_eq!(next_k, k);
                // Compare rational values by cross-multiplying; no test needs
                // floating point, an epsilon, or a rounded reference delta.
                let denominator = i64::from(stops) * 100;
                assert!((i64::from(steps) * denominator - i64::from(ratio)).abs() < denominator);
                position += steps;
                planner.commit(action);
                if k == u32::from(stops) {
                    first_revolution = position;
                }
                let ideal_cumulative =
                    (i64::from(k) * i64::from(ratio) + denominator / 2) / denominator;
                assert_eq!(i64::from(position), ideal_cumulative);
            }
            assert_eq!(i64::from(first_revolution), (i64::from(ratio) + 50) / 100);
            assert_eq!(i64::from(position), (10 * i64::from(ratio) + 50) / 100);
            assert_eq!(planner.k(), 10 * u32::from(stops));
        }
    }
}

#[test]
fn planner_zero_and_stops_reset_origin_and_counter_only_as_specified() {
    let mut planner = planner(RATIO_X100, 36);
    let mut controller = Controller::new();
    execute(&mut planner, &mut controller, Command::Next).unwrap();
    finish(&mut controller);
    assert_eq!(planner.k(), 1);
    execute(&mut planner, &mut controller, Command::Step(-5)).unwrap();
    finish(&mut controller);
    let position = controller.snapshot().position;
    assert_ne!(position, 0);
    assert_eq!(
        planner.k(),
        1,
        "manual STEP must not reset the stop counter"
    );
    execute(&mut planner, &mut controller, Command::Stops(24)).unwrap();
    assert_eq!(planner.origin(), position);
    assert_eq!(planner.k(), 0);
    assert_eq!(planner.stops(), 24);
    let phase = controller.phase();
    execute(&mut planner, &mut controller, Command::Next).unwrap();
    finish(&mut controller);
    execute(&mut planner, &mut controller, Command::Zero).unwrap();
    assert_eq!(controller.snapshot().position, 0);
    assert_eq!(planner.origin(), 0);
    assert_eq!(planner.k(), 0);
    assert_eq!(planner.stops(), 24);
    assert_ne!(controller.phase(), phase);
    let retained = controller.phase();
    execute(&mut planner, &mut controller, Command::Zero).unwrap();
    assert_eq!(
        controller.phase(),
        retained,
        "ZERO changes coordinates, not phase"
    );
}

#[test]
fn planner_busy_and_range_rejections_are_transactional() {
    let mut planner = planner(RATIO_X100, 36);
    let mut controller = Controller::new();
    let stale_next = planner
        .prepare(Command::Next, controller.snapshot())
        .unwrap();
    execute(&mut planner, &mut controller, Command::Next).unwrap();
    assert_eq!(planner.k(), 1);
    assert_eq!(
        execute(&mut planner, &mut controller, Command::Next),
        Err(Error::Busy)
    );
    assert_eq!(controller.apply(stale_next), Err(Error::Busy));
    assert_eq!(planner.k(), 1);
    finish(&mut controller);
    for (position, command) in [
        (POSITION_LIMIT, Command::Step(1)),
        (-POSITION_LIMIT, Command::Step(-1)),
        (POSITION_LIMIT, Command::Rot(1)),
        (-POSITION_LIMIT, Command::Rot(-1)),
        (POSITION_LIMIT, Command::Next),
    ] {
        let before = (planner.k(), planner.origin(), planner.stops());
        let snapshot = Snapshot {
            state: State::Idle,
            position,
            period_ms: 2,
        };
        assert_eq!(planner.prepare(command, snapshot), Err(Error::Range));
        assert_eq!((planner.k(), planner.origin(), planner.stops()), before);
    }
    assert_eq!(
        Controller::checked_destination(POSITION_LIMIT - 1, 1),
        Ok(POSITION_LIMIT)
    );
    assert_eq!(
        Controller::checked_destination(-POSITION_LIMIT + 1, -1),
        Ok(-POSITION_LIMIT)
    );
    assert_eq!(
        Controller::checked_destination(POSITION_LIMIT, i32::MAX),
        Err(Error::Range)
    );
    assert_eq!(
        Controller::checked_destination(-POSITION_LIMIT, i32::MIN),
        Err(Error::Range)
    );
    assert_eq!(controller.start(i32::MAX), Err(Error::Range));
    assert_eq!(controller.start(i32::MIN), Err(Error::Range));
    assert_eq!(controller.snapshot().state, State::Idle);
}

#[test]
fn planner_next_keeps_the_specified_delta_after_manual_motion() {
    let mut planner = planner(RATIO_X100, 3);
    let snapshot = Snapshot {
        state: State::Idle,
        position: 0,
        period_ms: 2,
    };
    let first = planner.prepare(Command::Next, snapshot).unwrap();
    planner.commit(first);
    let second = planner
        .prepare(
            Command::Next,
            Snapshot {
                position: -1234,
                ..snapshot
            },
        )
        .unwrap();
    assert_eq!(second, PreparedCommand::Next { steps: 1358, k: 2 });
}

#[test]
fn all_busy_states_reject_only_the_five_mutating_commands() {
    let planner = planner(RATIO_X100, 36);
    for state in [
        State::Energise,
        State::Stepping,
        State::Hold,
        State::Release,
        State::Settle,
    ] {
        let snapshot = Snapshot {
            state,
            position: 0,
            period_ms: 2,
        };
        for command in [
            Command::Step(0),
            Command::Rot(0),
            Command::Next,
            Command::Zero,
            Command::Stops(1),
        ] {
            assert_eq!(planner.prepare(command, snapshot), Err(Error::Busy));
        }
        for command in [
            Command::Pos,
            Command::Status,
            Command::Speed(1),
            Command::Speed(50),
        ] {
            assert!(planner.prepare(command, snapshot).is_ok());
        }
    }
}

#[test]
fn sequencer_is_the_frozen_table_with_one_coil_change_in_both_directions() {
    assert_eq!(HALF_STEPS, [8, 12, 4, 6, 2, 3, 1, 9]);
    for direction in [Direction::Forward, Direction::Reverse] {
        let mut sequence = Sequencer::new();
        let mut index = 0_usize;
        for _ in 0..24 {
            let previous = sequence.pattern();
            index = match direction {
                Direction::Forward => (index + 1) % 8,
                Direction::Reverse => (index + 7) % 8,
            };
            assert_eq!(sequence.advance(direction), HALF_STEPS[index]);
            assert_eq!((previous ^ sequence.pattern()).count_ones(), 1);
        }
    }
}

#[test]
fn controller_all_periods_directions_exact_timing_patterns_count_and_done() {
    for period in [1_u8, 2, 50] {
        for direction in [1_i32, -1] {
            let mut controller = Controller::new();
            controller.apply(PreparedCommand::Speed(period)).unwrap();
            let count = 17;
            assert_eq!(controller.start(direction * count).unwrap().done, None);
            assert_eq!(controller.snapshot().state, State::Energise);
            assert_eq!(
                controller.pattern(),
                0,
                "start waits for the timer boundary"
            );
            let first_tick = controller.tick();
            assert_eq!(first_tick.pattern, HALF_STEPS[0]);
            assert_eq!(first_tick.done, None);

            let last_step_at = i32::from(ENERGISE_MS) + count * i32::from(period);
            let release_at = last_step_at + i32::from(HOLD_MS);
            let done_at = release_at + i32::from(SETTLE_MS);
            let mut emitted = 0;
            let mut phase = 0_usize;
            let mut previous_pattern = first_tick.pattern;
            let mut previous_position = 0;
            let mut done_count = 0;
            for elapsed in 1..=done_at + 20 {
                let output = controller.tick();
                let snapshot = controller.snapshot();
                if snapshot.position != previous_position {
                    emitted += 1;
                    assert_eq!(
                        elapsed,
                        i32::from(ENERGISE_MS) + emitted * i32::from(period)
                    );
                    phase = if direction > 0 {
                        (phase + 1) % 8
                    } else {
                        (phase + 7) % 8
                    };
                    assert_eq!(output.pattern, HALF_STEPS[phase]);
                    assert_eq!((previous_pattern ^ output.pattern).count_ones(), 1);
                    assert_eq!(snapshot.position - previous_position, direction);
                }
                if elapsed < i32::from(ENERGISE_MS) {
                    assert_eq!(snapshot.state, State::Energise);
                } else if elapsed < last_step_at {
                    assert_eq!(snapshot.state, State::Stepping);
                } else if elapsed < release_at {
                    assert_eq!(snapshot.state, State::Hold);
                } else if elapsed == release_at {
                    assert_eq!(snapshot.state, State::Release);
                } else if elapsed < done_at {
                    assert_eq!(snapshot.state, State::Settle);
                    assert_eq!(controller.start(1), Err(Error::Busy));
                } else {
                    assert_eq!(snapshot.state, State::Idle);
                }
                if elapsed >= release_at {
                    assert_eq!(output.pattern, 0);
                }
                if let Some(position) = output.done {
                    done_count += 1;
                    assert_eq!(elapsed, done_at);
                    assert_eq!(position, direction * count);
                }
                previous_pattern = output.pattern;
                previous_position = snapshot.position;
            }
            assert_eq!(emitted, count);
            assert_eq!(done_count, 1);
            assert_eq!(controller.snapshot().position, direction * count);
            assert_eq!(controller.phase() as usize, phase);
            // A new accepted move energises the retained phase; it does not
            // emit a step or silently reset the sequencer after release.
            controller.start(-direction).unwrap();
            assert_eq!(controller.tick().pattern, HALF_STEPS[phase]);
            assert_eq!(finish(&mut controller), direction * (count - 1));
        }
    }
}

#[test]
fn speed_during_motion_is_accepted_and_affects_only_the_next_move() {
    let mut controller = Controller::new();
    controller.apply(PreparedCommand::Speed(50)).unwrap();
    controller.start(2).unwrap();
    controller.tick(); // start of physical energisation
    for elapsed in 1..=120 {
        if elapsed == 21 {
            assert!(controller.apply(PreparedCommand::Speed(1)).is_ok());
        }
        controller.tick();
        let expected_position = if elapsed < 70 {
            0
        } else if elapsed < 120 {
            1
        } else {
            2
        };
        assert_eq!(controller.snapshot().position, expected_position);
    }
    assert_eq!(finish(&mut controller), 2);
    assert_eq!(controller.snapshot().period_ms, 1);
    controller.start(1).unwrap();
    controller.tick();
    for _ in 0..20 {
        controller.tick();
        assert_eq!(controller.snapshot().position, 2);
    }
    controller.tick();
    assert_eq!(controller.snapshot().position, 3);
    assert_eq!(finish(&mut controller), 3);
}

#[test]
fn zero_length_and_every_accepted_move_yield_one_position_bearing_event() {
    let mut planner = planner(RATIO_X100, 36);
    let mut controller = Controller::new();
    let mut pending_done = None;
    for steps in [0, 0, 1, 0, -2, 0, 7] {
        let output = execute(&mut planner, &mut controller, Command::Step(steps)).unwrap();
        if steps == 0 {
            assert_eq!(output.done, Some(controller.snapshot().position));
            pending_done = output.done;
            assert_eq!(controller.snapshot().state, State::Idle);
            assert_eq!(output.pattern, 0);
            for _ in 0..200 {
                assert_eq!(controller.tick().done, None);
            }
        } else {
            assert_eq!(output.done, None);
            assert_eq!(
                execute(&mut planner, &mut controller, Command::Step(0)),
                Err(Error::Busy)
            );
            pending_done = Some(finish(&mut controller));
        }
    }
    assert_eq!(controller.snapshot().position, 6);
    // Firmware detaches this old event atomically before applying ZERO; the
    // event carries 6 even after the position and planner origin become zero.
    assert_eq!(pending_done, Some(6));
    execute(&mut planner, &mut controller, Command::Zero).unwrap();
    assert_eq!(pending_done, Some(6));
    assert_eq!(controller.snapshot().position, 0);
}

fn feed(
    assembler: &mut LineAssembler,
    byte: u8,
    corrupted: bool,
) -> Option<Result<Command, Error>> {
    assembler
        .push(byte, corrupted)
        .map(|line| line.and_then(Command::parse))
}

fn drain(rx: &mut RxBuffer, assembler: &mut LineAssembler) -> Vec<Result<Command, Error>> {
    let mut events = Vec::new();
    while let Some(item) = rx.pop() {
        if let Some(event) = feed(assembler, item.byte, item.corrupted) {
            events.push(event);
        }
    }
    events
}

#[test]
fn assembler_drops_corrupted_line_then_accepts_next_clean_line_and_ignores_cr() {
    let mut assembler = LineAssembler::new();
    let mut events = Vec::new();
    for (index, &byte) in b"STEP 10\nrot -7.5\r\nP\rO\rS\n".iter().enumerate() {
        if let Some(event) = feed(&mut assembler, byte, index == 5) {
            events.push(event);
        }
    }
    assert_eq!(
        events,
        vec![Err(Error::Line), Ok(Command::Rot(-75)), Ok(Command::Pos)]
    );
}

#[test]
fn assembler_length_limit_and_one_error_per_long_burst() {
    let mut assembler = LineAssembler::new();
    let mut events = Vec::new();
    for byte in [b'X'; 40] {
        if let Some(event) = feed(&mut assembler, byte, false) {
            events.push(event);
        }
    }
    assert_eq!(
        events,
        vec![Err(Error::Line)],
        "report even without a newline"
    );
    for &byte in b"\nPOS\n" {
        if let Some(event) = feed(&mut assembler, byte, false) {
            events.push(event);
        }
    }
    assert_eq!(events, vec![Err(Error::Line), Ok(Command::Pos)]);
    for &byte in b"STEP                   0\n" {
        if let Some(event) = feed(&mut assembler, byte, false) {
            assert_eq!(event, Ok(Command::Step(0)));
        }
    }
}

#[test]
fn ring_overflow_without_newline_reports_once_but_keeps_sticky_discard() {
    let mut rx = RxBuffer::new();
    let mut assembler = LineAssembler::new();
    for _ in 0..40 {
        rx.receive(b'X', false);
    }
    assert_eq!(drain(&mut rx, &mut assembler), vec![Err(Error::Line)]);
    assert!(rx.has_error());
    assert!(drain(&mut rx, &mut assembler).is_empty());
    for &byte in b"tail\nPOS\n" {
        rx.receive(byte, false);
    }
    assert_eq!(drain(&mut rx, &mut assembler), vec![Ok(Command::Pos)]);
    assert!(!rx.has_error());
}

#[test]
fn production_ring_preserves_all_32_bytes_and_wraps_in_fifo_order() {
    let mut rx = RxBuffer::new();
    for byte in 0..32 {
        rx.receive(byte, false);
    }
    assert!(!rx.has_error());
    for byte in 0..16 {
        assert_eq!(rx.pop().unwrap().byte, byte);
    }
    for byte in 32..48 {
        rx.receive(byte, false);
    }
    for byte in 16..48 {
        let item = rx.pop().unwrap();
        assert_eq!(item.byte, byte);
        assert!(!item.corrupted);
    }
    assert_eq!(rx.pop(), None);
}

#[test]
fn production_ring_overflow_sticky_error_and_lost_newline_recover_once() {
    for newline_is_overflow_byte in [false, true] {
        let mut rx = RxBuffer::new();
        let mut assembler = LineAssembler::new();
        // Prefix already consumed by main must also be discarded on overflow.
        for &byte in b"STEP " {
            assert_eq!(feed(&mut assembler, byte, false), None);
        }
        let burst = if newline_is_overflow_byte { 32 } else { 40 };
        for _ in 0..burst {
            rx.receive(b'1', false);
        }
        if !newline_is_overflow_byte {
            assert!(rx.has_error());
        }
        rx.receive(b'\n', false);
        assert!(
            rx.has_error(),
            "sticky until main consumes the recovery marker"
        );
        for &byte in b"POS\n" {
            rx.receive(byte, false);
        }
        assert_eq!(
            drain(&mut rx, &mut assembler),
            vec![Err(Error::Line), Ok(Command::Pos)]
        );
        assert!(!rx.has_error());
    }
}

#[test]
fn production_ring_error_cannot_clear_at_an_older_queued_newline() {
    let mut rx = RxBuffer::new();
    let mut assembler = LineAssembler::new();
    for &byte in b"POS\nSTEP 1" {
        rx.receive(byte, false);
    }
    rx.receive(b'0', true); // truncated STEP 10 must never become valid STEP 1
    rx.receive(b'\n', false);
    for &byte in b"STEP 3\n" {
        rx.receive(byte, false);
    }
    assert_eq!(
        drain(&mut rx, &mut assembler),
        vec![Err(Error::Line), Ok(Command::Step(3))]
    );
}

#[test]
fn hardware_error_newline_is_not_a_trustworthy_recovery_delimiter() {
    let mut rx = RxBuffer::new();
    let mut assembler = LineAssembler::new();
    for &byte in b"STEP 1" {
        rx.receive(byte, false);
    }
    rx.receive(b'\n', true);
    let mut events = drain(&mut rx, &mut assembler);
    assert!(rx.has_error());
    for &byte in b"0\nPOS\n" {
        rx.receive(byte, false);
    }
    events.extend(drain(&mut rx, &mut assembler));
    assert_eq!(events, vec![Err(Error::Line), Ok(Command::Pos)]);
    assert!(!rx.has_error());
}

#[test]
fn an_error_after_dequeuing_a_complete_line_cannot_modify_that_command() {
    let mut rx = RxBuffer::new();
    let mut assembler = LineAssembler::new();
    for &byte in b"STEP 10\n" {
        rx.receive(byte, false);
    }
    let completed = drain(&mut rx, &mut assembler);
    // Simulate an RX ISR between dequeue/parse and dispatch of the complete
    // command. This error belongs to a later line; the parsed value is owned.
    rx.receive(b'9', true);
    rx.receive(b'\n', false);
    assert_eq!(completed, vec![Ok(Command::Step(10))]);
    assert_eq!(drain(&mut rx, &mut assembler), vec![Err(Error::Line)]);
}

#[test]
fn interleaved_done_output_does_not_change_input_parsing() {
    let mut rx = RxBuffer::new();
    let mut assembler = LineAssembler::new();
    let mut controller = Controller::new();
    controller.start(1).unwrap();
    let mut commands = Vec::new();
    let mut tx = String::new();
    // RX continues while main is in the blocking writer. TX is a separate UART
    // direction and is never fed into LineAssembler (there is no local echo).
    for &byte in b"ROT -" {
        rx.receive(byte, false);
    }
    commands.extend(drain(&mut rx, &mut assembler));
    let done = finish(&mut controller);
    tx.push_str(&format!("DONE {done}\r\n"));
    for &byte in b"7.5\nPOS\n" {
        rx.receive(byte, false);
    }
    commands.extend(drain(&mut rx, &mut assembler));
    assert_eq!(tx, "DONE 1\r\n");
    assert_eq!(commands, vec![Ok(Command::Rot(-75)), Ok(Command::Pos)]);
}
