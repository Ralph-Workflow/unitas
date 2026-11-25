# Sample 10: Mobile Game ("Prism Reef")

## Overview

Prism Reef is a touch-driven light puzzle with an arcade bonus round. On each
board the player taps, drags, and twists mirrors and prisms to bend a beam of
light through a coral reef until every sea anemone glows. Each chapter ends
with Current Dash, a 30-second arcade run in which the player steers a
glowing fish through a current with an on-screen stick or by tilting the
device. A puzzle takes 30 seconds to 3 minutes. The 15 puzzles and 3 dashes
take about 40 minutes, and three-starring every board takes much longer. The
tone is calm and luminous, with soft chimes over an ambient score.

The game survives interruptions, backgrounding, touch cancellation, permitted
orientation changes, and safe-area changes, and restores saved progress
exactly. It never uses XR features.

This is an original, redistributable unitas sample. Its rules, art, audio, and
level design are unitas content with recorded licences, and it is not derived
from any Unity sample or third-party game.

Compatibility tiers follow the [Feature catalog](../feature-catalog.md). This
document owns this game's acceptance under SMP-001.

## Targets and Input Modes

* **Automated targets:** iOS players on the iOS Simulator, on a macOS host
  with the required Xcode, at the game's minimum iOS 15 runtime and the newest
  runtime of the host's Xcode, with a phone profile that has a sensor housing
  and home indicator and a tablet profile; and Android players on the Android
  emulator, with an ARM64 system image on ARM hosts and an x86-64 image on
  x86-64 hosts, at API level 26 (the game's minimum) and the current store
  target API level, with a phone profile that has a display cutout and a
  10-inch tablet profile. On a host without the Apple toolchain, the iOS
  target is reported Externally unavailable with that cause.
* **Optional targets:** physical iOS and Android devices. A physical-device
  run records a capability report of the device: haptics, accelerometer,
  microphone, ASTC support, thermal state, and battery level, each as exposed,
  denied, or absent.
* **Also:** the editor's play mode and the headless editor, which use a
  synthetic touchscreen device. Lifecycle, permission, and deployment
  scenarios run only on the simulator, emulator, and devices.
* **Declared aspect ratios:** 19.5:9 and 20:9 phones, and 4:3 and 16:10
  tablets, each in portrait and landscape.
* **Render pipeline:** URP 14.x (Forward) with a Mobile quality level.
* **Input modes:** touch (taps, drags, long press, pinch, two-finger twist,
  pan, double tap), on-screen stick and button, accelerometer tilt where the
  target exposes one, and microphone breath input where granted. A Bluetooth
  keyboard or gamepad is not a declared mode.

A hardware-dependent feature is reported as passing only on a target that
exposes it. On the iOS Simulator, haptics and the accelerometer are absent and
their checks are reported not applicable. The Android emulator's virtual
accelerometer counts as exposed and is recorded as emulated.

## Game Design

### Core loop

1. The title shows Play, Current Dash (after chapter 1), Settings, and About.
   Play opens the chapter map, a horizontal pager of three chapters with five
   boards each.
2. A board shows a light source, mirrors, prisms, blockers, anemones, and
   optional pearls. The beam is traced every time a piece changes.
3. The player changes pieces until every anemone is lit. The reef blooms,
   the stars earned appear (3 at or under par moves, 2 at par plus 3, 1
   otherwise), and Next, Retry, and Map are offered.
4. After board 5 of each chapter, Current Dash starts. The fish swims
   forward; the player steers it to collect plankton and avoid jellyfish for
   30 seconds, then sees the score and medal.

### Pieces and gestures

| Piece or action | Gesture | Effect |
|---|---|---|
| Mirror | Tap | Flips between `/` and `\` |
| Prism | Two-finger twist past 45 degrees | Rotates 90 degrees; splits the beam into two at right angles |
| Crystal | Drag to an empty cell | Moves along any free path; blocks or tints the beam |
| Any piece | Long press 0.5 s | Shows its rule card |
| Board (chapter 3) | Pinch, two-finger pan, double tap | Zooms 1.0 to 2.5 times, pans, resets zoom |
| Undo | Undo button | Reverts the last move, up to 20 moves; counts as a move |
| Current Dash steering | On-screen stick, or tilt when enabled | Moves the fish within the current |
| Boost | On-screen button | Doubles speed for 1 s, with a 3 s cooldown |
| Bubble shield | Shield button, or a breath into the microphone when enabled | Shields the fish for 2 s, with a 6 s cooldown |

Only two touches drive gestures at once. A third touch is ignored and never
cancels the first two.

### Chapters

| Chapter | Boards | Board size | New elements |
|---|---|---|---|
| 1. Shallows | 1 to 5 | 6 x 9 | mirrors, pearls |
| 2. Kelp Maze | 6 to 10 | 6 x 9 | prisms, draggable crystals, undo pressure |
| 3. Midnight Shelf | 11 to 15 | 10 x 14 | color crystals, anemones that need a color, zoom and pan |

### Scoring, failure, and restart

* A move is any tap, twist, or drag that changes a piece, and any undo.
* A pearl on the beam's path is collected when the board is solved. Pearls
  buy hints: a hint costs 3 pearls and highlights one piece to change.
* A board cannot be lost. Retry resets the board, moves, and undo stack.
* In Current Dash, a plankton collected within 0.8 s of the previous one
  extends the chain, and the multiplier is the chain length, capped at x8.
  Each plankton scores 1 point times the multiplier. A jellyfish hit stuns
  the fish for 1.5 s and resets the chain.

### What makes it fun

* **Hook.** Board 1 opens with the beam stopping at a glowing mirror. One tap
  bends it, the first anemone blooms with a chime, and a short haptic pulse
  fires on devices that expose haptics. Board 2 appears by second 20 with two
  mirrors and a pearl just off the obvious route.
* **Mastery curve.** A new player solves by trial; a skilled player reads the
  beam path two reflections ahead, uses one prism to light two anemones, and
  plans crystal drags so that undo is never needed. Par moves for board 10 are
  9; a first attempt usually takes 15 to 20.
* **Risk and reward.** Routing the beam through a pearl usually costs one or
  two extra moves and can lose the third star. A hint saves time but spends
  pearls that another board's hint might need. In Current Dash, boosting
  through a gap of plankton doubles the chain gain but leaves no time to dodge
  a jellyfish.
* **Variety and escalation.** Each chapter adds the elements in the chapters
  table, and chapter 3 boards are large enough to need zoom and pan. Each
  Current Dash is faster than the last: 4, 5, and 6 units per second.
* **Juice.** A tapped mirror turns over 0.12 s with a click; the beam redraws
  on the same frame. Each newly lit anemone plays the next note of a
  pentatonic scale, so solving a board plays a short melody. The full bloom
  scales the reef by 1.05 for 0.2 s and fires one 50 ms haptic pulse where
  exposed. A plankton chain shows a rising counter, and every fifth plankton
  flashes the screen edge. A jellyfish hit shakes the camera by 0.1 units for
  0.2 s. Reduced motion removes the shake, the flash, and the reef scale.
* **Replay reasons.** Stars per board, a pearl total, and the fewest moves per
  board as a record. Current Dash awards Bronze, Silver, and Gold at 40, 80,
  and 120 points and records the best score. Three stars on every board of a
  chapter unlocks that chapter's Night Reef palette, and Gold in all three
  dashes unlocks the lantern-fish skin.

### Pause and lifecycle

* The pause button sets `Time.timeScale` to zero and shows Resume, Retry,
  Settings, and Map. The pause panel names its reason: "Paused" for the
  button, "Paused: the app was in the background" after backgrounding or an
  operating-system interruption, and "Paused: focus was lost" after focus loss
  without backgrounding.
* Backgrounding saves the board and progress in the same frame as
  `OnApplicationPause(true)`, pauses audio, and leaves the game paused on
  return.
* Android's system back closes a panel or opens the pause panel; on the title
  it asks to exit. iOS offers no exit.

### Settings

Music and effects volume, haptics on or off, tilt steering on or off,
microphone shield on or off, orientation (Auto, Portrait, Landscape), Battery
Saver (30 frames per second instead of 60), reduced motion, left-handed
controls (swaps the stick and buttons), and Reset Progress with a
confirmation. Settings persist through `PlayerPrefs`.

## Content

| Asset | Source format | What matters on import |
|---|---|---|
| `Title.unity`, `Map.unity`, `Board.unity`, `Dash.unity` | Unity YAML scene | canvases, prefab instances, event system |
| `Mirror.prefab`, `Prism.prefab`, `Crystal.prefab` | prefabs and color variants | variant overrides of tint and rule card |
| `Boards/*.asset` | ScriptableObject board data | cell layout, par moves, pearls |
| Reef art | PNG textures | Android and iOS overrides: ASTC 6x6 for board art, ASTC 4x4 for the UI atlas, mipmaps off for UI |
| Fish and anemone models | FBX | URP Lit and Unlit materials, one UV set |
| Sounds and music | WAV effects, OGG music | decompress-on-load chimes in 12 notes, streamed music, per-target Vorbis compression |
| `Reef.mixer` | Audio Mixer | Master, Music, Effects groups; `Play`, `Paused`, and `Interrupted` snapshots |
| Font | TrueType with a TextMeshPro SDF font asset | SDF atlas, fallback font |
| `Controls.inputactions` | Input System action asset | Touch, OnScreen, and Tilt schemes |
| `link.xml` | linker XML | preserves the `PrismReef.Save` namespace for Newtonsoft Json |
| `Assets/Plugins/Android/AndroidManifest.xml` | custom manifest | declares `RECORD_AUDIO` and `VIBRATE`, portrait and landscape |
| `ProjectSettings/` | Unity YAML | IL2CPP, ARM64, stripping level High, Mobile quality level, iOS microphone usage description, allowed orientations |
| `Licenses/` | text files | content licences shown on the About screen |

Packages: URP 14.x, Input System 1.x, uGUI 1.0.x, TextMeshPro 3.0.x, and
Newtonsoft Json 3.x.

## Feature Coverage

| ID | Behavior this game exercises | Scenarios |
|---|---|---|
| PRJ-001 | The project opens in place with no conversion | S1 |
| PRJ-003 | Board data, prefab, and texture identities stay stable | S1 |
| PRJ-004 | Per-target ASTC overrides, FBX models, WAV and OGG | S1, S11 |
| PRJ-006 | URP, Input System, uGUI, TextMeshPro, and Newtonsoft Json resolve | S1 |
| PRJ-007 | Orientation, scripting backend, stripping, quality, Android and iOS settings | S1, S7 |
| SER-002, SER-003 | Board scenes, piece prefabs and color variants | S1, S2 |
| RUN-003 | `OnApplicationPause`, `OnApplicationFocus`, lifecycle order | S5 |
| RUN-004 | Paused time, cooldowns, unscaled pause animation | S5, S9 |
| RUN-009 | Save files under `Application.persistentDataPath`, Newtonsoft Json | S6, S12 |
| REN-002 | Mobile quality tier, ASTC textures, URP Lit and Unlit materials | S11 |
| REN-003 | Mobile quality level settings | S11 |
| REN-007 | `Application.targetFrameRate` at 60 and 30 | S11 |
| UI-001 | Safe-area layout, pagers, panels, physical touch-target sizes | S2, S7 |
| TXT-001 | TextMeshPro HUD and rule cards | S2 |
| INP-002 | Enhanced touch, gestures, multi-touch, on-screen stick and button, accelerometer | S2, S3, S4, S9 |
| AUD-001 | Chimes, music, mixer snapshots, interruption and resume, microphone input | S5, S8 |
| SAV-001 | Progress and board restoration, backups, corrupt data, settings | S6, S10 |
| PLT-001 | iOS and Android lifecycle, orientation, safe areas, interruptions, permissions, capability report | S4, S5, S7, S8, S16 |
| BLD-001 | Android packages, iOS app and Xcode project, build reports | S15 |
| BLD-002 | Headless runs of every scenario | all |
| BLD-003 | IL2CPP ahead-of-time compilation, High stripping, `link.xml` | S12 |
| BLD-004 | Device discovery, deployment, development-player connection | S13 |
| DIA-003 | Crash reports, symbolication, device logs | S14 |
| PRF-001 | Profiling a connected development player | S13, S16 |
| LIC-001 | License record in the build result and the packaged app | S15 |
| PRD-002 | No network connection or telemetry, privacy manifest, local-only microphone use | S8, S15 |
| AUT-007 | Pairing and scopes for development-player connections | S13 |
| TST-002 | Scenarios, checkpoints, approved references, input replay | all |
| AUT-002 | Agents play, inspect, and capture the game in editor play sessions through the editor's MCP interface | all |

## Engine Behavior This Game Depends On

### Scripting and runtime

* `OnApplicationFocus(false)` and `OnApplicationPause(true)` arrive in the
  target's order when backgrounding, and `OnApplicationFocus(false)` alone
  arrives for focus loss without backgrounding.
* A file written under `Application.persistentDataPath` during
  `OnApplicationPause(true)` is durable when the target's documentation says
  it is, and the game observes write failures.

### Project and content

* Per-target texture overrides apply in each build, and the build report lists
  each texture's format. A target without ASTC reports the asset, target, and
  fallback format.

### Rendering

* The Mobile quality level turns off MSAA, HDR, and main-light shadows and
  allows 2 additional lights per object. `Application.targetFrameRate` paces
  frames at 60, or 30 with Battery Saver, and `vSyncCount` is ignored, as
  REN-007 defines for mobile targets.

### Input

* `EnhancedTouch` reports each finger with a stable identity from Began to
  Ended or Canceled. Gestures use those identities, so a third finger never
  changes the gesture of the first two.
* A system gesture or an interruption during a drag delivers Canceled. The
  dragged crystal returns to its cell and no move is counted.
* `OnScreenStick` and `OnScreenButton` feed the Dash actions as a virtual
  device. With tilt on, the `Accelerometer` device steers after a 0.05 g dead
  zone; on targets without an accelerometer, the tilt toggle is disabled with
  the reason shown.
* Long press uses a hold interaction of 0.5 s, and double tap a multi-tap
  interaction of 0.3 s.

### UI and text

* The root panel fits `Screen.safeArea` and relayouts within one frame of a
  safe-area or orientation change. In landscape the HUD moves to the side
  away from the sensor housing.
* Every touch target is at least 9 mm on its shorter side, computed from
  `Screen.dpi`.

### Audio

* Chimes start within one frame of their anemone lighting. An interruption
  moves to the `Interrupted` snapshot; resuming returns to `Play`.
* The microphone shield measures loudness only. No microphone audio is stored
  or leaves the device.

### Platform, permissions, and capabilities

* The microphone permission is requested only when the player turns the shield
  setting on, with `Application.RequestUserAuthorization` on iOS and the
  Android microphone permission on Android. Denial, revocation in system
  settings, and an absent microphone each return explicit status; the
  setting turns off and shows the reason, and the shield button still works.
* `Handheld.Vibrate` reports an unavailable status on targets without haptics,
  and the game hides the haptics setting there.
* `Application.lowMemory` makes the game unload the other chapters' art and
  log the event.

### Player and build

* Builds use IL2CPP with ARM64 for devices and the simulator or emulator's
  architecture. Stripping is High.
* The release player opens no network connection, includes no telemetry, and
  sends no crash report off the device, because the project configures no
  destination.

### Editor and automation

* An agent can list devices, deploy, launch, read board state, send touches,
  rotate the simulated device, trigger backgrounding, capture frames, read
  device logs, and remove the app through the editor's MCP interface.

## Presentation and Performance

* Visual checkpoints with unitas-approved references, per phone and tablet
  profile, in portrait and landscape: the title; board 1 at start; board 1
  solved and blooming; board 13 at 2.5 times zoom; a Current Dash mid-run; the
  pause panel with each reason; and the About screen with licences.
* Audio checkpoint: the board 1 chime within one frame of the anemone lighting.
* Project-declared expectations: on the declared reference physical phone
  class, 60 frames per second on chapter 3 boards with no frame over 33 ms in
  a 5-minute run, startup to title under 4 s, and resident memory under 350 MB;
  a 10-minute session records thermal state and battery level where exposed.
  Simulator and emulator runs check behavior, not frame rate. These are the
  sample's own expectations, not the PRF-002 engine budgets.

## Acceptance Scenarios

Every scenario runs on the iOS Simulator and the Android emulator profiles,
and headless through the editor's MCP interface against them, unless it says otherwise. S2, S3, S9,
and S10 also run in editor play mode with the synthetic touchscreen.

* **S1. Open and inspect.** No import cache exists. Open the project.
  * Expected: all assets import with no errors; every package is Supported
    with its tier; per-target texture overrides and the custom manifest are
    reported; reopening yields identical identities.
  * (PRJ-001, PRJ-003, PRJ-004, PRJ-006, PRJ-007, SER-002)
* **S2. Happy path.** Replay touch input that solves boards 1 to 5 at par and
  plays the chapter 1 dash.
  * Expected: each board blooms with 3 stars, the chimes play in scale order,
    the dash scores at least 80 points, and progress shows chapter 2 unlocked.
  * (INP-002, UI-001, TXT-001, AUD-001, SER-003)
* **S3. Gestures and multi-touch.** On board 13: tap, drag, long press, twist,
  pinch to 2.5 times, pan, and double tap; then add a third finger mid-pinch.
  * Expected: each gesture has its effect and counts the stated moves; the
    zoom never exceeds 2.5 or goes below 1.0; the third finger changes
    nothing; every touch keeps one identity from start to end.
  * (INP-002)
* **S4. Touch cancellation.** Start a crystal drag, then pull down the system
  notification area on Android and open Control Center on iOS.
  * Expected: the drag receives Canceled, the crystal returns to its cell,
    the move count is unchanged, and the pause panel says focus was lost.
  * (INP-002, PLT-001)
* **S5. Backgrounding and interruption.** Mid-board, send the app to the
  background and return; on Android place an emulated incoming call, and on
  iOS lock the simulated device; then press the pause button.
  * Expected: each case pauses with its own stated reason; the board, moves,
    and undo stack are unchanged; music moves to the `Interrupted` snapshot and
    resumes only after Resume; the lifecycle messages arrive in the target's
    order.
  * (PLT-001, RUN-003, RUN-004, AUD-001)
* **S6. Progress restoration.** Mid-board 8 with 4 moves and an undo stack of
  3, background the app and terminate its process; relaunch. Then corrupt
  `progress.json` and relaunch; then corrupt the backup too.
  * Expected: the board, moves, undo stack, stars, and pearls are restored
    exactly; a corrupt main file loads the backup and reports it; with both
    corrupt, the game reports corrupt data, not absent data, offers a new
    profile, and leaves both files on disk.
  * (SAV-001, RUN-009, PLT-001)
* **S7. Orientation and safe areas.** Rotate through portrait, landscape left,
  and landscape right during a board and a dash, on every profile; then set
  the orientation setting to Portrait and rotate again. Repeat on each
  declared aspect ratio.
  * Expected: upside-down portrait is never used; no control is outside
    `Screen.safeArea`, which the layout follows within one frame of each
    safe-area change; board state survives each rotation; a rotation during a
    drag cancels it as in S4; the Portrait setting holds portrait.
  * (PLT-001, UI-001, PRJ-007)
* **S8. Permissions and capabilities.** Turn on the microphone shield and
  deny the request; turn it on again and grant it; revoke it in system
  settings and return; repeat on a profile with no microphone input.
  * Expected: denial and revocation turn the setting off with the reason; a
    grant lets a breath trigger the shield; an absent microphone reports
    absent; the shield button works in every case; no audio leaves the device;
    haptics report unavailable on the iOS Simulator.
  * (PLT-001, AUD-001, PRD-002)
* **S9. Current Dash and the chain.** Collect 10 plankton 0.5 s apart with the
  on-screen stick, wait 1 s, collect one more, hit a jellyfish, then boost.
  Repeat with tilt where an accelerometer is exposed, and with left-handed
  controls.
  * Expected: the chain rises to x8 and holds; plankton score 1 times the
    chain; the 1 s gap resets the chain; the hit stuns for 1.5 s; boost lasts
    1 s and is unavailable for 3 s; tilt is reported not applicable on the
    iOS Simulator.
  * (INP-002, RUN-004)
* **S10. Stars, pearls, medals, and unlocks.** Solve board 4 at par without
  its pearl, then at par plus one with it, use a hint, and replay dashes at 79
  and 80 points; three-star chapter 1.
  * Expected: the first solve gives 3 stars, the second 2 stars and one pearl;
    the hint costs 3 pearls; 79 points gives Bronze and 80 Silver; the Night
    Reef palette unlocks; records and unlocks survive relaunch.
  * (SAV-001, UI-001)
* **S11. Quality tier and textures.** Build for both targets and solve board
  13 with Battery Saver off and on.
  * Expected: the build report lists ASTC formats, or the reported fallback
    on a target without ASTC; captures match their approved references; the
    achieved frame pacing follows 60 and 30.
  * (REN-002, REN-003, REN-007, PRJ-004, TST-002)
* **S12. Ahead-of-time compilation and stripping.** Build development players
  at High stripping without the `PrismReef.Save` entry in `link.xml`, save a
  board, then restore the entry and rebuild.
  * Expected: the build reports the stripping level, removed members, and no
    ahead-of-time findings; without the entry the save fails with an error
    naming the stripped type and member and the preservation mechanisms; with
    it the save round-trips.
  * (BLD-003, RUN-009, SAV-001)
* **S13. Deployment, connection, and profiling.** Through the editor's MCP interface, list devices,
  deploy a development player, launch it, connect the profiler and log stream,
  capture 600 frames on board 13, and remove the app. Repeat the connection
  without the pairing credential.
  * Expected: every device is listed with target, architecture, OS, and state;
    the capture includes the `BeamTrace` marker and records device, quality,
    and scene; the connection without the credential is refused and reported;
    the credential never appears in logs.
  * (BLD-004, PRF-001, AUT-007)
* **S14. Crash reports and device logs.** In a development build, use the
  About screen's developer menu to throw an unhandled managed exception, then
  to force a native crash. Repeat the native crash in a release build made
  with the `PRISMREEF_CRASH_TEST` define, which is the only release
  configuration that keeps the menu, and copy its report from the device.
  * Expected: each report records the build identifier, unitas version, device
    model, OS, managed stacks, and the native stack; the release report is
    symbolicated with the matching dSYM or Android symbols; device log entries
    are structured by time, severity, and source; no report leaves the device.
  * (DIA-003, PRD-002)
* **S15. Package, licence, and privacy.** Build release players: an Android
  App Bundle and APK, and an iOS Xcode project and simulator app. Play board 1
  with network capture on, and open About.
  * Expected: each build report lists signing state, packaging form,
    permissions, size, and build identifier; it records the governing licence
    and its obligations; the app shows the GPLv3 text and third-party notices;
    the iOS build includes the privacy manifest; no network connection opens.
  * (BLD-001, LIC-001, PRD-002)
* **S16. Physical-device capability report.** Optional, on physical devices
  only. Run S2 and a 10-minute session.
  * Expected: the capability report lists haptics, accelerometer, microphone,
    ASTC, thermal state, and battery level as exposed, denied, or absent; each
    absent capability's checks are not applicable, never passed.
  * (PLT-001, PRF-001)

## Manual Playtest Guidance

A person judges the following, and a headless run reports these checkpoints as
awaiting human review, never as passed:

* Whether the first tap and bloom make the player want the next board.
* Whether reading beam paths feels like a skill that grows.
* Whether pearls feel worth an extra move.
* Whether chapter 3 boards feel bigger without feeling tedious to pan.
* Whether Current Dash steering feels fair with the stick and with tilt.
* Whether the chime melody and haptic pulse make a solve feel rewarding.
* Whether one-handed portrait play is comfortable on a phone.

## Agent Authoring Change Requests

Add a new piece, the Splitter Gate, which passes the beam only while an
adjacent anemone is lit, with a rule card, a sprite, a tap sound, and two new
boards in chapter 3 that use it. The request includes a test of the gate rule
and a scenario that solves both boards at par.

A second change request adds a Current Dash power-up that doubles plankton
value for 5 s, with a scenario proving the chain and the doubled value
combine as stated.

## Accessibility

Every action has a single-finger alternative: twist also works as a tap on
the prism's rotate handle, and pinch as the zoom buttons. Touch targets are at
least 9 mm. Colors on chapter 3 crystals and anemones also differ by shape.
Reduced motion removes shake, flashes, and scale pulses. Left-handed controls
mirror the dash controls. Menus expose accessibility names.

## Saved and Reproducible States

* Board 8 mid-solve with 4 moves and 3 undo steps.
* Board 13 at 2.5 times zoom.
* Current Dash of chapter 2 at 20 seconds with a x6 chain.
* A profile with chapter 1 three-starred and 10 pearls.

## Known Limitations and Unsupported Conditions

The game uses no XR features. On simulators and emulators, frame rate and
thermal behavior are not representative, and their checks are reported as
not evaluated there. Any behavior this document requires that is not
demonstrated is reported with its real compatibility state.

## Acceptance Outcomes

* The mobile game survives interruption, backgrounding, and orientation change
  and restores saved progress. (SMP-001, PLT-001, SAV-001)
* Touch gestures, multi-touch, cancellation, on-screen controls, and tilt
  behave as stated, and a canceled drag never counts a move. (SMP-001,
  INP-002, PLT-001)
* Each permission and capability reports grant, denial, revocation, or absence
  explicitly, and no hardware-dependent check passes on a target without the
  hardware. (SMP-001, PLT-001)
* Stars, pearls, chains, medals, records, and unlocks follow their stated
  thresholds and persist. (SMP-001, SAV-001, INP-002)
* IL2CPP builds at High stripping report their stripping, and `link.xml`
  keeps save types working. (SMP-001, BLD-003)
* An agent deploys, connects to, profiles, and removes a development player,
  and crash reports are symbolicated with matching symbols. (SMP-001,
  BLD-004, PRF-001, DIA-003)
* Each release package records its licence, includes the licence texts and
  notices, and opens no network connection. (SMP-001, BLD-001, LIC-001,
  PRD-002)
