# Sample 6: Kart Racer ("Harbor Karts")

## Overview

One to four players race go-karts over three laps against AI karts. Drifting
charges boosts, item boxes hand out turbos and traps, and each track hides one
shortcut. The results screen ranks the field and offers a replay of the last
lap. Time Trial races the player against a ghost of their best lap for medals.
A race takes three to four minutes.

This is an original, redistributable unitas sample. Its karts, tracks, music,
and sounds are made for unitas and are distinct from any official Unity sample
or commercial racing game. The first track, Harbor Loop, ships in the player.
The second, Quarry Switchback, arrives after release as an Addressables content
update. The game is one of the demonstrations of post-release content
delivery; that capability's acceptance is owned by BLD-005 in
[Builds, platforms, XR, and networking](../build-platforms-xr-networking.md).

Compatibility tiers follow the [Feature catalog](../feature-catalog.md). This
document owns the acceptance of this game under SMP-001.

## Targets and Input Modes

### Targets

* **Automated:** macOS, Windows, and Linux desktop players.
* **Capability-reported:** steering wheels, gamepad rumble, and multiple
  physical gamepads.
* **Editor play mode:** yes.
* **Headless editor:** yes.

### Render pipelines

URP 14.x, Forward+ rendering path, HDR on, MSAA 4x.

### Input modes and devices

Keyboard (one player), one gamepad per player (up to four), and
steering-wheel-class joysticks that expose a steering axis, through an Input
System 1.x action asset with Keyboard, Gamepad, and Wheel control schemes.
Menus also accept the pointer.

Headless runs use synthetic devices of the same layouts, marked synthetic;
haptic and feel checkpoints need physical hardware and are otherwise
unevaluable.

## Game Design

### Core loop and flow

1. The title screen shows Race, Time Trial, Settings, and Quit.
2. Race opens the join screen. A player joins by pressing South on a gamepad,
   Enter on the keyboard, or a wheel's first button, up to four players. A
   joined device belongs to that player until the race ends.
3. The track screen lists Harbor Loop and Quarry Switchback. Quarry
   Switchback shows its download size when the update is available, "Not
   installed" when it is not, and "Unavailable offline" when the content
   server is unreachable.
4. An 8-second Timeline intro flies over the track and grid. Any Confirm skips
   it. A 3-second light countdown follows, and karts cannot move before "GO".
5. Six karts race three laps: the human players plus AI karts. A lap counts
   only after all 12 checkpoints are passed in order.
6. When every human finishes, or 30 seconds after the first human finishes,
   the results list positions, total times, and best laps. Unfinished karts
   rank by distance covered.
7. Results offer Replay Last Lap, Restart, and Back to Menu. Restart reloads
   the track with every transient value reset. Back to Menu releases the
   track content.
8. Quit on the title exits the player.

Time Trial is single-player with no AI or items. A translucent ghost replays
the best saved lap, and a new best lap replaces the ghost file.

### Rules and mechanics

* Each kart is a 180 kg Rigidbody with interpolation, continuous dynamic
  collision, and four WheelColliders: radius 0.25 m, suspension distance
  0.15 m, spring 35,000, damper 4,500, target position 0.5. Forward friction
  has extremum slip 0.4 at 1.0 and asymptote slip 0.8 at 0.6, stiffness 1.5.
  Sideways friction has extremum slip 0.25 at 1.0 and asymptote slip 0.5 at
  0.75, stiffness 1.6. Vehicle substeps are `ConfigureVehicleSubsteps(5, 12,
  15)`, and the fixed time step is 0.01 s.
* Top speed is 28 m/s. Holding Drift while steering above 10 m/s lowers rear
  sideways stiffness to 0.7. Asphalt, dirt, and grass have separate physic
  materials; grass halves forward stiffness.
* **Rocket start.** Throttle pressed within 0.2 s after "GO" gives 32 m/s for
  0.8 s. Throttle held before "GO" stalls the kart for 0.5 s.
