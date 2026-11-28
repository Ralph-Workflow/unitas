# Sample 12: XR Interaction Room ("The Tinker's Bench")

## Overview

The Tinker's Bench is a room-scale escape puzzle for XR headsets. The player
stands in a cluttered clockmaker's workshop, 5 m by 4 m, with a broken music
box on the bench and a locked door. A chain of five tactile puzzles repairs
the music box. Its melody unlocks the door. The room is also full of physics
toys to pick up, throw, stack, and play. A first solve takes 6 to 12 minutes,
and a practiced solve takes under 4.

On declared XR targets, the player sees correctly scaled stereo content, uses
tracked input, selects and grabs objects, interacts with world-space UI,
experiences spatial audio and, on devices that expose them, haptics, changes
comfort options, and recovers from tracking or focus interruption. The sample
distinguishes device capability differences and never reports a feature as
passing on hardware that does not expose it.

This is an original, redistributable unitas sample. Its room, puzzles, models,
melody, and sounds are original unitas content, and it implies no vendor SDK,
store, or platform service. It is one of the demonstrations of XR sessions,
tracked interaction, and comfort; the acceptance of those features is owned by
XR-001 in [Builds, platforms, XR, and networking](../build-platforms-xr-networking.md).

Compatibility tiers follow the [Feature catalog](../feature-catalog.md). This
document owns this game's acceptance under SMP-001.

## Targets and Input Modes

### Targets

* **Automated:** an OpenXR simulation runtime, meaning an OpenXR 1.0 runtime
  that simulates a headset and two tracked controllers without hardware. It is
  the active OpenXR runtime on Windows x86-64 and Linux x86-64 test hosts.
  Scenario steps drive its head and controller poses, buttons, and session
  state from recorded tracks. Every result records the runtime's name and
  version.
* **Capability-reported:** Windows x86-64 and Linux x86-64 desktop players with
  an OpenXR runtime and a physical headset. Each session writes a capability
  report. A feature passes on a headset only when that report shows the
  headset exposes it.
* **Editor play mode:** yes, on the simulation runtime.
* **Headless editor:** yes, on the simulation runtime.

### Render pipelines

URP 14.x, Forward, with 4x MSAA. The render views come from the runtime's
preferred view configuration and are drawn as the Stereo rendering section of
[Rendering and animation](../rendering-and-animation.md) defines; the stereo
mode in effect is reported.

### Input modes and devices

Tracked controllers; tracked hands where the runtime exposes hand tracking; and
the host keyboard for menus, recentering, and pause. A desktop window mirrors
the left eye.

Headless runs use the simulation runtime's synthetic devices, marked
synthetic; haptic and feel checkpoints need physical hardware and are
otherwise unevaluable.

## Game Design

### Core loop and flow

