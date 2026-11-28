# Sample 3: Third-Person Adventure ("Hollow Light")

## Overview

A storm called the Hollowing is draining the light from a valley. The player
guides a hero and a small companion through three connected areas, a meadow,
ruins, and a cavern, to recover three sigil stones and relight the cavern
altar. The hero walks, runs, jumps, dodges, rides moving platforms, interacts
with objects, and reaches checkpoints, while husk enemies hunt by sight and
sound. Lighting the altar starts a 45-second escape through a collapsing
tunnel. A first run takes 20 to 30 minutes, and a skilled run under 15.

The game is original, redistributable unitas content and uses no Unity-owned
sample content. It is one of the demonstrations of imported humanoid
characters, animation, cinematics, character movement, navigation, terrain,
baked lighting, video, localization, streaming, and persistence working
together in one playable loop; the acceptance of those features is owned by
the domain specifications it cites.

Compatibility tiers follow the [Feature catalog](../feature-catalog.md). This
document owns the acceptance of this game under SMP-001.

## Targets and Input Modes

### Targets

* **Automated:** macOS, Windows, and Linux desktop players.
* **Capability-reported:** gamepad rumble, which depends on the gamepad's
  motors.
* **Editor play mode:** yes.
* **Headless editor:** yes.

### Render pipelines

URP 14.x, Forward path, one URP asset per quality level.

### Input modes and devices

Keyboard and mouse, and gamepad, through one Input System 1.x action asset with
the control schemes KeyboardMouse and Gamepad. Active input handling is Input
System only.

Headless runs use synthetic devices marked synthetic; haptic and feel
checkpoints need physical hardware and are otherwise unevaluable.

## Game Design

### Core loop and flow

1. The title screen shows Continue (when a save exists), New Game, Load Game,
   Chapters (after a completion), Records, Settings, and Quit.
2. New Game plays the opening cutscene (75 s), then gives control at once.
3. In each area the player crosses terrain, solves one interaction (a lever
   bridge, a timed platform ring, a brazier sequence), and takes the sigil. A
   carried sigil is held in both hands by the upper body.
4. Area portals stream the next area in and the previous one out with no
   loading screen. The mid-game cutscene (40 s) plays after the second sigil.
5. Placing the third sigil starts the escape. Reaching daylight shows the
   completion panel with time, shards, hearts lost, medals, and Title and Quit.
6. Quit on the title exits the player.

### Rules and mechanics

* Walk 2.0 m/s, run 5.5 m/s. A standing jump clears 2.2 m and a running jump
  4.0 m. A dodge moves 3 m in 0.4 s, ignores hits for its first 0.3 s, and has
  a 1.0 s cooldown. Ground locomotion is root motion.
* Husks hear a running hero within 10 m and a walking one within 4 m, see 14 m
  in a 90° cone, give up after 6 s without contact, and never enter the
  Sanctuary area around checkpoints. A lit brazier repels husks within 6 m.
* The companion warps beside the hero when more than 25 m away or on respawn.
  Husks cross off-mesh links in a scripted 0.6 s arc.
* The footstep script accepts a `Footstep` event only from a clip with blend
  weight 0.5 or more, so each foot contact plays one grass, stone, or water
  sound, chosen by the dominant splat layer or physic material, and emits 6
  dust particles on grass and dirt.
* Disconnecting the gamepad in use pauses with "Controller disconnected";
  reconnecting it or using the keyboard dismisses it.

### Levels and progression

| Area | Interaction | Shards | Husks | New elements |
|---|---|---|---|---|
| Meadow | lever bridge | 8 | 0 | traversal, terrain, waterfall |
| Ruins | timed platform ring | 8 | 2 | timed platforms, video screen |
| Cavern | brazier sequence | 8 | 6 | stealth, braziers that carve safe paths |
| Escape | none | 0 | every remaining husk | falling stones, no checkpoint, music rising each 15 s |

Four medals per run: Gold (under 15 minutes), Ember (all 24 shards), Untouched
(no heart lost), and Unseen (no husk chase in the cavern). A completion unlocks
Chapters and Hollow mode (one heart, no shard hearts, husks 20% faster).

### Scoring, failure, and edge rules

