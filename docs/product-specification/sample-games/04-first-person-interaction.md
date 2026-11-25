# Sample 4: First-Person Interaction

## Overview

The player is the night technician at a small research lab after a power
failure. In first person, they move through four compact rooms, grab, carry,
stack, and throw objects, work switches, levers, and doors, follow a radio
signal by ear, and crack the vault code. Each room is one physical puzzle that
reuses what the last one taught, with a twist. Carrying the lumen core from the
vault to the reactor socket restores the lab and completes the game. A first
run takes 10 to 15 minutes, and a skilled run under 6.

The game is original, redistributable unitas content and uses no Unity-owned
sample content. It proves pointer look and cursor capture, rebinding, physics
queries, dynamic bodies, joints, cloth, cameras and render textures, real-time
lighting and shadows, Shader Graph, impact feedback, spatial audio, focus
behavior, saving, and performance profiles across quality levels.

Compatibility tiers follow the [Feature catalog](../feature-catalog.md). This
document owns the acceptance of this game under SMP-001.

## Targets and Input Modes

* Targets: macOS, Windows, and Linux desktop players, plus the editor's play
  mode and the headless editor.
* Render pipelines: four project variants with the same scenes, scripts, and
  puzzles. All four are accepted, each against its own approved references.
  * Built-in Render Pipeline, saved by Unity 2022.3 LTS.
  * URP 14.x, Forward path, saved by Unity 2022.3 LTS.
  * URP 12.x content, saved by Unity 2021.3 LTS with URP and Shader Graph 12.x.
  * URP 10.x content, saved by Unity 2020.3 LTS with URP and Shader Graph 10.x.

  As the [Compatibility contract](../compatibility-contract.md) defines, the
  URP 12.x and 10.x variants open as migrated projects: their serialized forms
  are upgraded to the Unity 2022.3 LTS forms (SER-005), and their URP and
  Shader Graph content is assessed against the 14.x lines. Behavior of the
  earlier lines that differs from 14.x is not matched. Every variant's
  manifest pins Input System 1.x, uGUI 1.0.x, and TextMeshPro 3.0.x versions
  that the Package Manager marks released for Unity 2022.3 LTS.
* Input modes: keyboard and mouse, and gamepad, through one Input System 1.x
  action asset with the control schemes KeyboardMouse and Gamepad.

## Game Design

### Core loop

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

The radio alcove, the flag direction, and all four digits come from a seed
that New Game draws and the saved state records.

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

Every gameplay binding except Pause on Escape and Start can be rebound.

### Rules

* The player is a `CharacterController` 1.8 m tall with a 2.5 m reach.
  Objects up to 10 kg can be carried. A held object follows a point 1.2 m in
  front of the eyes and drops if it is pushed more than 0.6 m from that point.
* **Failure.** Dropping the fuse or the lumen core into the Workshop
  incinerator destroys it and shows the failure panel "The fuse is gone" or
  "The core is gone", with Retry from last room and Quit to title. Three wrong
  codes trigger a 20 s lockout with an alarm and a glitching hologram, after
  which entry resumes.
* **Saving.** Completing a room and Save and Quit write the puzzle state file.
  Continue restores it. Settings and bindings are saved separately.
* **Hints.** A terminal in each room offers a hint after 60 s in that room.
  Each hint adds 30 s to the run time.
* **Pause and focus.** Pause sets `Time.timeScale` to zero, unlocks and shows
  the cursor, pauses gameplay audio, and shows Resume, Settings, Save and
  Quit, and Quit to title. Losing window focus pauses the same way. Regaining
  focus leaves the game paused, and Resume locks the cursor again.
* **Settings.** Master, effects, and voice volume; mouse sensitivity (0.1 to
  3.0), invert Y, field of view (60° to 100°); control rebinding; captions;
  reduced motion; display mode, resolution, vertical sync; quality Low,
  Medium, High, or Ultra. Settings persist through `PlayerPrefs` and apply at
  once.

### What makes it fun

* **Hook.** Within 30 s the player sees the fuse lit by the swinging lamp,
  throws a crate, watches the bracket snap and the fuse tumble down, and hears
  it clink on the floor. A replayed beginner path breaks the bracket within
  30 s.
* **Mastery.** Better players throw accurately with charged throws, stack and
  wedge without trial and error, read the flag at a glance, and skip the
  fuse box walk by throwing the fuse into the socket's catch funnel from
  across the incinerator pit. Room splits show where time went.
