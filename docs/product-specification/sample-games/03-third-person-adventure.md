# Sample 3: Third-Person Adventure

## Overview

A storm called the Hollowing is draining the light from a valley. The player
guides a hero and a small companion through three connected areas, a meadow,
ruins, and a cavern, to recover three sigil stones and relight the cavern
altar. The hero walks, runs, jumps, dodges, rides moving platforms, interacts
with objects, and reaches checkpoints, while husk enemies hunt by sight and
sound. Lighting the altar starts a 45-second escape through a collapsing
tunnel. A first run takes 20 to 30 minutes, and a skilled run under 15.

The game is original, redistributable unitas content and uses no Unity-owned
sample content. It proves imported humanoid characters, animation, cinematics,
character movement, navigation, terrain, baked lighting, video, localization,
streaming, and persistence working together in one playable loop.

Compatibility tiers follow the [Feature catalog](../feature-catalog.md). This
document owns the acceptance of this game under SMP-001.

## Targets and Input Modes

* Targets: macOS, Windows, and Linux desktop players, plus the editor's play
  mode and the headless editor.
* Render pipeline: URP 14.x, Forward path, one URP asset per quality level.
* Input modes: keyboard and mouse, and gamepad, through one Input System 1.x
  action asset with the control schemes KeyboardMouse and Gamepad. Active input
  handling is Input System only.

## Game Design

### Core loop

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

### Rules

* Walk 2.0 m/s, run 5.5 m/s. A standing jump clears 2.2 m and a running jump
  4.0 m. A dodge moves 3 m in 0.4 s, ignores hits for its first 0.3 s, and has
  a 1.0 s cooldown. Ground locomotion is root motion.
* The hero starts with three hearts, up to six. A husk hit or a chasm fall
  costs one heart and respawns the hero at the last checkpoint after a 1.5 s
  fade. At zero hearts the failure panel "You were overcome" offers Retry from
  checkpoint, Load Game, and Quit to title.
* Husks hear a running hero within 10 m and a walking one within 4 m, see 14 m
  in a 90° cone, give up after 6 s without contact, and never enter the
  Sanctuary area around checkpoints. A lit brazier repels husks within 6 m.
* Checkpoints autosave to the current slot. The pause menu saves to any of
  three slots. Saving is refused, with "You can't save now", while airborne,
  in a cutscene, during the escape, or while a husk is chasing. A damaged slot
  shows "Damaged" and cannot be loaded.
* Pause sets `Time.timeScale` to zero and shows Resume, Save, Load, Settings,
  and Quit to title, plus Skip cutscene during a cutscene. Timeline, video,
  animators, particles, agents, and gameplay audio stop, and the mixer moves
  to the Paused snapshot while menu sounds play.
* Settings: five volume sliders (master, music, effects, ambience, dialogue);
  subtitles on or off and size; language (English, French, German, Japanese);
  camera sensitivity, invert Y, run hold or toggle; reduced motion; display
  mode, resolution, vertical sync; quality Low, Medium, or High. They persist
  through `PlayerPrefs` and apply at once. The language persists through the
  Localization player preference selector.

### What makes it fun

* **Hook.** The opening cutscene ends with the ledge under the hero cracking.
  Control starts within one frame of the end signal, and stones fall 1.5 s
  after the hero first touches them. A 20-second sprint and jump run reaches
  the first checkpoint, where the first sigil's glow is visible across the
  valley.
* **Mastery.** Better players chain running jumps, walk to slip past husks
  unheard, dodge through attacks, time the ruins platform ring, and find
  shortcuts that skip about 90 s of the meadow and ruins.
* **Risk and reward.** Each area hides 8 ember shards (24 in all) on risky
  routes: ledges over chasms, husk patrols, and crumbling stones. Shards stay
  unbanked until the next checkpoint. Dying leaves the unbanked shards in a
  glowing cache where the hero fell, and dying again before reaching the cache
  loses them. Every 8 banked shards add a heart, up to six.
