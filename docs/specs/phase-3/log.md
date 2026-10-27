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