* **Risk and reward.** The cross-pit fuse throw saves about 20 s, but a miss
  destroys the fuse and fails the run. Hints save time when stuck but cost
  30 s each and the No Hints medal. Firing the Weigh Room pneumatic launcher at
  hidden targets earns the Bullseye medal but takes time.
* **Escalation.** Each room adds one idea to the ones before it: grab and
  throw, then weight and physical doors, then listening and reading the
  environment, then using a camera to see what the player cannot. The finale
  carries the core past the incinerator again.
* **Juice.** Look responds on the first rendered frame after mouse movement.
  A grabbed object eases to the hand in 0.08 s with a click. Impact sounds
  scale in volume with impulse. Impacts leave decals or particle and mesh
  marks. Buttons travel 6 mm with a click and a light. Door creak pitch follows
  angular speed. A correct code flashes green with a chime, and power
  cascades on light by light.
* **Replay.** Four medals: Gold (under 6 minutes), No Hints, Steady (the fuse
  and core are never dropped), and Bullseye (five hidden targets hit). Records
  keep the best time and best room splits. The seed changes the code, alcove,
  and flag each New Game. A completion unlocks Speedrun mode with a split
  timer and instant room restart on Backspace or Select.

## Content

| Asset | Source format | What matters on import |
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

Packages: Built-in variant: Shader Graph 14.x, Input System 1.x, uGUI 1.0.x,
TextMeshPro 3.0.x. URP 14.x variant: URP and Shader Graph 14.x plus the same.
URP 12.x and 10.x variants: URP and Shader Graph 12.x or 10.x, migrated and
assessed against the 14.x lines, plus the same. All variants: Unity Test
Framework 1.x.

## Feature Coverage

| ID | Behavior this game exercises | Scenarios |
|---|---|---|
| PRJ-001, PRJ-003, PRJ-007 | Four variants open in place; stable identities; layers, quality, time step | S1 |
| PRJ-004, SER-005 | Model, texture, and audio import; migration of the URP 12.x and 10.x variants | S1 |
| CMP-002 | Matrix report of URP and Shader Graph lines per variant | S1 |
| SER-001, SER-003 | Prefab variants, serialized joint and event values | S1, S16 |
| RUN-001, RUN-002, RUN-003 | Objects, scripts, `OnApplicationFocus`, `OnJointBreak`, collision messages | S2, S3 |
| RUN-004 | `Time.timeScale` for pause and lockout timers | S3, S14 |
| RUN-008 | Switches, plates, and doors wired through UnityEvents | S7, S16 |
| RUN-009 | Puzzle state file under `Application.persistentDataPath` with `JsonUtility`; seeded randomness | S13 |
| PHY-001 | Rigidbodies, continuous collision, hinge, spring, fixed, and configurable joints, raycast priority | S5, S6, S7 |
| PHY-003 | `CharacterController` movement, crouch, stairs | S2 |
| PHY-004 | Cloth curtains and flag | S8 |
| REN-001 | Security camera render texture, culling masks, stacked tool camera | S10 |
| REN-002 | Standard and URP Lit materials, emission, alpha clipping | S15 |
| REN-003 | Real-time point and spot lights with shadows, quality levels | S15 |
| REN-004 | Sparks, steam, impact particles | S9 |
| REN-006 | Shader Graph hologram, URP renderer data, DecalProjector on 14.x and 12.x | S9, S11 |
| REN-007 | Frame rates, vertical sync, display modes | S6, S15 |
| UI-001, TXT-001 | Menus, prompts, captions, keypad, rebinding screen | S2, S4 |
| INP-002 | Actions, cursor lock, focus, rebinding, conflicts, overrides | S3, S4 |
| AUD-001 | Spatial radio with occlusion, impact audio, reverb zone, pause | S3, S12 |
| SAV-001 | Settings, binding overrides, puzzle state, damaged file | S4, S13 |
| PRF-001 | Profiles on the benchmark path at every quality level | S15 |
| ACC-001 | Captions, reduced motion, rebinding, hints | S4, S12 |
| BLD-001 | Packaged players for each variant | S16 |
| BLD-002, TST-002, AUT-002 | Headless runs, scenarios, references, input replay, MCP control | all |
| EDT-004 | Play and stop with edit-state restoration | S16 |

## Engine Behavior This Game Depends On

### Scripting and runtime

* A switch, plate, or lever invokes its UnityEvent persistent listeners in
  serialized order, for example `HingeDoor.Open`, `Light.set_enabled` with a
  static `true`, and `AudioSource.Play`. A listener whose target is missing is
  reported on load and on invocation, naming the owner, field, target, and
  method, and is not invoked.
* `OnApplicationFocus(false)` runs on focus loss, and the game's pause runs in
  the same frame.