* **Escalation.** The meadow is traversal only. The ruins add timed platforms
  and two husks. The cavern adds six husks, stealth, and braziers that the
  player lights to carve safe paths. The escape has no checkpoint: falling
  stones, every remaining husk chasing, and the music rising each 15 s.
* **Juice.** Movement responds on the first rendered frame after input.
  Landings from above 2 m dip the camera 0.1 m and puff dust. A husk hit adds
  80 ms of hit-stop, a red vignette, and 0.2 s of gamepad rumble. Taking a
  sigil plays a light burst, a music stinger, and 0.5 s at time scale 0.3.
  Each shard collected within 3 s of the last plays one semitone higher, up
  to eight steps.
* **Replay.** Four medals per run: Gold (under 15 minutes), Ember (all 24
  shards), Untouched (no heart lost), and Unseen (no husk chase in the
  cavern). Records keep the best time and most shards per slot. A completion
  unlocks Chapters and Hollow mode (one heart, no shard hearts, husks 20%
  faster).

## Content

| Asset | Source format | What matters on import |
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
| Lightmaps, shadowmasks, reflection captures, sky | EXR and PNG baked in unitas (and readable by Unity 2022.3 LTS), EXR cubemap | Lightmap and Shadowmask types, cube shape |
| Dialogue; footsteps and effects; music and ambience | WAV 48 kHz mono; WAV; OGG | Compressed In Memory Vorbis 70; Decompress On Load; Streaming, looping |
| `Archive.mp4` | H.264 1280 x 720 at 30 fps, AAC stereo, 24 s | VideoClip, no transcoding |
| `Main.mixer` | AudioMixer | groups, sends, four snapshots, exposed volumes |
| Locales, tables | Localization 1.x | `UI`, `Subtitles`, `Objectives` string tables, `Signs` asset table, Smart Strings |
| Fonts | TrueType with TextMeshPro SDF assets | Latin primary, Japanese fallback |
| `Adventure.inputactions` | Input System action asset | maps Gameplay and UI, two schemes |
| Volume profiles | URP Volume Profile | `Global`, `Cavern`, `Cutscene` |
| Addressables groups | Addressables 1.x | one group and label per area, local paths |

Packages: URP 14.x, Cinemachine 2.x, Timeline 1.x, Input System 1.x,
Localization 1.x, AI Navigation 1.x, Addressables 1.x, uGUI 1.0.x,
TextMeshPro 3.0.x, and Unity Test Framework 1.x.

## Feature Coverage

| ID | Behavior this game exercises | Scenarios |
|---|---|---|
| PRJ-001, PRJ-003, PRJ-007 | Opens in place; stable avatar, clip, and sub-asset identities; layers, quality, time step, input handling | S1, S16 |
| PRJ-004 | FBX humanoid import, clip splitting, events, blend shapes, EXR lightmaps, video and audio | S1 |
| SER-001, SER-002, SER-003 | Additive scene sets, prefab variants, serialized values | S1, S7, S16 |
| RUN-001, RUN-002, RUN-003 | Objects, scripts, `OnAnimatorMove`, `OnAnimatorIK`, `OnControllerColliderHit` | S2, S5 |
| RUN-004 | Async operations, `Time.timeScale`, fade and hit-stop coroutines | S7, S11 |
| RUN-005 | Single and additive loads, `SetActiveScene`, unloading | S7, S8 |
| RUN-006 | Addressables loads by label, reference counts, release on unload | S7 |
| RUN-008 | Signal receiver reactions and interaction events | S3 |
| RUN-009 | Save files under `Application.persistentDataPath` with `JsonUtility` | S8, S14 |
| PHY-001 | Raycasts for foot IK and sight, triggers, kinematic platforms and stones | S5, S6 |
| PHY-003 | `CharacterController` on slopes, steps, platforms, respawn teleport | S5, S6 |
| NAV-001 | Companion and husk agents, area masks, off-mesh links, path states | S13 |
| ENV-001 | Terrain height, splat layers, trees, detail grass, wind | S15 |
| REN-001, REN-007 | Cinemachine-driven camera, resolution changes, frame pacing | S3, S15 |
| REN-002 | Skinned humanoids, facial blend shapes, URP Lit materials | S3, S15 |
| REN-003 | Baked lightmaps, light probes, reflection probes, fog, post-processing, quality | S3, S15 |
| REN-004 | Footstep dust, waterfall, fire, sigil burst | S5, S15 |
| REN-006 | URP 14.x Forward with Volumes | S15 |
| ANI-001 | Layers, blend trees, transitions, animation events, Timeline animation | S2, S3, S5 |
| ANI-002 | Shared avatar, humanoid retargeting, foot and look-at IK, avatar masks | S1, S5 |
| CIN-001 | Timeline cutscenes, Cinemachine cuts and blends, skip | S3, S4 |
| UI-001, TXT-001 | Menus, subtitles, objective and medal text, Japanese fallback | S2, S9 |
| INP-002 | Actions, scheme switching, device changes, rumble status | S10 |
| AUD-001 | Spatial sources, reverb zones, mixer snapshots, dialogue, shard pitch steps | S3, S11, S15 |
| MED-001 | Video on an in-world screen and full screen with synchronized audio | S12 |
| LOC-001 | String and asset tables, Smart Strings, locale change during play | S9 |
| SAV-001 | Settings, slots, records, corruption, migration | S8, S14, S17 |
| ACC-001 | Subtitles, reduced motion, run toggle | S9, S11 |
| BLD-001, BLD-005 | Packaged players and unitas-built Addressables content | S7, S16 |
| BLD-002, TST-002, AUT-002 | Headless runs, scenarios, references, input replay, play control through the editor's MCP interface | all |
| EDT-004 | Play and stop with edit-state restoration | S16 |

