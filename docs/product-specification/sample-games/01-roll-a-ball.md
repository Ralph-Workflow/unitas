# Sample 1: Roll-a-Ball-Style Arena ("Marble Rush")

## Overview

Marble Rush is a fast momentum game. The player rolls a heavy marble through
three handcrafted arenas and collects every pickup as quickly and stylishly as
possible. Chaining pickups builds a score multiplier. Boost pads, bumpers, and
jump pads let a skilled player carve fast routes. Medals and a ghost of the best
run make every arena worth replaying. One arena takes 45 to 90 seconds. A full
playthrough takes about ten minutes, and mastering every gold medal takes much
longer.

This is an original, redistributable unitas sample inspired by the familiar
rolling-ball learning scenario. It is distinct from any official Unity sample
and makes no claim to redistribute or modify Unity-owned sample content. It is
the first game in the suite. It proves the complete loop from opening a Unity
project to playing a packaged build. It is not the end goal of the engine.

Compatibility tiers follow the [Feature catalog](../feature-catalog.md). This
document owns this game's acceptance under SMP-001.

## Targets and Input Modes

* **Targets:** macOS, Windows, and Linux desktop players, and the web player
  (WebAssembly with WebGPU, and WebGL 2 as the fallback) in current Chrome, Edge,
  Firefox, and Safari on desktop and mobile. Also the editor's play mode and the
  headless editor.
* **Web budgets:** the web build meets the engine web budgets in the "Web target"
  section of [Builds, platforms, XR, and networking](../build-platforms-xr-networking.md).
  That means at most 15 MB compressed transfer, a first interactive frame within
  3 s on a cold cache and within 1 s on a warm cache, and 60 fps at 1080p on the
  reference laptop and phone. Every scenario that runs in a packaged player also
  runs in the web player.
* **Render pipelines:** the project ships in two variants that are otherwise
  identical, one on the Built-in Render Pipeline and one on URP 14.x (Forward).
  Both variants are accepted.
* **Input modes:** keyboard, mouse (menus only), and gamepad. The movement code
  exists in two variants: legacy `Input` axes (INP-001) and an Input System 1.x
  action map (INP-002). Both variants are accepted.

## Game Design

### Core loop

1. The title screen shows Play, Settings, and Quit. Behind the menu, the best
   ghost of Arena 1 rolls through its route as an attract mode.
2. Arena select shows three arenas, each with its best time, best score, and
   earned medal. Arena 2 unlocks after Arena 1 is completed. Arena 3 unlocks
   after Arena 2 is completed.
3. A three-second countdown starts the run. Each arena holds twelve pickups,
   and the timer runs until the last one is collected.
4. The player rolls, dashes, and launches to collect pickups. Each collection
   adds points times the current multiplier.
5. After the twelfth pickup, time slows to 25 percent for 0.5 seconds of real
   time. The results panel then shows time, score, the best combo, the medal
   earned, and whether a new record was set. Its buttons are Retry, Next Arena,
   and Arena Select.

### Mechanics

* **Momentum movement.** Input applies force to a Rigidbody marble with mass 2,
  and a physic material gives it grip on the floor and bounce off walls. Top
  rolling speed under input alone is 9 m/s. Boost pads and slopes take it past
  that.
* **Dash.** Pressing Dash applies an 8 m/s impulse in the input direction, or in
  the direction of travel when there is no input. It has a 2-second cooldown, and
  a ring around the marble shows the cooldown.
* **Combo multiplier.** Collecting a pickup within 1.5 seconds of the previous
  one raises the multiplier by one step, from x1 up to x5. A combo bar under the
  score drains over the 1.5-second window. When it empties, the multiplier
  returns to x1.
* **Gold pickup (risk and reward).** When the multiplier reaches x3, a gold
  pickup spawns at a fixed, hard-to-reach spot in the arena for 4 seconds. It is
  worth 5 points times the multiplier and refills the combo bar. It is optional,
  and it does not count toward the twelve required pickups. Reaching it usually
  means a risky dash or a jump-pad launch.
* **Boost pads.** These push the marble along their arrow to 16 m/s.
* **Bumpers.** These bounce the marble off with a restitution of 1.4 and give a
  small camera shake.
