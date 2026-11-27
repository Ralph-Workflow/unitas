# Roll-a-Ball ("Marble Rush") — sample status

This file records the **demonstrated** state of the sample, as the product
specification for it (`docs/product-specification/sample-games/01-roll-a-ball.md`)
requires: every behavior the document asks for that is not demonstrated is
reported with its real state, never hidden.

## What the sample contains

- One gameplay scene, `Assets/Scenes/RollABall.unity`: the first arena (a 12 x 12
  unit floor with four walls), one marble (`PlayerBall`, radius 0.5), a pickup
  sensor, a directional light and one camera (`Main Camera`). The pickups are
  partly authored as prefab instances (`Assets/Prefabs/Pickup.prefab`,
  `PickupGold.prefab`) under the `Pickups` root, next to four authored pickup
  objects, and the scene also carries an authored prop (`ArenaProp`), a
  collect burst, a landing dust effect and the collect chime.
- Two more arena scenes, `Assets/Scenes/Arena2.unity` (the Clockwork arena) and
  `Assets/Scenes/Arena3.unity` (the Sky Garden arena), each carrying the
  `ArenaController` script that lays that arena's twelve pickups on the arena's
  own layout and drives the Sky Garden's moving blocks. Their scenario
  `Scenarios/arenas_2_3.json` passes 11 of 11 checkpoints with `--repeat 2`
  and runs in the `make verify-samples` scenario list.