## Engine Behavior This Game Depends On

### Scripting and runtime

* `OnAnimatorMove` passes `Animator.deltaPosition` and gravity to
  `CharacterController.Move`; root motion is not also applied automatically.
* Async additive loads keep `Core` loaded, and `SetActiveScene` makes the
  entered area supply fog, ambient light, and skybox. Each area's Addressables
  label loads with its scene, and unloading the area releases every handle to
  a reference count of zero.
* Signal receiver reactions are UnityEvents and run once per crossing.

### Project and content

* `Companion.fbx` resolves `HeroAvatar` by reference, and the `Anim_*.fbx`
  clips play on all three avatars. An invalid avatar is reported with its bones.
* Lightmaps, shadowmasks, light probes, and reflection captures are baked in
  the unitas editor, interactively or headless through the editor's MCP interface, and load with their
  renderer assignments. Lighting baked earlier by Unity 2022.3 LTS loads the same
  way. Moving a static renderer in the editor marks the bake stale and names the
  renderer. Rebaking replaces the stale data, and the result is still readable by
  Unity 2022.3 LTS.

### Simulation

* The `CharacterController` has height 1.8 m, radius 0.35 m, slope limit 45°,
  step offset 0.35 m, and skin width 0.05 m. It climbs 0.3 m steps, stops at
  0.5 m steps, walks 40° slopes, and slides down 50° slopes at 2 m/s or more.
  A grounded hero rides a kinematic platform moving at 1.5 m/s without sliding.
* Agents walk NavMeshSurface data from AI Navigation 1.x. A 3 m bidirectional
  NavMeshLink crosses a ruins gap, and a one-way link drops 2 m from a ledge.
  The companion traverses links automatically. Husks have automatic traversal
  off and cross in a scripted 0.6 s arc ending with `CompleteOffMeshLink`.
  Husk area masks exclude Sanctuary. The companion warps beside the hero when
  more than 25 m away or on respawn.

### Rendering and animation

* The hero's controller has Base (locomotion blend tree, jump, fall, land,
  dodge, interact), UpperBody (override, `UpperBody.mask`, Carry), and
  Breathing (additive, weight 0.5) layers. Legs stay on Base while carrying.
* Foot IK keeps each foot within 3 cm of the ground on 30° slopes and 0.2 m
  stair risers, with the pelvis lowered to match. Look-at IK turns the head to
  an interactable within 4 m at weight 0.7.