* **Drift tiers.** A drift charges three boost tiers, shown by spark color and
  a numeral: blue (1) at 0.8 s (32 m/s for 0.6 s), orange (2) at 1.6 s (34 m/s
  for 1.0 s), and purple (3) at 2.4 s (36 m/s for 1.4 s). Releasing before
  0.8 s gives nothing. Touching a wall cancels the drift charge.
* **Items.** Item boxes sit in rows of four at two points per lap and respawn
  5 seconds after pickup. A box gives one item: Turbo (36 m/s for 1.5 s), Oil
  Slick (dropped behind; a kart crossing it has rear sideways stiffness 0.4 for
  1.0 s), or Bumper (absorbs the next oil slick or kart hit). Roll weights
  (Turbo / Oil Slick / Bumper) by race position: 1st 20 / 40 / 40; 2nd and 3rd
  35 / 35 / 30; 4th to 6th 60 / 20 / 20. Rolls come from a race seed set with
  `Random.InitState`, so a seeded race repeats its item rolls.
* **AI.** AI karts follow a closed waypoint racing line (64 points on Harbor
  Loop, 80 on Quarry Switchback) with a per-kart lateral offset. Easy, Normal,
  and Hard cap AI speed at 85%, 92%, and 98% of human top speed. An AI kart
  slower than 1 m/s for 3 seconds respawns at its last waypoint with zero
  velocity.
* **Engine audio.** The kart script sets engine pitch to 0.6 plus 1.4 times RPM
  over maximum RPM, from 0.6 to 2.0. Other karts' engines are 3D with Doppler
  level 1.0. One AudioListener follows player 1's camera.
* **Cameras.** Each player has Chase (5 m behind, 2 m up) and Orbit virtual
  cameras on that player's layer. Change Camera blends them in 0.4 seconds. The
  replay uses a clear-shot camera over eight trackside cameras.
* **Viewports.** Full screen for one player, halves for two, and quadrants for
  three or four; with three, the fourth quadrant shows a track overview. Each
  player's rear-view camera renders to a 512 by 128 render texture shown in
  that player's HUD.
* Rear wheels emit 8-second skid trails while sideways slip exceeds 0.3, and
  tire smoke at 0 to 60 particles per second in proportion to slip.

### Levels and progression

| Track | Delivery | Shortcut | New elements | Time Trial medals (Bronze / Silver / Gold) |
|---|---|---|---|---|
| Harbor Loop | in the player | grass cut, about 2 s saved | wide first hairpin for a first drift | 60 / 56 / 53 s |
| Quarry Switchback | content update | tighter grass cut | a jump, elevation | 64 / 60 / 57 s |

A medal is earned by a lap at or under its threshold. A staff ghost ships with
each track. Gold on both tracks unlocks the Copper kart paint.

### Scoring, failure, and edge rules

* No kart passes through a barrier, a fence, or another kart at any speed or
  supported frame rate. A kart below the kill height respawns at its last
  checkpoint within 1 second.
* Driving backward through a checkpoint shows "Wrong way" after 1 second and
  never advances the lap count.
* A disconnected device pauses the race with "Reconnect controller for Player
  N". Reconnecting the same device resumes control.
* A damaged ghost file shows "Ghost file damaged", is kept as
  `<track>.ghost.corrupt`, and a new best lap writes a valid file.

### Controls

| Action | Keyboard | Gamepad | Wheel | Mouse |
|---|---|---|---|---|
| Steer | A and D, or Left and Right | Left stick X | Steering axis | — |
| Throttle | W or Up | Right trigger | Throttle pedal | — |
| Brake and reverse | S or Down | Left trigger | Brake pedal | — |
| Drift (hold) | Left Shift or Space | Right shoulder | Button 3 | — |
| Use item | E | West button | Button 4 | — |
| Change camera | C | North button | Button 5 | — |
| Pause | Escape or P | Start | Button 6 | — |
| Menus: navigate, confirm, back | Arrows and Tab, Enter, Escape | D-pad, South, East | Buttons 1 and 2 | Pointer, left click |

Keyboard steering ramps to full lock in 0.15 seconds. The wheel's steering
axis maps its reported range to -1 to 1, with a 2% dead zone at the center.