1. The Boot scene checks XR. If XR is unavailable or fails to start, the
   desktop window says which ("No XR runtime found" or "XR failed to start:
   <reason>"), logs the two cases distinctly, and offers Retry and Quit.
2. The player appears at the bench facing a world-space start panel: Play,
   Free Play (after the first solve), Settings, Records, and Quit.
3. Play starts the timer. The music box plays a broken, stuttering melody. A
   chalkboard reads "Fix me."
4. The player solves the puzzle chain, using hints if needed.
5. The repaired box plays the full melody, and the door unlocks. Walking or
   teleporting through the door ends the run. A results panel shows time,
   hints used, cogs found, the medal, and any new record. Its buttons are
   Restart, Free Play, and Title.
6. Quit, or the runtime's exit request, exits the player.

### Rules and mechanics

* **Toys.**
  * A xylophone with eight bars and two mallets. Pitch depends on the bar.
    Volume depends on impact speed, from silent below 0.2 m/s to full at 3 m/s.
  * Six paper balls and a waste-bin hoop 2 m from the bench. A chalkboard
    counts the current streak of baskets.
  * A Newton's cradle of five steel balls on hinge joints.
  * Twelve stackable wooden blocks.
  * A desk bell that rings on a downward tap.
* **Grabbing.** Grabbed objects follow their interactor with velocity tracking
  and keep the release velocity on throw. Only one interactor holds an object
  at a time, except the box lid during the two-handed step.
* **Sockets.** A socket accepts only its matching part. A wrong part shows a
  red ghost and does not snap.
* **Teleport.** Teleporting onto furniture, walls, or outside the room is
  refused, with a red reticle and no move.
* **Bounds.** A part that leaves the room bounds returns to its start pose
  after 3 seconds.
* **Haptics.** On devices that expose them: 0.3 amplitude for 0.05 s on grab,
  0.6 for 0.1 s on a socket snap, 0.2 for 0.02 s for every 30 degrees of crank
  turn, and xylophone hits scaled by impact speed, all multiplied by the
  haptics strength setting. Haptic and hand controls are hidden where the
  capability report shows them not exposed.
* **Capability report.** Each session writes `capabilities.json` under
  `Application.persistentDataPath`. It records the runtime name and version,
  the system name, offered and active refresh rates, the stereo rendering mode,
  the active interaction profiles, whether hand tracking and haptics are
  exposed per hand, and recentering support.
* **Fragile cylinder.** Dropping the cylinder from higher than 1.2 m cracks it,
  and a new one appears in its panel after 10 seconds of scaled time.
* **Lighting.** The room's lamps grow warmer and brighter after each solved
  step.

### Levels and progression

| Step | Puzzle | Interaction |
|---|---|---|
| 1. Crank | The crank sits on a shelf 2.4 m high | ray grab from the floor, or direct grab after teleporting to the Shelf anchor |
| 2. Drawer | The crank fits a socket on the cabinet; three full turns open the drawer, which holds three gears | socket, then a grab constrained by a hinge joint |
| 3. Clock | Small, medium, and large gears fit only their matching sockets on the wall clock; then the player sets the hands to the time on a postcard pinned by the window (4:20) | sockets, grabbable clock hands that step every 5 minutes |
| 4. Cylinder | The clock chimes and a wall panel opens with the music cylinder; the box lid falls shut unless held, so one hand holds the lid while the other places the cylinder | two-handed interaction, socket |
| 5. Lever | Pulling the box's lever past 50 degrees plays the melody | grab constrained by a hinge joint (0 to 60 degrees) |

Medals by time and hints: gold under 4 minutes with no hints, silver under 7
minutes with at most one hint, and bronze for any solve. Five brass cogs hidden
around the room earn the Collector badge. Free Play unlocks after the first
solve: the puzzles reset at will and the timer is off.

### Scoring, failure, and edge rules

* The hint lamp's pull chain gives the next hint as text and voice, and each
  hint lowers the best medal available.
* Records keep the best time and the longest basket streak.
* There is no fail state; a cracked cylinder costs time only.

### Controls

| Action | Tracked controllers | Tracked hands (where exposed) | Host keyboard |
|---|---|---|---|
| Direct grab | Grip near an object | Pinch near an object | — |
| Ray grab | Trigger while the ray points at an object | Pinch while the hand ray points at it | — |
| Throw | Release grip while moving | Open the pinch while moving | — |
| Teleport | Push the thumbstick forward to aim, release to teleport | Pinch with palm up to aim, release | — |
| Snap or smooth turn | Thumbstick left or right | — | Q and E |
| Press UI | Trigger on the ray, or poke | Poke with the index finger | Tab and arrow keys to move focus, Enter to press |
| Menu | Menu button | Poke the wrist button | M |
| Recenter | Hold the menu button for 1 s | — | R |
| Pause | — | — | Escape |

World-space buttons are at least 4 cm wide.

### Pause, focus, and lifecycle

* The menu and Escape pause the game: `Time.timeScale` is 0, the timer stops,
  gameplay sounds pause while UI sounds keep playing, and the menu panel
  appears 1.2 m in front of the user.
* When the runtime reports the session not focused (for example, a system menu
  is open), the game pauses the same way. When focus returns, the pause panel
  stays open until the player presses Resume.
* When head position tracking is lost for 0.25 s, the room dims to 20 percent, a
  head-locked "Tracking lost" notice appears 1 m ahead, and the game pauses.
  When tracking has been back for 1 s, the notice hides and play resumes. With
  rotation-only tracking, play continues and the notice reads "Limited
  tracking."
* When a controller or hand loses tracking for 0.25 s, its model and ray hide.
  An object it holds stays held for 0.5 s, then drops with zero velocity. When
  tracking returns, its interactors work again within one frame.

### Settings

The world-space Settings panel, at the bench, offers:

* turn mode: snap (30, 45, or 90 degrees) or smooth (60, 90, or 120 degrees per
  second)
* comfort vignette: Off, Low, or High, shown during teleport and turning
* seated or standing, and floor height calibration
* dominant hand for the default ray
* haptics strength (0, 50, or 100 percent)
* master, music, and effects volume
* subtitles for hints on or off
* refresh rate, from the rates the runtime offers

Settings, records, medals, the Collector badge, and the Free Play unlock
persist through `PlayerPrefs`.

### What makes it fun

* **Hook.** Within reach on spawn are a paper ball and the broken music box.
  Grabbing the ball gives a haptic tick on devices that expose haptics, and the
  first basket lands with a swish and a chalk mark. Within 30 seconds the
  player has thrown something, heard the broken melody, and seen the first
  clue.
* **Mastery curve.** A first-time player explores, uses one or two hints, and
  solves in 6 to 12 minutes. A practiced player ray-grabs the crank without
  teleporting, cranks in one smooth motion, and places the cylinder two-handed,
  solving in under 4 minutes. The basket streak rewards a steady throw.
* **Risk and reward.** Hints save time but lower the medal. The fragile
  cylinder punishes careless handling. Hunting cogs costs time on a speed run.
* **Variety and escalation.** Each step adds a new kind of interaction: a reach
  at a distance, a mechanical motion, a clue to read, then two-handed
  coordination.
* **Game feel.**
  * Haptics matched to each event, on devices that expose them.
  * Sockets show a ghost of the part on hover, and snap with a click and a
    sparkle.
  * Gears spin and tick once placed. The drawer slides with a wooden rumble.
  * The clock chimes from its position in the room.
  * Thrown objects keep their release velocity, so throws feel natural.
* **Replay.** Medals, records, the Collector badge, and Free Play.

## Content

### Assets

| Asset | Source format | Import settings that matter |
|---|---|---|
| `Boot.unity`, `Workshop.unity` | Unity YAML scene | object identity, lighting settings, interaction layers |
| `PlayerRig.prefab` | Unity YAML prefab | XR Origin, camera, two controllers with direct, ray, and poke interactors, teleport and turn providers, tunneling vignette |
| Part prefabs: `Crank`, `Gear` with Small, Medium, and Large variants, `Cylinder`, `Lever` | prefab and prefab variants | `XRGrabInteractable`, `Rigidbody`, colliders, interaction layer overrides |
| `Socket.prefab` with one variant per part | prefab variants | `XRSocketInteractor` with interaction layer mask and hover material |
| Toy prefabs: xylophone, mallets, paper balls, hoop, cradle, blocks, bell | Unity YAML prefab | `HingeJoint`, physic materials, continuous collision detection |
| `TeleportAnchor.prefab`, floor teleport area | Unity YAML prefab | `TeleportationAnchor` and `TeleportationArea` |
| Room, furniture, and part models | FBX | mesh, material slots, scale factor 1 (1 unit equals 1 m) |
| Textures | PNG, sRGB, mipmaps; normal maps | sRGB, normal-map mode, compression |
| `RoomInput.inputactions` | Input System action asset | bindings for XR controller grip, trigger, thumbstick, and menu; hand pinch and poke; head pose; keyboard |
| Melody, broken melody | OGG, original composition | compressed-in-memory load type, 3D settings |
| Clock ticks, chimes, xylophone notes, snaps, creaks | WAV, mono | decompress-on-load, 3D settings, logarithmic rolloff |
| Hint voice lines | WAV, mono | 3D settings at the hint lamp |
| Fonts | TrueType font with a TextMeshPro SDF font asset | SDF atlas, fallback font |
| `PuzzleChain.asset`, `ComfortDefaults.asset` | ScriptableObject | step order, hint texts, default comfort values |
| Lighting | lightmaps, light probe group, and two reflection probes, baked in the unitas editor | mixed lighting (Shadowmask), lightmap UVs generated on model import, probe placement around the bench |
| `ProjectSettings/` and XR settings | Unity YAML | XR Plugin Management loaders per target, OpenXR render mode and interaction profiles, physics time step 0.0111 s, quality levels |

### Packages

XR Plugin Management 4.x, XR Interaction Toolkit 2.x, Input System 1.x, URP
14.x, uGUI 1.0.x, TextMeshPro 3.0.x, and Unity Test Framework 1.x, at the
version lines CMP-002 lists. The game also requires the OpenXR provider plug-in
(`com.unity.xr.openxr`) and, for tracked hands, the XR Hands package
(`com.unity.xr.hands`). Both are reported against their own package and
version, as XR-001 requires, and both are Supported. Packages are resolved from
their registry when the project opens and are never vendored in the sample
repository.

### Save data

Settings, records, medals, the badge, and the unlock in `PlayerPrefs`;
`capabilities.json` per session under `Application.persistentDataPath`.

### Licences and provenance

Every asset family's source, author, and licence is recorded in
`Licenses/content-licenses.json`. Unity packages' licence terms are recorded in
the build's licence report.

## Feature Coverage

| ID | Behavior this game exercises | Scenarios |
|---|---|---|
| PRJ-001 | The project opens in place with no conversion | S1 |
| PRJ-006 | XR packages and the provider plug-in reported with their versions and states | S1 |
| PRJ-007 | XR Plugin Management loaders, OpenXR settings, physics time step | S1 |
| SER-003 | Part and socket prefab variants with interaction layer overrides | S4 |
| RUN-003 | `OnApplicationFocus` and `OnApplicationPause` during focus loss | S9 |
| RUN-004 | `Time.timeScale` pause, run timer, respawn delays | S9, S16 |
| PHY-001 | Grab rigidbodies, throws, hinge joints, triggers, continuous collision detection | S4, S15 |
| XR-001 | Stereo views, tracked controllers and hands, direct and ray grab, sockets, teleport, snap and smooth turn, vignette, haptics, focus, tracking loss, capability reporting | S2–S14 |
| INP-002 | XR controller, hand, and keyboard actions; haptic status | S4, S6, S8 |
| UI-001 | World-space panels with ray, poke, and keyboard | S6 |
| TXT-001 | TextMeshPro panels, notices, and hint subtitles | S6, S16 |
| AUD-001 | Spatial melody, clock, xylophone, and hint voice; pause behavior | S7, S9, S15 |
| REN-001 | Per-eye cameras and projections, desktop mirror | S3 |
| REN-002 | Room materials and the socket hover ghost | S4 |
| REN-003 | Lightmaps, light probes, and reflection probes baked in unitas; lamps that warm after each step | S1, S2 |
| REN-004 | Socket sparkles | S4 |
| REN-007 | Frame rate locked to the headset refresh, missed-frame reporting | S13 |
| SAV-001 | Settings, records, medals, badge, and unlocks persist | S6, S16 |
| PLT-001 | XR unavailable and initialization failure handling | S12 |
| BLD-001 | Windows and Linux players with build reports | S1 |
| BLD-002 | Headless runs on the simulation runtime | S1–S12, S14–S16 |
| TST-002 | Scenarios, pose tracks, unevaluable checkpoints, approved references | all |
| PRF-001 | Frame timing captures on the simulation runtime and on headsets | S13 |
| AUT-002 | Play control, track playback, and captures through the editor's MCP interface | S1–S12, S14–S16 |
| ACC-001 | Seated mode, comfort options, keyboard menus, subtitles | S5, S14 |

## Engine Behavior This Game Depends On

Only engine and package behavior, each tied to a catalog ID. Game rules are
defined under Game Design and implemented by the sample's own scripts; the
engine contains no code specific to this game. Package behavior (XR
Interaction Toolkit, XR Plugin Management, OpenXR, XR Hands, Input System)
comes from the package's own code at the pinned version line, compiled and run
unmodified (CMP-002); the items below state only the engine surface the
packages call. Agents reach the game only through the editor's generic MCP
object, property, method-invocation, input, capture, and track-playback
commands, with no game-specific command.

### Scripting and runtime

* `Time.timeScale` 0 stops physics while unscaled UI interaction still works.
  (RUN-004)
* `OnApplicationFocus(false)` arrives when the runtime reports the session not
  focused, and `OnApplicationFocus(true)` when it is focused again. (RUN-003,
  XR-001)

### Project and content

* The rig prefab and every interactable keep their interaction layers and
  references after import. (SER-003)
* XR Plugin Management settings select the OpenXR loader for each desktop
  target, and the compatibility report names the provider's package and
  version. (PRJ-006, PRJ-007)

### Simulation

* Rigidbodies driven by velocity from script, hinge joints with limits, and
  continuous collision detection at a fixed step of 0.0111 s behave as PHY-001
  defines. (PHY-001)

### Rendering and animation

* Each eye renders the same scene content with the runtime's eye pose and
  projection; the distance between the eye cameras equals the runtime's
  reported eye separation. The desktop mirror shows the left eye. (XR-001,
  REN-001)
* Lightmaps, light probes, and reflection probes are baked in the unitas
  editor, interactively or headless. Moving a static object marks the affected
  bake stale. Rebaking replaces it. Dynamic objects take their lighting from
  the light probes. (REN-003)
* The frame rate follows the active display refresh rate that the runtime
  reports, and missed frames are reported. (REN-007, XR-001)

### Audio and video

* 3D sources with logarithmic rolloff between their minimum and maximum
  distances, with the listener following the head, behave as AUD-001 defines.
  Pausing a mixer group leaves other groups playing. (AUD-001)

### Input

* The runtime's head and controller poses drive their tracked devices on the
  frame they are sampled for, and tracking state distinguishes position and
  rotation. (XR-001)
* Controller, hand, and keyboard actions from one action asset survive a switch
  between controllers and hands mid-session with no action lost or duplicated.
  (INP-002, XR-001)
* Haptic impulses reach only devices that report haptic support; elsewhere the
  call returns an explicit unsupported status. Hand tracking devices exist only
  when the runtime exposes hand tracking. (XR-001, INP-002)

### UI and text

* World-space canvases receive ray and poke events through the toolkit's UI
  input module and tracked-device raycaster; the keyboard moves focus between
  the same selectables, and the selected element's visual state is shown.
  (UI-001, XR-001)

### Player and build

* Windows and Linux players start in the first scene in the build, keep
  `PlayerPrefs` between launches, and exit cleanly on `Application.Quit` and on
  the runtime's exit request. With no OpenXR runtime active, or a runtime that
  fails to create a session, XR initialization reports which, with no unhandled
  exception. (BLD-001, PLT-001, XR-001)

### Editor and automation

* Through the editor's generic MCP commands an agent starts the game on the
  simulation runtime, plays pose and button tracks, reads any component field
  (interactor and interactable state), reads files under the storage root (the
  capability report), and captures each eye's view. (AUT-002)

### XR

* The engine's OpenXR provider takes the place of the OpenXR package's native
  library and serves the package's managed code, as XR-001 defines. (XR-001)

## Presentation and Performance

### Visual checkpoints

Each has a unitas-approved reference, captured per eye on the simulation
runtime:

* the spawn view with the start panel
* a socket hover ghost (valid and invalid)
* the opened clock panel with the cylinder
* the teleport reticle (valid and invalid)
* the vignette at Low and High
* the "Tracking lost" notice
* the results panel

### Audio checkpoints

The melody comes from the box's position. A socket snap sound starts within
one frame of the snap.

### Project-declared expectations

* On each physical headset, during a 5-minute happy path at the active refresh
  rate (72, 90, or 120 Hz as offered), at least 99 percent of frames are
  presented within the refresh interval. No more than two consecutive frames
  are missed.
* CPU frame time stays under 8 ms at 90 Hz on the declared reference PC class.
* Workshop load takes under 5 seconds.
* On the simulation runtime, frame timing is recorded and labeled as simulation
  evidence. It is never presented as headset performance.

Expectations are evaluated only on their declared class; other runs record the
measurement and report the expectation as unevaluable (TST-002). These are the
sample's own expectations, not the PRF-002 engine budgets.

### PRF-002 measurements

None.

## Acceptance Scenarios

Outcomes use the TST-002 vocabulary: passed, failed, skipped, unevaluable, and
awaiting human review. Each perturbation names its mechanism: a TST-002
replay-stream event, a harness action outside the player, a pose or session
track played on the simulation runtime, or a development-build debug command
implemented in the sample's scripts. A checkpoint that needs a capability the
active device does not expose is reported as unevaluable, naming the
capability and the device. It is never reported as passed, and a run with an
unevaluable checkpoint is never reported as a full pass.

### S1. Open, packages, and capabilities

* **Runs in:** headless editor; simulation runtime in editor play mode.
* **Preconditions:** no import cache exists.
* **Steps:** open the project, bake its lighting, build Windows and Linux
  players, and start a session on the simulation runtime.
* **Expected:** all assets import with no errors. XR Plugin Management 4.x and XR
  Interaction Toolkit 2.x are reported with their tiers and states. The OpenXR
  provider and XR Hands packages are reported with their own versions and
  states. The bake produces lightmaps, light probes, and reflection probe
  captures that match their approved reference. Where Unity 2022.3 LTS is
  available, it opens the baked scene; otherwise that check is unevaluable.
  Build reports list scenes, assets, sizes, and the build identifier.
  `capabilities.json` lists every field above.
* **Demonstrates:** PRJ-001, PRJ-006, PRJ-007, XR-001, REN-003, BLD-001

### S2. Happy path

* **Runs in:** simulation runtime in editor play mode, headless, and packaged
  players.
* **Preconditions:** a fresh profile.
* **Steps:** play recorded tracks that teleport to the Shelf anchor, ray-grab
  the crank, socket and turn it, place the gears, set the clock, place the
  cylinder two-handed, pull the lever, and walk through the door.
* **Expected:** each step unlocks only after the previous one. The drawer opens
  after exactly three turns. The clock chimes when the hands read 4:20. The
  melody plays when the lever passes 50 degrees, and the door unlocks. The
  room's lamps step warmer and brighter after each solved step. The results
  panel shows time, hints, cogs, and medal.
* **Demonstrates:** XR-001, PHY-001, UI-001, AUD-001, REN-003

### S3. Stereo views and scale

* **Runs in:** simulation runtime in editor play mode, headless, and packaged
  players.
* **Preconditions:** a 0.5 m calibration cube 2 m in front of the head.
* **Steps:** capture both eyes.
* **Expected:** the eye-camera separation equals the runtime's eye separation
  within 0.1 mm. In each eye, the cube's edges land within 1 pixel of the
  positions that eye's projection predicts. Both eyes show the same objects.
  The stereo rendering mode in effect is reported.
* **Demonstrates:** XR-001, REN-001

### S4. Grabs, sockets, and throws

* **Runs in:** simulation runtime in editor play mode, headless, and packaged
  players.
* **Preconditions:** the workshop at spawn.
* **Steps:** direct-grab a gear, ray-grab the crank from 3 m, offer the large
  gear to the small socket, then to the large one, and throw a paper ball at
  4 m/s.
* **Expected:** the direct grab keeps the hand's offset. The ray grab brings the
  crank to the controller. The small socket shows a red ghost and refuses the
  large gear, and the large socket snaps it within 0.1 s with a click and a
  sparkle. The ball leaves at 4 m/s within 0.2 m/s in the release direction.
* **Demonstrates:** XR-001, PHY-001, SER-003, REN-002, REN-004, INP-002

### S5. Teleport, turning, and vignette

* **Runs in:** simulation runtime in editor play mode, headless, and packaged
  players.
* **Preconditions:** the workshop at spawn.
* **Steps:** teleport to each anchor and to three floor points. Aim at the bench
  top and a wall. Snap-turn at 45 degrees eight times. Smooth-turn at 90 degrees
  per second for 2 seconds. Repeat with the vignette at Off, Low, and High.
* **Expected:** each valid teleport places the head over the target within
  0.05 m. Invalid targets are refused. Eight snap turns return to the start
  heading within 0.1 degrees, each rotating about the head position. Smooth
  turning rotates 180 degrees within 1 degree. The vignette appears only during
  teleport and turning, at the configured strength, and never with Off.
* **Demonstrates:** XR-001, ACC-001

### S6. World-space UI

* **Runs in:** simulation runtime in packaged players; headless with a
  play-session restart.
* **Preconditions:** a fresh profile.
* **Steps:** change every setting with the ray, then by poke where hands or poke
  are exposed, then with the host keyboard. Quit and relaunch (a harness action
  that restarts the player, or, headless, a new play session with the same
  storage root).
* **Expected:** each method presses the same buttons with visible focus. Every
  setting applies at once and is restored after relaunch.
* **Demonstrates:** UI-001, TXT-001, INP-002, SAV-001

### S7. Spatial audio

* **Runs in:** simulation runtime in editor play mode, headless, and packaged
  players.
* **Preconditions:** the workshop at spawn, the melody playing.
* **Steps:** capture audio while the head faces the music box, then turns 90
  degrees left, then steps from 1 m to 2 m away.
* **Expected:** facing the box, the channel levels are within 1 dB. After the
  turn, the right channel is at least 6 dB louder than the left. Doubling the
  distance lowers the level as the rolloff curve predicts, within 1 dB.
* **Demonstrates:** AUD-001, XR-001

### S8. Haptics where exposed

* **Runs in:** simulation runtime in editor play mode, headless, and packaged
  players.
* **Preconditions:** the simulation runtime configurations "full capabilities"
  and "no haptics".
* **Steps:** grab, socket, crank, and hit the xylophone on each configuration.
* **Expected:** with haptics exposed, each event sends an impulse with the
  stated amplitude and duration, scaled by the strength setting. Without them,
  the calls return the unsupported status, the capability report shows haptics
  as not exposed, and the haptic checkpoints are unevaluable rather than
  passed.
* **Demonstrates:** XR-001, INP-002

### S9. Focus loss and recovery

* **Runs in:** simulation runtime in editor play mode, headless, and packaged
  players.
* **Preconditions:** holding a gear.
* **Steps:** set the simulation runtime's session to visible but not focused (a
  session track), wait 5 seconds, then restore focus.