* The hero starts with three hearts, up to six. A husk hit or a chasm fall
  costs one heart and respawns the hero at the last checkpoint after a 1.5 s
  fade. At zero hearts the failure panel "You were overcome" offers Retry from
  checkpoint, Load Game, and Quit to title. Retry restores three hearts with no
  husk still chasing.
* Shards stay unbanked until the next checkpoint. Dying leaves the unbanked
  shards in a glowing cache where the hero fell, and dying again before
  reaching the cache loses them. Every 8 banked shards add a heart, up to six.
* Checkpoints autosave to the current slot. The pause menu saves to any of
  three slots. Saving is refused, with "You can't save now", while airborne,
  in a cutscene, during the escape, or while a husk is chasing.
* A damaged slot shows "Damaged" and cannot be loaded. A slot from a future
  version is reported incompatible and left unchanged. A version 1 slot
  migrates to version 2 with progress kept, and the migration is reported.

### Controls

| Action | Keyboard and mouse | Gamepad |
|---|---|---|
| Move, camera | WASD, mouse delta | Left stick, right stick |
| Run (hold, or toggle in settings) | Left Shift | Left stick press |
| Jump, dodge | Space, Left Ctrl | South button, East button |
| Interact, pick up, place | E | West button |
| Skip cutscene (hold 1.0 s) | Space | South button |
| Pause | Escape | Start |
| Menus: navigate, confirm, back | Arrows, Tab, pointer; Enter or click; Escape | D-pad or left stick; South; East |

### Pause, focus, and lifecycle

Pause sets `Time.timeScale` to zero and shows Resume, Save, Load, Settings, and
Quit to title, plus Skip cutscene during a cutscene. The game's scripts pause
Timeline, video, animators, particles, agents, and gameplay audio, and move the
mixer to the Paused snapshot while menu sounds play.

### Settings

Five volume sliders (master, music, effects, ambience, dialogue); subtitles on
or off and size; language (English, French, German, Japanese); camera
sensitivity, invert Y, run hold or toggle; reduced motion; display mode,
resolution, vertical sync; quality Low, Medium, or High. They persist through
`PlayerPrefs` and apply at once. The language persists through the
Localization player preference selector.

### What makes it fun

* **Hook.** The opening cutscene ends with the ledge under the hero cracking.
  Control starts within one frame of the end signal, and stones fall 1.5 s
  after the hero first touches them. A 20-second sprint and jump run reaches
  the first checkpoint, where the first sigil's glow is visible across the
  valley.
* **Mastery curve.** Better players chain running jumps, walk to slip past
  husks unheard, dodge through attacks, time the ruins platform ring, and find
  shortcuts that skip about 90 s of the meadow and ruins.
* **Risk and reward.** Each area hides 8 ember shards (24 in all) on risky
  routes: ledges over chasms, husk patrols, and crumbling stones. The shard
  cache makes every death after a risky pickup a gamble.
* **Variety and escalation.** The meadow is traversal only. The ruins add timed
  platforms and two husks. The cavern adds six husks, stealth, and braziers.
  The escape combines falling stones and every remaining husk.
* **Game feel.** Movement responds on the first rendered frame after input.
  Landings from above 2 m dip the camera 0.1 m and puff dust. A husk hit adds
  80 ms of hit-stop, a red vignette, and 0.2 s of gamepad rumble. Taking a
  sigil plays a light burst, a music stinger, and 0.5 s at time scale 0.3.
  Each shard collected within 3 s of the last plays one semitone higher, up to
  eight steps.
* **Replay.** The four medals; records of the best time and most shards per
  slot; Chapters and Hollow mode after a completion.

## Content

### Assets

