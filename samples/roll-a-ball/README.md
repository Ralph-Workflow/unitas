# Roll-a-Ball ("Marble Rush") — sample status

This file records the **demonstrated** state of the sample, as the product
specification for it (`docs/product-specification/sample-games/01-roll-a-ball.md`)
requires: every behavior the document asks for that is not demonstrated is
reported with its real state, never hidden.

## What the sample contains

- One gameplay scene, `Assets/Scenes/RollABall.unity`: one arena (a 12 x 12 unit
  floor with four walls), one marble (`PlayerBall`, radius 0.5), a pickup
  sensor, a directional light and one camera (`Main Camera`). The pickups are
  partly authored as prefab instances (`Assets/Prefabs/Pickup.prefab`,
  `PickupGold.prefab`) under the `Pickups` root, next to four authored pickup
  objects, and the scene also carries an authored prop (`ArenaProp`), a
  collect burst, a landing dust effect and the collect chime.
- One physics-probe scene, `Assets/Scenes/PhysicsLab.unity`, used by the
  physics scenarios; it is not an arena and carries no game scripts.
- Seven scripts under `Assets/Scripts/`: `PlayerController`, `ScoreManager`,
  `RecordsStore`, `Rotator`, `PhysicsLabProbe`, `Pickup`, `GoldPickupMarker`.
  Every script file has its `.meta` file, `RecordsStore.cs.meta` included
  (GUID `6f708192a3b4c5d6e7f8091a2b3c4d5e`), so the GUID hygiene gap is
  closed.
- The gameplay scripts use the Unity-canonical message style: the magic methods
  are `private` and non-virtual (`private new void Awake()`, `private new void
  Update()`, `private new void OnTriggerEnter(Collider other)`) and the engine
  reaches them through the generated glue handler. The `new` keyword is
  required because the engine's `MonoBehaviour` declares the same methods as
  `public virtual`; without it the compiler reports CS0114.
- Project settings for the Built-in render pipeline, Unity 2022.3.62f1.

## Acceptance scenarios of the specification, and their real state

| Scenario | State | Evidence |
|---|---|---|
| S2 collect every pickup | demonstrated | `Scenarios/collect_all.json` passes in the lane's scripting step and in the headless editor-host step (see the lane state below) |
| S5 combo multiplier, gold pickup window, dash cooldown | demonstrated | `Scenarios/combo_multiplier.json`, `Scenarios/gold_window.json`, `Scenarios/gold_collect.json`, `Scenarios/dash_cooldown.json` — all four pass |
| S6 settings and persistence | partly demonstrated | `Scenarios/persist_write.json`, `Scenarios/persist_read.json` cover persistence of records; no settings panel exists yet |
| S7 input modes | partly demonstrated | `Scenarios/frame_independence.json` injects keyboard input (`W`, `A`) and sweeps 30/60/144/240 fps, and `Scenarios/gamepad_drive.scenario.json` injects the gamepad axes. `Scenarios/pointer_ui.scenario.json` now drives the pointer (`inject_pointer` to the Play button at the 800x600 reference-resolution centre, then `inject_mouse_button`), but its click assertion does **not** pass yet: the native runtime detects the button hit and queues the clicked object id, and the managed `UnityEngine.UI.Button` wrapper exists, but the bridge does not yet install the managed uGUI provider that pumps those clicks into `Button.onClick`. The scenario is therefore not in the `make verify-samples` scenario list yet, and the lane stays green |
| S8 visual checkpoints | partly demonstrated — see below | `Scenarios/s8_*.scenario.json`, `References/*.bmp` — all four pass, their `render_capture` checkpoints included |
| S10 package | partly demonstrated | the lane packages a macOS player with a build report and replays input in it (`packaged-player-proof` passes); only one desktop target is built, not each |
| S9 edit and restore | partly demonstrated — see below | `Scenarios/scene_edit_restore.json` edits the scene through the editor's own commands, proves the edit landed, undoes it and proves the scene file is byte identical to the original; the person-facing editor half and play-mode edit-state restoration are not demonstrated |
| S11 collision edge, S12 combo timing sweep, S14 medals/records/ghost, S16 | not demonstrated | no scenario covers them yet |

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
the x5 combo and the gold pickup, capture the arena at each of those moments,
and run in the `make verify-samples` lane. Their log checkpoints pass, and
their `render_capture` checkpoints now compare against approved reference
images under `References/` (see the gap below for what the references can and
cannot show).