* **Expected:** the game pauses, the timer stops, and gameplay audio pauses. On
  focus return, the pause panel remains until Resume. The gear is still held if
  the grip is still pressed, and drops with zero velocity otherwise.
* **Demonstrates:** XR-001, RUN-003, RUN-004, AUD-001

### S10. Tracking loss and recovery

* **Runs in:** simulation runtime in editor play mode, headless, and packaged
  players.
* **Preconditions:** the right controller holding the crank.
* **Steps:** drop head position tracking for 3 seconds, then provide
  rotation-only tracking for 3 seconds, then drop the right controller's
  tracking for 1 second (pose tracks).
* **Expected:** the "Tracking lost" notice appears after 0.25 s, the room dims,
  and play pauses. Play resumes 1 s after tracking returns. Rotation-only
  tracking shows "Limited tracking" without pausing. The controller's model
  hides after 0.25 s, the crank drops with zero velocity after 0.5 s, and the
  controller works within one frame of returning.
* **Demonstrates:** XR-001

### S11. Capability gating

* **Runs in:** simulation runtime in packaged players and headless; physical
  headsets, when present.
* **Preconditions:** the simulation runtime configuration with no hand tracking
  and no haptics.
* **Steps:** run S2, S6, and S8 on that configuration, and on each physical
  headset.