| Asset | Source format | Import settings that matter |
|---|---|---|
| `Title`, `Core`, `Area_Meadow`, `Area_Ruins`, `Area_Cavern`, `Gym_Traversal` | Unity YAML scenes | build order, additive loading, per-area lighting settings and baked data |
| `Hero`, `Companion`, `Husk`, `HuskSwift` (variant), `MovingPlatform`, `FallingStone`, `Checkpoint`, `Shard`, `Brazier`, `AreaPortal` | Unity YAML prefabs | nesting, variant overrides of speed and material |
| `Hero.fbx` | FBX, Humanoid, Create From This Model | 62 bones, 12 facial blend shapes, Calculate blend shape normals, extracted materials |
| `Companion.fbx` | FBX, Humanoid, Copy From Other Avatar (`HeroAvatar`) | the shared avatar |
| `Husk.fbx` | FBX, Humanoid with its own avatar | longer arms and shorter legs than the hero |
| `Anim_*.fbx` | FBX on a separate source rig, Humanoid | split clips with loop time and loop pose, Idle root bakes, Optimal compression, `Footstep` events |
| Environment and tree models | FBX | generated lightmap UVs, LOD0 to LOD2, collider meshes |
| Animator controllers | Unity YAML | three layers, 2D freeform directional blend tree; `Companion.overrideController` |
| `UpperBody.mask` | avatar mask | humanoid body parts above the hips |
| `Opening.playable`, `Midgame.playable` | Timeline 1.x | Cinemachine, animation, audio, signal, and activation tracks |
| `Meadow.asset` | TerrainData | 256 x 40 x 256 m, heightmap 513, alphamap 512, 4 TerrainLayers, 2 tree and 2 detail prototypes |
| Albedo, normal, and mask textures | PNG and TGA | sRGB albedo, Normal map type, linear masks |
| Lightmaps, shadowmasks, reflection captures, sky | EXR and PNG baked in the unitas editor, EXR cubemap | Lightmap and Shadowmask types, cube shape |
| Dialogue; footsteps and effects; music and ambience | WAV 48 kHz mono; WAV; OGG | Compressed In Memory Vorbis 70; Decompress On Load; Streaming, looping |
| `Archive.webm` | VP9 1280 x 720 at 30 fps, Opus stereo, 24 s | VideoClip, no transcoding |
| `Main.mixer` | AudioMixer | groups, sends, four snapshots, exposed volumes |
| Locales, tables | Localization 1.x | `UI`, `Subtitles`, `Objectives` string tables, `Signs` asset table, Smart Strings |
| Fonts | TrueType with TextMeshPro SDF assets | Latin primary, Japanese fallback |
| `Adventure.inputactions` | Input System action asset | maps Gameplay and UI, two schemes |
| Volume profiles | URP Volume Profile | `Global`, `Cavern`, `Cutscene` |
| Addressables groups | Addressables 1.x | one group and label per area, local paths |
| `Fixtures/UnityBuilt/` | AssetBundles built by Unity 2022.3 LTS for each desktop target | committed fixtures, with the producing Unity release recorded |

### Packages

URP 14.x, Cinemachine 2.x, Timeline 1.x, Input System 1.x, Localization 1.x, AI
Navigation 1.x, Addressables 1.x, uGUI 1.0.x, TextMeshPro 3.0.x, and Unity Test
Framework 1.x, at the version lines CMP-002 lists. Packages are resolved from
their registry when the project opens and are never vendored in the sample
repository.

### Save data

Three slots and an autosave as versioned `JsonUtility` files under
`Application.persistentDataPath` (current format version 2); records in a
separate file; settings in `PlayerPrefs`. Committed fixture slots: valid
version 2, version 1, truncated, and future version.

### Licences and provenance

Every asset family's source, author, and licence is recorded in
`Licenses/content-licenses.json`. Unity packages' licence terms are recorded in
the build's licence report.

## Feature Coverage

