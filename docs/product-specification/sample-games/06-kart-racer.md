# Sample 6: Kart Racer

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
update, which makes this game the suite's proof of post-release content
delivery.

Compatibility tiers follow the [Feature catalog](../feature-catalog.md). This
document owns the acceptance of this game under SMP-001.

## Targets and Input Modes

* Targets: macOS, Windows, and Linux desktop players, plus the editor's play
  mode and the headless editor.
* Render pipeline: URP 14.x, Forward+ rendering path, HDR on, MSAA 4x.
* Input modes: keyboard (one player), one gamepad per player (up to four),
  and steering-wheel-class joysticks that expose a steering axis, through an
  Input System 1.x action asset with Keyboard, Gamepad, and Wheel control
  schemes. Menus also accept the pointer.
* Gamepad, rumble, and wheel checkpoints need the physical device. Without it
  they are reported as not evaluable, never as passed. Headless runs use
  synthetic devices of the same layouts, marked synthetic in the evidence.

## Game Design

### Core loop

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

Time Trial is single-player with no AI or items. A translucent ghost replays
the best saved lap, and a new best lap replaces the ghost file.

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

### Handling, items, and AI

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
* Item boxes sit in rows of four at two points per lap and respawn 5 seconds
  after pickup. A box gives one item: Turbo (36 m/s for 1.5 s), Oil Slick
  (dropped behind; a kart crossing it has rear sideways stiffness 0.4 for
  1.0 s), or Bumper (absorbs the next oil slick or kart hit). Rolls are
  weighted by race position, and they come from a race seed set with
  `Random.InitState`, so a seeded race repeats its item rolls.
* AI karts follow a closed waypoint racing line (64 points on Harbor Loop, 80
  on Quarry Switchback) with a per-kart lateral offset. Easy, Normal, and Hard
  cap AI speed at 85%, 92%, and 98% of human top speed. An AI kart slower than
  1 m/s for 3 seconds respawns at its last waypoint with zero velocity.

### What makes it fun

* **Hook.** In the first 30 seconds the player gets a rocket start and a first
  drift boost. Throttle pressed within 0.2 s after "GO" gives 32 m/s for 0.8 s.
  Throttle held before "GO" stalls the kart for 0.5 s. Harbor Loop's first
  corner is a wide hairpin built for a first drift.
* **Mastery curve.** A drift charges three boost tiers, shown by spark color:
  blue at 0.8 s (32 m/s for 0.6 s), orange at 1.6 s (34 m/s for 1.0 s), and
  purple at 2.4 s (36 m/s for 1.4 s). Releasing before 0.8 s gives nothing.
  Chaining drifts and taking the shortcut separate a 62-second lap from a
  52-second lap on Harbor Loop.
* **Risk and reward.** Touching a wall cancels the drift charge. Each track's
  shortcut saves about 2 seconds but crosses grass that only a boosted kart
  clears without losing speed. A held item can be kept as protection or used
  now for speed.
* **Variety and escalation.** The final lap raises music tempo by 8% and
  shows "Final lap". Trailing karts roll Turbo more often, so races stay close.
  Quarry Switchback adds a jump, elevation, and a tighter shortcut.
* **Juice.** Boosts widen the camera field of view from 60 to 70 degrees and
  emit flame particles. Each tier change plays a rising chime. Kart and wall
  hits rumble the gamepad (low 0.4, high 0.7, for 0.15 s) where motors exist.
  A rocket start shows "Rocket start!".
* **Replay reasons.** Time Trial awards Bronze, Silver, and Gold for lap times
  under 60, 56, and 53 seconds on Harbor Loop (64, 60, and 57 on Quarry
  Switchback). A staff ghost ships with each track. Gold on both tracks
  unlocks the Copper kart paint. Best laps and medals persist.

### Pause and settings

Pause sets `Time.timeScale` to zero, shows Resume, Restart, Settings, and Quit
to Menu, and moves the mixer to the Paused snapshot. Only the pausing player's
device controls the panel. Settings are master, music, and effects volume,
display mode, resolution, vertical sync, frame-rate cap (60, 120, 144, 240,
uncapped), quality (Low, Medium, High), motion blur, rear-view mirror, AI
difficulty, rumble, reduced motion, automatic throttle, and per-player
rebinding. They persist through `PlayerPrefs` and apply immediately.