### Pause, focus, and lifecycle

Pause sets `Time.timeScale` to zero, shows Resume, Restart, Settings, and Quit
to Menu, and moves the mixer to the Paused snapshot (Engines and Effects at
-30 dB, Music low-passed at 800 Hz). Replay moves it to the Replay snapshot
(Engines -6 dB). Each snapshot transition takes 0.3 seconds. Only the pausing
player's device controls the panel. Losing window focus pauses the race until a
player resumes.

### Settings

Master, music, and effects volume, display mode, resolution, vertical sync,
frame-rate cap (60, 120, 144, 240, uncapped), quality (Low, Medium, High),
motion blur, rear-view mirror, HUD text size (100% or 150%), AI difficulty,
rumble, reduced motion, automatic throttle, and per-player rebinding. They
persist through `PlayerPrefs` and apply immediately.

### What makes it fun

* **Hook.** In the first 30 seconds the player gets a rocket start and a first
  drift boost on Harbor Loop's wide first hairpin.
* **Mastery curve.** Chaining drifts and taking the shortcut separate a
  62-second lap from a 52-second lap on Harbor Loop.
* **Risk and reward.** Wall contact cancels a drift charge. Each shortcut
  crosses grass that only a boosted kart clears without losing speed. A held
  item can be kept as protection or used now for speed.
* **Variety and escalation.** The final lap raises music tempo by 8% and shows
  "Final lap". Position-weighted item rolls keep races close. Quarry
  Switchback adds a jump, elevation, and a tighter shortcut.
* **Game feel.** Boosts widen the camera field of view from 60 to 70 degrees
  and emit flame particles. Each tier change plays a rising chime. Kart and
  wall hits rumble the gamepad (low 0.4, high 0.7, for 0.15 s) where motors
  exist. A rocket start shows "Rocket start!".
* **Replay.** Time Trial medals, staff ghosts, the Copper paint unlock, and
  persisted best laps.

## Content

### Assets

| Asset | Source format | Import settings that matter |
|---|---|---|
| `Title.unity`, `HarborLoop.unity` | Unity YAML scene | prefab instances, lighting and reflection probes baked in the unitas editor |
| `QuarrySwitchback.unity` | Unity YAML scene in a remote Addressables group | address, label, scene dependencies |
| `Kart.prefab` with `KartAI` and `KartGhost` variants | Unity YAML prefab and variants | WheelCollider and friction values, variant overrides |
| Karts and trackside props | FBX, Read/Write off | LOD meshes `_LOD0` to `_LOD2`, mesh compression |
| Track surfaces and barriers | FBX with mesh colliders | non-convex cooking, physic material assignment |
| Textures | PNG, sRGB, mipmaps, BC7 on desktop | normal maps for kart bodies |
| Materials | URP Lit and Unlit `.mat` | GPU instancing on cone and tire-stack materials |
| Reflection probe captures | `.exr` cubemaps baked in the unitas editor | box projection, blend distance |
| Engine loop, tire, impact, chime sounds | WAV 44.1 kHz mono | Decompress On Load, loopable engine clip |
| Music | OGG Vorbis | Streaming load type, normal and final-lap stems |
| `KartMixer.mixer` | Unity YAML audio mixer | groups Music, Engines, Effects, UI; snapshots Race, Paused, Replay |
| `RaceIntro.playable`, `Replay.playable` | Timeline assets | Cinemachine shot, audio, activation, and signal tracks |
| `KartControls.inputactions` | Input action asset | control schemes, processors, generated C# class |
| Staff ghosts | `.ghost` binary files in each track's content | same format as player ghosts |
| Fonts | TrueType with TextMeshPro SDF assets | SDF atlas, fallback font |
| Addressables settings | Addressables 1.x groups and profiles | Local and Remote profiles, content state file |
| `Fixtures/UnityBuilt/` | Quarry Switchback bundles built by Unity 2022.3 LTS for each desktop target | committed fixtures, with the producing Unity release recorded |

### Packages