| ID | Behavior this game exercises | Scenarios |
|---|---|---|
| PRJ-001 | Opens in place | S1 |
| PRJ-003 | Stable avatar, clip, and sub-asset identities | S1, S16 |
| PRJ-004 | FBX humanoid import, clip splitting, events, blend shapes, EXR lightmaps, video and audio | S1 |
| PRJ-007 | Layers, quality, time step, input handling | S1 |
| SER-001 | Serialized values | S16 |
| SER-002 | Additive scene sets | S1, S7 |
| SER-003 | Prefab variants | S1 |
| RUN-001 | Objects and respawn | S6 |
| RUN-002 | Compiled scripts | S1 |
| RUN-003 | `OnAnimatorMove`, `OnAnimatorIK`, `OnControllerColliderHit` | S2, S5 |
| RUN-004 | Async operations, `Time.timeScale`, fade and hit-stop coroutines | S7, S11 |
| RUN-005 | Single and additive loads, `SetActiveScene`, unloading | S7, S8 |
| RUN-006 | Addressables loads by label, reference counts, release on unload | S7 |
| RUN-008 | Signal receiver reactions and interaction events | S3 |
| RUN-009 | Save files under `Application.persistentDataPath` with `JsonUtility` | S8, S14 |
| PHY-001 | Raycasts for foot IK and sight, triggers, kinematic platforms and stones | S5, S6 |
| PHY-003 | `CharacterController` on slopes, steps, platforms, respawn teleport | S5, S6 |
| NAV-001 | Companion and husk agents, area masks, off-mesh links, path states | S13 |
| ENV-001 | Terrain height, splat layers, trees, detail grass, wind | S15 |
| REN-001 | Cinemachine-driven camera, resolution changes | S3, S15 |
| REN-002 | Skinned humanoids, facial blend shapes, URP Lit materials | S3, S15 |
| REN-003 | Baked lightmaps, light probes, reflection probes, fog, post-processing, quality | S3, S15 |
| REN-004 | Footstep dust, waterfall, fire, sigil burst | S5, S15 |
| REN-006 | URP 14.x Forward with Volumes | S15 |
| REN-007 | Frame pacing | S15 |
| ANI-001 | Layers, blend trees, transitions, animation events, Timeline animation | S2, S3, S5 |
| ANI-002 | Shared avatar, humanoid retargeting, foot and look-at IK, avatar masks | S1, S5 |
| CIN-001 | Timeline cutscenes, Cinemachine cuts and blends, skip | S3, S4 |
| UI-001 | Menus, subtitles, objective and medal text | S2, S9 |
| TXT-001 | Japanese fallback | S9 |
| INP-002 | Actions, scheme switching, device changes, rumble status | S10 |
| AUD-001 | Spatial sources, reverb zones, mixer snapshots, dialogue, shard pitch steps | S3, S11, S15 |
| MED-001 | Video on an in-world screen and full screen with synchronized audio | S12 |
| LOC-001 | String and asset tables, Smart Strings, locale change during play | S9 |
| SAV-001 | Settings, slots, records, corruption, migration | S8, S14, S17 |
| ACC-001 | Subtitles, reduced motion, run toggle | S9, S11 |
| BLD-001 | Packaged players | S16 |
| BLD-005 | unitas-built and Unity-built Addressables content | S7 |
| BLD-002 | Headless runs | S1–S15, S17 |
| TST-002 | Scenarios, references, input replay | all |
| AUT-002 | Play control through the editor's MCP interface | S1–S15, S17 |
| EDT-004 | Play and stop with edit-state restoration | S16 |

## Engine Behavior This Game Depends On

Only engine and package behavior, each tied to a catalog ID. Game rules are
defined under Game Design and implemented by the sample's own scripts; the
engine contains no code specific to this game. Package behavior (Cinemachine,
Timeline, Localization, AI Navigation, Addressables) comes from the package's
own code at the pinned version line, compiled and run unmodified (CMP-002); the
items below state only the engine surface the packages call. Agents reach the
game only through the editor's generic MCP object, property,
method-invocation, input, and capture commands, with no game-specific command.

### Scripting and runtime

* `OnAnimatorMove` replaces automatic root motion: a script that passes
  `Animator.deltaPosition` to `CharacterController.Move` moves the character
  exactly once by that delta. (RUN-003, ANI-001, PHY-003)
* Async additive loads keep other loaded scenes loaded, and `SetActiveScene`
  makes the active scene supply fog, ambient light, and skybox. (RUN-005)
* Addressables loads by label and releases by handle; unloading the scene that
  owns the handles brings every reference count to zero. (RUN-006)
* Signal receiver reactions are UnityEvents and run once per crossing.
  (RUN-008)

### Project and content

* An FBX that copies another model's avatar resolves it by reference, and clips
  imported on a separate source rig play on every humanoid avatar. An invalid
  avatar is reported with its bones. (PRJ-004, ANI-002)
* Lightmaps, shadowmasks, light probes, and reflection captures are baked in
  the unitas editor, interactively or headless, and load with their renderer
  assignments. Moving a static renderer in the editor marks the bake stale and
  names the renderer. Rebaking replaces the stale data. (REN-003)