* **Expected:** every hand and haptic checkpoint on a device that does not
  expose the capability is unevaluable and names the capability and device.
  Each run's result is "passed with unevaluable checkpoints," never a full
  pass. Every other checkpoint passes. Results on headsets record the device,
  runtime, refresh behavior, and input capabilities.
* **Demonstrates:** XR-001, TST-002

### S12. XR unavailable and initialization failure

* **Runs in:** packaged players; headless editor play sessions.
* **Preconditions:** none.
* **Steps:** start the player with no OpenXR runtime active, then with a
  simulation runtime configured to fail session creation (harness actions that
  set the active runtime). Then press Quit.
* **Expected:** the desktop window shows "No XR runtime found" in the first case
  and "XR failed to start: <reason>" in the second, with Retry and Quit. The log
  distinguishes the two. No exception is unhandled, and Quit exits with
  status 0.
* **Demonstrates:** XR-001, PLT-001

### S13. Headset refresh-rate performance

* **Runs in:** physical headsets only, in packaged players; otherwise
  unevaluable.
* **Preconditions:** a physical headset.
* **Steps:** play the happy path for 5 minutes at each refresh rate the runtime
  offers, capturing frame timing.
* **Expected:** each run meets the project-declared expectation for that rate.
  The capture records device, runtime, active refresh rate, frame timing,
  missed frames, and tracking events. The same capture on the simulation
  runtime is labeled as simulation evidence.