URP 14.x, Cinemachine 2.x, Timeline 1.x, Input System 1.x, Addressables 1.x,
uGUI 1.0.x, and TextMeshPro 3.0.x, at the version lines CMP-002 lists. Packages
are resolved from their registry when the project opens and are never vendored
in the sample repository.

### Save data

Settings, best laps, medals, unlocks, and per-player binding overrides in
`PlayerPrefs`; ghost files as versioned binary files under
`Application.persistentDataPath`.

### Licences and provenance

Every asset family's source, author, and licence is recorded in
`Licenses/content-licenses.json`. Unity packages' licence terms are recorded in
the build's licence report.

## Feature Coverage

| ID | Behavior this game exercises | Scenarios |
|---|---|---|
| PRJ-001 | Open in place | S1 |
| PRJ-003 | Stable identities | S1 |
| PRJ-004 | FBX LODs, audio | S1 |
| PRJ-007 | Fixed step and layers | S1, S13 |
| SER-001 | Serialized wheel values | S1 |
| SER-002 | Scenes | S1, S2 |
| SER-003 | Kart prefab variants | S1, S2 |
| RUN-001 | Grid instantiation, AI respawn transforms, ghost playback | S2, S10, S15 |
| RUN-002 | Scripts | S1 |
| RUN-003 | Checkpoint triggers | S2 |
| RUN-004 | Countdown, pause | S2, S11 |
| RUN-005 | Scene loads | S2, S8 |
| RUN-006 | Addressables catalogs, update check, download size, handles, release | S8, S9 |
| RUN-009 | Ghost files under `Application.persistentDataPath`, seeded item rolls | S5, S10 |
| PHY-001 | Rigidbodies, continuous collision, mesh-collider barriers, triggers | S4, S15 |
| PHY-004 | WheelColliders, suspension, friction curves, drift tiers, boosts | S3, S4 |
| REN-001 | Split-screen viewports, per-player cameras, rear-view render texture | S6, S14 |
| REN-002 | Meshes, LOD groups | S14 |
| REN-003 | Baked reflection probes, bloom, motion blur | S14 |
| REN-004 | Skid-mark trails, tire smoke, boost flames | S3, S14 |
| REN-006 | URP Forward+, GPU instancing of trackside props | S14 |
| REN-007 | Frame-rate caps, vertical sync, frame pacing to 240 frames per second | S13 |
| CIN-001 | Chase, orbit, and replay cameras; Timeline intro and last-lap replay | S7 |
| AUD-001 | Pitch set from script, Doppler, mixer snapshots for pause and replay | S7 |
| INP-002 | Joining, one device per player, wheel axes, rumble, device loss | S6, S11, S12 |
| UI-001 | Menus, join screen | S2, S6 |
| TXT-001 | Per-player HUD text | S2, S6 |
| SAV-001 | Settings, best laps, medals, ghosts, corrupt-ghost handling | S10 |
| ACC-001 | Keyboard-only play, rebinding, reduced motion, automatic throttle, HUD text size | S16 |
| BLD-001 | Desktop players | S16 |
| BLD-005 | A content update with only changed bundles; Unity-built bundles | S8, S9 |
| BLD-002 | Headless runs | S1–S7, S10–S15 |
| TST-002 | Replayed input, captures | all |
| AUT-002 | Race state read through the editor's MCP interface | S1–S7, S10–S15 |

## Engine Behavior This Game Depends On

Only engine and package behavior, each tied to a catalog ID. Game rules are
defined under Game Design and implemented by the sample's own scripts; the
engine contains no code specific to this game. Package behavior (Cinemachine,
Timeline, Input System, Addressables) comes from the package's own code at the
pinned version line, compiled and run unmodified (CMP-002). Agents reach the
game only through the editor's generic MCP object, property,
method-invocation, input, and capture commands, with no game-specific command.

### Scripting and runtime

* `OnTriggerEnter` is delivered once per body crossing, including when six
  bodies cross one trigger in the same physics step. (RUN-003)