* **Jump pads.** These launch the marble on a fixed arc to a raised platform.
* **Hazards.** Rotating sweeper arms (Arena 2) and moving blocks (Arena 3) are
  kinematic bodies that knock the marble around. Falling off a raised platform
  drops the marble to the lower floor of the same arena. The marble keeps its
  pickups but loses its combo. The marble never leaves the arena.

### Arenas and escalation

| Arena | Layout | What it teaches |
|---|---|---|
| 1. Courtyard | flat floor, one ramp, two boost pads, a ring of pickups around a fountain | momentum, combos, the ramp launch in the first 10 seconds |
| 2. Clockwork | rotating platforms, four bumpers, two sweeper arms, a lower floor | timing, using bumpers as shortcuts, recovering from knock-backs |
| 3. Sky Garden | narrow raised paths, jump pads, moving blocks, a gold pickup above a gap | precision, dash control, the gold-pickup gamble |

### What makes it fun

* **Hook.** Within the first 10 seconds of Arena 1, a ramp launches the marble
  through a ring of four pickups. The chime rises one semitone per combo step,
  so a beginner hears a x4 chain right away.
* **Mastery curve.** A skilled player chains all twelve pickups at x5, uses
  bumpers and dashes to cut corners, and finishes Arena 1 in under 30 seconds.
  A new player takes about 60 seconds.
* **Risk and reward.** The gold pickup is worth up to 25 points, but chasing it
  can break the route or drop the marble to the lower floor. Dashing early
  speeds up a route, but leaves no dash for recovery.
* **Escalation.** Each arena adds one new kind of element and combines it with
  the ones before.
* **Game feel.**
  * A burst of particles and a short scale pop on each collection.
  * A speed trail behind the marble above 10 m/s.
  * A camera field-of-view kick on each boost.
  * A light camera shake on each bumper hit.
  * A dust puff on landing.
  * A rising chime pitch per combo step.
  * A short controller rumble on bumps, on gamepads that support it.
  * A slow-motion finish on the last pickup.
* **Replay.**
  * Bronze, silver, and gold medals per arena, at fixed time thresholds (for
    Arena 1: 60, 40, and 30 seconds).
  * Best score and best time records.
  * A translucent ghost of the player's best run in every attempt, which can be
    turned off.
  * Unlocking the next arena.

### Controls

| Action | Keyboard | Gamepad | Mouse |
|---|---|---|---|
| Roll | WASD or arrow keys | Left stick | — |
| Dash | Space | South button | — |
| Retry instantly | R | North button | — |
| Pause | Escape or P | Start | — |
| Navigate menus | Arrow keys, Tab | D-pad, left stick | Pointer |
| Confirm | Enter, Space | South button | Left click |
| Back | Escape | East button | — |

### Pause

Pause sets `Time.timeScale` to zero. It shows a pause panel with Resume, Retry,
Settings, and Quit to arena select. Gameplay sounds stop while menu sounds keep
playing. Pickups, platforms, and the combo bar freeze. Resume restores the
previous time scale.

### Settings

The settings are:

* master, music, and effects volume
* fullscreen or windowed mode, resolution, and vertical sync
* quality level (Low, Medium, or High)
* ghost on or off
* camera shake on or off
* reduced motion

All settings persist through `PlayerPrefs` and take effect immediately.

### Failure and edge rules

* The marble cannot leave an arena. Walls, floors, and continuous collision
  detection keep it inside at every supported frame rate and at boost speed.
* A pickup can be collected only once. This holds even when the marble touches
  it on two consecutive physics steps, and even during the frame it is
  destroyed.
* Losing window focus pauses the game. Regaining focus leaves it paused until
  the player resumes.
* An instant retry resets the arena, the timer, the combo, and the gold-pickup
  state within one frame.

## Content