* The `Footstep` event fires on each foot contact of Walk and Run. The script
  accepts it only from a clip with blend weight 0.5 or more, so each contact
  plays one grass, stone, or water sound, chosen by the dominant splat layer or
  physic material, and emits 6 dust particles on grass and dirt.
* Timeline animation tracks drive the blend shapes `JawOpen`, `Blink_L`,
  `Blink_R`, `Smile`, and `BrowUp`, rendered on the frame they are evaluated.
* A Cinemachine 2.x FreeLook with the collider extension follows the hero.
  Cutscenes cut and blend between four virtual cameras on a Cinemachine track,
  and afterwards the brain blends back to the FreeLook over 1.0 s.
* The `Cutscene` volume (priority 10, Bokeh depth of field) is active only in
  cutscenes. The global volume applies ACES tonemapping, bloom (threshold 1.0,
  intensity 0.6), and color adjustments; the cavern volume blends a cool grade
  in over 5 m. Fog is Exponential Squared at 0.008 in the meadow and Linear
  from 5 m to 40 m in the cavern.
* The meadow terrain renders 4 splat layers with URP Terrain Lit, about 600
  trees with LOD, billboards, and colliders, and instanced detail grass to 80 m
  at High that waves with the terrain wind.
* The waterfall emits continuously and collides with its pool. Braziers emit
  fire with the light module and spark sub-emitters.

### Audio

* Mixer groups Music, Ambience, Effects, Dialogue, and UI; snapshots Explore,
  Cavern, Cutscene (music −6 dB, effects −10 dB), and Paused (lowpass 900 Hz),
  with 0.5 s transitions.
* The waterfall rolls off logarithmically from 3 m to 60 m and braziers
  linearly from 1 m to 15 m. Reverb zones: Stone Corridor in the ruins hall
  (8 m to 16 m) and Cave in the cavern (12 m to 30 m).

### Input

* Using a device of the other scheme switches the scheme and the prompt glyphs
  within one input update. Disconnecting the gamepad in use pauses with
  "Controller disconnected"; reconnecting it or using the keyboard dismisses it.
  No action fires twice or is lost across a device change.
* Rumble on a gamepad without motors returns its explicit status and the hit
  feedback still plays.

### UI, text, and build

* Subtitles show the speaker's name and line from the `Subtitles` table within
  one frame of the line's signal and clear on its end signal. Japanese renders
  through the fallback font with no missing glyphs. The objective is a Smart
  String with plural forms per locale.
* The player loads Addressables content built by unitas, and the same content
  built by Unity 2022.3 LTS (BLD-005). A corrupt bundle is reported by name and
  is never loaded partially.
* Through the editor's MCP interface an agent can skip or seek a cutscene, load a slot, switch locale,
  teleport to a checkpoint, read hearts, shards, medals, active scene, handles,
  and path states, and capture frames and audio.

## Presentation and Performance

* Visual checkpoints with unitas-approved references at each quality level:
  the opening cutscene at 12.5 s, the meadow vista, the ruins hall, the video
  screen at frame 360, the cavern altar, the escape at 20 s, the subtitle box
  in all four locales, and the completion panel with medals.
* Audio checkpoints: dialogue starts within one frame of its Timeline clip;
  footsteps align with foot contact within one frame; cavern reverb is present
  at the altar and absent in the meadow; shard pitch steps rise one semitone.
* Project-declared expectations: 60 frames per second at 1080p at High on the
  declared reference desktop class and 30 at Low on the declared minimum class;
  no frame over 50 ms during an area transition at High on the reference
  class; start to title under 4 s; Continue to control under 6 s. These are the
  sample's own expectations, not the PRF-002 engine budgets.

## Acceptance Scenarios

Every scenario runs in editor play mode, in a packaged player, and headless
through the editor's MCP interface, unless it says otherwise.

* **S1. Open and inspect.** No cache exists. Open the project. Expected: no
  import errors; the three avatars validate; `Companion.fbx` references
  `HeroAvatar`; split clips keep loop, bake, and `Footstep` settings; 12 blend
  shapes exist; reopening yields identical identities. (PRJ-001, PRJ-003,
  PRJ-004, PRJ-007, SER-002, ANI-002)
