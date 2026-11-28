# Sample 4: First-Person Interaction ("Night Shift")

## Overview

The player is the night technician at a small research lab after a power
failure. In first person, they move through four compact rooms, grab, carry,
stack, and throw objects, work switches, levers, and doors, follow a radio
signal by ear, and crack the vault code. Each room is one physical puzzle that
reuses what the last one taught, with a twist. Carrying the lumen core from the
vault to the reactor socket restores the lab and completes the game. A first
run takes 10 to 15 minutes, and a skilled run under 6.

The game is original, redistributable unitas content and uses no Unity-owned
sample content. It is one of the demonstrations of pointer look and cursor
capture, rebinding, physics queries, dynamic bodies, joints, cloth, cameras and
render textures, real-time lighting and shadows, Shader Graph, impact
feedback, spatial audio, focus behavior, saving, and performance profiles
across quality levels; the acceptance of those features is owned by the domain
specifications it cites.

Compatibility tiers follow the [Feature catalog](../feature-catalog.md). This
document owns the acceptance of this game under SMP-001.

## Targets and Input Modes

### Targets

* **Automated:** macOS, Windows, and Linux desktop players.
* **Capability-reported:** none.
* **Editor play mode:** yes.
* **Headless editor:** yes.

### Render pipelines

Four project variants with the same scenes, scripts, and puzzles. All four are
accepted, each against its own approved references.

* Built-in Render Pipeline, in Unity 2022.3 LTS forms.
* URP 14.x, Forward path, in Unity 2022.3 LTS forms.
* URP 12.x content in the forms Unity 2021.3 LTS saves, with URP and Shader
  Graph 12.x. This variant is a committed fixture, with its producing Unity
  release recorded.
* URP 10.x content in the forms Unity 2020.3 LTS saves, with URP and Shader
  Graph 10.x. This variant is a committed fixture, with its producing Unity
  release recorded.

As the [Compatibility contract](../compatibility-contract.md) defines, the URP
12.x and 10.x variants open as migrated projects: their serialized forms are
upgraded to the Unity 2022.3 LTS forms (SER-005), and their URP and Shader
Graph content is assessed against the 14.x lines. Behavior of the earlier
lines that differs from 14.x is not matched.

### Input modes and devices

Keyboard and mouse, and gamepad, through one Input System 1.x action asset with
the control schemes KeyboardMouse and Gamepad.

Headless runs use synthetic devices marked synthetic; haptic and feel
checkpoints need physical hardware and are otherwise unevaluable.

## Game Design

### Core loop and flow

1. The title screen shows Continue (when a saved state exists), New Game,
   Records, Settings, and Quit. New Game starts in the dark Workshop with only
   the flashlight.
2. **Workshop.** The fuse sits on a high shelf held by a bracket (a fixed
   joint that breaks at 600 N). The player throws a crate or the 0.5 kg wrench
   at the bracket to drop the fuse, inserts it in the fuse box, and pulls the
   lever. Lights come on room by room at 0.15 s intervals.
3. **Weigh Room.** A spring-loaded pressure plate opens the hinged gate while
   it carries 12 kg or more. The crates weigh 4, 4, and 8 kg. Past the gate, a
   second plate opens the exit, and the crates are still on the first plate.
   The twist: the gate is a hinged physics door with a closing spring, so a
   crate placed in its swing arc wedges it open.
4. **Radio Room.** A radio beacon plays from one of four curtained alcoves,
   muffled by walls. The flag outside the window streams toward the right
   alcove. The radio speaks the first code digit, with a caption.
5. **Vault Antechamber.** The hologram above the keypad shows scrambled
   glyphs to the player. The security monitor shows the room from a wall
   camera that cannot see the hologram. The aha: the camera sits on a swivel
   mount the player can grab and turn. Aimed at the hologram, the monitor shows
   the three real digits, because the true digits render only on a layer the
   security camera sees.
6. Entering the four-digit code opens the vault. The player carries the lumen
   core back to the Workshop reactor socket, and the completion panel shows
   time, room splits, hints used, medals, and Title and Quit.
7. Quit on the title exits the player.

The radio alcove, the flag direction, and all four digits come from a seed
that New Game draws and the saved state records.

### Rules and mechanics

* The player is a `CharacterController` 1.8 m tall with a 2.5 m reach.
  Objects up to 10 kg can be carried. A held object follows a point 1.2 m in
  front of the eyes and drops if it is pushed more than 0.6 m from that point.