* `Time.timeScale = 0` stops wheel simulation, scaled particles, and coroutines
  waiting on scaled time; unscaled-time animation continues. Reloading a scene
  leaves no object or static engine state behind. (RUN-004, RUN-005)

### Project and content

* `Addressables.CheckForCatalogUpdates`, `UpdateCatalogs`,
  `GetDownloadSizeAsync`, `DownloadDependenciesAsync`, and `LoadSceneAsync`
  return handles with status, progress, and result as the package defines.
  Releasing a handle brings its reference count to zero and unloads its
  bundles, as residency inspection shows. (RUN-006)
* Content built by Unity 2022.3 LTS for this target loads like unitas-built
  content (BLD-005); a bundle built for another target, or whose hash does not
  match its catalog, is reported by name and is never partly loaded. (BLD-005)

### Simulation

* Motor torque, brake torque, and steer angle act on the next simulation step.
  Each wheel reports `isGrounded`, `rpm`, and `GetGroundHit`. Friction follows
  the authored curves, scaled by stiffness and the surface material.
  `ConfigureVehicleSubsteps` applies its thresholds. (PHY-004)
* Wheel results repeat exactly for the same inputs on the same target.
  Continuous collision keeps a 36 m/s body out of 0.2 m fences at 30, 60, 144,
  and 240 frames per second. (PHY-001, PHY-004)

### Rendering and animation

* Cameras with viewport rectangles render split-screen; a camera rendering to a
  RenderTexture shows in a UI `RawImage` on the same frame. (REN-001)
* LOD groups switch at their screen-height thresholds per rendering camera.
  Materials with GPU instancing render many renderers sharing mesh and material
  as instances with per-instance colors from `MaterialPropertyBlock`, producing
  the same image as non-instanced rendering; the instanced draw count is
  reported in the rendering diagnostics. (REN-002, REN-006)
* Baked reflection probes with box projection and a global URP Volume with
  bloom and motion blur render as REN-003 defines. Trail renderers and particle
  emission rates set from script render as REN-004 defines. (REN-003, REN-004)
* Rigidbody interpolation presents the body between physics steps, so a camera
  following it sees a frame-to-frame screen-space offset of the body from the
  camera's target point of at most 2 pixels at 1080p during steady driving.
  (PHY-001, REN-001)

### Audio and video

* `AudioSource.pitch` set from script takes effect within one frame. Doppler
  level 1.0 shifts the pitch of a moving source as AUD-001 defines. Mixer
  snapshot transitions reach their levels in their stated time. (AUD-001)

### Input

* `PlayerInputManager` joins up to its maximum player count, one device each,
  with a split-screen region per player; a joined device cannot join twice.
  (INP-002)
* Device removal and reconnection appear in `InputSystem.onDeviceChange`
  without duplicating or losing an action; a reconnected device is paired to
  its previous user. Rumble returns an explicit status on devices without
  motors. Binding overrides persist per player. (INP-002)

### UI and text

* Per-player canvases render on their player's camera. (UI-001, TXT-001)

### Player and build

* The release player ships Harbor Loop only. The content update build contains
  only Quarry Switchback bundles and a new catalog, and states which player
  versions can load it. (BLD-005)
* A local content server fixture on `http://127.0.0.1:8088` serves remote
  content and can switch to offline, slow, or truncated responses. (BLD-005)

### Editor and automation

* Cinemachine brains filtered by camera layer show only their own player's
  virtual cameras; the live camera is inspectable. (CIN-001)
* Through the editor's generic MCP commands an agent joins synthetic players,
  sends per-player input, reads any component field (positions, laps, wheel
  slip, items, live cameras), and captures each viewport. (AUT-002)

## Presentation and Performance

### Visual checkpoints

Each has a unitas-approved reference: the title, the grid after the intro, a
purple-tier drift with skid marks and smoke, a boost with bloom and motion
blur, four-player split-screen at the first corner, the results screen, and
the Quarry Switchback grid.

### Audio checkpoints

Engine pitch tracks RPM, a Doppler shift on a scripted pass, the three tier
chimes, and each snapshot transition.

### Project-declared expectations

