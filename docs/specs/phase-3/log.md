# Log: phase-3

Verification runs and review rounds, without model names; the models that served each run are logged under `~/.cache/routing/`.

- 2026-10-26: spec drafted.
- 2026-10-26: attack: verdict needs-attention, 3 finding(s).
- 2026-10-26: attack round 1 findings, all confirmed by the hub. 1 (high, phone timer drift into moves): resolved by
  the user's redesign, a steady platter turn with no phone synchronisation (v2). 2 (medium, torn EEPROM writes):
  accepted, two slots with the generation byte written last and CRC-8. 3 (medium, 300-photo limit): accepted,
  `SCAN` refuses an estimate over 300. The clock correction was dropped with the synchronisation.
- 2026-10-26: attack: verdict needs-attention, 2 finding(s).
- 2026-10-26: attack round 2 findings, both confirmed by the hub and accepted by the user (v3).
  1 (high, consecutive interrupted saves can load a mixed record): reproduced, a ratio of 378879 that nobody
  saved loaded with a valid CRC. Fixed by clearing a valid destination's magic byte first and writing it last.
  A simulation of every cut position and torn value (`0x00`, `0xFF`, intended, intended XOR `0x5A`, `0x54`) over up
  to three consecutive saves found 0 bad states in 117538, from adjacent generations including the wrap and from
  a blank EEPROM. The draft of the fix that cleared the magic unconditionally failed 8 cases: a complete but
  uncommitted record revived by a torn clear that left `0x54`.
  2 (medium, photo estimate ignores the slow clock and an early phone start): the boundary scan accepted at 300
  really took 303. Fixed with a 1 % clock margin and a 10 s start allowance.
- 2026-10-26: two attack rounds used, the limit. The user decided to freeze v3 without a third attack, and that
  the hub writes this suite on route `coding.default` (one-off; the `coding.tests` writer was out of credits until
  2026-10-28T00:26Z).
- 2026-10-26: spec v3 frozen. Acceptance suite `turntable-core/tests/phase3.rs` written from the criteria (P, S, C,
  D, H, N), and `ARM 10` dropped from the Phase 2 rejected list (P6).
  - Checks: type-checked and clippy/rustfmt-clean against throwaway `todo!()` stubs of the spec's interface
    (scratch only, not committed). `cargo test --lib` passes, 37 Phase 2 tests.
  - Baseline `cargo test --manifest-path turntable-core/Cargo.toml` exits 101, as it should: the interface does not
    exist yet. `spec.sh ready` protects `tests/phase3.rs` and `src/tests.rs`.
  - Brief note for the implementer: implementing `Iterator for Scan` satisfies the spec's `Scan::next` and clippy's
    `should_implement_trait`; the suite calls `next` in a form that works either way.
- 2026-10-26: implementation run 1 (route `coding.feature`, 47 min) reported a conflict: the protected Phase 2
  helper `canonical()` in `src/tests.rs` matches every `Command` variant, so the library tests cannot compile
  against the spec's new variants (E0004). Confirmed by the hub. With a catch-all arm, on a scratch copy: Phase 2
  37 passed, Phase 3 32 passed, clippy and fmt clean. The user chose v4: the hub adds the arm, P6 names it, no
  re-attack. The implementation was set aside so the baseline runs without it.
- 2026-10-27: spec v4 suite-ready. Baseline without the implementation exits 101 (the interface does not exist);
  `tests/phase3.rs` is unchanged, `src/tests.rs` gains the catch-all arm.
- 2026-10-27: implementation run 1 verified by the hub on the v4 base (`--offline` builds).
  - F1: `turntable-core` `cargo test`: Phase 2 suite 37 passed, Phase 3 suite 32 passed; `clippy --all-targets
    -D warnings` and `fmt --check` clean. `firmware`: Nano and UNO release builds, `clippy -D warnings` and
    `fmt --check` clean. `spec.sh check` passes (protected tests untouched).
  - F2 (Nano release ELF): `.text` 15076, `.data` 434, `.bss` 149, so `.text + .data` = 15510 ≤ 16384 and
    `.data + .bss` = 583 ≤ 640.
  - F3, recomputed by the hub with a longest-path counter over `avr-objdump -d` (ATmega328P cycle table, the slower
    figure for every conditional, callees charged their own bound, panic calls end a path; script and listing in
    `~/.cache/routing/turning-table/f3/`):
    - TC2 ISR: 773 cycles for the body (both `Controller::tick` calls at 181, `Controller::halt` at 17, `reti`),
      plus 4 response and 3 for the vector `jmp`: 780 ≤ 800.
    - `platter_command`'s accept section: static bound 336, through a path that starts a move in `apply` and then
      takes the `Stops` commit, which cannot happen (`apply` never starts a move for `Stops`). Per variant:
      `Next` 312, `Move` 296, `Stops` 169, `Zero` 154; worst 312 ≤ 320.
    - `SPEED`'s section (two `apply(Speed)` calls): static 520 through `apply`'s move start, which `Speed` never
      reaches; with the move start excluded, at most 186.
    - Every other section in main: at most 193 (homing step), 188 (scan start). The whole USART ISR is 135 + 7 =
      142, its inner section 67.
    - EEPROM: the strobe is `in SREG; cli; sbi EEMPE; sbi EEPE; out SREG` (6 cycles masked); the `EEPE` waits run
      with interrupts enabled, unchanged bytes are skipped, every written byte is read back through `EERE`.