* **S2. Happy path.** New Game, then a recorded input path through all areas
  and the escape. Expected: the objective reads 0/3 to 3/3, both cutscenes
  play, each sigil shows the Carry pose at UpperBody weight 1, and the
  completion panel appears. (RUN-003, ANI-001, PHY-003, CIN-001, UI-001, INP-002)
* **S3. Opening cutscene and hook.** Play the opening cutscene. Expected: cuts
  and blends occur at their authored times; six dialogue lines play under the
  Cutscene snapshot; subtitles appear within one frame of their signals;
  `JawOpen` follows its curve within 0.01; the Cutscene volume is active only
  during playback; control returns within one frame of the end signal, the
  first stone falls 1.5 s after contact, and the replayed sprint reaches the
  checkpoint in 20 to 30 s. (CIN-001, ANI-001, REN-002, REN-003, AUD-001,
  LOC-001, RUN-008)
* **S4. Skip.** Tap Skip, then hold it 1.0 s, in each cutscene and from the
  pause menu. Expected: a tap only shows the prompt; the hold jumps to 0.1 s
  before the end, the end signal fires once, and the world state equals a
  watched cutscene. (CIN-001)
* **S5. Traversal gym.** In `Gym_Traversal`, cross 20°, 40°, and 50° ramps,
  0.2, 0.3, and 0.5 m steps, a staircase, a moving platform, and 2.2 m and
  4.0 m gaps, at 30, 60, and 144 frames per second. Expected: the Simulation
  rules hold at each rate; feet stay within 3 cm of the surface; one sound and
  one dust burst per contact. (PHY-003, ANI-002, ANI-001, REN-004, PHY-001)
* **S6. Fall, respawn, and failure.** Jump into a chasm, then take three hits.
  Expected: each loss costs one heart and respawns at the checkpoint with the
  companion warped beside the hero; the last shows the failure panel; Retry
  restores three hearts with no husk still chasing. (PHY-003, NAV-001, RUN-001)
* **S7. Streaming and Addressables.** Walk meadow to ruins to cavern.
  Expected: at most two areas loaded; the active scene and fog change at each
  boundary; each area's handles reach a reference count of zero after unload;
  no transition frame exceeds the expectation. With Unity-built bundles
  substituted, the same walk loads them and gives the same result. A corrupted
  bundle is reported by name and not loaded. (RUN-005, SER-002, RUN-006,
  BLD-005)
* **S8. Save and resume.** In the ruins with the second sigil, two hearts, and
  3 unbanked shards, save to slot 2, quit, and Continue. Expected: same area,
  checkpoint, position within 0.01 m, rotation within 0.5°, hearts, banked and
  unbanked shards, Carry pose, companion position, objective, and cutscenes
  seen. Saving while airborne is refused. (SAV-001, RUN-009, RUN-005, ANI-001)
* **S9. Locale change during play.** During the opening cutscene, and in the
  ruins with a husk chasing, switch through all four locales. Expected:
  subtitle, objective, menus, and signs update on the next frame; cutscene
  time, the chase, and game state are unchanged; the locale persists across a
  relaunch. (LOC-001, TXT-001, ACC-001)
* **S10. Device changes.** Connect a gamepad, alternate devices, then unplug
  it mid-run. Expected: scheme and glyphs switch within one input update; the
  unplug pauses with the prompt; no jump or dodge fires twice or is lost; a
  motorless gamepad reports rumble status. (INP-002)
* **S11. Pause and juice.** Pause during a cutscene, the video, and a chase;
  then take a hit and take a sigil. Expected: Timeline time, video frame,
  animator states, particles, and agents hold while paused under the Paused
  snapshot, and dialogue resumes at the same sample; a hit gives 80 ms of
  hit-stop and the vignette; a sigil gives 0.5 s at time scale 0.3 and the
  stinger. Reduced motion removes camera shake and landing dips. (RUN-004,
  AUD-001, CIN-001, MED-001, ACC-001)