### Simulation

* A `CharacterController` with height 1.8 m, radius 0.35 m, slope limit 45°,
  step offset 0.35 m, and skin width 0.05 m climbs 0.3 m steps, stops at 0.5 m
  steps, walks 40° slopes, and slides down 50° slopes at 2 m/s or more. A
  grounded controller rides a kinematic platform moving at 1.5 m/s without
  sliding. (PHY-003)
* Agents walk NavMeshSurface data. A bidirectional NavMeshLink and a one-way
  link are traversed automatically when auto traversal is on; with it off,
  `isOnOffMeshLink` is true at the link and `CompleteOffMeshLink` ends the
  traversal. Area masks exclude areas from paths, and a path into an excluded
  area is partial. (NAV-001)

### Rendering and animation

* An Animator with a base layer, an override layer with an avatar mask, and an
  additive layer at weight 0.5 blends as ANI-001 defines; masked-out bones keep
  the base layer's pose. (ANI-001, ANI-002)
* Foot IK set in `OnAnimatorIK` keeps each foot within 3 cm of the surface it
  targets on 30° slopes and 0.2 m stair risers; look-at IK turns the head at the
  set weight. (ANI-002)
* Animation events fire at their clip times for every clip with nonzero blend
  weight, and report that weight. (ANI-001)
* Timeline animation tracks drive blend shapes, rendered on the frame they are
  evaluated. (ANI-001, CIN-001, REN-002)
* Cinemachine's brain drives the camera through the engine camera API; cuts
  take effect on the frame of the cut and blends over their stated time.
  (CIN-001, REN-001)
* URP Volumes apply by priority and blend distance; ACES tonemapping, bloom,
  color adjustments, and Bokeh depth of field render as REN-003 defines. Fog
  modes Exponential Squared and Linear render as defined. (REN-003, REN-006)
* Terrain renders splat layers with URP Terrain Lit, trees with LOD, billboards,
  and colliders, and instanced detail grass to its detail distance, waving with
  terrain wind. (ENV-001)
* Particle systems collide with world colliders, and the light module and
  sub-emitters behave as REN-004 defines. (REN-004)

### Audio and video

* Mixer snapshots transition over their stated time; logarithmic and linear
  rolloff and reverb zones apply between their minimum and maximum distances.
  (AUD-001)
* A VideoPlayer in Material Override and in Camera Near Plane modes plays with
  its audio routed to an AudioSource or directly; audio and video stay within
  40 ms; an undecodable clip fires `errorReceived` naming source and codec.
  (MED-001)

### Input

* Using a device of another control scheme switches the scheme within one
  input update. No action fires twice or is lost across a device change.
  (INP-002)
* Rumble on a gamepad without motors returns its explicit status. (INP-002)

### UI and text

* TextMeshPro renders Japanese through the fallback font with no missing
  glyphs. (TXT-001)
* Localization string tables and Smart Strings with plural forms resolve per
  locale; a locale change raises the package's change event on the next frame.
  (LOC-001)

### Player and build

* The player loads Addressables content built by unitas, and the committed
  Unity-built fixture bundles, as BLD-005 defines. A corrupt bundle is reported
  by name and is never loaded partially. (BLD-005)
* Each build report lists scenes, Addressables content, sizes, and the build
  identifier. (BLD-001)

### Editor and automation

* Stopping play restores edited scene values. (EDT-004)
* Through the editor's generic MCP commands an agent seeks a `PlayableDirector`,
  invokes the game's own load and locale methods, sets transforms, reads any
  component field (hearts, shards, medals, active scene, handle counts, path
  states), and captures frames and audio. (AUT-002)

## Presentation and Performance

### Visual checkpoints

Each has a unitas-approved reference at each quality level: the opening
cutscene at 12.5 s, the meadow vista, the ruins hall, the video screen at frame
360, the cavern altar, the escape at 20 s, the subtitle box in all four
locales, and the completion panel with medals.

### Audio checkpoints

Dialogue starts within one frame of its Timeline clip; footsteps align with
foot contact within one frame; cavern reverb is present at the altar and absent
in the meadow; shard pitch steps rise one semitone.

### Project-declared expectations