* **Interaction priority.** The interaction script raycasts from the view
  center, 2.5 m, against the Interactable and World layers, ignoring triggers
  and the Held layer. If the first hit is interactable it is the target.
  Otherwise a 0.12 m sphere cast along the same ray picks, among unoccluded
  interactables, the highest priority (Button 3, Lever 2, Pickup 1), then the
  nearest. Nothing is targeted through World geometry or lab glass.
* The launcher fires a 0.5 kg canister at 45 m/s. Thrown objects and canisters
  use Continuous Dynamic collision detection.
* Doors are hinge joints with limits 0° to 100°, a closing spring, and a motor
  that switches drive. Plates and the hanging lamp are spring joints; a plate
  counts as pressed at 3 cm of travel. The lever and the camera swivel are
  configurable joints: the lever rotates 0° to 70° and fires at 60°; the swivel
  yaws ±120° and pitches ±30° and holds its pose when released. A drawer is a
  configurable joint with 0.4 m linear travel.
* Each curtain (1.2 x 2.0 m) pins its top row and collides only with the
  player's capsule. The flag (1.0 x 0.6 m) is driven by an external
  acceleration of 6 m/s² toward the seeded alcove and random acceleration of
  2 m/s². The game calls `ClearTransformMotion` after any teleport of a
  cloth's parent and after loading a saved state.
* **Radio occlusion.** Every 0.1 s the radio script raycasts from the listener
  to the radio against World. When occluded, it moves an `AudioLowPassFilter`
  to 1,200 Hz and drops the source 6 dB over 0.2 s; when clear, the cutoff
  returns to 22,000 Hz. Curtains do not occlude.
* Impact sounds scale in volume with impulse from 0.2 to 1.0. Impacts above
  3 m/s leave a mark: on URP 14.x and 12.x, a `DecalProjector` from a pool of
  32 with the DBuffer technique; on Built-in and URP 10.x content, which have
  no decal system in their content, a dust particle burst and an `ImpactMark`
  mesh quad from a pool of 32. Marks fade after 20 s.
* The security camera renders every frame at Medium and above and at 15 Hz at
  Low. The hologram's `_Reveal` rises from 0 to 1 over 1.5 s after power-on,
  and `_GLITCH` is enabled during lockout.

### Levels and progression

| Room | Puzzle | New idea |
|---|---|---|
| Workshop | break the bracket, insert the fuse, pull the lever | grab and throw |
| Weigh Room | 12 kg on the plate, wedge the hinged gate | weight and physical doors |
| Radio Room | find the alcove by ear and the flag | listening and reading the environment |
| Vault Antechamber | turn the security camera to read the digits | seeing what the player cannot |
| Finale | carry the core past the incinerator | all of the above |

Four medals: Gold (under 6 minutes), No Hints, Steady (the fuse and core are
never dropped), and Bullseye (five hidden targets hit with the Weigh Room
pneumatic launcher). A completion unlocks Speedrun mode with a split timer and
instant room restart on Backspace or Select.

### Scoring, failure, and edge rules

* Dropping the fuse or the lumen core into the Workshop incinerator destroys
  it and shows the failure panel "The fuse is gone" or "The core is gone"
  within 0.5 s, with Retry from last room and Quit to title. Retry restores the
  last saved room state with a fresh fuse on the shelf.
* Three wrong codes trigger a 20 s lockout with an alarm and a glitching
  hologram, after which entry resumes.
* Completing a room and Save and Quit write the puzzle state file. Continue
  restores it. A damaged file shows "Saved state is damaged", offers New Game,
  and is not deleted.
* A terminal in each room offers a hint after 60 s in that room. Each hint adds
  30 s to the run time.

### Controls

| Action | Keyboard and mouse | Gamepad |
|---|---|---|
| Move, look | WASD, mouse delta | Left stick, right stick |
| Jump, crouch | Space, Left Ctrl | South button, East button |
| Interact, grab, release | E or left click | West button or right trigger |
| Throw (hold 0.2 s to 1.0 s for 4 m/s to 14 m/s) | Right click | Left trigger |
| Rotate held object | R plus mouse delta | Right bumper plus right stick |
| Flashlight | F | North button |
| Pause | Escape | Start |
| Menus: navigate, confirm, back | Arrows, Tab, pointer; Enter or click; Escape | D-pad or left stick; South; East |