On the reference desktop class at 1080p, High quality: one player at 144 frames
per second or more, four players at 60 or more, and track loads under 2
seconds. Expectations are evaluated only on the declared class; other runs
record the measurement and report the expectation as unevaluable (TST-002).
These are not PRF-002 engine budgets.

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
* **Expected:** every asset imports without errors, each package is reported
  Supported, and reopening yields identical identities.
* **Demonstrates:** PRJ-001, PRJ-003, PRJ-004, PRJ-007, SER-001, SER-002

### S2. Happy path

* **Runs in:** editor play mode; headless editor; desktop packaged players.
* **Preconditions:** one keyboard player, Harbor Loop, Normal AI, race seed 7.
* **Steps:** replay a recorded three-lap input, then choose Restart.
* **Expected:** no kart moves before "GO", laps count 1 to 3, results list six
  karts, and Restart resets every value, leaving no kart, trail, or particle
  from the previous race.
* **Demonstrates:** RUN-001, RUN-003, RUN-004, RUN-005, UI-001, TXT-001

### S3. Rocket start and drift tiers

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** Harbor Loop, one player.
* **Steps:** press throttle 0.1 s after "GO", then drift the hairpin for 0.7,
  0.9, 1.7, and 2.5 seconds on four runs, and on a fifth touch the wall at
  1.0 s.
* **Expected:** the rocket start gives 32 m/s for 0.8 s. The runs give no boost,
  blue, orange, and purple at the stated speeds and durations, each with its
  spark color and chime. The wall cancels the charge.
* **Demonstrates:** PHY-004, REN-004, AUD-001

### S4. High-speed collision

* **Runs in:** headless editor; desktop packaged players.
* **Preconditions:** Harbor Loop, one player.
* **Steps:** at 36 m/s, hit a 0.2 m fence and a stationary kart at 30, 60, 144,
  and 240 frames per second, three times each.
* **Expected:** no tunneling, and wheel results repeat across runs at the same
  frame rate.
* **Demonstrates:** PHY-001, PHY-004

### S5. Items and shortcut

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** race seed 7.
* **Steps:** collect four item boxes in 4th place, use each item, and take the
  shortcut boosted and unboosted.
* **Expected:** the item sequence is the one the 4th-to-6th weights and seed 7
  produce, recorded in the scenario. Oil Slick drops rear stiffness to 0.4 for
  1.0 s, and Bumper absorbs one hit. The boosted shortcut saves at least 1.5 s
  over the racing line; the unboosted one saves none.
* **Demonstrates:** RUN-009, PHY-004

### S6. Four-player split-screen

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** four synthetic gamepads and a fifth.
* **Steps:** join four gamepads, then try the fifth; race; then repeat with
  three players.
* **Expected:** four viewports, each with its own camera, mirror, and HUD, each
  steered only by its device. The fifth gamepad cannot join. With three players
  the fourth quadrant shows the overview.
* **Demonstrates:** INP-002, REN-001, UI-001

### S7. Cameras, intro, replay, and audio

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** Harbor Loop, one player.
* **Steps:** watch the intro, then skip it at 2 seconds on a second run. Finish,
  then replay the last lap while capturing audio, including a scripted AI pass
  at 25 m/s and a pause.
* **Expected:** the countdown starts once in both runs. The replay cuts at
  least four times. Pitch rises with RPM, the pass shifts pitch down as it
  leaves, and each snapshot reaches its levels in 0.3 s.
* **Demonstrates:** CIN-001, AUD-001

### S8. Content update

* **Runs in:** release desktop packaged players.
* **Preconditions:** the release player is installed and the update is on the
  local content server fixture.
* **Steps:** download Quarry Switchback, then race it.
* **Expected:** the update check finds the new catalog, the reported size
  equals the bundle bytes, progress reaches 100%, and the track completes S2's
  flow. The update build lists only the new bundles and the catalog.
* **Demonstrates:** RUN-006, BLD-005, RUN-005

### S9. Content failure and recovery