**Not demonstrated — the exact gap.**

- The specification asks for a capture of **every** visual checkpoint on
  **both** pipelines at **every** quality level. The sample has one pipeline
  (Built-in) and never sweeps the three levels its `QualitySettings.asset`
  declares (Low, Medium, High), so no pipeline or quality sweep is possible
  yet.
- The specification's checkpoints are a title screen with an attract ghost,
  every arena at its countdown, Arena 1 at x5 with its speed trail, and a
  results panel with the gold medal. This sample has **no** title screen, **no**
  attract ghost, **no** speed trail, **no** results panel and **no** gold-medal
  display: the four captures are Arena 1 at four moments of play, and the
  title-screen capture is the arena at startup, not a title screen.
- The sample now **carries on-screen text in the scene** but does not yet
  show it. `Assets/Scenes/RollABall.unity` carries a legacy uGUI `Canvas`
  (`HudCanvas`) with a `CanvasScaler` at an 800x600 reference resolution,
  three HUD labels (`ScoreLabel`, `ComboLabel`, `TimerLabel`), a title panel
  (`TitlePanel`, `TitleText`, `PlayButton`, `QuitButton`) and a results panel
  (`ResultsPanel`, `ResultsTitle`, `ResultsTime`, `ResultsScore`,
  `RetryButton`, `ArenaSelectButton`), and `Assets/Scripts/HudController.cs`
  drives them. The engine side parses that hierarchy
  (`UiBinding`), resolves the canvas rects and the label text, and the runtime
  exposes `unitas_runtime_ui_text_set`, `unitas_runtime_ui_text_style` and
  `unitas_runtime_ui_pointer_over` for the managed side. What is missing is
  the render step: the canvas draw items are not composited into the windowed
  frame or into the `render_capture` frame, so the score, the multiplier and
  the results text still exist only as C# state and as log lines
  (`ScoreManager.cs:198` logs the combo multiplier, `:227` the gold pickup,
  `:292` the countdown), and a capture cannot show them. The four approved
  references are therefore unchanged and still show the arena alone.
- The sample has one arena; the specification names three (Courtyard,
  Clockwork, Sky Garden), so "each arena at its countdown" is not covered.
- **The four captures render the arena and are reproducible, and approved
  reference images now exist for them.** Each 256 x 256 capture shows
  the arena floor, its four walls, the four bumper boxes, the gold pickups and
  the player ball: 33 408 non-background pixels of 65 536 against the declared
  `expected_non_background_pixels: 1000`, so all four `render_capture`
  checkpoints pass. The capture used to be **not deterministic** — consecutive
  runs of the same scenario drew different images, because the arena floor and
  the bumper boxes swapped their materials between runs (measured over four
  runs of `s8_title_checkpoint`: 3.1 %-31.9 % differing pixels). The cause was
  the native renderer ordering its draw list by `HashMap` iteration order,
  which is seeded per process, so two runs could disagree on which of two
  coincident surfaces won the depth test.
  `crates/unitas-runtime/src/runtime.rs` now sorts the renderer handles
  before it builds the instance list; three consecutive runs of every scenario
  produce byte-identical BMPs, and the four approved references under
  `References/` are compared at `tolerance: 0.005` (0 differing pixels on the
  machine that produced them; the tolerance only absorbs rasterizer
  floating-point differences between processors and stays far below the 3.1 %
  the old draw order produced). What the references still cannot show is the
  specification's title screen, attract ghost, speed trail and results panel:
  this sample has no such scene content, so the references record the arena,
  not the specification's screens.

The specification text is not weakened by this note. The gap above is what
remains before S8 can be claimed complete.