### Failure and edge rules

* No kart passes through a barrier, a fence, or another kart at any speed or
  supported frame rate. A kart below the kill height respawns at its last
  checkpoint within 1 second.
* Driving backward through a checkpoint shows "Wrong way" after 1 second and
  never advances the lap count.
* A disconnected device pauses the race with "Reconnect controller for Player
  N". Reconnecting the same device resumes control. Losing window focus
  pauses the race until a player resumes.

## Content

| Asset | Source format | What matters on import |
|---|---|---|
| `Title.unity`, `HarborLoop.unity` | Unity YAML scene | prefab instances, Unity-baked lighting and reflection probes |
| `QuarrySwitchback.unity` | Unity YAML scene in a remote Addressables group | address, label, scene dependencies |
| `Kart.prefab` with `KartAI` and `KartGhost` variants | Unity YAML prefab and variants | WheelCollider and friction values, variant overrides |
| Karts and trackside props | FBX, Read/Write off | LOD meshes `_LOD0` to `_LOD2`, mesh compression |
| Track surfaces and barriers | FBX with mesh colliders | non-convex cooking, physic material assignment |
| Textures | PNG, sRGB, mipmaps, BC7 on desktop | normal maps for kart bodies |
| Materials | URP Lit and Unlit `.mat` | GPU instancing on cone and tire-stack materials |
| Reflection probe captures | Unity-baked `.exr` cubemaps | box projection, blend distance |
| Engine loop, tire, impact, chime sounds | WAV 44.1 kHz mono | Decompress On Load, loopable engine clip |
| Music | OGG Vorbis | Streaming load type, normal and final-lap stems |
| `KartMixer.mixer` | Unity YAML audio mixer | groups Music, Engines, Effects, UI; snapshots Race, Paused, Replay |
| `RaceIntro.playable`, `Replay.playable` | Timeline assets | Cinemachine shot, audio, activation, and signal tracks |
| `KartControls.inputactions` | Input action asset | control schemes, processors, generated C# class |
| Staff ghosts | `.ghost` binary files in each track's content | same format as player ghosts |
| Fonts | TrueType with TextMeshPro SDF assets | SDF atlas, fallback font |
| Addressables settings | Addressables 1.x groups and profiles | Local and Remote profiles, content state file |

Packages: URP 14.x, Cinemachine 2.x, Timeline 1.x, Input System 1.x,
Addressables 1.x, uGUI 1.0.x, and TextMeshPro 3.0.x.

## Feature Coverage

| ID | Behavior this game exercises | Scenarios |
|---|---|---|
| PRJ-001, PRJ-003, PRJ-004, PRJ-007 | Open in place, stable identities, FBX LODs, audio, fixed step and layers | S1, S8 |
| SER-001, SER-002, SER-003 | Scenes, kart prefab variants, serialized wheel values | S1, S2 |
| RUN-001 | Grid instantiation, AI respawn transforms, ghost playback | S2, S10, S15 |
| RUN-002, RUN-003, RUN-004, RUN-005 | Scripts, checkpoint triggers, countdown, pause, scene loads | S2, S11 |
| RUN-006 | Addressables catalogs, update check, download size, handles, release | S8, S9 |
| RUN-009 | Ghost files under `Application.persistentDataPath`, seeded item rolls | S5, S10 |
| PHY-001 | Rigidbodies, continuous collision, mesh-collider barriers, triggers | S4, S15 |
| PHY-004 | WheelColliders, suspension, friction curves, drift tiers, boosts | S3, S4 |
| REN-001 | Split-screen viewports, per-player cameras, rear-view render texture | S6, S14 |
| REN-002, REN-003 | Meshes, LOD groups, baked reflection probes, bloom, motion blur | S14 |
| REN-004 | Skid-mark trails, tire smoke, boost flames | S3, S14 |
| REN-006 | URP Forward+, GPU instancing of trackside props | S14 |
| REN-007 | Frame-rate caps, vertical sync, frame pacing to 240 frames per second | S13 |
| CIN-001 | Chase, orbit, and replay cameras; Timeline intro and last-lap replay | S7 |
| AUD-001 | Engine pitch by RPM, Doppler, mixer snapshots for pause and replay | S7 |
| INP-002 | Joining, one device per player, wheel axes, rumble, device loss | S6, S11, S12 |
| UI-001, TXT-001 | Menus, join screen, per-player HUD text | S2, S6 |
| SAV-001 | Settings, best laps, medals, ghosts, corrupt-ghost handling | S10 |
| ACC-001 | Keyboard-only play, rebinding, reduced motion, automatic throttle | S16 |
| BLD-001, BLD-005 | Desktop players; a content update with only changed bundles | S8, S16 |
| BLD-002, TST-002, AUT-002 | Headless runs, replayed input, captures, race state read through the editor's MCP interface | all |

