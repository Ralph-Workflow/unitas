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
one of the demonstrations of the loop from opening a Unity project to playing a
packaged build; that loop's acceptance is owned by the domain specifications it
cites.

Compatibility tiers follow the [Feature catalog](../feature-catalog.md). This
document owns this game's acceptance under SMP-001.

## Targets and Input Modes

### Targets

* **Automated:** macOS, Windows, and Linux desktop players, and the Web target
  ([Web target](../web-target.md)) in current
  Chrome, Edge, Firefox, and Safari on desktop and mobile.
* **Capability-reported:** none.
* **Editor play mode:** yes.
* **Headless editor:** yes.

The Web build meets the engine web budgets that the
[Web target](../web-target.md#engine-web-budgets) section
declares; this document does not restate them.

### Render pipelines

The project ships in two variants that are otherwise identical, one on the
Built-in Render Pipeline and one on URP 14.x (Forward). Both variants are
accepted.

### Input modes and devices

Keyboard, mouse (menus only), and gamepad. The movement code exists in two
variants: legacy `Input` axes (INP-001) and an Input System 1.x action map
(INP-002). Both variants are accepted.

Headless runs use synthetic devices marked synthetic; haptic and feel
checkpoints need physical hardware and are otherwise unevaluable.

## Game Design

### Core loop and flow

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
6. Quit on the title screen, or the window's close button, exits the player.

### Rules and mechanics

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
* **Bumpers.** The bumper's script sets the marble's outgoing speed to 1.4 times
  its incoming speed along the contact normal, and triggers a small camera
  shake. The bumper's physic material has a bounciness within Unity's range; the
  speed-up comes from the script.
* **Jump pads.** These launch the marble on a fixed arc to a raised platform.
* **Hazards.** Rotating sweeper arms (Arena 2) and moving blocks (Arena 3) are
  kinematic bodies that knock the marble around. Falling off a raised platform
  drops the marble to the lower floor of the same arena. The marble keeps its
  pickups but loses its combo. The marble never leaves the arena.

### Levels and progression

| Arena | Layout | What it teaches | Medal times (bronze, silver, gold) |
|---|---|---|---|
| 1. Courtyard | flat floor, one ramp, two boost pads, a ring of pickups around a fountain | momentum, combos, the ramp launch in the first 10 seconds | 60 s, 40 s, 30 s |
| 2. Clockwork | rotating platforms, four bumpers, two sweeper arms, a lower floor | timing, using bumpers as shortcuts, recovering from knock-backs | 75 s, 55 s, 40 s |
| 3. Sky Garden | narrow raised paths, jump pads, moving blocks, a gold pickup above a gap | precision, dash control, the gold-pickup gamble | 90 s, 65 s, 50 s |

A medal is earned when the arena's finishing time is at or below its threshold.

### Scoring, failure, and edge rules

* Each required pickup is worth 1 point times the multiplier; the gold pickup is
  worth 5 points times the multiplier.
* The marble cannot leave an arena. Walls, floors, and continuous collision
  detection keep it inside at every supported frame rate and at boost speed.
* A pickup can be collected only once. This holds even when the marble touches
  it on two consecutive physics steps, and even during the frame it is
  destroyed.
* An instant retry resets the arena, the timer, the combo, and the gold-pickup
  state within one frame.

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

### Pause, focus, and lifecycle

* Pause sets `Time.timeScale` to zero. It shows a pause panel with Resume,
  Retry, Settings, and Quit to arena select. Pickups, platforms, and the combo
  bar freeze. Resume restores the previous time scale.
* While paused, the game's scripts pause the gameplay mixer group and keep the
  menu group playing.
* Losing window focus pauses the game. Regaining focus leaves it paused until
  the player resumes.

### Settings

* master, music, and effects volume
* fullscreen or windowed mode, resolution, and vertical sync
* quality level (Low, Medium, or High)
* ghost on or off
* camera shake on or off
* reduced motion
* assist mode (combo window 2.5 seconds; records set with it are marked)

All settings persist through `PlayerPrefs` and take effect immediately.

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
* **Variety and escalation.** Each arena adds one new kind of element and
  combines it with the ones before.
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
  * Bronze, silver, and gold medals per arena, at the thresholds in
    [Levels and progression](#levels-and-progression).
  * Best score and best time records.
  * A translucent ghost of the player's best run in every attempt, which can be
    turned off.
  * Unlocking the next arena.

## Content

### Assets

| Asset | Source format | Import settings that matter |
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
| Physic materials | `.physicMaterial` | marble grip, bumper and wall bounce within Unity's range |
| `ProjectSettings/` | Unity YAML | tags, layers, physics settings, quality levels, input axes, time step |

### Packages

uGUI 1.0.x, TextMeshPro 3.0.x, Input System 1.x (for the input variant), and URP
14.x (for the URP variant), at the version lines CMP-002 lists. Each package
resolves to unitas's own implementation of that version line
([Packages](../architecture.md#packages)); no Unity-published package code is
vendored in the sample repository, fetched, or compiled.

### Save data

Medals, records, and arena unlocks go in `PlayerPrefs`. Each arena's best-run
ghost goes in a versioned binary file under `Application.persistentDataPath`.

### Licences and provenance

Every asset family's source, author, and licence is recorded in
`Licenses/content-licenses.json`. Unity packages' licence terms are recorded in
the build's licence report.

## Feature Coverage

| ID | Behavior this game exercises | Scenarios |
|---|---|---|
| PRJ-001 | The project opens in place with no conversion | S1 |
| PRJ-003 | GUIDs and local identifiers of scenes, prefabs, and materials stay stable | S1, S9 |
| PRJ-004 | Textures, normal maps, primitive and FBX meshes, WAV and OGG clips, fonts | S1, S8 |
| PRJ-007 | Physics, quality, time, tags and layers, input axes | S1, S6 |
| SER-001 | Serialized field values | S1, S9 |
| SER-002 | Scene loads | S1, S2 |
| SER-003 | Nested prefabs and a prefab variant | S1, S2 |
| RUN-001 | Activation, `Destroy`, `Instantiate` of the gold pickup, transforms | S2, S4, S13 |
| RUN-002 | Ordinary `MonoBehaviour` scripts compile from `Assets/` | S1 |
| RUN-003 | `Awake`, `OnEnable`, `Start`, `Update`, `FixedUpdate`, `OnTriggerEnter`, `OnCollisionEnter`, `OnApplicationFocus` | S2, S5, S12 |
| RUN-004 | `Time.deltaTime`, `Time.timeScale` for pause and slow motion, `WaitForSecondsRealtime`, coroutines for the countdown and the gold-pickup timer | S3, S13, S14 |
| RUN-005 | Single-scene loading for title, arena select, arenas, and retry | S4 |
| RUN-009 | `Application.persistentDataPath` for ghost files | S14 |
| PHY-001 | Rigidbody, sphere and box colliders, triggers, kinematic platforms and sweepers, physic materials, impulses, continuous collision detection | S2, S3, S11, S15 |
| INP-001 | Legacy axes, keyboard and gamepad | S7 |
| INP-002 | Input System actions, keyboard and gamepad, gamepad rumble | S2, S7 |
| REN-001 | Perspective follow camera with field-of-view kick, resolution and aspect changes | S6 |
| REN-002 | Primitive and FBX meshes, Standard and URP Lit materials, textures, normal maps, a translucent ghost material | S8, S14 |
| REN-003 | A directional light with real-time shadows, ambient light, skybox, quality levels | S6, S8 |
| REN-004 | Collect bursts, landing dust, and the speed trail | S8, S12 |
| REN-007 | Vertical sync, frame pacing, windowed and fullscreen modes | S3, S6 |
| UI-001 | Canvas, buttons, focus, keyboard and gamepad navigation, the combo bar | S2, S7, S12 |
| TXT-001 | TextMeshPro score, multiplier, and results text | S2, S8 |
| AUD-001 | Pitched one-shot effects, streamed music, mixer groups | S2, S5, S12 |
| SAV-001 | Settings, medals, unlocks, records, and ghost files persist | S6, S14 |
| BLD-001 | Packaged desktop and Web players with a build report | S10 |
| BLD-002 | Headless runs of every scenario that lists the headless editor | S1–S9, S11–S16 |
| TST-002 | Scenarios, checkpoints, approved references, input replay | all |
| EDT-004 | Play, pause, step, and stop with edit-state restoration in the editor | S9 |
| AUT-002 | Agents play, inspect, and capture the game in editor play sessions through the editor's MCP interface | S1–S9, S11–S16 |
| ACC-001 | Reduced motion, camera-shake toggle, keyboard-only play | S7, S16 |

## Engine Behavior This Game Depends On

Only engine and package behavior, each tied to a catalog ID. Game rules are
defined under Game Design and implemented by the sample's own scripts; the
engine contains no code specific to this game. Package behavior comes from
unitas's own implementation of each package's public API at the pinned version
line ([Packages](../architecture.md#packages), CMP-002). Agents reach the game
only through the editor's generic MCP object, property, method-invocation,
input, and capture commands, with no game-specific command.

### Scripting and runtime

* Scripts in `Assets/Scripts/` compile without changes. Each one attaches to its
  component through its script GUID. (RUN-002)
* `OnTriggerEnter` runs once for each pickup contact, after the physics step and
  before `Update`, as it does in Unity. `OnCollisionEnter` reports bumper and
  hazard hits with contact points, normal, and impulse. (RUN-003, PHY-001)
* `Destroy` takes effect at the end of the frame. A destroyed pickup no longer
  receives trigger messages, and a comparison of a destroyed object to `null`
  returns `true`. (RUN-001)
* `Time.timeScale` values of 0 and 0.25 scale `FixedUpdate`, `deltaTime`, and
  coroutines that wait on scaled time. `WaitForSecondsRealtime` advances in real
  time. (RUN-004)
* Reloading a scene destroys every object of the previous load and leaves no
  static state, unless the game's scripts keep it. (RUN-005)

### Project and content

* The project opens without a converted copy. Reopening an unchanged project
  produces the same identities. (PRJ-001, PRJ-003)
* The prefab variant applies its overrides to every instance of it. Nested
  prefab instances keep their overrides. (SER-003)

### Simulation

* `AddForce`, impulses, rolling, friction, and bounce follow the project's
  physics settings and physic materials. Setting `Rigidbody.velocity` from
  `OnCollisionEnter` takes effect on the next physics step. (PHY-001)
* Kinematic bodies moved with `MovePosition` and `MoveRotation` push dynamic
  bodies through contact, and a rotating kinematic platform carries a resting
  body around with it. (PHY-001)
* Continuous collision detection keeps a fast sphere inside closed geometry at
  every frame rate in [S3](#s3-frame-rate-independence), with a fixed time step
  of 0.02 s or 0.01 s. (PHY-001)
* Physics with fixed time steps produces the same results for the same inputs
  on the same target, so replayed input reproduces a run's pickup order and
  time. (PHY-001, TST-002)

### Rendering and animation

* Lit scenes render with real-time shadows from one directional light on both
  pipelines, and each pipeline matches its own approved references.
  (REN-002, REN-003)
* Emissive materials, normal maps, tiled textures, particle bursts, trail
  renderers, and transparent materials are visible at every quality level.
  Shadow resolution and distance follow the quality level. (REN-002, REN-003,
  REN-004)
* Rigidbody interpolation presents the body between physics steps, so a camera
  following it in `LateUpdate` sees a frame-to-frame screen-space offset of the
  body from the camera's target point of at most 2 pixels at 1080p during
  steady rolling. (PHY-001, REN-001)

### Audio and video

* A one-shot started from a script plays within one frame of the call, at the
  pitch the script sets. (AUD-001)
* A streamed OGG clip loops without a gap. (AUD-001)
* Mixer group volumes and `AudioMixerGroup` pause states set from script take
  effect on the next audio buffer. (AUD-001)

### Input

* Keyboard and gamepad bindings produce the same action values. Connecting or
  disconnecting a gamepad during play does not interrupt keyboard input.
  (INP-001, INP-002)
* Rumble runs on gamepads that support it and is silently skipped on others.
  (INP-002)
* Input stops when the window loses focus and resumes cleanly when focus
  returns, with no stuck keys. (INP-001, INP-002)

### UI and text

* uGUI navigation works with keyboard, gamepad, and pointer, and the selected
  element's visual state is always shown. (UI-001)
* TextMeshPro text renders from its SDF atlas at every supported resolution and
  matches the approved text checkpoints. Text set during a frame renders that
  frame. (TXT-001)

### Player and build

* The packaged player starts on the first scene in the build, keeps
  `PlayerPrefs` and persistent files between launches, and exits cleanly from
  `Application.Quit` and from the window's close button. (BLD-001, SAV-001)
* The build report lists the included scenes, assets, sizes, and the build
  identifier. (BLD-001)

### Editor and automation

* The game plays in editor play mode. Stopping play restores every edited scene
  value. (EDT-004)
* Through the editor's generic MCP commands, an agent opens the project, starts
  play, sends input, reads any component field (such as the score script's
  fields), captures frames, and stops play. (AUT-002)

## Presentation and Performance

### Visual checkpoints

Each has a unitas-approved reference for each pipeline:

* the title screen with the attract ghost
* each arena at its countdown
* Arena 1 at x5 multiplier with the speed trail visible
* the results panel with a gold medal

### Audio checkpoints

The collect chime occurs within one frame of the collection event. Its pitch
rises one semitone per combo step in the captured audio stream.

### Project-declared expectations

On the declared reference desktop class: 60 frames per second or more at 1080p,
start to the title screen in under 3 seconds, arena load in under 1 second, and
instant retry in under 100 ms. Expectations are evaluated only on the declared
class; other runs record the measurement and report the expectation as
unevaluable (TST-002). These are the sample's own expectations, not the PRF-002
engine budgets.

### PRF-002 measurements

None.

## Acceptance Scenarios

Outcomes use the TST-002 vocabulary: passed, failed, skipped, unevaluable, and
awaiting human review. Each perturbation names its mechanism: a TST-002
replay-stream event, a harness action outside the player, or a
development-build debug command implemented in the sample's scripts.

### S1. Open and inspect

* **Runs in:** headless editor; editor play mode.
* **Preconditions:** no import cache exists.
* **Steps:** open the project; reopen it.
* **Expected:** all assets import with no errors, and the compatibility report
  lists no unsupported item. Reopening yields identical GUIDs and local
  identifiers.
* **Demonstrates:** PRJ-001, PRJ-003, PRJ-004, PRJ-007, SER-002

### S2. Happy path

* **Runs in:** editor play mode; headless editor; packaged players on every
  automated target.
* **Preconditions:** a fresh save.
* **Steps:** from the title screen, enter Arena 1 and drive a recorded input
  path that collects all twelve pickups.
* **Expected:** the pickup count reads 1 to 12 in order, and each collection
  plays a chime. The results panel appears after the twelfth pickup.
* **Demonstrates:** RUN-003, PHY-001, INP-002, UI-001, TXT-001, AUD-001

### S3. Frame-rate independence

* **Runs in:** headless editor; desktop packaged players. The Web target runs
  it at rates up to the display refresh.
* **Preconditions:** a fresh save.
* **Steps:** repeat S2 at 30, 60, 144, and 240 frames per second (desktop and
  headless) with the same input timing.
* **Expected:** every pickup is collected exactly once, the marble never leaves
  the arena, and the final time differs by no more than one fixed step.
* **Demonstrates:** PHY-001, RUN-004, REN-007

### S4. Retry without leftover state

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** Arena 1 started.
* **Steps:** collect five pickups at x3, then press instant retry.
* **Expected:** within one frame, the score is 0, the multiplier is x1, all
  twelve pickups exist, no gold pickup exists, the timer reads 0, and no object
  from the previous attempt remains.
* **Demonstrates:** RUN-001, RUN-005

### S5. Focus loss and recovery

* **Runs in:** editor play mode; headless editor; desktop packaged players.
* **Preconditions:** Arena 1 in play.
* **Steps:** remove and then restore window focus, through a focus event in the
  replay stream (headless and editor) or a harness action that changes the OS
  window focus (packaged players).
* **Expected:** the game pauses, gameplay audio stops, no input is stuck
  afterwards, and play resumes only after Resume.
* **Demonstrates:** RUN-003, AUD-001, INP-002

### S6. Settings and persistence

* **Runs in:** packaged players on every automated target; headless editor
  with a play-session restart.
* **Preconditions:** a fresh save.
* **Steps:** change every setting, earn a medal, then quit and relaunch: a
  harness action that ends and restarts the player process, or, headless, ends
  the play session and starts a new one with the same storage root.
* **Expected:** every setting, the medal, the unlocks, and the records are
  restored. A resolution or quality change applies immediately and keeps the
  marble in view.
* **Demonstrates:** SAV-001, REN-001, REN-003, REN-007, PRJ-007

### S7. Input modes

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** a fresh save; both input variants built.
* **Steps:** complete S2 once with keyboard only and once with gamepad only, and
  use the menus with the pointer. Repeat with the legacy-input variant.
* **Expected:** the same outcome every time.
* **Demonstrates:** INP-001, INP-002, UI-001, ACC-001

### S8. Visual checkpoints

* **Runs in:** headless editor; packaged players on every automated target.
* **Preconditions:** each saved state needed by a checkpoint.
* **Steps:** capture every visual checkpoint on both pipelines at every quality
  level.
* **Expected:** each capture matches its approved reference within that
  reference's tolerance.
* **Demonstrates:** REN-002, REN-003, REN-004, TXT-001, TST-002

### S9. Edit and restore

* **Runs in:** editor play mode, visual and headless.
* **Preconditions:** Arena 1 open in the editor.
* **Steps:** change a pickup's score value in the Inspector, enter play mode,
  move a bumper during play, and stop play.
* **Expected:** the Inspector change persists and the bumper returns to its
  edited position. Where Unity 2022.3 LTS is available, the saved scene opens
  in it unchanged; otherwise that check is unevaluable.
* **Demonstrates:** EDT-004, SER-001, PRJ-003

### S10. Package

* **Runs in:** headless editor (build) and packaged players on every automated
  target.
* **Preconditions:** none.
* **Steps:** build players for each desktop target and for the Web target.
* **Expected:** each player launches and completes S2 through replayed input.
  Its build report lists scenes, assets, sizes, and the build identifier.
* **Demonstrates:** BLD-001

### S11. Collision edge

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** Arena 2 started.
* **Steps:** dash the marble from a boost pad into a wall corner, then into a
  bumper.
* **Expected:** no tunneling, and wall bounces follow each physic material. The
  bumper produces an outgoing speed 1.4 times the incoming speed along the
  contact normal.
* **Demonstrates:** PHY-001

### S12. Combo multiplier

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** Arena 1 started.
* **Steps:** collect five pickups, each within 1.5 seconds of the previous one.
  Then wait 2 seconds and collect a sixth.
* **Expected:** the multiplier reads x1, x2, x3, x4, x5, then x1. The awarded
  points are 1, 2, 3, 4, 5, and 1. The chime pitch rises one semitone per step.
  The combo bar empties exactly 1.5 seconds after the fifth pickup.
* **Demonstrates:** RUN-003, UI-001, AUD-001, REN-004

### S13. Gold pickup window

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** the saved state "Arena 3 at x3".
* **Steps:** reach the gold pickup within 4 seconds. In a second run, let it
  expire.
* **Expected:** the gold pickup appears at its fixed spot on reaching x3 and
  awards 5 times the multiplier, refilling the combo bar. If not collected, it
  disappears after exactly 4 seconds of scaled time. It never counts toward the
  twelve pickups.
* **Demonstrates:** RUN-001, RUN-004

### S14. Medals, records, and ghost

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** a fresh save.
* **Steps:** finish Arena 1 in under 30 seconds through replayed input, then
  start Arena 1 again.
* **Expected:** a gold medal and a new record are awarded and saved, and Arena 2
  unlocks. The ghost file is written. On the next attempt, a translucent ghost
  follows the recorded route, matching the recorded positions within 1 cm at
  every fixed step.
* **Demonstrates:** SAV-001, RUN-009, REN-002, PHY-001

### S15. Dash and cooldown

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** Arena 1 in play.
* **Steps:** dash, then press Dash again at 1.0 and 2.1 seconds.
* **Expected:** the first dash applies an 8 m/s impulse. The press at 1.0
  seconds is ignored and the press at 2.1 seconds dashes. The cooldown ring
  matches the remaining time.
* **Demonstrates:** PHY-001, INP-002

### S16. Reduced motion

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** a fresh save.
* **Steps:** enable reduced motion and turn camera shake off, then hit a bumper
  and boost.
* **Expected:** no camera shake, no field-of-view kick, and no slow-motion
  finish. The gameplay outcome is the same as without the options.
* **Demonstrates:** ACC-001

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

### CR1. New pickup type

* **Request:** add a new pickup type. It awards a different score value, has a
  distinct material and collection sound, appears at specified positions in an
  arena, and shows a score-feedback message in the UI when collected.
* **Acceptance scenario:** in Arena 1 with the new pickups placed, collecting
  one awards its score value times the multiplier, plays its sound, and shows
  the feedback message; all existing scenarios still pass.
* **Test:** a test that checks the awarded score.

### CR2. Magnet power-up

* **Request:** add a magnet power-up. For 5 seconds, it pulls pickups within 3 m
  toward the marble.
* **Acceptance scenario:** with the magnet active, pulled pickups are still
  collected exactly once, and pickups farther than 3 m do not move.
* **Test:** a test that counts collection events per pickup while the magnet is
  active.

## Accessibility

* Every action can be performed with the keyboard (S7).
* Menus expose accessibility names (S7).
* Text has a contrast ratio of at least 4.5:1 against its background (S8).
* Reduced motion stops pickup rotation, camera shake, field-of-view kicks, and
  the slow-motion finish (S16).
* The assist mode setting extends the combo window to 2.5 seconds and marks
  records set with it (S12 with assist mode on).

## Saved and Reproducible States

| State | How it is reached | Used by |
|---|---|---|
| Arena 1 with eleven pickups collected at x4, the marble beside the last one | save file | S8 |
| Arena 3 at x3, with the gold pickup just spawned | save file | S13 |
| Arena 2, paused, with the settings panel open | save file | S8 |
| All three arenas unlocked and Arena 1 at a gold medal | save file | S8, S14 |

## Known Limitations and Unsupported Conditions

None. Any behavior this document requires that is not demonstrated is reported
with its real compatibility state.

## Acceptance Outcomes

* The game collects every pickup exactly once at different frame rates, keeps
  the marble inside its boundaries, and completes the same scenario with
  keyboard and gamepad in editor and player. (SMP-001, PHY-001, INP-002, UI-001)
* The same scenarios pass on the Built-in Render Pipeline and URP 14.x
  variants, and each matches its approved references. (SMP-001, REN-002,
  REN-003, REN-004, TST-002)
* The combo multiplier, the gold pickup window, and the dash cooldown follow
  their stated timings. (SMP-001, RUN-003, RUN-004, PHY-001)
* Medals, records, unlocks, and best-run ghosts persist across launches, and a
  ghost replays its recorded route within 1 cm. (SMP-001, SAV-001, PHY-001)
* Retrying leaves no transient state from the previous attempt, and settings
  persist across launches. (SMP-001, RUN-005, SAV-001)
* Each desktop player and the Web target player is packaged with a build report
  and completes the replayed happy path. (SMP-001, BLD-001)