## S9 edit and restore — what is demonstrated and what is not

**Demonstrated.** The editor owns the scene content as a document, and the
document is edited through the editor's own commands, not by writing files
behind the editor's back. `Scenarios/scene_edit_restore.json` runs in the
headless editor and does exactly that against the real
`Assets/Scenes/RollABall.unity`: it records the scene file's digest, opens the
scene (`scene.open`), moves a pickup with `scene.set_property` on the
pickup's `Transform`, saves the scene (`scene.save`), and asserts that the
file on disk really changed. It then undoes the edit (`editor.undo`), saves
again, and asserts that the file is **byte identical** to the digest recorded
before the edit. The scenario finishes by loading the restored scene in the
runtime and checking the pickup's position, so the restored file is proved to
be the playable scene it was, not just a matching byte sequence.

The edit is reachable by an agent through the same registry every other
command uses: the eight scene commands (`scene.open`, `scene.hierarchy`,
`scene.set_property`, `scene.create_object`, `scene.destroy_object`,
`scene.save`, `editor.undo`, `editor.redo`) are MCP tools with scopes, and a
scene edit is recorded as a change set that a person can review.

**Not demonstrated.**

- The **person-facing half** of the editor is not shown: there is no scene
  view, inspector or hierarchy window yet, so a person cannot yet edit a scene
  by hand. The commands exist and are agent-facing; the windows that drive
  them do not.
- **Play-mode edit-state restoration** is not shown: the scenario edits the
  scene on disk outside a play session. Restoring the edit state of a running
  play session is still open.
- Only a **single-object property edit** is exercised. Creating and deleting
  objects, multi-object selection, prefab variants and component templates are
  implemented in the command surface but are not covered by this scenario.

The specification text is not weakened by this note. The gap above is what
remains before S9 can be claimed complete.

## Non-default lane

`make verify-samples` runs this sample's scenarios and is the lane that owns
this file. Its metadata (owner, trigger, lane, evidence, review condition) is
declared in `tools/verify_samples.py` and in the `Makefile`.

The lane also runs a **native scripted launch** (`unitas-player --sample
--frames 120`): the native player boots the project's own managed C# in
process through the CoreCLR host and reports `script_host=coreclr` and
`frames_stepped=120`. That path is green.

**Current lane state (honest).** The lane runs 23 steps and all 23 pass at
the current tree: the 19 scripting scenarios (including `frame_independence`
and the four `s8_*` visual checkpoints), the 2 headless editor scenarios,
the native scripted launch and the packaged-player proof.

- `editor-scenario-collect_all`, `editor-scenario-retry_clean`: these were
  red until the headless editor host learned to load the generated glue
  assembly when it registers project scripts (`LoadGeneratedGlue` in
  `editor/Unitas.Editor/Play/ScenarioTools.cs`). Before that fix the editor
  host produced no log message at all because the sample's private,
  Unity-canonical magic methods were never dispatched; with the glue loaded
  the log checkpoints pass and both steps are green.

The steps this note owns are recorded here rather than left implicit:

- `frame_independence`: every frame rate sweeps green
  (`sweep rate 30/60/144/240: pass`) and the sweep verdict passes
  (`sweep: pass`), so the gameplay message order is frame-rate independent.
  The earlier `sweep: log order differs between rate 30 and rate 60` verdict
  was fixed upstream in commit `ad3ae72` and this paragraph was stale until
  now.
- `s8_title_checkpoint`, `s8_arena_countdown`, `s8_arena_x5_checkpoint`,
  `s8_results_gold_checkpoint`: the captures are wired into the lane and run,
  and each `render_capture` checkpoint passes its
  `expected_non_background_pixels: 1000` check with 33 408 observed
  non-background pixels of 65 536. Each of them now also compares against the
  approved reference image under `References/` at `tolerance: 0.005`, and the
  lane fails the step when a capture drifts past that tolerance; the
  reference-approval record is `References/README.md` and the determinism
  fix that made the approval honest is recorded above.
