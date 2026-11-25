# Roll-a-Ball ("Marble Rush") — sample status

This file records the **demonstrated** state of the sample, as the product
specification for it (`docs/product-specification/sample-games/01-roll-a-ball.md`)
requires: every behavior the document asks for that is not demonstrated is
reported with its real state, never hidden.

## What the sample contains

- One gameplay scene, `Assets/Scenes/RollABall.unity`: one arena (a 12 x 12 unit
  floor with four walls), one marble (`PlayerBall`, radius 0.5), four pickups
  with a pickup sensor, a directional light and one camera (`Main Camera`).
- One physics-probe scene, `Assets/Scenes/PhysicsLab.unity`, used by the
  physics scenarios; it is not an arena and carries no game scripts.
- Five scripts under `Assets/Scripts/`: `PlayerController`, `ScoreManager`,
  `RecordsStore`, `Rotator`, `PhysicsLabProbe`.
- Project settings for the Built-in render pipeline, Unity 2022.3.62f1.

## Acceptance scenarios of the specification, and their real state

| Scenario | State | Evidence |
|---|---|---|
| S2 collect every pickup | demonstrated | `Scenarios/collect_all.json` in the samples lane |
| S5 combo multiplier, gold pickup window, dash cooldown | demonstrated | `Scenarios/combo_multiplier.json`, `Scenarios/gold_window.json`, `Scenarios/gold_collect.json`, `Scenarios/dash_cooldown.json` |
| S6 settings and persistence | partly demonstrated | `Scenarios/persist_write.json`, `Scenarios/persist_read.json` cover persistence of records; no settings panel exists yet |
| S7 input modes | not demonstrated | no gamepad or pointer scenario exists yet |
| S8 visual checkpoints | partly demonstrated — see below | `Scenarios/s8_*.scenario.json` |
| S9 edit and restore, S10 package, S11 collision edge, S12 combo timing sweep, S14 medals/records/ghost, S16 | not demonstrated | no scenario covers them yet |

## S8 visual checkpoints — what is demonstrated and what is not

**Demonstrated.** The playtest framework can compare a rendered capture
against an approved reference image at a declared tolerance: the
`render_capture` checkpoint takes a `reference` (a reference BMP) and a
`tolerance` (the maximum allowed fraction of differing pixels), and the
comparison reports the differing-pixel count and the allowed tolerance. A
missing reference file, an unreadable reference, a reference whose size does
not match the capture, or a difference above the tolerance fails the
checkpoint; it is never reported as an unevaluable checkpoint. Four roll-a-ball
scenarios (`Scenarios/s8_title_checkpoint.scenario.json`,
`s8_arena_countdown.scenario.json`, `s8_arena_x5_checkpoint.scenario.json`,
`s8_results_gold_checkpoint.scenario.json`) drive the sample to the countdown,
the x5 combo and the gold pickup and capture the arena at each of those
moments, and they run in the `make verify-samples` lane.

**Not demonstrated — the exact gap.**

- The specification asks for a capture of **every** visual checkpoint on
  **both** pipelines at **every** quality level. The sample has one pipeline
  (Built-in) and no quality levels at all (the project has no
  `QualitySettings.asset`), so no pipeline or quality sweep is possible yet.
- The specification's checkpoints are a title screen with an attract ghost,
  every arena at its countdown, Arena 1 at x5 with its speed trail, and a
  results panel with the gold medal. This sample has **no** title screen, **no**
  attract ghost, **no** speed trail, **no** results panel and **no** gold-medal
  display: the four captures are Arena 1 at four moments of play, and the
  title-screen capture is the arena at startup, not a title screen.
- The sample has **no on-screen text at all**: no canvas, no TextMeshPro, no
  HUD. The score, the combo multiplier and the countdown exist only as C#
  state and as log lines (`ScoreManager.cs:198` logs the combo multiplier,
  `:227` the gold pickup, `:292` the countdown), so a capture cannot show the
  score, the multiplier or the results text.
- The sample has one arena; the specification names three (Courtyard,
  Clockwork, Sky Garden), so "each arena at its countdown" is not covered.

The specification text is not weakened by this note. The gap above is what
remains before S8 can be claimed complete.

## Non-default lane

`make verify-samples` runs this sample's scenarios and is the lane that owns
this file. Its metadata (owner, trigger, lane, evidence, review condition) is
declared in `tools/verify_samples.py` and in the `Makefile`.
