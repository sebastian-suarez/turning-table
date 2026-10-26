# Log: phase-3

Verification runs and review rounds, without model names; the models that served each run are logged under `~/.cache/routing/`.

- 2026-10-26: spec drafted.
- 2026-10-26: attack: verdict needs-attention, 3 finding(s).
- 2026-10-26: attack round 1 findings, all confirmed by the hub. 1 (high, phone timer drift into moves): resolved by
  the user's redesign, a steady platter turn with no phone synchronisation (v2). 2 (medium, torn EEPROM writes):
  accepted, two slots with the generation byte written last and CRC-8. 3 (medium, 300-photo limit): accepted,
  `SCAN` refuses an estimate over 300. The clock correction was dropped with the synchronisation.