Every gameplay binding except Pause on Escape and Start can be rebound. The
rebinding screen cancels with Escape, excludes pointer position and delta, and
times out after 5 s keeping the old binding. A control already bound in the
Gameplay map opens "E is already used by Interact" with Swap and Cancel.
Overrides save to `PlayerPrefs`; a stale override is reported and ignored;
Reset to defaults clears overrides.

### Pause, focus, and lifecycle

* Pause sets `Time.timeScale` to zero, unlocks and shows the cursor, pauses
  gameplay audio through `AudioListener.pause` (UI sources ignore it), and
  shows Resume, Settings, Save and Quit, and Quit to title.
* Losing window focus pauses the same way, in the same frame. Regaining focus
  leaves the game paused, the click that refocuses the window does not fire
  Interact, and Resume locks the cursor again.
* During play, the cursor is locked and hidden; in menus it is unlocked and
  visible.

### Settings

Master, effects, and voice volume; mouse sensitivity (0.1 to 3.0), invert Y,
field of view (60° to 100°); control rebinding; captions; reduced motion;
display mode, resolution, vertical sync; quality Low, Medium, High, or Ultra.
Settings persist through `PlayerPrefs` and apply at once.

### What makes it fun

* **Hook.** Within 30 s the player sees the fuse lit by the swinging lamp,
  throws a crate, watches the bracket snap and the fuse tumble down, and hears
  it clink on the floor. A replayed beginner path breaks the bracket within
  30 s.
* **Mastery curve.** Better players throw accurately with charged throws,
  stack and wedge without trial and error, read the flag at a glance, and skip
  the fuse box walk by throwing the fuse into the socket's catch funnel from
  across the incinerator pit. Room splits show where time went.
* **Risk and reward.** The cross-pit fuse throw saves about 20 s, but a miss
  destroys the fuse and fails the run. Hints save time when stuck but cost
  30 s each and the No Hints medal. Hitting hidden targets earns the Bullseye
  medal but takes time.
* **Variety and escalation.** Each room adds one idea to the ones before it, and
  the finale carries the core past the incinerator again.
* **Game feel.** Look responds on the first rendered frame after mouse
  movement. A grabbed object eases to the hand in 0.08 s with a click. Impacts
  leave marks. Buttons travel 6 mm with a click and a light. Door creak pitch
  follows angular speed. A correct code flashes green with a chime, and power
  cascades on light by light.
* **Replay.** The four medals; records of the best time and best room splits;
  a new seed each New Game; Speedrun mode after a completion.

## Content

### Assets

| Asset | Source format | Import settings that matter |
|---|---|---|
| `Title`, `Lab`, `Gym_Physics` | Unity YAML scenes | build order, lighting settings, layers per object |
| `Crate4kg`, `Crate8kg` (variant), `Wrench`, `Fuse`, `LumenCore`, `PressurePlate`, `HingeDoor`, `Lever`, `Switch`, `Keypad`, `SecurityCamera`, `Monitor`, `ImpactMark` | Unity YAML prefabs | joints, UnityEvent listeners, variant overrides of mass and material |
| Room shells and props | FBX | collider meshes, scale factor 1, generated normals and tangents |
| `Curtain.fbx`, `Flag.fbx` | FBX, skinned | Cloth component with per-vertex constraints |
| Albedo, normal, and mask textures | PNG and TGA | sRGB albedo, Normal map type, linear masks |
| `Glyphs.png`, `Digits.png` | PNG, alpha is transparency | hologram atlases, point filtering |
| `SecurityFeed.renderTexture` | RenderTexture | 512 x 512, depth 24, sRGB |
| `Hologram.shadergraph`, `Scanlines.shadersubgraph` | Shader Graph | URP target (all URP variants) and Built-in target (Built-in variant), exposed properties, keyword `_GLITCH` |
| Materials | `.mat` (Standard on Built-in, URP Lit elsewhere) | emission on screens, alpha clipping on grates |
| Impacts, voice, radio, ambience | WAV and OGG | Decompress On Load for impacts, Streaming for ambience |
| `Lab.mixer` | AudioMixer | groups Effects, Voice, Ambience, UI, exposed volumes |
| Fonts | TrueType with TextMeshPro SDF assets | captions and UI |
| `Lab.inputactions` | Input System action asset | maps Gameplay and UI, two schemes |
| URP assets and renderer data | URP 14.x, 12.x, and 10.x forms | one URP asset per quality level, Decal renderer feature on 14.x and 12.x |
| `ProjectSettings/` | Unity YAML | layers Interactable, Held, Tool, SecureHolo, HoloPublic; quality levels; fixed time step 0.02 s |