* **Demonstrates:** REN-007, XR-001, PRF-001

### S14. Recenter, seated, and dominant hand

* **Runs in:** simulation runtime in editor play mode, headless, and packaged
  players.
* **Preconditions:** the workshop at spawn.
* **Steps:** turn the head 90 degrees and recenter. Switch to seated with a
  1.2 m floor offset. Switch the dominant hand to left.
* **Expected:** after recentering, the user faces the bench within 2 degrees.
  Seated mode raises the view so the shelf crank is reachable by ray from a
  chair. The default ray moves to the left controller.
* **Demonstrates:** XR-001, ACC-001

### S15. Toys

* **Runs in:** simulation runtime in editor play mode, headless, and packaged
  players.
* **Preconditions:** the workshop at spawn.
* **Steps:** hit the xylophone's lowest bar at 0.1, 1.5, and 3 m/s. Throw five
  paper balls into the hoop, then miss one. Pull back one cradle ball 20 degrees
  and release.
* **Expected:** the 0.1 m/s hit is silent, and the 1.5 and 3 m/s hits play the
  same pitch at increasing levels, matching the volume curve within 1 dB. The
  chalkboard counts 1 to 5, then resets on the miss, and the longest streak is
  recorded. The cradle swings the last ball out.
* **Demonstrates:** PHY-001, AUD-001