| Asset | Source format | What matters on import |
|---|---|---|
| `Title.unity`, `ArenaSelect.unity`, `Arena1.unity`, `Arena2.unity`, `Arena3.unity` | Unity YAML scene | object identity, prefab instances, references, lighting settings |
| `Pickup.prefab`, `Wall.prefab`, `BoostPad.prefab`, `Bumper.prefab`, `JumpPad.prefab`, `Sweeper.prefab` | Unity YAML prefab | nested prefabs, instance overrides of transform, material, and fields |
| `PickupGold.prefab` | prefab variant of `Pickup.prefab` | variant overrides of material, score value, and lifetime |
| Marble, pickups, walls, floors | built-in primitives (Sphere, Cube, Cylinder, Plane) | primitive meshes with stable sub-asset identity |
| Fountain and arena props | FBX models | mesh, material slots, scale factor |
| Floor and wall textures | PNG, sRGB, mipmaps, repeat wrap | sRGB decoding, mip generation, tiling |
| Marble normal map | PNG, normal-map import | normal-map import mode |
| Materials | `.mat` (Standard on Built-in, URP Lit on URP) | colors, textures, metallic and smoothness, emission on pickups and pads |
| Collect burst, landing dust, speed trail | Particle System and Trail Renderer components | emission bursts, color over lifetime, trail width over speed |
| Combo chimes (five pitches), bumper, boost, dash, and results sounds | WAV and OGG | decompress-on-load and compressed-in-memory load types |
| Music, one track per arena | OGG, streaming | streaming load type, seamless looping |
| Fonts | TrueType font with a TextMeshPro SDF font asset | SDF atlas, fallback font |
| Physic materials | `.physicMaterial` | marble grip, bumper bounce, wall bounce |
| `ProjectSettings/` | Unity YAML | tags, layers, physics settings, quality levels, input axes, time step |

Packages: uGUI 1.0.x, TextMeshPro 3.0.x, Input System 1.x (for the input
variant), and URP 14.x (for the URP variant).

Save data: medals, records, and arena unlocks go in `PlayerPrefs`. Each arena's
best-run ghost goes in a binary file under `Application.persistentDataPath`.

## Feature Coverage

| ID | Behavior this game exercises | Scenarios |
|---|---|---|
| PRJ-001 | The project opens in place with no conversion | S1 |
| PRJ-003 | GUIDs and local identifiers of scenes, prefabs, and materials stay stable | S1, S9 |
| PRJ-004 | Textures, normal maps, primitive and FBX meshes, WAV and OGG clips, fonts | S1, S8 |
| PRJ-007 | Physics, quality, time, tags and layers, input axes | S1, S6 |
| SER-001, SER-002, SER-003 | Scene loads, nested prefabs, a prefab variant, serialized field values | S1, S2, S9 |
| RUN-001 | Activation, `Destroy`, `Instantiate` of the gold pickup, transforms | S2, S4, S13 |
| RUN-002 | Ordinary `MonoBehaviour` scripts compile from `Assets/` | S1 |
| RUN-003 | `Awake`, `OnEnable`, `Start`, `Update`, `FixedUpdate`, `OnTriggerEnter`, `OnCollisionEnter`, `OnApplicationFocus` | S2, S5, S12 |
| RUN-004 | `Time.deltaTime`, `Time.timeScale` for pause and slow motion, `WaitForSecondsRealtime`, coroutines for the countdown and the gold-pickup timer | S3, S13, S14 |
| RUN-005 | Single-scene loading for title, arena select, arenas, and retry | S4 |
| RUN-009 | `Application.persistentDataPath` for ghost files | S14 |
| PHY-001 | Rigidbody, sphere and box colliders, triggers, kinematic platforms and sweepers, physic materials, impulses, continuous collision detection | S2, S3, S11, S15 |
| INP-001, INP-002 | Legacy axes and Input System actions, keyboard and gamepad, gamepad rumble | S2, S7 |
| REN-001 | Perspective follow camera with field-of-view kick, resolution and aspect changes | S6 |
| REN-002 | Primitive and FBX meshes, Standard and URP Lit materials, textures, normal maps, a translucent ghost material | S8, S14 |
| REN-003 | A directional light with real-time shadows, ambient light, skybox, quality levels | S6, S8 |
| REN-004 | Collect bursts, landing dust, and the speed trail | S8, S12 |
| REN-007 | Vertical sync, frame pacing, windowed and fullscreen modes | S3, S6 |
| UI-001 | Canvas, buttons, focus, keyboard and gamepad navigation, the combo bar | S2, S7, S12 |
| TXT-001 | TextMeshPro score, multiplier, and results text | S2, S8 |
| AUD-001 | Pitched one-shot effects, streamed music, volume groups, pause behavior | S2, S5, S12 |
| SAV-001 | Settings, medals, unlocks, records, and ghost files persist | S6, S14 |
| BLD-001 | Packaged desktop players with a build report | S10 |
| BLD-002 | Headless runs of every scenario | all |
| TST-002 | Scenarios, checkpoints, approved references, input replay | all |
| EDT-004 | Play, pause, step, and stop with edit-state restoration in the editor | S9 |
| AUT-002 | Agents play, inspect, and capture the game in editor play sessions through the editor's MCP interface | all |
| ACC-001 | Reduced motion, camera-shake toggle, keyboard-only play | S7, S16 |