### Packages

Built-in variant: Shader Graph 14.x, Input System 1.x, uGUI 1.0.x, TextMeshPro
3.0.x. URP 14.x variant: URP and Shader Graph 14.x plus the same. URP 12.x and
10.x variants: URP and Shader Graph 12.x or 10.x, migrated and assessed against
the 14.x lines, plus the same. All variants: Unity Test Framework 1.x. Every
variant's manifest pins Input System, uGUI, and TextMeshPro versions that the
Package Manager marks released for Unity 2022.3 LTS. Each package resolves to
unitas's own implementation of that version line
([Packages](../architecture.md#packages)); no Unity-published package code is
vendored in the sample repository, fetched, or compiled.

### Save data

The puzzle state file is a versioned `JsonUtility` file under
`Application.persistentDataPath`. Settings and binding overrides are in
`PlayerPrefs`.

### Licences and provenance

Every asset family's source, author, and licence is recorded in
`Licenses/content-licenses.json`. Unity packages' licence terms are recorded in
the build's licence report.

## Feature Coverage

| ID | Behavior this game exercises | Scenarios |
|---|---|---|
| PRJ-001 | Four variants open in place | S1 |
| PRJ-003 | Stable identities | S1 |
| PRJ-004 | Model, texture, and audio import | S1 |
| PRJ-007 | Layers, quality, time step | S1 |
| SER-005 | Migration of the URP 12.x and 10.x variants | S1 |
| CMP-002 | Matrix report of URP and Shader Graph lines per variant | S1 |
| SER-001 | Serialized joint and event values | S16 |
| SER-003 | Prefab variants | S1 |
| RUN-001 | Object destruction and retry | S14 |
| RUN-002 | Compiled scripts | S1 |
| RUN-003 | `OnApplicationFocus`, `OnJointBreak`, collision messages | S2, S3 |
| RUN-004 | `Time.timeScale` for pause and lockout timers | S3, S14 |
| RUN-008 | Switches, plates, and doors wired through UnityEvents | S7, S16 |
| RUN-009 | Puzzle state file under `Application.persistentDataPath` with `JsonUtility`; seeded randomness | S13 |
| PHY-001 | Rigidbodies, continuous collision, hinge, spring, fixed, and configurable joints, raycasts and sphere casts | S5, S6, S7 |
| PHY-003 | `CharacterController` movement, crouch, stairs | S2 |
| PHY-004 | Cloth curtains and flag | S8 |
| REN-001 | Security camera render texture, culling masks, stacked tool camera | S10 |
| REN-002 | Standard and URP Lit materials, emission, alpha clipping | S15 |
| REN-003 | Real-time point and spot lights with shadows, quality levels | S15 |
| REN-004 | Sparks, steam, impact particles | S9 |
| REN-006 | Shader Graph hologram, URP renderer data, DecalProjector on 14.x and 12.x | S9, S11 |
| REN-007 | Frame rates, vertical sync, display modes | S6, S15 |
| UI-001 | Menus, prompts, keypad, rebinding screen | S2, S4 |
| TXT-001 | Captions and prompts | S2, S12 |
| INP-002 | Actions, cursor lock, focus, rebinding, conflicts, overrides | S3, S4 |
| AUD-001 | Spatial radio with a lowpass filter, impact audio, reverb zone, pause | S3, S12 |
| SAV-001 | Settings, binding overrides, puzzle state, damaged file | S4, S13 |
| PRF-001 | Profiles on the benchmark path at every quality level | S15 |
| ACC-001 | Captions, reduced motion, rebinding, hints | S4, S12 |
| BLD-001 | Packaged players for each variant | S16 |
| BLD-002 | Headless runs | S1–S15 |
| TST-002 | Scenarios, references, input replay | all |
| AUT-002 | Play control through the editor's MCP interface | S1–S15 |
| EDT-004 | Play and stop with edit-state restoration | S16 |

## Engine Behavior This Game Depends On

Only engine and package behavior, each tied to a catalog ID. Game rules are
defined under Game Design and implemented by the sample's own scripts; the
engine contains no code specific to this game. Package behavior (Input System,
Shader Graph, URP) comes from unitas's own implementation of each package's
public API at the pinned version line ([Packages](../architecture.md#packages),
CMP-002). Agents reach the game only through the editor's generic MCP object,
property, method-invocation, input, and capture commands, with no game-specific
command.

### Scripting and runtime

* A UnityEvent invokes its persistent listeners in serialized order, including
  static-argument calls such as `Light.set_enabled` with `true`. A listener
  whose target is missing is reported on load and on invocation, naming the
  owner, field, target, and method, and is not invoked. (RUN-008)
* `OnApplicationFocus(false)` runs on focus loss in the same frame, before
  `Update`. (RUN-003)
* `Random.InitState(seed)` makes the subsequent `Random` sequence reproducible.
  (RUN-009)

### Project and content

* The URP 12.x and 10.x variants are reported as migrated with their declared
  releases. Opening rewrites no asset. The first save of an edit lists the
  assets it upgrades and states that they then need Unity 2022.3 LTS.
  (SER-005)
* The project compatibility report lists each variant's URP and Shader Graph
  package, resolved version, tier, and matrix state. (CMP-002)

### Simulation

* `Physics.Raycast` and `Physics.SphereCast` honor layer masks and
  `QueryTriggerInteraction.Ignore`, and report the first hit along the ray.
  (PHY-001)
* Continuous Dynamic bodies do not pass through 2 cm or thicker colliders at 30,
  60, 144, or 240 frames per second, with the fixed time step at 0.02 s or
  0.01 s, at speeds up to 45 m/s. (PHY-001)
* Hinge joints with limits, springs, and motors; spring joints; fixed joints
  with a break force that calls `OnJointBreak`; and configurable joints with
  angular and linear limits behave as PHY-001 defines. A 4 kg or heavier body
  in a hinged door's swing arc holds it open against its spring. (PHY-001)
* Cloth honors per-vertex maximum distance constraints, capsule colliders,
  external and random acceleration, and `ClearTransformMotion`. (PHY-004)

### Rendering and animation

* A camera rendering to a RenderTexture honors its culling mask and target
  rate. On URP an overlay camera in a base camera's stack, and on Built-in a
  second camera with higher depth and Depth Only clear, draws on top of the
  main view. (REN-001)
* A Shader Graph that is unlit, transparent, and additive, with fresnel and a
  sub graph, renders from its URP target or its Built-in target; properties set
  through a `MaterialPropertyBlock` and keyword `_GLITCH` take effect on the
  next frame. (REN-006)
* A spot light with shadows at every quality level and point lights whose
  shadows depend on the quality level render as REN-003 defines; on URP the
  Forward per-object additional light limit of each URP asset applies. (REN-003)
* `DecalProjector` with the DBuffer technique renders on URP 14.x and 12.x
  content; particle bursts and pooled mesh quads render on all variants.
  (REN-006, REN-004)

### Audio and video

* Logarithmic rolloff from 1 m to 25 m, an `AudioLowPassFilter` whose cutoff is
  set from script, and an `AudioReverbZone` (Room preset, 4 m to 10 m) behave as
  AUD-001 defines. `AudioListener.pause` silences sources except those with
  `ignoreListenerPause`. (AUD-001)

### Input

* `Cursor.lockState` Locked hides and confines the cursor; None restores it.
  A click that refocuses the window is not delivered as an action. (INP-002)
* `PerformInteractiveRebinding` with a cancel control, excluded controls, and a
  timeout; `SaveBindingOverridesAsJson` and `LoadBindingOverridesFromJson`;
  and reporting of overrides whose action no longer exists behave as INP-002
  defines. (INP-002)

### UI and text

* Prompt and caption text set in a frame renders that frame. (TXT-001, UI-001)

### Player and build

* Each variant builds desktop players with a build report. (BLD-001)

### Editor and automation

* Stopping play restores edited values. (EDT-004)
* Through the editor's generic MCP commands an agent invokes the game's own
  seed and room-load methods, reads any component field (the current target
  and its priority, joint angles, plate states, digits found, cursor state),
  and captures frames, audio, and profiles. (AUT-002)

## Presentation and Performance

### Visual checkpoints

Each has a unitas-approved reference per variant and quality level: the dark
Workshop by flashlight, the Workshop after power-on, the Weigh Room with the
wedged gate, the curtains and flag, the hologram from the player's view, the
monitor showing the digits, an impact mark set, and the completion panel.

### Audio checkpoints

The radio is muffled behind a wall and clear in view; the vault reverb is
present only in the vault; impacts start within one frame of contact.

### Project-declared expectations

60 frames per second at 1440p at Ultra and at 1080p at High on the declared
reference desktop class, and at 1080p at Low on the declared minimum desktop
class; no frame over 33 ms when 20 objects impact at once on those classes;
start to title under 3 s. Expectations are evaluated only on the declared
class; other runs record the measurement and report the expectation as
unevaluable (TST-002). These are the sample's own expectations, not the PRF-002
engine budgets.

### PRF-002 measurements

None.

## Acceptance Scenarios

Outcomes use the TST-002 vocabulary: passed, failed, skipped, unevaluable, and
awaiting human review. Each perturbation names its mechanism: a TST-002
replay-stream event, a harness action outside the player, or a
development-build debug command implemented in the sample's scripts. Unless a
scenario says otherwise, it runs for each of the four variants.

### S1. Open and inspect

* **Runs in:** headless editor; editor play mode.
* **Preconditions:** no import cache exists.
* **Steps:** open each variant; reopen it.
* **Expected:** no import errors; the URP 12.x and 10.x variants are reported
  migrated with their declared releases and assessed against URP and Shader
  Graph 14.x; every other package is reported Supported; reopening yields
  identical identities and no asset is rewritten.
* **Demonstrates:** PRJ-001, PRJ-003, PRJ-004, PRJ-007, SER-005, CMP-002

### S2. Happy path

* **Runs in:** editor play mode; headless editor; desktop packaged players.
* **Preconditions:** a fresh profile.
* **Steps:** with seed 1234, replay the recorded solution from New Game.
* **Expected:** the bracket breaks, power cascades on, both plates open their
  doors, the radio digit and the three monitor digits are found, the code opens
  the vault, and the completion panel appears with splits.
* **Demonstrates:** RUN-003, PHY-001, PHY-003, UI-001, TXT-001, INP-002

### S3. Focus loss and recovery

* **Runs in:** editor play mode; headless editor; desktop packaged players.
* **Preconditions:** Workshop in play, holding a crate.
* **Steps:** while pressing W, remove and restore window focus, through a focus
  event in the replay stream (headless and editor) or a harness action that
  changes the OS window focus (packaged players); click the window; then
  Resume.
* **Expected:** the game pauses in the same frame, the cursor unlocks, gameplay
  audio pauses, the crate holds its pose, no key is stuck afterwards, the
  refocusing click fires nothing, and Resume locks the cursor again.
* **Demonstrates:** INP-002, RUN-003, RUN-004, AUD-001

### S4. Rebinding conflicts

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** a fresh profile.
* **Steps:** rebind Throw to E, choose Cancel, then Swap; start a rebind and let
  it time out; press Escape during a rebind; relaunch (a harness action that
  restarts the player, or, headless, a new play session with the same storage
  root); then load an override for a removed action (a committed fixture
  override string).
* **Expected:** Cancel keeps both bindings; Swap exchanges them; the timeout and
  Escape keep the old binding; overrides survive the relaunch; the stale
  override is reported and ignored.
* **Demonstrates:** INP-002, SAV-001, UI-001, ACC-001

### S5. Interaction priority

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** `Gym_Physics`.
* **Steps:** aim at a button with a crate 0.1 m to its side, at a pickup behind
  lab glass, at a lever behind a trigger volume, and at a crate while holding
  another.
* **Expected:** the button wins over the crate, nothing is targeted through
  glass, the lever is targeted through the trigger, and the held crate is
  never the target.
* **Demonstrates:** PHY-001

### S6. Fast objects across frame rates

* **Runs in:** headless editor; desktop packaged players.
* **Preconditions:** `Gym_Physics`.
* **Steps:** fire the launcher at the glass, a wall, and the floor, and throw at
  14 m/s, at 30, 60, 144, and 240 frames per second, with fixed steps of 0.02 s
  and 0.01 s.
* **Expected:** no tunneling in any run; contact points lie on the struck
  surface; the puzzle outcome of S2 is unchanged at every rate.
* **Demonstrates:** PHY-001, REN-007

### S7. Joints and the wedge

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** the saved state "Weigh Room with 12 kg on plate one".
* **Steps:** walk through, take one 4 kg crate back into the swing arc, and step
  off. Break the bracket with a thrown crate.
* **Expected:** the gate opens at 12 kg and closes below it, the wedged gate
  holds 30° or more, `OnJointBreak` runs once, the lever fires at 60°, the
  swivel holds its pose, and the plate's UnityEvents open and close the gate.
* **Demonstrates:** PHY-001, RUN-008

### S8. Cloth

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** the saved state "Radio Room with the gate wedged".
* **Steps:** walk through each curtain and teleport the flag's parent by 5 m
  (a development-build debug command implemented in the sample's scripts).
* **Expected:** the top rows never move from their pins, the curtain parts
  around the player capsule and ignores crates, the flag streams toward the
  seeded alcove, and the teleport adds no swing.
* **Demonstrates:** PHY-004

### S9. Impact feedback per variant

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** `Gym_Physics`.
* **Steps:** throw a crate at a wall at 8 m/s 40 times.
* **Expected:** URP 14.x and 12.x show `DecalProjector` decals; Built-in and URP
  10.x content show a particle burst and an `ImpactMark` quad; at most 32 marks
  exist at once and each fades after 20 s.
* **Demonstrates:** REN-006, REN-004

### S10. Security camera aha

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** the saved state "Antechamber with the swivel facing away".
* **Steps:** read the hologram directly, then grab the swivel and turn it toward
  the hologram.
* **Expected:** the main view shows only scrambled glyphs; the monitor shows the
  three seeded digits once the hologram is in frame; the render texture is
  512 x 512; the tool overlay renders above a wall the player presses against.
* **Demonstrates:** REN-001, PHY-001

### S11. Hologram

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** the saved state "Antechamber with two wrong codes entered".
* **Steps:** power on, then enter a third wrong code.
* **Expected:** `_Reveal` reaches 1 at 1.5 s; `_GLITCH` is on for the 20 s
  lockout and the alarm plays; entry resumes afterwards; each variant matches
  its approved hologram reference.
* **Demonstrates:** REN-006, RUN-004

### S12. Follow the sound

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** Radio Room with captions on.
* **Steps:** stand behind the radio room wall, then in line of sight.
* **Expected:** the cutoff is 1,200 Hz behind the wall and 22,000 Hz in view,
  with a 6 dB difference; the caption shows the digit; the flag points at the
  radio's alcove.
* **Demonstrates:** AUD-001, ACC-001, TXT-001

### S13. Puzzle state file

* **Runs in:** packaged players; headless editor with a play-session restart.
* **Preconditions:** the saved state "Radio Room with the gate wedged", one
  crate on a plate.
* **Steps:** Save and Quit; relaunch (a harness action that restarts the player,
  or, headless, a new play session with the same storage root) and Continue;
  then replace the file with a truncated copy (a harness action on the storage
  root) and relaunch.
* **Expected:** every body, door angle, plate, digit, the seed, and the lockout
  timer are restored, with bodies asleep where they were saved; the truncated
  file shows "Saved state is damaged", offers New Game, and is not deleted.
* **Demonstrates:** SAV-001, RUN-009

### S14. Failure and retry

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** the saved state "Workshop, dark, seed 1234", fuse dropped.
* **Steps:** throw the fuse into the incinerator, then Retry.
* **Expected:** the failure panel appears within 0.5 s, Retry restores the
  Workshop state from the last save with a fresh fuse on the shelf, and no
  object from the failed attempt remains.
* **Demonstrates:** RUN-001, RUN-004

### S15. Quality levels and profiles

* **Runs in:** headless editor; desktop packaged players.
* **Preconditions:** the 60 s benchmark path.
* **Steps:** run the benchmark path at Low, Medium, High, and Ultra, and capture
  checkpoints.
* **Expected:** shadow casters, resolution, and light counts follow each level;
  each capture matches its reference; each PRF-001 profile records the quality
  level, target, and scene and attributes cloth, physics, and the security
  camera render. The project-declared expectations are evaluated on their
  declared classes and reported unevaluable elsewhere.
* **Demonstrates:** REN-003, PRF-001, REN-002, REN-007, TST-002

### S16. Medals, missing listener, and package

* **Runs in:** editor play mode (visual and headless); headless editor (build);
  desktop packaged players.
* **Preconditions:** a fresh profile.
* **Steps:** replay a sub-6-minute solution with no hints and relaunch; remove
  the target of one switch listener in the editor, play, and stop; then build
  each variant.
* **Expected:** Gold, No Hints, and Steady are awarded, persist across the
  relaunch, and Speedrun mode unlocks; the missing listener is reported and not
  invoked; stopping play restores edited values; each player completes S2 by
  replay with a build report.
* **Demonstrates:** SAV-001, RUN-008, EDT-004, BLD-001, SER-001

## Manual Playtest Guidance

A person judges the following, and a headless run reports these checkpoints as
awaiting human review, never as passed:

* Whether the first throw lands as a satisfying moment within 30 seconds.
* Whether the door wedge and the camera swivel each produce an "aha" without a
  hint.
* Whether the puzzles escalate without feeling unfair.
* Whether grabbing and throwing feel responsive.
* Whether the medals invite another run.
* Look comfort at each field of view, the occlusion's realism, and the impact
  sounds.

## Agent Authoring Change Requests

### CR1. Valve wheel

* **Request:** add a valve wheel on a hinge joint that must turn 720° to drain
  a flooded floor panel and reveal a fifth code digit, wired through
  UnityEvents.
* **Acceptance scenario:** the valve drains the panel only after 720°, the digit
  appears on the monitor, and the puzzle state file restores the valve angle.
* **Test:** a test of the valve's angle tracking.

### CR2. Breakable glass

* **Request:** add a breakable glass pane on a fixed joint with a shatter sound
  and impact mark.
* **Acceptance scenario:** the launcher breaks the pane and a 4 m/s throw does
  not.
* **Test:** a test that applies both impacts and checks the joint state.

## Accessibility

* Every action works with the keyboard and mouse or the gamepad alone, and
  every gameplay binding can be rebound (S4).
* Captions show the radio and alarm with the speaker named (S12).
* The flag clue is also stated by the hint terminal (S12).
* Reduced motion removes head bob and the lamp's camera-visible sway (S15 with
  reduced motion on).
* Field of view and look sensitivity are adjustable (S15).

## Saved and Reproducible States

| State | How it is reached | Used by |
|---|---|---|
| Workshop, dark, seed 1234 | save file | S14 |
| Weigh Room with 12 kg on plate one | save file | S7 |
| Radio Room with the gate wedged | save file | S8, S13 |
| Antechamber with the swivel facing away | save file | S10 |
| Antechamber with two wrong codes entered | save file | S11 |
| A damaged puzzle state file | committed fixture file | S13 |

## Known Limitations and Unsupported Conditions

* The URP 12.x and 10.x variants are assessed against the URP and Shader Graph
  14.x lines. Behavior of those earlier lines that differs from 14.x is not
  matched, and saved edits write Unity 2022.3 LTS forms.
* Built-in Render Pipeline projects have no decal system, so the Built-in
  variant uses particle and mesh feedback by design.

## Acceptance Outcomes

* The first-person interaction sample recovers from focus loss, resolves input
  rebinding conflicts, and keeps fast-moving objects from passing through
  colliders. (SMP-001, INP-002, PHY-001)
* The first-person interaction sample shows `DecalProjector` impact decals on
  URP 12.x and 14.x and particle and mesh feedback on the Built-in Render
  Pipeline and URP 10.x. (SMP-001, REN-006)
* The same solution completes on all four variants at 30 to 240 frames per
  second, with the URP 12.x and 10.x variants migrated and assessed against
  the 14.x lines. (SMP-001, PHY-001, SER-005, CMP-002)
* Hinge, spring, fixed, and configurable joints, cloth, and raycast priority
  behave as declared, and switches and doors act through UnityEvents.
  (SMP-001, PHY-001, PHY-004, RUN-008)
* The security monitor shows the render texture view that unlocks the code,
  the tool overlay stays on top, and the hologram matches its references.
  (SMP-001, REN-001, REN-006)
* Real-time point and spot shadows follow each quality level, and each level
  has a profile. (SMP-001, REN-003, PRF-001)
* The radio is muffled when occluded, the puzzle state file restores every
  body and door, and medals and unlocks persist. (SMP-001, AUD-001, SAV-001)