### S16. Medals, hints, and the fragile cylinder

* **Runs in:** simulation runtime in editor play mode, headless, and packaged
  players.
* **Preconditions:** a fresh profile.
* **Steps:** solve once under 4 minutes with no hints through recorded tracks.
  Solve again using two hints with subtitles on. In a third run, drop the
  cylinder from 1.5 m, and collect all five cogs.
* **Expected:** the first run awards gold and a best-time record. The second
  shows each hint as text and voice, with a subtitle, and awards bronze. The
  dropped cylinder cracks, and a new one appears in its panel after 10 seconds
  of scaled time. The Collector badge is awarded and saved. Free Play is
  unlocked after the first solve.
* **Demonstrates:** SAV-001, RUN-004, TXT-001

## Manual Playtest Guidance

A person in a headset judges the following, and a headless run reports these
checkpoints as awaiting human review, never as passed:

* Whether the first 30 seconds invite play without instructions.
* Whether grabbing, throwing, and cranking feel physical and responsive.
* Whether each puzzle clue is findable, and whether the hints help without
  giving the answer away.
* Whether the two-handed cylinder step feels like a satisfying finale.
* Whether teleport, turning, and each vignette level are comfortable.
* Whether spatial audio helps the player locate the clock and the box.
* Whether haptics feel matched to each event on headsets that expose them.