* **Runs in:** release desktop packaged players.
* **Preconditions:** the release player is installed.
* **Steps:** switch the content server fixture to offline; then cut a download
  at 40% and retry; then serve the committed Unity-built fixture bundles; then
  serve a bundle whose hash does not match its catalog. All are harness actions
  on the server fixture.
* **Expected:** offline shows "Unavailable offline" and Harbor Loop stays
  playable; the cut download completes on retry; the Unity-built bundles load
  and play the same track; the mismatched bundle is reported by name and not
  loaded.
* **Demonstrates:** RUN-006, BLD-005

### S10. Ghosts, medals, and corruption

* **Runs in:** packaged players; headless editor with a play-session restart.
* **Preconditions:** a fresh profile.
* **Steps:** set a 55-second Time Trial lap, relaunch (a harness action that
  restarts the player, or, headless, a new play session with the same storage
  root), and race the ghost. Then truncate the ghost file (a harness action on
  the storage root) and race again. Then earn Gold on both tracks.
* **Expected:** Silver is awarded and persists, and the ghost follows the saved
  lap within 0.05 m. The damaged file shows "Ghost file damaged", is kept as
  `<track>.ghost.corrupt`, and a new best lap writes a valid file. Gold on both
  tracks unlocks the Copper paint.
* **Demonstrates:** SAV-001, RUN-009, RUN-001

### S11. Device loss

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** a two-player race.
* **Steps:** remove and re-add player 2's gamepad, through device-removal and
  device-addition events in the replay stream.
* **Expected:** the race pauses with the prompt, then resumes with player 2 on
  the same device; no action fires twice.
* **Demonstrates:** INP-002, RUN-004

### S12. Wheel and rumble

* **Runs in:** editor play mode; headless editor; packaged players. With
  synthetic devices the rumble feel checkpoint is unevaluable.
* **Preconditions:** a wheel-class joystick and a gamepad with motors, physical
  or synthetic.
* **Steps:** drive one lap with the wheel, then hit a wall with the gamepad.
* **Expected:** Steer follows the axis with the 2% dead zone, pedals drive
  throttle and brake, and a 0.15 s rumble command is issued with low 0.4 and
  high 0.7.
* **Demonstrates:** INP-002

### S13. Frame pacing

* **Runs in:** desktop packaged players on the declared reference desktop
  class; headless editor records the measurements as unevaluable.
* **Preconditions:** Harbor Loop, one player.
* **Steps:** race one lap at each cap with vertical sync off, then at 60 with
  vertical sync on.
* **Expected:** at each cap the median frame interval is within 5% of the cap's
  interval and no more than 1% of frames exceed twice it; the chase camera
  meets the 2-pixel screen-space offset bound; the race result matches S2.
* **Demonstrates:** REN-007, PRJ-007

### S14. Visual checkpoints

* **Runs in:** headless editor; packaged players.
* **Preconditions:** each saved state needed by a checkpoint.
* **Steps:** capture each checkpoint at every quality level, with rendering
  diagnostics.
* **Expected:** each matches its approved reference within tolerance, with LODs,
  instanced props, probes, bloom, motion blur, trails, and the mirror present.
  The diagnostics report the cones and tire stacks drawn as instanced draws,
  not one draw per prop.
* **Demonstrates:** REN-001, REN-002, REN-003, REN-004, REN-006, TST-002

### S15. AI and wrong way

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** Harbor Loop with AI.
* **Steps:** pin an AI kart against a wall, then drive the human kart backward
  through a checkpoint.
* **Expected:** the AI kart respawns at its last waypoint after 3 s; "Wrong way"
  appears after 1 s, and the lap count does not change.
* **Demonstrates:** RUN-001, PHY-001

### S16. Package and accessibility

* **Runs in:** headless editor (build) and desktop packaged players.
* **Preconditions:** none.
* **Steps:** build each desktop player and complete S2 with keyboard only,
  reduced motion, automatic throttle, and 150% HUD text.
* **Expected:** each player finishes; camera shake, field-of-view change, and
  motion blur are off; HUD text is 150% of its default size; the build report
  lists scenes, sizes, and the build identifier.