## Engine Behavior This Game Depends On

### Scripting and runtime

* Scripts in `Assets/Scripts/` compile without changes. Each one attaches to its
  component through its script GUID.
* `OnTriggerEnter` runs once for each pickup contact, after the physics step and
  before `Update`, as it does in Unity. `OnCollisionEnter` reports bumper and
  hazard hits with contact points and impulse.
* `Destroy` takes effect at the end of the frame. A destroyed pickup no longer
  receives trigger messages, and a comparison of a destroyed object to `null`
  returns `true`.
* `Time.timeScale` values of 0 and 0.25 scale `FixedUpdate`, `deltaTime`, and
  coroutines that wait on scaled time. `WaitForSecondsRealtime` advances in real
  time.
* Reloading a scene destroys every object of the previous load and leaves no
  static state, unless the game's scripts keep it.

### Project and content

* The project opens without a converted copy. Reopening an unchanged project
  produces the same identities.
* The prefab variant applies its overrides to every instance of it. Nested
  prefab instances keep their overrides.

### Simulation

* The marble's `AddForce` motion, dash impulses, rolling, friction, and bounce
  follow the project's physics settings and physic materials. Bumper restitution
  is greater than 1 and produces a larger outgoing speed.
* Kinematic platforms and sweepers move the marble through contact, and a
  rotating platform carries the marble around with it.
* Continuous collision detection keeps the marble inside the arenas at 30, 60,
  144, and 240 frames per second, with a fixed time step of 0.02 s or 0.01 s,
  and at boost speed.
* Physics with fixed time steps is deterministic for the same inputs on the
  same target. As a result, replaying a recorded input sequence reproduces the
  run's pickup order and time.

### Rendering and animation

* The arenas render lit, with real-time shadows from one directional light on
  both pipelines. Each pipeline matches its own approved references.
* Emissive pickups and pads, the normal-mapped marble, tiled textures, particle
  bursts, the speed trail, and the translucent ghost are visible at every
  quality level. Shadow resolution and distance follow the quality level.
* The follow camera keeps the marble in view without jitter at every frame rate.
  It interpolates the marble between physics steps.

### Audio

* The collect chime starts within one frame of collection, at a pitch that
  depends on the combo step.
* Music streams and loops without a gap.
* Volume settings change the mixer group levels. Pausing silences gameplay
  sounds while menu sounds keep playing.

### Input

* Keyboard and gamepad produce the same movement and dash. Connecting or
  disconnecting a gamepad during play does not interrupt keyboard input.
* Rumble runs on gamepads that support it and is silently skipped on others.
* Input stops when the window loses focus and resumes cleanly when focus
  returns, with no stuck keys.

### UI and text

* Every menu works with keyboard, gamepad, and pointer, and the focus is always
  visible.
* Text is crisp at every supported resolution. The score, multiplier, and combo
  bar update on the frame of collection.

### Player and build

* The packaged player starts on the title screen, keeps settings and progress
  between launches, and exits cleanly from Quit and from the window's close
  button.
* The build report lists the included scenes, assets, sizes, and the build
  identifier.

### Editor and automation

* The game plays in editor play mode. Stopping play restores every edited scene
  value.
* An agent can do all of the following through the editor's MCP interface: open the project, start
  play, send input, read score, multiplier, and timer state, capture frames, and
  stop play.

## Presentation and Performance

* **Visual checkpoints**, each with a unitas-approved reference for each
  pipeline:
  * the title screen with the attract ghost
  * each arena at its countdown
  * Arena 1 at x5 multiplier with the speed trail visible
  * the results panel with a gold medal
* **Audio checkpoints:** the collect chime occurs within one frame of the
  collection event. Its pitch rises one semitone per combo step in the captured
  audio stream.
* **Project-declared expectations:** 60 frames per second or more at 1080p on
  the declared reference desktop class, start to the title screen in under 3
  seconds, arena load in under 1 second, and instant retry in under 100 ms.
  These are the sample's own expectations, not the PRF-002 engine budgets.