- One physics-probe scene, `Assets/Scenes/PhysicsLab.unity`, used by the
  physics scenarios; it is not an arena and carries no game scripts. It now
  also carries the bodies the S11 scenarios need: the `EdgeMarble` (a 2 kg
  sphere with `m_Drag: 0.5`, `m_Interpolate: 1` and `m_CollisionDetection: 2`),
  the `EdgeWall` corner (`EdgeWall`, `EdgeWall_Side` and `EdgeWall_End`), the
  `ArenaBumper` sphere with
  `Assets/Materials/ArenaBumper.physicMaterial` (`m_Bounciness: 1.4`,
  `m_BounceCombine: 2`, Unity's Maximum rule), the `TriggerPairA` and
  `TriggerPairB` trigger spheres that carry no Rigidbody, and the dynamic
  `PushedMarble` next to the kinematic `Pusher`. It also carries the two
  objects the mesh-and-compound scenario needs: a GameObject whose
  `MeshCollider` names the concave frame mesh under `Assets/Models/`, and a
  GameObject that carries a `BoxCollider` and an offset `SphereCollider`.
- Eight scripts under `Assets/Scripts/`: `PlayerController`, `ScoreManager`,
  `RecordsStore`, `Rotator`, `PhysicsLabProbe`, `Pickup`, `GoldPickupMarker`,
  `HudController`.
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
| S4 retry without leftover state | demonstrated | `Scenarios/restart_recovery.scenario.json` presses `R` in the middle of a run and asserts that the ball is back at its spawn point afterwards, that the pickups can be collected again and that the run reaches `All 12 pickups collected` after the retry; six further checkpoints pin the reset state the specification's S4 asks for — `retry-score-reset`, `retry-pickups-restored` (all twelve pickups exist), `retry-multiplier-reset` (x1), `retry-gold-removed` (no gold pickup exists), `retry-timer-reset` (the timer reads 0) and `gold-spawned-again` — and the sample's retry now resets the run timer and logs the **actual** reset state (`Retry complete score 0 pickups 12 multiplier x1 timer 0.000`) instead of a hardcoded line, so those pins fail when a reset regresses; the scenario passes 16 of 16 checkpoints with `--repeat 2` in the lane's scripting step, and passes all 16 in the headless editor host, which runs the scenario once. The specification's "no object from the previous attempt remains" rests on the sample destroying its own pickups and resetting the marble; the engine destroys a replaced scene's objects on a single-mode load and creates the reloaded scene's objects before the load returns, so a lookup after a reload resolves the new attempt's objects and never the replaced attempt's — `Scenarios/ghost_replay.scenario.json` pins that over the registry (`next_attempt_scene_reloaded`) |
| S12 combo multiplier, S13 gold pickup window, S15 dash and cooldown | demonstrated | `Scenarios/combo_multiplier.json`, `Scenarios/gold_window.json`, `Scenarios/gold_collect.json`, `Scenarios/dash_cooldown.json` — all four pass |
| S6 settings and persistence | partly demonstrated | `Scenarios/persist_write.json`, `Scenarios/persist_read.json` cover persistence of records, and the title scene's settings panel (`Assets/Scenes/Title.unity`, `Assets/Scripts/SettingsController.cs`) writes the master, music and effects volume, fullscreen, resolution, vertical sync, quality, ghost, camera-shake and `reduced_motion` preferences — `Scenarios/reduced_motion.json` opens that panel, flips the reduced-motion row (`Settings reduced_motion 1`) and proves the preference survives the load into the gameplay scene. Only the reduced-motion row has a scenario; the other rows are written by the panel but no scenario pins them yet |
| S7 input modes | partly demonstrated | `Scenarios/frame_independence.json` injects keyboard input (`W`, `A`) and sweeps 30/60/144/240 fps, and `Scenarios/gamepad_drive.scenario.json` injects the gamepad axes. `Scenarios/pointer_ui.scenario.json` drives the pointer (`inject_pointer` to the Play button at the 800x600 reference-resolution centre, then `inject_mouse_button`) and is in the `make verify-samples` scenario list: the native runtime detects the button hit and queues the clicked object id, the bridge binds the scene's uGUI buttons to their managed `UnityEngine.UI.Button` counterparts and pumps that queue every frame, so the click reaches `Button.onClick`, `HudController` logs `Title play clicked`, and the scenario's log checkpoint passes |
| S8 visual checkpoints | partly demonstrated — see below | `Scenarios/s8_*.scenario.json`, `References/*.bmp` — all five pass, their `render_capture` checkpoints included, and the approved references were re-approved on 2026-11-27 with the arena's authored materials (textured and normal-mapped pickups, a normal-mapped marble, a tiled floor) and the HUD drawn from the engine-bundled font in the picture. The two extra arenas' content is covered by `Scenarios/arenas_2_3.json` (11 of 11 checkpoints, in the lane), but neither of them has a capture or an approved reference image of its own |
| S10 package | partly demonstrated | the lane packages a macOS player with a build report and replays input in it (`packaged-player-proof` passes); only one desktop target is built, not each |
| S9 edit and restore | partly demonstrated — see below | `Scenarios/scene_edit_restore.json` edits the scene through the editor's own commands, creates and destroys a GameObject, proves the edit landed, undoes it and proves the scene file is byte identical to the original; `Scenarios/play_mode_edit_restore.json` proves that a play session's runtime edit is restored when the session ends; the person-facing editor half is not demonstrated |
| S11 collision edge | demonstrated | `Scenarios/collision_edge.json` proves the collision edge, and `Scenarios/trigger_without_rigidbody.json` and `Scenarios/collision_payload.json` prove the trigger rule and the collision payload it rides on (see the S11 section below) |
| S5 focus loss and recovery | engine half demonstrated | the windowed player clears every held key, mouse button and axis on `WindowEvent::Focused(false)` and freezes the simulation (`time_scale` 0), and restores the time scale and clean input on focus return (`FocusPause` in `crates/unitas-runtime/src/input.rs`, wired in `crates/unitas-runtime/src/window.rs`); the four focus tests live in `crates/unitas-runtime/src/input.rs` and `crates/unitas-runtime/src/window.rs`. The person-facing half — a pause overlay and an explicit Resume — does not exist yet, so "play resumes only after Resume" is not demonstrated |
| S12 combo timing sweep | demonstrated | `Scenarios/combo_timing_sweep.json` drives the marble around the pickup ring with the same travel per leg and sweeps only the gap between consecutive pickups: 1.400 s (below the 1.5 s combo window) raises the multiplier to x4, the window then expires 1.5167 s later (`Combo expired`), the next gap of 1.600 s (above the window) resets the multiplier to x1 and awards a single point, and a further 1.400 s gap re-arms it to x2; 17 of 17 checkpoints pass with `--repeat 2` |
| S14 medals/records/ghost | demonstrated | `Scenarios/persist_state_write.json` awards the gold medal with the new best time and best score and unlocks Arena 2 (`Records saved medals arena1=gold bestTime 4.900 bestScore 50 arenas arena1,arena2 quality high bytes 160`) and writes the ghost file (`Ghost written version 2 steps 245 bytes 1996`); `Scenarios/persist_state_read.json` restores both on the next launch (`Records loaded medals arena1=gold ...`, `Ghost restored version 2 steps 245 bytes 1996`); `Scenarios/ghost_replay.scenario.json` finishes Arena 1 through replayed input in 796 frames (the `frames_under_30s` checkpoint), replays the recorded route on the ghost object within 1 cm at every fixed step (`Ghost replay within 0.01 m`, `Ghost attempt replay within 0.01 m`) and starts the next attempt against the reloaded scene (`next_attempt_origin`, `next_attempt_scene_reloaded`). All three run in the `make verify-samples` lane. The ghost's translucent material is not pinned by a render capture |
| S16 reduced motion | partly demonstrated | `Assets/Scripts/ReducedMotion.cs` owns the `reduced_motion` PlayerPrefs preference and `PlayerController` stops the speed trail while it is on (`Reduced motion active trail off`); `Scenarios/reduced_motion.json` runs: it opens the settings row on the existing `Assets/Scenes/Title.unity` title scene, flips reduced motion on, loads `Assets/Scenes/RollABall.unity` and pins that the speed trail is switched off. The story's camera-shake half has no setting to switch off |

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
  (Built-in), so a pipeline sweep is still impossible; the **quality** sweep is
  possible now: `Scenarios/s8_quality_sweep.scenario.json` drives the three
  levels its `QualitySettings.asset` declares (Low, Medium, High) through the
  new `set_quality` scenario step and captures the arena at each of them
  (`References/s8_quality_low.bmp`, `s8_quality_medium.bmp`,
  `s8_quality_high.bmp`), and the lane runs it beside the other four. The three
  captures differ from each other: the Low level draws the arena with the
  shadow map off, the Medium level with the hard shadow map, and the High level
  with the soft shadow map. The headless capture path applies the requested
  level since 2026-11-27 (`Runtime::render_frame` takes the shadow settings
  from the runtime's quality level, which the `set_quality` step now changes);
  before that, the three approved files were byte identical to each other and
  to `s8_title_checkpoint.bmp`, which is why `References/README.md` records a
  separate re-approval for them.
- The specification's checkpoints are a title screen with an attract ghost,
  every arena at its countdown, Arena 1 at x5 with its speed trail, and a
  results panel with the gold medal. This sample has **no** title screen, **no**
  attract ghost, **no** speed trail, **no** results panel and **no** gold-medal
  display: the four captures are Arena 1 at four moments of play, and the
  title-screen capture is the arena at startup, not a title screen.
- The sample now **carries on-screen text in the scene and shows it in the
  captures**. `Assets/Scenes/RollABall.unity` carries a legacy uGUI `Canvas`
  (`HudCanvas`) with a `CanvasScaler` at an 800x600 reference resolution,
  three HUD labels (`ScoreLabel`, `ComboLabel`, `TimerLabel`), a title panel
  (`TitlePanel`, `TitleText`, `PlayButton`, `QuitButton`) and a results panel
  (`ResultsPanel`, `ResultsTitle`, `ResultsTime`, `ResultsScore`,
  `RetryButton`, `ArenaSelectButton`), and `Assets/Scripts/HudController.cs`
  drives them. The engine side parses that hierarchy
  (`UiBinding`), resolves the canvas rects and the label text, and the runtime
  exposes `unitas_runtime_ui_text_set`, `unitas_runtime_ui_text_style` and
  `unitas_runtime_ui_pointer_over` for the managed side. The runtime
  composites the canvas draw items onto the rendered frame
  (`Runtime::composite_canvas_onto_frame` in
  `crates/unitas-runtime/src/runtime.rs`), so the `render_capture` checkpoints
  show the HUD labels and the panels over the arena. The four approved
  references were re-approved on 2026-11-26 with the HUD in the picture; see
  `References/README.md` for what that re-approval changed. The machine
  dependence the earlier references gave up is closed: the canvas text is
  rasterized from the **engine-bundled** Noto Sans face
  (`crates/unitas-render/assets/NotoSans-Regular.ttf`, OFL-1.1, byte-identical
  to the sample's licensed copy), which `FontLibrary::load_bundled_font`
  loads before the system fonts; the system fonts stay the fallback for the
  glyphs the bundled face lacks, so the HUD letterforms are the same on every
  machine.
- The managed side of the interface is bound now: the bridge installs the
  `UnityEngine.UI.IUiProvider`, binds the scene's uGUI `Text` and `Button`
  components to their managed `UnityEngine.UI.Text` and
  `UnityEngine.UI.Button` counterparts, and pumps the native click queue every
  frame, so a managed `Text.text` write reaches the canvas and a native click
  reaches `Button.onClick`. The HUD therefore shows the text `HudController`
  writes, and the pointer scenario's click assertion passes.
- The windowed player **now shows the canvas**: the windowed path builds the
  frame with the scene and the text overlay, composites the canvas with the
  same `Runtime::composite_canvas_image` the capture path uses (now one
  `composite_canvas_into` implementation behind both), and presents that
  composed image on the window surface
  (`GpuFramePresenter::render_window_frame` and `present_window_frame` in
  `crates/unitas-runtime/src/presenter.rs`, with
  `Renderer::present_image_to_surface` in `crates/unitas-render/src/gpu.rs`).
  There is still no second canvas renderer: the capture and the window share
  one composition.
- The sample now carries three arenas — Courtyard (`Assets/Scenes/RollABall.unity`),
  Clockwork (`Assets/Scenes/Arena2.unity`) and Sky Garden
  (`Assets/Scenes/Arena3.unity`) — and the two extra arenas' content is now
  covered: `Scenarios/arenas_2_3.json` passes 11 of 11 checkpoints with
  `--repeat 2` in the lane. Both arena scenes load, the arena controller's own
  log is captured (`[roll-a-ball] arena Arena2 pickups 12` and `[roll-a-ball]
  arena Arena3 pickups 12`), the twelve pickups sit on the arena's own layout
  instead of the positions the scene authors gave them (the clockwork ring at
  `(0, 0.75, 3.75)`, the sky garden path at `(0, 0.75, 2)`), the marble starts
  and stays inside the arena and collects pickups, and the sky garden spawns
  its gold pickup.
- The two extra arenas are still not covered by the **visual** checkpoint: the
  scenario takes no `render_capture`, so "each arena at its countdown" has no
  capture and no approved reference image of its own and is still not covered.
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

- **The captures now show the arena's authored materials, through the real
  scene path.** The material contract carries the whole material: the asset
  crate parses `_BumpMap`, `_EmissionMap`, `_Metallic`, `_Glossiness` and
  `_EmissionColor` beside `_Color` and `_MainTex`
  (`crates/unitas-asset/src/material.rs`), the scene assembly resolves the
  texture guids to paths (`crates/unitas-asset/src/scene.rs`), the runtime
  decodes them into the renderer's material maps and keeps the meshes' UVs and
  tangents (`crates/unitas-runtime/src/runtime.rs`), and the presenter draws
  the scene through the render crate's material path with cached mesh buffers
  and cached material bind groups (`crates/unitas-render/src/gpu.rs`,
  `crates/unitas-runtime/src/presenter.rs`) — the windowed and the headless
  capture paths share it. The sample's own content exercises it: `Pickup.mat`
  carries its albedo and normal map and now an emission map with an emission
  colour, and `Marble.mat` and `Floor.mat` carry a normal-mapped marble and a
  tiled floor over four procedurally generated, original PNGs in
  `Assets/Textures/`. The captures are the evidence that the materials reach
  the pixels.

The specification text is not weakened by this note. The gap above is what
remains before S8 can be claimed complete.

## S9 edit and restore — what is demonstrated and what is not

**Demonstrated.** The editor owns the scene content as a document, and the
document is edited through the editor's own commands, not by writing files
behind the editor's back. `Scenarios/scene_edit_restore.json` runs in the
headless editor and does exactly that against the real
`Assets/Scenes/RollABall.unity`: it records the scene file's digest and opens
the scene (`scene.open`). It then creates a GameObject
(`scene.create_object`), saves the scene (`scene.save`) and asserts that the
file on disk really changed; it destroys that GameObject again
(`scene.destroy_object`), saves, and asserts that the file is **byte
identical** to the digest recorded before the edit, so the create-and-destroy
round trip is exact. Next it moves a pickup with `scene.set_property` on the
pickup's `Transform`, saves, and asserts that the file changed again. It then
undoes the edit (`editor.undo`), saves again, and asserts that the file is
**byte identical** to the digest recorded before the edit. The scenario
finishes by loading the restored scene in the runtime and checking the
pickup's position, so the restored file is proved to be the playable scene it
was, not just a matching byte sequence.

The edit is reachable by an agent through the same registry every other
command uses: the scene commands (`scene.open`, `scene.hierarchy`,
`scene.set_property`, `scene.create_object`, `scene.destroy_object`,
`scene.save`, `editor.undo`, `editor.redo`) are MCP tools with scopes, and a
scene edit is recorded as a change set that a person can review.

**Demonstrated (play-mode edit state).**
`Scenarios/play_mode_edit_restore.json` runs in the headless editor and drives
a real play session: it moves the pickup at edit time (`scene.set_property`),
saves the scene and records the digest of the saved file; it then enters play
(`play.enter`), changes the same property in the running session
(`play.set_runtime_property`) and leaves play again (`play.exit`). The editor
reports that it captured the edit state when play was entered and restored it
when play was left, and the scenario saves and asserts that the file on disk
is still **byte identical** to the digest taken before the play session, so
the runtime edit never reached the scene file. It then undoes the edit-time
change (`editor.undo`), saves, and asserts that the file is **byte identical**
to the digest recorded before any edit, and finishes by loading the restored
scene in the runtime and checking the pickup's position.

**Not demonstrated.**

- The **person-facing half** of the editor is not shown: there is no scene
  view, inspector or hierarchy window yet, so a person cannot yet edit a scene
  by hand. The commands exist and are agent-facing; the windows that drive
  them do not.
- Multi-object selection, prefab variants and component templates are
  implemented in the command surface but are not covered by this scenario.

The specification text is not weakened by this note. The gap above is what
remains before S9 can be claimed complete.

## S11 collision edge — what is demonstrated and what is not

**Demonstrated.** The specification asks for a marble dashed from a boost pad
into a wall corner and then into a bumper, with no tunneling, with bounces that
follow each physic material, and with the bumper's outgoing speed above its
incoming speed
(`docs/product-specification/sample-games/01-roll-a-ball.md:387-391`). Three
scenarios over the sample's physics-probe scene now cover that, and all three
run in the `make verify-samples` lane with `--repeat 2`, so the lane also proves
that two runs of the same scenario give the same result:

- `Scenarios/collision_edge.json` loads `Assets/Scenes/PhysicsLab.unity` twice
  and ticks it twice over the same 2.4 seconds of simulated time, 120 frames at
  0.02 s and 240 frames at 0.01 s. It asserts that the `EdgeMarble` starts at
  its authored position, that it ends at the same corner position
  `[-20, 0.5, -15.25]` after the coarse step and after the fine step within
  0.25, that the bumper contact is reported (`EdgeMarble bumper outgoing`),
  that the outgoing speed is above the incoming speed (`restitution gained`),
  and that the contact carried a non-zero impulse
  (`EdgeMarble contact ArenaBumper non-zero impulse`).
- `Scenarios/trigger_without_rigidbody.json` asserts the literal absence of a
  trigger message for the `TriggerPairA`/`TriggerPairB` pair, two overlapping
  trigger spheres that carry no Rigidbody, and that the kinematic `Pusher`
  reports a collision against the dynamic `PushedMarble`.
- `Scenarios/collision_payload.json` asserts that the probe's contact line is
  printed and that it carries the contact count (`contacts 1`).

**Not demonstrated.**

- The scenarios run in `Assets/Scenes/PhysicsLab.unity`, the physics-probe
  scene, and not in the gameplay arena `Assets/Scenes/RollABall.unity`. The
  specification's dash from a boost pad is not exercised: the probe launches the
  marble, and no boost pad, dash or `PlayerController` is involved.
- "The bounces follow each physic material" is shown for one material pair per
  run. The `ArenaBumper` carries
  `Assets/Materials/ArenaBumper.physicMaterial` (`m_Bounciness: 1.4`, the
  restitution the specification asks for at
  `docs/product-specification/sample-games/01-roll-a-ball.md:78`, and
  `m_BounceCombine: 2`, Unity's Maximum rule). No scenario puts two materials
  with different combine rules against each other, so the per-material rule is
  proven for the maximum rule alone.
- The payload is proven through the probe's log line, which carries the contact
  count, the impulse, the relative speed, the contact point and the normal. The
  checkpoints match parts of that line (`non-zero impulse`, `contacts 1`), so
  they do not pin the relative velocity, the contact point or the normal to a
  value. No scenario runs `OnCollisionEnter` in the gameplay arena.
- The lane proves that two runs of each scenario agree. It does not prove that
  the arena's own bumper and wall geometry behaves the same way, because the
  arena content the specification names (`BoostPad.prefab`, `Bumper.prefab`,
  `Wall.prefab` and the three arena scenes) does not exist in this sample yet.

The specification text is not weakened by this note. The gap above is what
remains before S11 is claimed complete in the arena.

## Mesh and compound colliders — what is demonstrated and what is not

**Demonstrated.** The engine simulates a mesh collider as the mesh it names
and not as the box around it, and a GameObject contributes every collider it
carries, so one body can carry more than one shape. The sample's
physics-probe scene carries the two objects that only behave correctly with
those two behaviours, and `Scenarios/mesh_and_compound_colliders.json` drives
them in the `make verify-samples` lane with `--repeat 2`, so the lane also
proves that two runs of the same scenario give the same result:

- the mesh object: a GameObject whose `MeshCollider` names the concave frame
  mesh under `Assets/Models/` (a square frame with an opening). The scenario
  fires the marble at the frame's opening and asserts that it passes through
  with no contact reported. A bounding-box approximation blocks the marble at
  the box around the frame, so that assertion fails for it.
- the compound object: a GameObject that carries a `BoxCollider` and an offset
  `SphereCollider`. The scenario fires the marble at the offset collider and
  asserts that the contact is reported, so the body carries both shapes and
  not only the first collider of the GameObject.

**Not demonstrated.**

- The proof runs in `Assets/Scenes/PhysicsLab.unity`, the physics-probe scene,
  and not in the gameplay arena `Assets/Scenes/RollABall.unity`, which still
  carries no mesh collider and no compound collider.
- The sample's mesh collider sits on a static body. A non-convex mesh collider
  on a dynamic body is refused, as Unity's rule requires, and that refusal is
  proven by the engine's own tests, not by a sample scenario.
- The step is newer than the last full-lane run, so the lane's own run of the
  merged tree is what proves it. A red run of this step is a red lane, and
  this note is not a pass on its own.

The specification text is not weakened by this note. The gap above is what
remains before the sample's arena carries a mesh collider or a compound
collider of its own.

## Gameplay audio — what is demonstrated and what is not

- **The collect chime plays on a pickup, through the real path.**
  `Assets/Audio/PickupChime.wav` is an original clip generated for this
  sample, and `Assets/Scripts/ScoreManager.cs` plays it on the `CollectChime`
  `AudioSource` of `Assets/Scenes/RollABall.unity` in the frame the pickup is
  collected. `Scenarios/collect_chime.json` asserts both: the pickup log
  (`Pickup collected score 1`) and the chime log
  (`[roll-a-ball] collect chime played pickup_frame=8 chime_frame=8
  state=Playing silent=None`) — the same frame, with the source playing and
  not silenced. The step runs in the `make verify-samples` lane.
- The path is the product path: the managed `AudioSource` → the bridge →
  the `AudioWorld` mixer → the cpal device, pumped every frame by
  `mix_and_push_audio` in `crates/unitas-runtime/src/presenter.rs`.
- **Mixer-group volume and pause-silences-gameplay are covered by tests.**
  `crates/unitas-audio/src/lib.rs` proves that a source on the `Sfx` bus is
  scaled by its group volume, that a muted group silences its sources, and
  that `set_gameplay_paused(true)` silences the gameplay source while a menu
  source keeps playing.
- **The collect chime is audible on the device, through the real path.** The
  device lane captures the samples the device output accepted to
  `target/device-lane/device_audio.wav` and asserts the chime energy in the
  window after the injected pickup against the silence floor
  (`tools/verify_device.py`), so the capture proves the sample's own chime
  clip reaching the device stream, not a synthetic tone.
- What is **not** demonstrated: the capture comes from a separate headless
  injection of the scene's `CollectChime` source, not from the windowed run,
  so the windowed player's own audio is still not captured. The scene's own
  `m_audioClip` reference **is** bound to the managed `AudioSource`: the
  runtime resolves the serialized clip guid to the imported
  `Assets/Audio/CollectChime.wav` and the managed source carries that
  native clip id, proven by `Scenarios/scene_audio_clip_binding.json` and
  `crates/unitas-runtime/tests/scene_audio_clip_binding.rs`. The managed
  materialiser still identifies the native source by a positional entity-id
  guess, so a miss would fall back to loading the clip by path.
  Streaming music and 3D spatialization are out of scope for this phase.

## Non-default lane

`make verify-samples` runs this sample's scenarios and is the lane that owns
this file. Its metadata (owner, trigger, lane, evidence, review condition) is
declared in `tools/verify_samples.py` and in the `Makefile`.

The lane also runs a **native scripted launch** (`unitas-player --sample
--frames 120`): the native player boots the project's own managed C# in
process through the CoreCLR host and reports `script_host=coreclr` and
`frames_stepped=120`. That path is green.

**Current lane state (honest).** The lane runs 49 steps: the 34
scripting scenarios (including `frame_independence`, `pointer_ui`,
`restart_recovery`, `combo_timing_sweep`, the arena scenario `arenas_2_3`,
the four `s8_*` visual checkpoints,
the four persistence scenarios `persist_write`, `persist_read`,
`persist_state_write` and `persist_state_read`, the ghost-replay scenario
`ghost_replay` and the four physics scenarios `collision_edge`,
`trigger_without_rigidbody`, `collision_payload` and
`mesh_and_compound_colliders`), the 13 headless editor
scenarios (`collect_all`, `retry_clean`, `scene_edit_restore`,
`play_mode_edit_restore`, `gold_window`, `gold_collect`, `combo_multiplier`,
`dash_cooldown`, `pause_holds_simulation`, `slowmo_finish`, `restart_recovery`,
`combo_timing_sweep`, `pointer_ui`), the native scripted launch and the
packaged-player proof. That number counts the lane's run steps only; before them
the lane runs its three build steps — the runtime, the bridge and the player
build. The headless editor step now runs the same gameplay scenarios the
scripting step runs, so a scenario's outcome is the same whether a person or
an agent drives it; the lane's own run of the current tree is what proves
every step, and a red run of any step is a red lane and this note is not a
pass on its own.

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
- `restart_recovery`: the lane runs it with `--repeat 2`, so the retry has to
  be reproducible as well as correct. The scenario presses `R` in the middle
  of a run and asserts that the ball is back at its spawn point afterwards
  (`restarted-position`), that the pickups can be collected again
  (`pickup-1-again` through `pickup-4`), and that the run reaches `All 12
  pickups collected` after the retry (`win-log`); it also pins the frame count
  at the retry (`restarted-frames`, 240) and at the end of the run
  (`total-frames`, 785), and the six reset checkpoints the specification's S4
  asks for (`retry-score-reset`, `retry-pickups-restored`,
  `retry-multiplier-reset`, `retry-gold-removed`, `retry-timer-reset`,
  `gold-spawned-again`). The scenario passes 16 of 16 checkpoints with
  `--repeat 2` in the lane's scripting step, and passes all 16 in the headless
  editor step, which runs the scenario once.
- `combo_timing_sweep`: the lane runs it with `--repeat 2` and the headless
  editor step runs the same file. The sweep varies only the gap between
  pickups, so the multiplier's behaviour at the combo window boundary is the
  thing under test.
- `ghost_replay`: the lane runs it with `--repeat 2` against a persistence
  root of its own, because the shared root already holds the completed run
  the four persistence scenarios leave behind and the scenario would restore
  that leftover ghost (`outcome AlreadyCurrent`) instead of proving its own
  write-and-restore cycle. It pins the ghost file, the route comparison, the
  replay within 0.01 m, and - over the engine's object registry - that the
  reloaded scene's marble is the new attempt's marble
  (`next_attempt_scene_reloaded`). All ten of its checkpoints pass against the
  live run: the write logs `Ghost written version 2 steps 245 bytes 1996 first
  -3.150 0.500 3.300 last 2.250 0.500 3.300`, the file round trip logs `Ghost
  replay max deviation 0.000 m over 245 steps` and `Ghost replay within 0.01
  m`, and the next attempt's replay logs `Ghost attempt replay max deviation
  0.000 m over 245 steps` and `Ghost attempt replay within 0.01 m`. The lane's
  `--repeat 2` comparison of the two runs needed the sample to hold the scene
  composition steady: the second repeat restores the ghost file the first
  repeat wrote, so it used to run with one more scene object, and the marble's
  settled height then differed by 7 um (`next_attempt_origin` read `0.499948` m
  in the first repeat and `0.499955` m in the second). The sample now creates
  the ghost replay object in every attempt, recorded route or not, so the
  attempt's scene composition does not depend on whether a previous attempt
  wrote a route; both repeats then observe the same marble position and the
  lane's comparison passes.
- The kinematic push is reported on the staying contact: the engine's contact
  between the kinematic `Pusher` and the dynamic `PushedMarble` starts one
  frame before the solver applies the push impulse, so the sample's probe
  reports the sustained push from `OnCollisionStay` as well as from
  `OnCollisionEnter`.
- `s8_title_checkpoint`, `s8_arena_countdown`, `s8_arena_x5_checkpoint`,
  `s8_results_gold_checkpoint`: the captures are wired into the lane and run,
  and each `render_capture` checkpoint passes its
  `expected_non_background_pixels: 1000` check with 33 408 observed
  non-background pixels of 65 536. Each of them now also compares against the
  approved reference image under `References/` at `tolerance: 0.005`, and the
  lane fails the step when a capture drifts past that tolerance; the
  reference-approval record is `References/README.md` and the determinism
  fix that made the approval honest is recorded above.