* **S12. Video interlude.** Approach the ruins screen, then watch full screen.
  Expected: Material Override on the screen with 3D audio at it; full screen
  in Camera Near Plane with 2D audio; at each sync marker (a white flash and a
  1 kHz beep every 5 s) audio and video align within 40 ms. An undecodable
  codec fires `errorReceived` naming source and codec, and the screen shows
  "Signal lost", never a silent black frame. (MED-001, AUD-001)
* **S13. Navigation and stealth.** Lead the companion across the gap and down
  the ledge; walk past a husk at 5 m, then run past at 8 m; stand in
  Sanctuary. Expected: the companion uses both links with `isOnOffMeshLink`
  true; walking is unheard and running starts a chase; the husk crosses links
  with its arc; its path to Sanctuary is partial and it stops at the edge.
  (NAV-001)
* **S14. Damaged and older saves.** Load a truncated slot, a future-version
  slot, and a version 1 slot. Expected: the truncated slot shows Damaged; the
  future version is reported incompatible and left unchanged; version 1
  migrates to version 2 with progress kept and the migration reported.
  (SAV-001, RUN-009)
* **S15. Visual and audio checkpoints.** Capture every checkpoint at Low,
  Medium, and High. Expected: each matches its reference within tolerance;
  tree, grass, and shadow distances follow the quality level; reverb and
  rolloff match the audio checkpoints. (REN-002, REN-003, REN-004, REN-006,
  ENV-001, AUD-001, TST-002)
* **S16. Equivalence, edit, and package.** Run S2 in the editor and in a
  player per desktop target; in the editor, move a platform during play and
  stop. Expected: identical checkpoint results; the platform returns to its
  edited position; each build report lists scenes, Addressables content,
  sizes, and the build identifier. (BLD-001, EDT-004, SER-001)
* **S17. Shard risk, medals, and unlocks.** Collect 5 shards, die before a
  checkpoint, recover the cache, then die again holding 2 new shards before
  their cache. Finish a replayed run under 15 minutes with no heart lost.
  Expected: the cache holds exactly 5; the second death loses the 2; banking
  24 shards gives six hearts; the panel awards Gold and Untouched; records,
  medals, Chapters, and Hollow mode persist across a relaunch; Hollow mode
  starts with one heart and husks 20% faster. (SAV-001, UI-001, AUD-001)

## Manual Playtest Guidance

A person judges whether the opening sprint grips within 30 seconds, whether
running jumps and dodges feel responsive and fair, whether shard routes tempt
without feeling cheap, whether the cavern stealth and the escape feel tense,
and whether the medals invite a second run. They also judge camera comfort,
foot placement, lip movement against dialogue, the mix in each area, and
subtitle readability. A headless run reports these checkpoints as awaiting
human review, never as passed.

## Agent Authoring Change Requests

Add a pressure plate in the ruins that opens a side gate while the hero or the
companion stands on it, with a hint line in all four locales. Its scenario
proves that the gate opens and closes, the companion's path uses it, and the
hint shows in each locale, with a test of the plate's state. A second request
adds a husk variant that throws a slow projectile, with a scenario proving that
the projectile costs one heart and that respawn and shard caches still work.

## Accessibility

Every action works with the keyboard alone and the gamepad alone. Subtitles
are on by default, have three sizes and a background box, and name the
speaker in text. Husk attacks telegraph by pose, sound, and color together.
Reduced motion removes camera shake, landing dips, and cutscene depth of
field. Run can be held or toggled, and cutscenes can be skipped.

## Saved and Reproducible States

* Meadow at the start of the opening sprint.
* Ruins with the second sigil, beside the video screen, 3 unbanked shards.
* Cavern with two sigils placed and one husk chasing; the escape at its start.
* `Gym_Traversal` at the start of the course.
* Slots: valid version 2, version 1, truncated, and future version.

## Known Limitations and Unsupported Conditions

* None that come from the engine. The `Archive.mp4` interlude plays on every
  declared target. unitas provides H.264 and AAC decoding wherever the platform
  does not.
* Any behavior this document requires that is not demonstrated is reported
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