* `Random.InitState(seed)` with the saved seed reproduces the same alcove,
  flag direction, and digits.

### Project and content

* The URP 12.x and 10.x variants are reported as migrated with their declared
  releases. Opening rewrites no asset. The first save of an edit lists the
  assets it upgrades and states that they then need Unity 2022.3 LTS.
* The project compatibility report lists each variant's URP and Shader Graph
  package, resolved version, tier, and matrix state.

### Simulation

* Interaction priority: a raycast from the view center, 2.5 m, against
  Interactable and World layers, ignoring triggers and the Held layer. If the
  first hit is interactable it is the target. Otherwise a 0.12 m sphere cast
  along the same ray picks, among unoccluded interactables, the highest
  priority (Button 3, Lever 2, Pickup 1), then the nearest. Nothing is
  targeted through World geometry or lab glass.
* Thrown objects and launcher canisters use Continuous Dynamic. The launcher
  fires a 0.5 kg canister at 45 m/s. No object passes through the 2 cm lab
  glass, 5 cm walls, or the floor at 30, 60, 144, or 240 frames per second,
  with the fixed time step at 0.02 s or 0.01 s.
* Joints: doors are hinge joints with limits 0° to 100°, a closing spring, and
  a motor that switches drive. Plates and the hanging lamp are spring joints;
  a plate counts as pressed at 3 cm of travel. The shelf bracket is a fixed
  joint with break force 600 N and calls `OnJointBreak`. The lever and the
  camera swivel are configurable joints: the lever rotates 0° to 70° and
  fires at 60°; the swivel yaws ±120° and pitches ±30° and holds its pose when
  released. A drawer is a configurable joint with 0.4 m linear travel.
* A crate of 4 kg or more in the gate's swing arc keeps it 30° or more open.
* Cloth: each curtain (1.2 x 2.0 m) pins its top row at maximum distance 0 and
  collides only with the player's capsule collider. The flag (1.0 x 0.6 m)
  uses external acceleration of 6 m/s² toward the seeded alcove and random
  acceleration of 2 m/s². Loading a saved state calls `ClearTransformMotion`.

### Rendering and animation

* The security camera renders to `SecurityFeed` every frame at Medium and
  above and at 15 Hz at Low. Its culling mask includes SecureHolo and excludes
  HoloPublic. The main camera's mask is the reverse.
* The tool camera renders the Tool layer at 55° field of view on top of the
  main view: as an overlay camera in the base camera's stack on URP, and as a
  second camera with higher depth and Depth Only clear on Built-in. The held
  flashlight never clips into a wall the player presses against.
* The hologram graph is unlit, transparent, and additive, with fresnel and a
  scanline sub graph. `_Reveal` rises from 0 to 1 over 1.5 s after power-on
  through a `MaterialPropertyBlock`, and `_GLITCH` is enabled during lockout.
  The Built-in variant uses the graph's Built-in target.
* Lights: the flashlight is a spot light (45°, 12 m) with shadows at every
  quality level. The swinging lamp and four ceiling lights are point lights
  whose shadows depend on the quality level. On URP, the Forward per-object
  additional light limit of each URP asset applies.
* Impact feedback above 3 m/s: on URP 14.x and 12.x, a `DecalProjector` from
  a pool of 32 with the DBuffer technique, fading after 20 s; on Built-in and
  URP 10.x content, which have no decal system in their content, a dust
  particle burst and an `ImpactMark` mesh quad from a pool of 32, fading after
  20 s. All variants emit sparks from the fuse box and steam from vents.

### Audio

* The radio rolls off logarithmically from 1 m to 25 m. Every 0.1 s the game
  raycasts from the listener to the radio against World. When occluded, an
  `AudioLowPassFilter` moves to 1,200 Hz and the source drops 6 dB over 0.2 s;
  when clear, the cutoff returns to 22,000 Hz. Curtains do not occlude.
* An `AudioReverbZone` in the vault (Room preset, 4 m to 10 m) applies there.
  Impact volume follows impulse from 0.2 to 1.0. While paused,
  `AudioListener.pause` silences gameplay, and UI sources ignore it.

### Input

* During play, `Cursor.lockState` is Locked and the cursor hidden; in menus it
  is None and visible. The click that refocuses the window does not fire
  Interact.
* Rebinding uses `PerformInteractiveRebinding` with Escape as the cancel
  control, pointer position and delta excluded, and a 5 s timeout that keeps
  the old binding. A control already bound in the Gameplay map opens "E is
  already used by Interact" with Swap and Cancel. Overrides save with
  `SaveBindingOverridesAsJson` to `PlayerPrefs`. A stale override is reported
  and ignored. Reset to defaults clears overrides.