## Engine Behavior This Game Depends On

### Scripting and runtime

* Checkpoint triggers deliver `OnTriggerEnter` once per kart crossing, so lap
  counts are exact when six karts cross in the same physics step.
* `Time.timeScale = 0` stops wheel simulation, AI, scaled particles, and the
  countdown coroutine; the pause menu animates on unscaled time. Restart
  leaves no kart, trail, particle, or static race state behind.

### Project and content

* `Addressables.CheckForCatalogUpdates`, `UpdateCatalogs`,
  `GetDownloadSizeAsync`, `DownloadDependenciesAsync`, and `LoadSceneAsync`
  return handles with status, progress, and result as the package defines.
  Releasing the track handle brings its reference count to zero and unloads
  its bundles, as residency inspection shows.
* Content built by Unity, or for another player version, is reported per
  bundle with its reason and is never partly loaded.

### Simulation

* Motor torque, brake torque, and steer angle act on the next simulation step.
  Each wheel reports `isGrounded`, `rpm`, and `GetGroundHit`. Friction follows
  the authored curves, scaled by stiffness and the surface material.
* Wheel results repeat exactly for the same inputs on the same target, so
  replayed races and ghost laps reproduce there. Continuous collision keeps a
  36 m/s kart out of 0.2 m fences at 30, 60, 144, and 240 frames per second.

### Rendering and animation

* Viewports are full screen for one player, halves for two, and quadrants for
  three or four; with three, the fourth quadrant shows a track overview.
* Each player's rear-view Camera renders to a 512 by 128 render texture shown
  in that player's HUD on the same frame.
* LOD groups switch props at 40%, 15%, and 4% screen height per viewport
  camera. 600 cones and 1,200 tire stacks render instanced with per-instance
  colors from `MaterialPropertyBlock`, producing the same image as
  non-instanced rendering.
* Three Unity-baked reflection probes light kart paint. A global URP Volume
  applies bloom (threshold 1.0, intensity 0.6) and motion blur (0.25).
* Rear wheels emit 8-second skid trails while sideways slip exceeds 0.3, and
  tire smoke at 0 to 60 particles per second in proportion to slip.

### Cinematics and cameras

* Each player has Chase (5 m behind, 2 m up) and Orbit virtual cameras on that
  player's layer, so each `CinemachineBrain` shows only its own player's
  cameras. Change Camera blends them in 0.4 seconds.
* The intro's final signal starts the countdown once, including when the
  intro is skipped by seeking to its end. The replay uses a clear-shot camera
  over eight trackside cameras, and the live camera is inspectable.

### Audio

* Engine pitch is 0.6 plus 1.4 times RPM over maximum RPM, from 0.6 to 2.0,
  and follows RPM within one frame. Other karts' engines are 3D with Doppler
  level 1.0. One AudioListener follows player 1's camera.
* Paused lowers Engines and Effects to -30 dB and low-passes Music at 800 Hz.
  Replay lowers Engines by 6 dB. Each transition takes 0.3 seconds.

### Input

* `PlayerInputManager` joins up to four players, one device each, with a
  split-screen region per player. A joined device cannot join twice.
* Device removal and reconnection appear in `InputSystem.onDeviceChange`
  without duplicating or losing an action. Rumble returns an explicit status
  on devices without motors. Binding overrides persist per player.

### Player, build, and automation

* The release player ships Harbor Loop only. The content update build
  contains only Quarry Switchback bundles and a new catalog, and states which
  player versions can load it.
* A local content server fixture on `http://127.0.0.1:8088` serves remote
  content and can switch to offline, slow, or truncated responses.