* **Demonstrates:** BLD-001, ACC-001, INP-002

## Manual Playtest Guidance

A person judges the following, and a headless run reports these checkpoints as
awaiting human review, never as passed:

* Whether a first-time player lands a blue boost within the first lap.
* Whether drift tiers feel earned rather than automatic.
* Whether the shortcut is tempting and fair.
* Whether trailing players stay in contention without leaders feeling robbed.
* Whether boosts feel fast.
* Split-screen readability and replay camera comfort.

## Agent Authoring Change Requests

### CR1. Lap-2 shortcut gate

* **Request:** add a shortcut gate on Harbor Loop that opens on lap 2, with a
  new boost pad that gives 32 m/s for 2.0 seconds and a distinct sound.
* **Acceptance scenario:** the gate is closed on lap 1 and open from lap 2;
  driving through the pad gives 32 m/s for 2.0 s and plays its sound.
* **Test:** a test that drives through the pad and checks the boost speed and
  duration.

### CR2. Magnet item

* **Request:** add a fourth item, Magnet, that pulls the user toward the kart
  ahead for 2 seconds.
* **Acceptance scenario:** using Magnet 20 m behind another kart closes the gap
  by more than both karts' unboosted travel would.
* **Test:** a test that measures the gap before and after the item.

## Accessibility

* Every race and menu action works with the keyboard alone, and controls can be
  rebound per player (S16).
* Reduced motion disables camera shake, the boost field-of-view change, motion
  blur, and the intro's fast moves (S16).
* Automatic throttle holds full throttle unless Brake is pressed (S16).
* HUD text scales to 150% (S16).
* Drift tiers show a numeral (1, 2, 3) beside the spark color (S3).

## Saved and Reproducible States

| State | How it is reached | Used by |
|---|---|---|
| Harbor Loop, lap 3, player 1 in second place, 50 m before the finish | save file | S14 |
| Four-player race paused after the first corner | save file | S14 |
| Time Trial with a saved 55.20-second ghost lap | save file | S10 |
| Release player with the Quarry Switchback update ready to download | release build plus server fixture | S8 |

## Known Limitations and Unsupported Conditions

* Split-screen has one AudioListener on player 1's camera.
* Wheel force feedback is not part of this sample.
* Wheel results are repeatable on one target only and are not promised to
  match Unity's numeric results.

Any behavior that this document requires and that is not demonstrated is
reported with its real compatibility state.

## Acceptance Outcomes

* The kart racer completes a three-lap race against AI karts in editor and
  player, counts every lap once, and keeps every kart out of barriers at 30 to
  240 frames per second. (SMP-001, PHY-001, PHY-004, REN-007)
* The rocket start, three drift-boost tiers, items, and the shortcut give
  their stated speeds and durations, repeatably for the same seed and input.
  (SMP-001, PHY-004, RUN-009, REN-004)
* Two to four players race in split-screen, each with their own device,
  viewport, cameras, mirror, and HUD. (SMP-001, INP-002, REN-001, CIN-001)
* The Timeline intro and last-lap replay play with Cinemachine cameras, and
  the countdown starts once whether or not the intro is skipped. (SMP-001,
  CIN-001)
* Engine pitch follows RPM, passing karts carry Doppler shift, and the pause
  and replay snapshots reach their levels in 0.3 seconds. (SMP-001, AUD-001)
* A released player downloads Quarry Switchback from an update holding only
  changed bundles and a new catalog, loads the committed Unity-built bundles,
  and reports offline, cut, or mismatched content without breaking Harbor Loop.
  (SMP-001, RUN-006, BLD-005)
* Ghosts, medals, and unlocks survive relaunch, and a corrupt ghost is kept
  aside and reported. (SMP-001, SAV-001, RUN-009)
* LOD groups, instanced props, baked probes, bloom, and motion blur match
  their approved references at every quality level. (SMP-001, REN-002,
  REN-003, REN-006, TST-002)
* Each desktop player is packaged with a build report and completes the
  replayed race. (SMP-001, BLD-001)