- 2026-10-27: review round 1 on `96a85bb..17d44e9`. Task reviewer (diff and spec): needs-attention, 1 medium,
  2 low, 1 spec ambiguity. Artifact reviewer (diff and test output only): needs-attention, 2 high, 2 medium. All
  confirmed by the hub against the code; two were found by both reviewers. Root of the first four: the TC2 ISR sets
  a motor `Idle` and fills its mailbox on the same tick, but main reads states and mailboxes at different moments.
  1. (high) `HOME` admitted with an earlier `ADONE` still queued: the stale event fails the new search, disarms the
     switch halt, prints `ERR HOME`, and the 80° search keeps driving the arm down unprotected.
  2. (high) A button press registered while a scan or move is active is judged after `service()` has processed the
     completion, so it starts a new scan instead of stopping.
  3. (medium) `service()` takes the mailboxes before it snapshots the motors: `END` can precede the final `DONE`
     and undercount; equal consecutive ring heights also undercount.
  4. (medium) `POS` during a scan can consume a turn's `DONE` without the turn accounting.
  5. (low) `STOP` during the last turn's hold or settle does not count a turn that finished all its half-steps.
  - Rejected: the reviewers' suggestion of deterministic interleaving tests for the firmware glue; the firmware
    crate has no test harness (`test = false`). W6-W8 exercise these paths on the board but cannot reproduce
    microsecond-wide races, so for those races the two reviews are the only evidence.
  - Spec ambiguity (low): `SCAN <r> <p> <i>` runs every check before it saves, so a refused scan is never saved;
    the spec's "saves them ..., then runs" can be read as save first. Put to the user.
  - Fix round 1 dispatched to the implementation route for findings 1-5.
- 2026-10-27: decision (user): `SCAN <r> <p> <i>` keeps the implemented order: every check runs first and only an
  accepted scan is saved, so the button never replays a scan it cannot run. An `ERR SAVE` leaves the new scan in RAM
  and does not run it. Recorded here and in the Phase 3 docs (task 9); no spec version bump.
- 2026-10-27: fix round 1 (`2346755`, `firmware/src/main.rs` only) verified by the hub.
  - Fixes: `service()` takes both snapshots and both mailboxes in one critical section and accounts for `DONE`
    before it advances the scan or prints `END`; `Run::done` prints every completion and does the turn accounting
    (used by `service`, `platter_command` and `HOME`); `HOME` detaches a queued `ADONE` after admission and prints
    it before `OK`; `home_step(Failed)` halts the arm before it disarms the switch; presses are judged before
    `service()`, a queued completion counts as busy, and a press after a busy poll acts as `STOP`; `STOP` counts a
    turn in `Hold`/`Release`/`Settle`. Hub addition: `STATUS` clamps the ring to the scan's ring count (the implementer
    reported that a `STOP` in the last turn's settle could show ring N+1 until `END`).
  - F1: Phase 2 37 passed, Phase 3 32 passed, clippy and fmt clean on both crates, Nano and UNO builds.
  - F2: `.text` 15222, `.data` 436, `.bss` 149: 15658 ≤ 16384 and 585 ≤ 640.
  - F3: the TC2 ISR is the same 280 instructions with the same opcodes (bound 780). `platter_command`, `apply`,
    `start_with_period`, `halt`, `tick` and the `SPEED` closure differ only in addresses, so their per-variant
    bounds hold (accept 312, `SPEED` 186). New or changed sections: `STOP` 70, `service` 40, failed homing 27,
    button 20 and 6, `HOME`'s `ADONE` 14.
  - Review round 2 started on `17d44e9..2346755`.
- 2026-10-27: review round 2 on `17d44e9..2346755`. Artifact reviewer: approve, no findings. Task reviewer: pass,
  every fix closes its scenario, no regression in the Phase 2 reply order; 2 low findings, both confirmed by the hub:
  1. (low, predates the fix) `start_scan` is the one command that starts motion without detaching queued
     completions: a console `ARM`/`STEP` completing just before a `SCAN` sent without waiting prints its
     `ADONE`/`DONE` after the scan's `OK`, where a host counting per-move events would take it for the scan's first.
     The button path cannot hit it (a queued completion counts as busy).
  2. (low, from fix 5) after a `STOP` in a non-final ring's settle, `STATUS` reports ring r+1 for up to 150 ms
     until `END`; the clamp covers only the last ring.
  Pass: no correctness finding of medium or above.