* Agents join synthetic players, send per-player input, read positions, laps,
  wheel slip, items, and live cameras, and capture each viewport through the editor's MCP interface.

## Presentation and Performance

* Visual checkpoints, each with a unitas-approved reference: the title, the
  grid after the intro, a purple-tier drift with skid marks and smoke, a boost
  with bloom and motion blur, four-player split-screen at the first corner,
  the results screen, and the Quarry Switchback grid.
* Audio checkpoints: engine pitch tracks RPM, a Doppler shift on a scripted
  pass, the three tier chimes, and each snapshot transition.
* Project-declared expectations on the reference desktop class at 1080p, High
  quality: one player at 144 frames per second or more, four players at 60 or
  more, and track loads under 2 seconds. These are not PRF-002 engine budgets.

## Acceptance Scenarios

Every scenario runs in editor play mode, in a packaged player, and headless
through the editor's MCP interface, unless it says otherwise.

* **S1. Open and inspect.** Precondition: no cache. Open the project.
  Expected: every asset imports without errors, each package is reported
  Supported, and reopening yields identical identities. (PRJ-001, PRJ-003,
  PRJ-004, PRJ-007, SER-001, SER-002)
* **S2. Happy path.** One keyboard player, Harbor Loop, Normal AI, race seed
  7. Replay a recorded three-lap input. Expected: no kart moves before "GO",
  laps count 1 to 3, results list six karts, and Restart resets every value.
  (RUN-001, RUN-003, RUN-004, RUN-005, UI-001, TXT-001)
* **S3. Rocket start and drift tiers.** Press throttle 0.1 s after "GO", then
  drift the hairpin for 0.7, 0.9, 1.7, and 2.5 seconds on four runs, and on a
  fifth touch the wall at 1.0 s. Expected: the rocket start gives 32 m/s for
  0.8 s. The runs give no boost, blue, orange, and purple at the stated speeds
  and durations, each with its spark color and chime. The wall cancels the
  charge. (PHY-004, REN-004, AUD-001)
* **S4. High-speed collision.** At 36 m/s, hit a 0.2 m fence and a stationary
  kart at 30, 60, 144, and 240 frames per second. Expected: no tunneling, and
  wheel results repeat across runs at the same frame rate. (PHY-001, PHY-004)
* **S5. Items and shortcut.** With race seed 7, collect four item boxes,
  use each item, and take the shortcut boosted and unboosted. Expected: the
  item sequence matches the seed's recorded sequence. Oil Slick drops rear
  stiffness to 0.4 for 1.0 s, and Bumper absorbs one hit. The boosted
  shortcut saves at least 1.5 s over the racing line; the unboosted one saves
  none. (RUN-009, PHY-004)
* **S6. Four-player split-screen.** Join four synthetic gamepads. Expected:
  four viewports, each with its own camera, mirror, and HUD, each steered only
  by its device. A fifth gamepad cannot join. With three players the fourth
  quadrant shows the overview. (INP-002, REN-001, UI-001)
* **S7. Cameras, intro, replay, and audio.** Watch the intro, then skip it at
  2 seconds. Finish, then replay the last lap while capturing audio, including
  a scripted AI pass at 25 m/s and a pause. Expected: the countdown starts
  once in both runs. The replay cuts at least four times. Pitch rises with RPM,
  the pass shifts pitch down as it leaves, and each snapshot reaches its
  levels in 0.3 s. (CIN-001, AUD-001)
* **S8. Content update.** Precondition: the release player is installed and
  the update is on the server. Download Quarry Switchback. Expected: the
  update check finds the new catalog, the reported size equals the bundle
  bytes, progress reaches 100%, and the track completes S2's flow. The update
  build lists only the new bundles and the catalog. (RUN-006, BLD-005,
  RUN-005)
* **S9. Content failure and recovery.** Make the server offline, cut a
  download at 40%, then serve a Unity-built bundle. Expected: offline shows
  "Unavailable offline" and Harbor Loop stays playable, the cut download
  completes on retry, and the Unity-built bundle loads and plays the same
  track. A bundle whose hash does not match its catalog is reported by name and
  not loaded. (RUN-006, BLD-005)