## Acceptance Scenarios

Every scenario runs in editor play mode, in a packaged player, and headless
through the editor's MCP interface, unless it says otherwise.

* **S1. Open and inspect.** Precondition: no import cache exists. Open the
  project.
  * Expected: all assets import with no errors, and the compatibility report
    lists no unsupported item. Reopening yields identical GUIDs and local
    identifiers.
  * (PRJ-001, PRJ-003, PRJ-004, PRJ-007, SER-002)
* **S2. Happy path.** From the title screen, enter Arena 1 and drive a recorded
  input path that collects all twelve pickups.
  * Expected: the pickup count reads 1 to 12 in order, and each collection plays
    a chime. The results panel appears after the twelfth pickup.
  * (RUN-003, PHY-001, INP-002, UI-001, TXT-001, AUD-001)
* **S3. Frame-rate independence.** Repeat S2 at 30, 60, 144, and 240 frames per
  second, with the same input timing.
  * Expected: every pickup is collected exactly once, the marble never leaves
    the arena, and the final time differs by no more than one fixed step.
  * (PHY-001, RUN-004, REN-007)
* **S4. Retry without leftover state.** Collect five pickups at x3, then press
  instant retry.
  * Expected: within one frame, the score is 0, the multiplier is x1, all twelve
    pickups exist, no gold pickup exists, the timer reads 0, and no object from
    the previous attempt remains.
  * (RUN-001, RUN-005)
* **S5. Focus loss and recovery.** During play, remove and then restore window
  focus.
  * Expected: the game pauses, gameplay audio stops, no input is stuck
    afterwards, and play resumes only after Resume.
  * (RUN-003, AUD-001, INP-002)
* **S6. Settings and persistence.** Change every setting, earn a medal, quit,
  and relaunch.
  * Expected: every setting, the medal, the unlocks, and the records are
    restored. A resolution or quality change applies immediately and keeps the
    marble in view.
  * (SAV-001, REN-001, REN-003, REN-007, PRJ-007)
* **S7. Input modes.** Complete S2 once with keyboard only and once with gamepad
  only, and use the menus with the pointer. Repeat with the legacy-input
  variant.
  * Expected: the same outcome every time.
  * (INP-001, INP-002, UI-001, ACC-001)
* **S8. Visual checkpoints.** Capture every visual checkpoint on both pipelines
  at every quality level.
  * Expected: each capture matches its approved reference within that
    reference's tolerance.
  * (REN-002, REN-003, REN-004, TXT-001, TST-002)
* **S9. Edit and restore.** In the editor, change a pickup's score value in the
  Inspector, enter play mode, move a bumper during play, and stop play.
  * Expected: the Inspector change persists, the bumper returns to its edited
    position, and the saved scene stays readable by Unity 2022.3 LTS.
  * (EDT-004, SER-001, PRJ-003)
* **S10. Package.** Build players for each desktop target.
  * Expected: each player launches and completes S2 through replayed input. Its
    build report lists scenes, assets, sizes, and the build identifier.
  * (BLD-001)
* **S11. Collision edge.** Dash the marble from a boost pad into a wall corner,
  then into a bumper.
  * Expected: no tunneling, and the bounces follow each physic material. The
    bumper produces an outgoing speed greater than the incoming speed.
  * (PHY-001)
* **S12. Combo multiplier.** Collect five pickups, each within 1.5 seconds of
  the previous one. Then wait 2 seconds and collect a sixth.
  * Expected: the multiplier reads x1, x2, x3, x4, x5, then x1. The awarded
    points are 1, 2, 3, 4, 5, and 1. The chime pitch rises one semitone per
    step. The combo bar empties exactly 1.5 seconds after the fifth pickup.
  * (RUN-003, UI-001, AUD-001, REN-004)
* **S13. Gold pickup window.** Reach x3 in Arena 3, then reach the gold pickup
  within 4 seconds. In a second run, let it expire.
  * Expected: the gold pickup appears at its fixed spot on reaching x3 and
    awards 5 times the multiplier, refilling the combo bar. If not collected, it
    disappears after exactly 4 seconds of scaled time. It never counts toward
    the twelve pickups.
  * (RUN-001, RUN-004)