### UI, text, and build

* Prompts name the target ("Grab Crate", "Press Switch") on the frame it is
  targeted. Captions show the speaker and the radio's line.
* Each variant builds desktop players with a build report.
* Through MCP an agent can set the seed, teleport to a room, read the current
  target and its priority, joint angles, plate states, digits found, and
  cursor state, and capture frames, audio, and profiles.

## Presentation and Performance

* Visual checkpoints with unitas-approved references per variant and quality
  level: the dark Workshop by flashlight, the Workshop after power-on, the
  Weigh Room with the wedged gate, the curtains and flag, the hologram from
  the player's view, the monitor showing the digits, an impact mark set, and
  the completion panel.
* Audio checkpoints: the radio is muffled behind a wall and clear in view; the
  vault reverb is present only in the vault; impacts start within one frame of
  contact.
* Project-declared expectations: 60 frames per second at 1440p at Ultra and at
  1080p at High on the declared reference desktop class, and at 1080p at Low
  on the declared minimum desktop class; no frame over 33 ms when 20 objects
  impact at once; start to title under 3 s. These are the sample's own
  expectations, not the PRF-002 engine budgets.

## Acceptance Scenarios

Every scenario runs in editor play mode, in a packaged player, and headless
through MCP, for each of the four variants, unless it says otherwise.

* **S1. Open and inspect.** No cache exists. Open each variant. Expected: no
  import errors; the URP 12.x and 10.x variants are reported migrated with
  their declared releases and assessed against URP and Shader Graph 14.x;
  every other package is reported Supported; reopening yields identical
  identities and no asset is rewritten. (PRJ-001, PRJ-003, PRJ-004, PRJ-007,
  SER-005, CMP-002)
* **S2. Happy path.** With seed 1234, replay the recorded solution from New
  Game. Expected: the bracket breaks, power cascades on, both plates open
  their doors, the radio digit and the three monitor digits are found, the
  code opens the vault, and the completion panel appears with splits. (RUN-003,
  PHY-001, PHY-003, UI-001, INP-002)
* **S3. Focus loss and recovery.** During play, while holding a crate and
  pressing W, remove and restore window focus; click the window; then Resume.
  Expected: the game pauses in the same frame, the cursor unlocks, gameplay
  audio pauses, the crate holds its pose, no key is stuck afterwards, the
  refocusing click fires nothing, and Resume locks the cursor again. (INP-002,
  RUN-003, RUN-004, AUD-001)
* **S4. Rebinding conflicts.** Rebind Throw to E, choose Cancel, then Swap;
  start a rebind and let it time out; press Escape during a rebind; relaunch;
  then load an override for a removed action. Expected: Cancel keeps both
  bindings; Swap exchanges them; the timeout and Escape keep the old binding;
  overrides survive the relaunch; the stale override is reported and ignored.
  (INP-002, SAV-001, UI-001, ACC-001)
* **S5. Interaction priority.** In `Gym_Physics`, aim at a button with a
  crate 0.1 m to its side, at a pickup behind lab glass, at a lever behind a
  trigger volume, and at a crate while holding another. Expected: the button
  wins over the crate, nothing is targeted through glass, the lever is
  targeted through the trigger, and the held crate is never the target.
  (PHY-001)
* **S6. Fast objects across frame rates.** Fire the launcher at the glass, a
  wall, and the floor, and throw at 14 m/s, at 30, 60, 144, and 240 frames per
  second, with fixed steps of 0.02 s and 0.01 s. Expected: no tunneling in any
  run; contact points lie on the struck surface; the puzzle outcome of S2 is
  unchanged at every rate. (PHY-001, REN-007)
* **S7. Joints and the wedge.** Load the Weigh Room state. Put 12 kg on plate
  one, walk through, take one 4 kg crate back into the swing arc, and step
  off. Break the bracket with a thrown crate. Expected: the gate opens at
  12 kg and closes below it, the wedged gate holds 30° or more, `OnJointBreak`
  runs once, the lever fires at 60°, the swivel holds its pose, and the
  plate's UnityEvents open and close the gate. (PHY-001, RUN-008)
* **S8. Cloth.** Walk through each curtain and teleport the flag's parent by
  5 m. Expected: the top rows never move from their pins, the curtain parts
  around the player capsule and ignores crates, the flag streams toward the
  seeded alcove, and the teleport adds no swing. (PHY-004)