* **S10. Ghosts, medals, and corruption.** Set a 55-second Time Trial lap,
  relaunch, and race the ghost. Then corrupt the ghost file. Expected: Silver
  is awarded and persists, and the ghost follows the saved lap within 0.05 m.
  The corrupt file shows "Ghost file damaged", is kept as
  `<track>.ghost.corrupt`, and a new best lap writes a valid file. Gold on
  both tracks unlocks the Copper paint. (SAV-001, RUN-009, RUN-001)
* **S11. Device loss.** In a two-player race, disconnect and reconnect player
  2's gamepad. Expected: the race pauses with the prompt, then resumes with
  player 2 on the same device; no action fires twice. (INP-002, RUN-004)
* **S12. Wheel and rumble.** Precondition: a wheel-class joystick and a
  gamepad with motors, physical or synthetic. Drive one lap with the wheel,
  then hit a wall with the gamepad. Expected: Steer follows the axis with the
  2% dead zone, pedals drive throttle and brake, and rumble runs for 0.15 s.
  Physical-device checkpoints without the hardware are not evaluable.
  (INP-002)
* **S13. Frame pacing.** Race one lap at each cap with vertical sync off, then
  at 60 with vertical sync on. Expected: the achieved rate meets each cap the
  hardware allows, the chase camera shows no kart jitter, and the race result
  matches S2. (REN-007, PRJ-007)
* **S14. Visual checkpoints.** Capture each checkpoint at every quality
  level. Expected: each matches its approved reference within tolerance, with
  LODs, instanced props, probes, bloom, motion blur, trails, and the mirror
  present. Instancing is a reported performance property. (REN-001, REN-002,
  REN-003, REN-004, REN-006, TST-002)
* **S15. AI and wrong way.** Pin an AI kart against a wall, then drive the
  human kart backward through a checkpoint. Expected: the AI kart respawns at
  its last waypoint after 3 s; "Wrong way" appears after 1 s, and the lap count
  does not change. (RUN-001, PHY-001)
* **S16. Package and accessibility.** Build each desktop player and complete
  S2 with keyboard only, reduced motion, and automatic throttle. Expected:
  each player finishes; camera shake, field-of-view change, and motion blur
  are off; the build report lists scenes, sizes, and the build identifier.
  (BLD-001, ACC-001, INP-002)

## Manual Playtest Guidance

A person judges whether a first-time player lands a blue boost within the
first lap, whether drift tiers feel earned rather than automatic, whether the
shortcut is tempting and fair, whether trailing players stay in contention
without leaders feeling robbed, and whether boosts feel fast. They also judge
split-screen readability and replay camera comfort. A headless run reports
these checkpoints as awaiting human review, never as passed.

## Agent Authoring Change Requests

Add a shortcut gate on Harbor Loop that opens on lap 2, with a new boost pad
that gives 32 m/s for 2.0 seconds and a distinct sound. Include a test that
drives through the pad and checks the boost speed and duration.

A second change request adds a fourth item, Magnet, that pulls the user
toward the kart ahead for 2 seconds, with a scenario proving the gap closes.

## Accessibility

Every race and menu action works with the keyboard alone, and controls can be
rebound per player. Reduced motion disables camera shake, the boost
field-of-view change, motion blur, and the intro's fast moves. Automatic
throttle holds full throttle unless Brake is pressed. HUD text scales to
150%. Drift tiers show a numeral (1, 2, 3) beside the spark color.

## Saved and Reproducible States

* Harbor Loop, lap 3, player 1 in second place, 50 m before the finish.
* Four-player race paused after the first corner.
* Time Trial with a saved 55.20-second ghost lap.
* Release player with the Quarry Switchback update ready to download.

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
  changed bundles and a new catalog, and offline, cut, or Unity-built content
  is reported without breaking Harbor Loop. (SMP-001, RUN-006, BLD-005)
* Ghosts, medals, and unlocks survive relaunch, and a corrupt ghost is kept
  aside and reported. (SMP-001, SAV-001, RUN-009)
* LOD groups, instanced props, baked probes, bloom, and motion blur match
  their approved references at every quality level. (SMP-001, REN-002,
  REN-003, REN-006, TST-002)
* Each desktop player is packaged with a build report and completes the
  replayed race. (SMP-001, BLD-001)
