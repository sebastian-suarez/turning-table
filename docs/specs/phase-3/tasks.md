# Tasks: phase-3

Ordered. Each task names its writer route and the check that proves it.

| # | Task | Route | Verification |
| --- | --- | --- | --- |
| 0 | Acceptance suite `turntable-core/tests/phase3.rs` for P, S, C, D, H, N; drop `ARM 10` from the Phase 2 rejected list | coding.tests | `cargo test` compiles against the spec's interface and fails only where the code is missing; the Phase 2 suite passes |
| 1 | Parser: new `Command` variants and `Error::{Home, Save}` | coding.feature | P1-P6 pass |
| 2 | `Settings`, record codec, defaults | coding.feature | S1-S5 pass |
| 3 | `Controller::halt` | coding.feature | C1-C3 pass; the Phase 2 controller tests pass |
| 4 | `Debouncer`, `Activity`, `admit`, `button_action` | coding.feature | D1-D4 pass |
| 5 | `Homing` | coding.feature | H1-H5 pass |
| 6 | `Scan` | coding.feature | N1-N6 pass |
| 7 | Firmware: arm coils, TC2 tick counter and inputs, homing halt, scan runner, button, EEPROM load/save, new replies | coding.feature | F1-F4; the full host suite passes |
| 8 | Hardware acceptance with the user | hub | W1-W10, results in `log.md` and on card B3 |
| 9 | Docs: `docs/PLAN.md` Phase 3, card B3, `CLAUDE.md` pins and protocol | hub | the docs match the merged code |