* **S9. Impact feedback per variant.** Throw a crate at a wall at 8 m/s 40
  times. Expected: URP 14.x and 12.x show `DecalProjector` decals; Built-in and
  URP 10.x content show a particle burst and an `ImpactMark` quad; at most 32
  marks exist at once and each fades after 20 s. (REN-006, REN-004)
* **S10. Security camera aha.** In the antechamber, read the hologram directly,
  then grab the swivel and turn it toward the hologram. Expected: the main
  view shows only scrambled glyphs; the monitor shows the three seeded digits
  once the hologram is in frame; the render texture is 512 x 512; the tool
  overlay renders above a wall the player presses against. (REN-001, PHY-001)
* **S11. Hologram.** Power on, then enter three wrong codes. Expected: `_Reveal`
  reaches 1 at 1.5 s; `_GLITCH` is on for the 20 s lockout and the alarm plays;
  entry resumes afterwards; each variant matches its approved hologram
  reference. (REN-006, RUN-004)
* **S12. Follow the sound.** With captions on, stand behind the radio room wall,
  then in line of sight. Expected: the cutoff is 1,200 Hz behind the wall and
  22,000 Hz in view, with a 6 dB difference; the caption shows the digit; the
  flag points at the radio's alcove. (AUD-001, ACC-001)
* **S13. Puzzle state file.** Save and Quit in the Radio Room with the gate
  wedged and one crate on a plate; Continue; then replace the file with a
  truncated copy. Expected: every body, door angle, plate, digit, the seed, and
  the lockout timer are restored, with bodies asleep where they were saved;
  the truncated file shows "Saved state is damaged", offers New Game, and is
  not deleted. (SAV-001, RUN-009)
* **S14. Failure and retry.** Throw the fuse into the incinerator, then Retry.
  Expected: the failure panel appears within 0.5 s, Retry restores the
  Workshop state from the last save with a fresh fuse on the shelf, and no
  object from the failed attempt remains. (RUN-001, RUN-004)
* **S15. Quality levels and profiles.** Run the 60 s benchmark path at Low,
  Medium, High, and Ultra, and capture checkpoints. Expected: shadow casters,
  resolution, and light counts follow each level; each capture matches its
  reference; each PRF-001 profile records the quality level, target, and
  scene and attributes cloth, physics, and the security camera render; the
  declared expectations hold on their classes. (REN-003, PRF-001, REN-002,
  REN-007, TST-002)
* **S16. Medals, missing listener, and package.** Replay a sub-6-minute
  solution with no hints, then remove the target of one switch listener in
  the editor, play, and stop; then build each variant. Expected: Gold, No
  Hints, and Steady are awarded, persist across a relaunch, and Speedrun mode
  unlocks; the missing listener is reported and not invoked; stopping play
  restores edited values; each player completes S2 by replay with a build
  report. (SAV-001, RUN-008, EDT-004, BLD-001)

## Manual Playtest Guidance

A person judges whether the first throw lands as a satisfying moment within
30 seconds, whether the door wedge and the camera swivel each produce an "aha"
without a hint, whether the puzzles escalate without feeling unfair, whether
grabbing and throwing feel responsive, and whether the medals invite another
run. They also judge look comfort at each field of view, the occlusion's
realism, and the impact sounds. A headless run reports these checkpoints as
awaiting human review, never as passed.

## Agent Authoring Change Requests

Add a new interactable: a valve wheel on a hinge joint that must turn 720° to
drain a flooded floor panel and reveal a fifth code digit, wired through
UnityEvents. Its scenario proves that the valve drains the panel only after
720°, that the digit appears on the monitor, and that the puzzle state file
restores the valve angle. It comes with a test of the valve's angle tracking.

A second change request adds a breakable glass pane on a fixed joint with a
shatter sound and impact mark, with a scenario proving that the launcher breaks
it and that a 4 m/s throw does not.

## Accessibility

Every action works with the keyboard and mouse or the gamepad alone, and every
gameplay binding can be rebound. Captions show the radio and alarm with the
speaker named. The flag clue is also stated by the hint terminal. Reduced
motion removes head bob and the lamp's camera-visible sway. Field of view and
look sensitivity are adjustable.

## Saved and Reproducible States

* Workshop, dark, seed 1234.
* Weigh Room with 12 kg on plate one.
* Radio Room with the gate wedged.
* Antechamber with the swivel facing away and two wrong codes entered.
* A damaged puzzle state file.

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
  has a profile and holds its declared expectations. (SMP-001, REN-003,
  PRF-001)
* The radio is muffled when occluded, the puzzle state file restores every
  body and door, and medals and unlocks persist. (SMP-001, AUD-001, SAV-001)