- 2026-10-27: the user chose to fix both round-2 lows now, as hub edits after the last review round (hub checks
  only, no review). `SCAN` detaches queued `DONE`/`ADONE` after admission and prints them before `OK`, as `HOME`
  does. `STATUS` prints a `ring` that is set when each scan move starts (`completed + 1`), so a `STOP` that counts
  a settled turn leaves it on that ring; this replaces the clamp. Commit `0b195f8`.
  - F1: Phase 2 37 passed, Phase 3 32 passed, clippy and fmt clean on both crates, Nano and UNO builds.
  - F2: `.text` 15324, `.data` 436, `.bss` 150: 15760 ≤ 16384 and 586 ≤ 640.
  - F3: the TC2 ISR, `platter_command`, `apply`, `start_with_period` and the `SPEED` closure are the same opcodes as
    before (bounds 780, 312, 186 hold); the new `SCAN` section is 26 cycles; nothing else changed.
- 2026-10-27: bench, W1 attempts. Flashed the release build of `6378e37` (avrdude, 15760 bytes verified). Every
  reset printed an `OK` right after `CFG` with no command sent, then `STATUS` showed the arm searching down and the
  `STEP`s got `ERR BUSY`: a start/stop press registered at boot and started `HOME`. The home switch on A1 worked
  (`HOME` then a press on it gave `ADONE 455`). With the A0 wire removed, and then A0 and A1 both disconnected, the
  press still registered at boot. The binary sets the pull-up on PC0 (`cbi DDRC,0; sbi PORTC,0`) and nothing else
  writes PORTC, DDRC or DIDR0, so PC0 reads low on this board with nothing wired to it.
- 2026-10-27: spec v5 at the user's request: the start/stop button moves to A2 (PC2). Firmware: the ISR samples
  bit 2 and main sets the pull-up on A2; hub edit, no review. `ready.json` re-recorded for v5 with the same tests;
  its baseline ran on the implemented tree, so it exits 0. F1: core suites unchanged (no core change), firmware
  Nano and UNO builds, clippy and fmt clean. F2: unchanged (`.text` 15324, `.data` 436, `.bss` 150). F3: the TC2 ISR
  differs only in `andi r18, 0x01` → `0x04` (bound 780); main differs only in the pull-up bits.
- 2026-10-27: flashed `4cbf4e4` (start/stop on A2). Reset with nothing on A2: no `OK` at boot.
- 2026-10-27: W1 pass. Boot: `DEFAULTS`, `CFG 409600 3276800 50 3 36 3000 10 35 60`. `STATUS Idle 2 409600 36 0
  Idle 0 0 -`; `STEP 512` → `DONE 512` (1.204 s from `OK`), `STEP -512` → `DONE 0`, `NEXT` → `DONE 114`; then
  `STATUS ... 36 1 Idle 0 0 -`.
- 2026-10-27: bench note: after the move to A2 the start/stop button did not respond at first; its GND wire sat in
  a dead hole of the second breadboard's − rail. Moved to the next hole, it works. (This cannot explain the A0
  fault, which read closed with nothing connected; A0 stays unused.)
- 2026-10-27: W2 pass, one round of three runs instead of six. Arm motor on the platter's driver, `STEP 40760`
  per run, end minus start: −19° (7 → 348), −17° (0 → 343), −17° (0 → 343). A 63.68:1 gearbox would read about
  +0.2°, a 64:1 one −17.6°: the arm motor is 64:1, so the default `ARATIO 3276800` (409600 × 8) is right and
  nothing was set. The first run's extra −2° is most likely backlash taken up during the run.
- 2026-10-27: W3 first attempt failed on wiring. Pointer after `HOME` 0°, after `ARM 50` 197°, after `ARM 5` 198°,
  after `ARM 50` again 27° (expected 0° each time): the arm motor did not follow its steps on the arm driver, while
  the same motor ran exactly on the platter driver in W2. The firmware mapping matches the pin table (IN1..IN4 on
  PD6, PD7, PB0, PB2). The arm driver's IN4 was not on D10; the user moved it to D10.
- 2026-10-27: W3 pass (IN4 on D10). `HOME` with the home switch pressed by hand during the search → `ADONE 455`;
  then `ARM 50`, `ARM 5`, `ARM 50`: the pointer read 5° after homing and after each of the three moves (one motor
  revolution per move at `ARATIO 3276800`), within the ±0.5° reading; no backlash visible on the reversals.
