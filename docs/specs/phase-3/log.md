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