* **S14. Medals, records, and ghost.** Finish Arena 1 in under 30 seconds through
  replayed input, then start Arena 1 again.
  * Expected: a gold medal and a new record are awarded and saved, and Arena 2
    unlocks. The ghost file is written. On the next attempt, a translucent ghost
    follows the recorded route, matching the recorded positions within 1 cm at
    every fixed step.
  * (SAV-001, RUN-009, REN-002, PHY-001)
* **S15. Dash and cooldown.** Dash, then press Dash again at 1.0 and 2.1 seconds.
  * Expected: the first dash applies an 8 m/s impulse. The press at 1.0 seconds
    is ignored and the press at 2.1 seconds dashes. The cooldown ring matches
    the remaining time.
  * (PHY-001, INP-002)
* **S16. Reduced motion.** Enable reduced motion and turn camera shake off, then
  hit a bumper and boost.
  * Expected: no camera shake, no field-of-view kick, and no slow-motion finish.
    The gameplay outcome is the same as without the options.
  * (ACC-001)

## Manual Playtest Guidance

A person judges the following, and a headless run reports these checkpoints as
awaiting human review, never as passed:

* Whether rolling feels weighty but responsive.
* Whether the first ramp launch in Arena 1 is exciting.
* Whether chaining combos is satisfying.
* Whether the gold pickup is a tempting but fair risk.
* Whether each arena feels new.
* Whether a near-miss of a medal makes the player want to retry.
* Whether the audio mix keeps the chimes audible over the music.

## Agent Authoring Change Requests

The reference change request, as defined under the Agent Authoring Benchmark in
the [suite index](INDEX.md), adds a new pickup type. The pickup awards a
different score value, has a distinct material and collection sound, appears at
specified positions in an arena, and shows a score-feedback message in the UI
when collected. The request includes a test that checks the awarded score.

Status of the reference change request: the benchmark lane runs the change
request against a copy of this sample and demonstrates that the project
compiles with no errors, that the test the request adds and the sample's
existing tests pass, that the request's acceptance scenario passes in editor
play mode, that the scenarios that passed at the starting revision still pass,
that the change set holds only the changes the request allows, and that
rejecting the change set returns the copy to its starting content. The run does
not demonstrate that the acceptance scenario passes in a player on a declared
target, because it builds no player, and it does not demonstrate that the
changed project opens in Unity 2022.3 LTS, because that editor is not present
in the environment the run executes in. The engine exposes no command that
creates or imports a binary asset, and the benchmark records that absence as a
parity failure of AUT-001 naming the capability.

A second change request adds a magnet power-up. For 5 seconds, it pulls pickups
within 3 m toward the marble. The request includes a scenario proving that
pulled pickups are still collected exactly once.

## Accessibility

Every action can be performed with the keyboard. Menus expose accessibility
names. Text meets contrast requirements against its background. Reduced motion
stops pickup rotation, camera shake, field-of-view kicks, and the slow-motion
finish. An optional assist mode extends the combo window to 2.5 seconds and is
marked on records set with it.

## Saved and Reproducible States

* Arena 1 with eleven pickups collected at x4, with the marble beside the last
  one.
* Arena 3 at x3, with the gold pickup just spawned.
* Arena 2, paused, with the settings panel open.
* A save with all three arenas unlocked and Arena 1 at a gold medal.

## Known Limitations and Unsupported Conditions

None declared. Any behavior this document requires that is not demonstrated is
reported with its real compatibility state.

## Acceptance Outcomes

* The Roll-a-Ball-style sample collects every pickup exactly once at different
  frame rates, keeps the ball inside its boundaries, and completes the same
  scenario with keyboard and gamepad in editor and player. (SMP-001, PHY-001,
  INP-002, UI-001)
* The same scenarios pass on the Built-in Render Pipeline and URP 14.x
  variants, and each matches its approved references. (SMP-001, REN-002,
  REN-003, REN-004, TST-002)
* The combo multiplier, the gold pickup window, and the dash cooldown follow
  their stated timings. (SMP-001, RUN-003, RUN-004, PHY-001)
* Medals, records, unlocks, and best-run ghosts persist across launches, and a
  ghost replays its recorded route within 1 cm. (SMP-001, SAV-001, PHY-001)
* Retrying leaves no transient state from the previous attempt, and settings
  persist across launches. (SMP-001, RUN-005, SAV-001)
* Each desktop player is packaged with a build report and completes the
  replayed happy path. (SMP-001, BLD-001)