## Agent Authoring Change Requests

### CR1. Combination padlock

* **Request:** add a sixth puzzle step: a combination padlock on the door with
  three rotating dials, whose code is written on the underside of a block. The
  padlock is static geometry, so the change includes rebaking the room's
  lighting.
* **Acceptance scenario:** the door opens only with the right code, each dial
  steps in fixed increments, and the rebaked lighting matches a newly approved
  reference.
* **Test:** a test of the dial steps and the code check.

### CR2. Toy drum

* **Request:** add a toy drum that plays louder with faster hits and sends a
  haptic impulse scaled by impact speed.
* **Acceptance scenario:** on the "no haptics" configuration the drum plays and
  the haptic checkpoint is unevaluable; on the "full capabilities"
  configuration the impulse amplitude follows impact speed.
* **Test:** a test of the volume curve.

## Accessibility

* Every menu action can be performed with the host keyboard (S6).
* Seated mode makes every puzzle reachable from a chair (S14).
* Snap turning, the vignette, and teleport provide comfortable locomotion, and
  smooth turning is optional (S5).
* Hints and announcements have subtitles (S16).
* Interaction prompts use shape and text as well as color (S4).
* The dominant hand can be switched (S14).

## Saved and Reproducible States

| State | How it is reached | Used by |
|---|---|---|
| The workshop after step 2, with the gears in the drawer | save file | S4 |
| The workshop after step 3, with the clock panel open | save file | S16 |
| The two-handed step with the lid held open | save file | S2 |
| A records file with gold, the Collector badge, and Free Play unlocked | save file | S16 |
| Simulation runtime configurations: full capabilities, no haptics, and no hand tracking | committed runtime configuration files | S8, S11 |