60 frames per second at 1080p at High on the declared reference desktop class
and 30 at Low on the declared minimum class; no frame over 50 ms during an area
transition at High on the reference class; start to title under 4 s; Continue
to control under 6 s. Expectations are evaluated only on the declared class;
other runs record the measurement and report the expectation as unevaluable
(TST-002). These are the sample's own expectations, not the PRF-002 engine
budgets.

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
* **Expected:** no import errors; the three avatars validate; `Companion.fbx`
  references `HeroAvatar`; split clips keep loop, bake, and `Footstep`
  settings; 12 blend shapes exist; reopening yields identical identities.
* **Demonstrates:** PRJ-001, PRJ-003, PRJ-004, PRJ-007, SER-002, ANI-002

### S2. Happy path

* **Runs in:** editor play mode; headless editor; desktop packaged players.
* **Preconditions:** a fresh profile.
* **Steps:** New Game, then a recorded input path through all areas and the
  escape.
* **Expected:** the objective reads 0/3 to 3/3, both cutscenes play, each sigil
  shows the Carry pose at UpperBody weight 1, and the completion panel appears.
* **Demonstrates:** RUN-003, ANI-001, PHY-003, CIN-001, UI-001, INP-002

### S3. Opening cutscene and hook

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** a fresh profile.
* **Steps:** play the opening cutscene, then replay the opening sprint.
* **Expected:** cuts and blends occur at their authored times; six dialogue
  lines play under the Cutscene snapshot; subtitles appear within one frame of
  their signals; `JawOpen` follows its curve within 0.01; the Cutscene volume is
  active only during playback; control returns within one frame of the end
  signal, the first stone falls 1.5 s after contact, and the replayed sprint
  reaches the checkpoint in 20 to 30 s.
* **Demonstrates:** CIN-001, ANI-001, REN-002, REN-003, AUD-001, LOC-001, RUN-008

### S4. Skip

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** each cutscene's saved state.
* **Steps:** tap Skip, then hold it 1.0 s, in each cutscene and from the pause
  menu.
* **Expected:** a tap only shows the prompt; the hold jumps to 0.1 s before the
  end, the end signal fires once, and the world state equals a watched
  cutscene.
* **Demonstrates:** CIN-001

### S5. Traversal gym

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** `Gym_Traversal` at the start of the course.
* **Steps:** cross 20°, 40°, and 50° ramps, 0.2, 0.3, and 0.5 m steps, a
  staircase, a moving platform, and 2.2 m and 4.0 m gaps, at 30, 60, and 144
  frames per second.
* **Expected:** the Simulation behavior holds at each rate; feet stay within
  3 cm of the surface; one sound and one dust burst per contact.
* **Demonstrates:** PHY-003, ANI-002, ANI-001, REN-004, PHY-001

### S6. Fall, respawn, and failure

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** meadow at the start of the opening sprint.
* **Steps:** jump into a chasm, then take three hits.
* **Expected:** each loss costs one heart and respawns at the checkpoint with
  the companion warped beside the hero; the last shows the failure panel; Retry
  restores three hearts with no husk still chasing.
* **Demonstrates:** PHY-003, NAV-001, RUN-001

### S7. Streaming and Addressables

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** meadow at the start of the opening sprint.
* **Steps:** walk meadow to ruins to cavern. Repeat with the committed
  Unity-built fixture bundles substituted. Then corrupt one bundle (a harness
  action on the build's content folder) and walk again.
* **Expected:** at most two areas loaded; the active scene and fog change at
  each boundary; each area's handles reach a reference count of zero after
  unload; no transition frame exceeds the expectation on the declared class.
  The Unity-built bundles load and give the same result. The corrupted bundle
  is reported by name and not loaded.
* **Demonstrates:** RUN-005, SER-002, RUN-006, BLD-005

### S8. Save and resume

* **Runs in:** packaged players; headless editor with a play-session restart.
* **Preconditions:** ruins with the second sigil, two hearts, and 3 unbanked
  shards.
* **Steps:** save to slot 2, then quit and relaunch (a harness action that ends
  and restarts the player process, or, headless, ends the play session and
  starts a new one with the same storage root), and Continue. Try to save while
  airborne.
* **Expected:** same area, checkpoint, position within 0.01 m, rotation within
  0.5°, hearts, banked and unbanked shards, Carry pose, companion position,
  objective, and cutscenes seen. Saving while airborne is refused.
* **Demonstrates:** SAV-001, RUN-009, RUN-005, ANI-001

### S9. Locale change during play

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** the opening cutscene, and the cavern with one husk chasing.
* **Steps:** in each state, switch through all four locales; relaunch.
* **Expected:** subtitle, objective, menus, and signs update on the next frame;
  cutscene time, the chase, and game state are unchanged; the locale persists
  across the relaunch.
* **Demonstrates:** LOC-001, TXT-001, ACC-001

### S10. Device changes

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** meadow in play with keyboard and mouse.
* **Steps:** add a gamepad, alternate devices, then remove it mid-run, through
  device-addition and device-removal events in the replay stream. Repeat with a
  gamepad that reports no motors.
* **Expected:** scheme and glyphs switch within one input update; the removal
  pauses with the prompt; no jump or dodge fires twice or is lost; the
  motorless gamepad reports rumble status and the hit feedback still plays.
* **Demonstrates:** INP-002

### S11. Pause and juice

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** each relevant saved state.
* **Steps:** pause during a cutscene, the video, and a chase; then take a hit
  and take a sigil. Repeat with reduced motion on.
* **Expected:** Timeline time, video frame, animator states, particles, and
  agents hold while paused under the Paused snapshot, and dialogue resumes at
  the same sample; a hit gives 80 ms of hit-stop and the vignette; a sigil
  gives 0.5 s at time scale 0.3 and the stinger. Reduced motion removes camera
  shake and landing dips.
* **Demonstrates:** RUN-004, AUD-001, CIN-001, MED-001, ACC-001

### S12. Video interlude

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** ruins beside the video screen.
* **Steps:** approach the ruins screen, then watch full screen. Replace the
  clip with one in a codec the target cannot decode (a development-build debug
  command implemented in the sample's scripts).
* **Expected:** Material Override on the screen with 3D audio at it; full screen
  in Camera Near Plane with 2D audio; at each sync marker (a white flash and a
  1 kHz beep every 5 s) audio and video align within 40 ms. The undecodable
  clip fires `errorReceived` naming source and codec, and the screen shows
  "Signal lost", never a silent black frame.
* **Demonstrates:** MED-001, AUD-001

### S13. Navigation and stealth

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** ruins with the companion beside the gap.
* **Steps:** lead the companion across the gap and down the ledge; walk past a
  husk at 5 m, then run past at 8 m; stand in Sanctuary.
* **Expected:** the companion uses both links with `isOnOffMeshLink` true;
  walking is unheard and running starts a chase; the husk crosses links with
  its arc; its path to Sanctuary is partial and it stops at the edge.
* **Demonstrates:** NAV-001

### S14. Damaged and older saves

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** the committed fixture slots installed in the storage root.
* **Steps:** load the truncated slot, the future-version slot, and the version 1
  slot.
* **Expected:** the truncated slot shows Damaged; the future version is reported
  incompatible and left unchanged; version 1 migrates to version 2 with
  progress kept and the migration reported.
* **Demonstrates:** SAV-001, RUN-009

### S15. Visual and audio checkpoints

* **Runs in:** headless editor; packaged players.
* **Preconditions:** each saved state needed by a checkpoint.
* **Steps:** capture every checkpoint at Low, Medium, and High.
* **Expected:** each matches its reference within tolerance; tree, grass, and
  shadow distances follow the quality level; reverb and rolloff match the audio
  checkpoints.
* **Demonstrates:** REN-002, REN-003, REN-004, REN-006, ENV-001, AUD-001, TST-002

### S16. Equivalence, edit, and package

* **Runs in:** editor play mode (visual and headless); headless editor (build);
  desktop packaged players.
* **Preconditions:** none.
* **Steps:** run S2 in the editor and in a player per desktop target; in the
  editor, move a platform during play and stop.
* **Expected:** identical checkpoint results; the platform returns to its edited
  position; each build report lists scenes, Addressables content, sizes, and
  the build identifier.
* **Demonstrates:** BLD-001, EDT-004, SER-001

### S17. Shard risk, medals, and unlocks

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** a fresh profile.
* **Steps:** collect 5 shards, die before a checkpoint, recover the cache, then
  die again holding 2 new shards before their cache. Finish a replayed run
  under 15 minutes with no heart lost. Relaunch, then start Hollow mode.
* **Expected:** the cache holds exactly 5; the second death loses the 2;
  banking 24 shards gives six hearts; the panel awards Gold and Untouched;
  records, medals, Chapters, and Hollow mode persist across the relaunch;
  Hollow mode starts with one heart and husks 20% faster.
* **Demonstrates:** SAV-001, UI-001, AUD-001

## Manual Playtest Guidance

A person judges the following, and a headless run reports these checkpoints as
awaiting human review, never as passed:

* Whether the opening sprint grips within 30 seconds.
* Whether running jumps and dodges feel responsive and fair.
* Whether shard routes tempt without feeling cheap.
* Whether the cavern stealth and the escape feel tense.
* Whether the medals invite a second run.
* Camera comfort, foot placement, lip movement against dialogue, the mix in
  each area, and subtitle readability.

## Agent Authoring Change Requests

### CR1. Pressure plate

* **Request:** add a pressure plate in the ruins that opens a side gate while
  the hero or the companion stands on it, with a hint line in all four locales.
* **Acceptance scenario:** the gate opens while either character stands on the
  plate and closes when both leave; the companion's path uses the open gate;
  the hint shows in each locale.
* **Test:** a test of the plate's state.

### CR2. Projectile husk

* **Request:** add a husk variant that throws a slow projectile.
* **Acceptance scenario:** a projectile hit costs one heart, and respawn and
  shard caches still behave as in S6 and S17.
* **Test:** a test that counts hearts lost per projectile hit.

## Accessibility

* Every action works with the keyboard alone and the gamepad alone (S10).
* Subtitles are on by default, have three sizes and a background box, and name
  the speaker in text (S9).
* Husk attacks telegraph by pose, sound, and color together (S13).
* Reduced motion removes camera shake, landing dips, and cutscene depth of
  field (S11).
* Run can be held or toggled, and cutscenes can be skipped (S4).

## Saved and Reproducible States

| State | How it is reached | Used by |
|---|---|---|
| Meadow at the start of the opening sprint | save file | S6, S7 |
| Ruins with the second sigil, beside the video screen, 3 unbanked shards | save file | S8, S12 |
| Ruins with the companion beside the gap | save file | S13 |
| Cavern with two sigils placed and one husk chasing | save file | S9, S11 |
| The escape at its start | save file | S15 |
| `Gym_Traversal` at the start of the course | scene | S5 |
| Slots: valid version 2, version 1, truncated, and future version | committed fixture files | S14 |

## Known Limitations and Unsupported Conditions

None. Any behavior this document requires that is not demonstrated is reported
with its real compatibility state, as a defect to fix.

## Acceptance Outcomes

* The third-person adventure resumes from a save with the same area, character
  state, and objective progress, and changes locale during play without losing
  state. (SMP-001, ANI-001, PHY-003, NAV-001, LOC-001, SAV-001)
* Both cutscenes play with synchronized dialogue, localized subtitles, facial
  blend shapes, and camera cuts and blends, and skipping reaches the same end
  state. (SMP-001, CIN-001, ANI-001, REN-002, LOC-001)
* Humanoid clips retarget onto every avatar, and foot IK, look-at IK, avatar
  masks, root motion, and footstep events behave as declared on slopes and
  stairs. (SMP-001, ANI-001, ANI-002, PHY-003, PRJ-004)
* Areas stream additively with their Addressables content released, and the
  video plays in the world and full screen with audio in sync. (SMP-001,
  RUN-005, SER-002, RUN-006, MED-001)
* Agents use off-mesh links, area masks, and hearing ranges, and the scenes
  match their references for terrain, baked lighting, post-processing, fog,
  particles, and spatial audio. (SMP-001, NAV-001, ENV-001, REN-003, REN-004,
  AUD-001)
* The hook, shard risk, hit and sigil feedback, medals, and unlocks behave as
  declared and persist. (SMP-001, SAV-001, RUN-004, UI-001)
* Device changes, damaged saves, and older saves cost no input and no
  progress. (SMP-001, INP-002, SAV-001)