## Known Limitations and Unsupported Conditions

* Physical headsets are capability-reported targets. A headset's result covers
  only the capabilities its report shows as exposed.
* No OpenXR runtime exists for macOS, so macOS is not an XR target.
* Hand tracking, haptics, and specific refresh rates depend on the headset and
  runtime, and are tested only where the capability report shows them.

Any behavior this document requires that is not demonstrated is reported with
its real compatibility state.

## Acceptance Outcomes

* The XR interaction room never reports a feature as passing on hardware that
  does not expose it. (SMP-001, XR-001)
* The full puzzle chain completes on the OpenXR simulation runtime through
  direct grab, ray grab, sockets, hinged mechanisms, and teleport, in editor,
  player, and headless runs. (SMP-001, XR-001, PHY-001)
* Stereo views match the runtime's eye separation and projections, and the
  room renders at true scale. (SMP-001, XR-001, REN-001)
* Snap turn, smooth turn, teleport, and the vignette follow their settings, and
  seated mode keeps every puzzle reachable. (SMP-001, XR-001, ACC-001)
* World-space UI works with ray, poke where exposed, and keyboard, and settings
  persist. (SMP-001, UI-001, SAV-001)
* Spatial audio places the music box, clock, and toys by head pose, and haptics
  fire only where exposed. (SMP-001, AUD-001, XR-001, INP-002)
* Focus loss and tracking loss pause or degrade play as specified and recover
  cleanly. (SMP-001, XR-001)
* On each physical headset, the room meets its refresh-rate expectation, and
  simulation timing is never presented as headset performance. (SMP-001,
  REN-007, XR-001)
* Medals, hints, collectibles, and the toys behave as specified. (SMP-001,
  SAV-001, PHY-001)
