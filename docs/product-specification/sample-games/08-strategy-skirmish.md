# Sample 8: Strategy Skirmish ("Ashfall Ridge")

## Overview

Ashfall Ridge is a real-time strategy skirmish. The player commands one of two
asymmetric factions on a 4 km by 4 km volcanic valley against an AI opponent.
The player gathers ore, builds a base, trains an army, and destroys the enemy
Command Hall. Battles grow from a dozen scouts to 5,000 units clashing at once.
A match takes 20 to 40 minutes at normal speed, and less with the speed
controls.

This is an original, redistributable unitas sample. Its factions, units, map,
art, and audio are original unitas content. It does not reproduce any existing
strategy game or any Unity sample.

The game is one of the demonstrations of the PRF-002 scale floor for scenes:
its battlefield scene holds at least 100,000 GameObjects and stays editable in
the visual editor and through the editor's MCP interface. The scale floor's
acceptance is owned by PRF-002 in
[Diagnostics, profiling, and playtesting](../diagnostics-profiling-playtesting.md).
The game reports the PRF-002 measurements it takes separately from its own
project-declared expectations.

Compatibility tiers follow the [Feature catalog](../feature-catalog.md). This
document owns this game's acceptance under SMP-001.

## Targets and Input Modes

### Targets

* **Automated:** macOS, Windows, and Linux desktop players.
* **Capability-reported:** none.
* **Editor play mode:** yes.
* **Headless editor:** yes.

### Render pipelines

URP 14.x with the Forward rendering path and a custom fog-of-war renderer
feature. There is no Built-in Render Pipeline variant.

### Input modes and devices

Keyboard and mouse, and keyboard alone through a keyboard cursor. All bindings
are Input System 1.x actions (INP-002). Gamepad is supported in menus only.

Headless runs use synthetic devices marked synthetic; haptic and feel
checkpoints need physical hardware and are otherwise unevaluable.

## Game Design

### Core loop and flow

1. The title screen shows New Skirmish, Load, Records, Settings, and Quit.
   Behind the menu, a recorded battle plays on the valley floor.
2. Skirmish setup selects the player's faction (Wardens or Ashborne), the AI's
   faction, the AI difficulty (Easy, Normal, Hard, or Ironclad once unlocked),
   one of four start positions, and a map seed shown as a number. A loading
   screen shows progress for scenes, terrain tiles, and navigation data.
3. The player starts with a Command Hall, six Workers, and ten Scouts on an
   overlook. Workers gather ore from ore fields and build structures. Structures
   train units. Units take move, attack, attack-move, gather, and hold orders.
4. The AI expands, builds, scouts, and attacks in waves.
5. Destroying the enemy Command Hall wins the match. Losing the player's own
   Command Hall loses it. The results panel shows the outcome, match time, an
   army-value graph over time, units built and lost, orders per minute, the
   share of damage dealt from the rear, the share of fights won with a
   favorable counter, the medal earned, and whether a record was set. Its
   buttons are Rematch (same seed and setup), New Skirmish, and Title.
6. Quit on the title exits the player.

### Rules and mechanics

* **Factions.**

  | | Wardens (blue) | Ashborne (orange) |
  |---|---|---|
  | Style | slow, armored, strong defense | fast, cheap, swarming |
  | Unique structure | Watchtower: 120 m sight, reveals fog | Ember Pit: units near it regenerate 2 hit points per second |
  | Unique trait | Shield Wall: infantry that holds still for 2 s takes 30 percent less damage | Kindling: Ashborne deaths leave burning ground for 5 s that deals 4 damage per second |

* **Units.** Each unit type is a `UnitDefinition` ScriptableObject with hit
  points, speed, turn rate, attack range, damage, cooldown, sight radius, ore
  cost, build time, footprint radius, and its LOD meshes. Each faction has its
  own variant of each type.

  | Unit | Role | Strong against (x1.5 damage) | Weak against (x0.75 damage) |
  |---|---|---|---|
  | Worker | gathers and builds | — | everything |
  | Scout | fast flanker, 14 m/s | Siege Crawler | Ranger |
  | Lancer | melee charge | Ranger | Siege Crawler |
  | Ranger | ranged, 40 m | Scout | Lancer |
  | Siege Crawler | area damage, 90 m | structures, clumped units | Scout |

* Attacks that hit a unit from behind (more than 120 degrees from its facing)
  deal 50 percent more damage. A Ranger standing at least 8 m higher than its
  target gains 25 percent range. Siege shells damage every unit within 6 m of
  the impact point.
* **Structures.** Command Hall, Barracks (Scouts, Lancers, Rangers), Foundry
  (Siege Crawlers), Ore Refinery, Wall Segment (8 m), and the faction structure.
  Each is a `BuildingDefinition` ScriptableObject. A placed structure starts as
  a construction site. Units cannot path through a site or a finished
  structure, so walls reshape the battlefield.
* **Map.** The valley is 64 terrain tiles of 512 m by 512 m. It has a central
  basin with three Ash Vents (rich ore fields worth twice the ore per trip),
  ridges that give high ground, two river fords, a plateau reachable only by
  one ramp, and four start positions. The map seed changes ore-field positions
  within authored bounds and the route of the opening supply caravan.
* **Terrain streaming.** The game's streaming script loads terrain tiles whose
  bounds come within 1,280 m of the camera's ground focus and unloads tiles
  beyond 1,536 m, with at most two tile loads in flight. Props are culled
  beyond 900 m, so no prop renders over an unloaded tile.
* **Simulation.** The simulation ticks in `FixedUpdate` with a fixed time step
  of 0.05 s. Squad steering runs as an `IJobParallelFor` over unit `NativeArray`
  data and writes poses through `IJobParallelForTransform` with a
  `TransformAccessArray`; target search builds a `NativeParallelMultiHashMap`
  spatial grid each tick. The jobs are `[BurstCompile]` with `FloatMode.Strict`.
  Squads request paths with `NavMesh.CalculatePath`; Workers use
  `NavMeshAgent`. Finishing or destroying a structure runs
  `NavMeshSurface.UpdateNavMesh` for the region around its footprint.
  Navigation data stays loaded when terrain scenes unload.
* **Fog of war.** `FogOfWar.compute` writes a 512 by 512 visibility texture per
  side, one cell per 8 m, into a random-write `RenderTexture`; `FogOfWarFeature`
  draws it over the scene before transparents; `Minimap.compute` composites the
  base image, fog, and unit blips into a 256 by 256 `RenderTexture` shown in a
  `RawImage`. With the Jobs fog renderer, the same cells come from a job and are
  uploaded to a `Texture2D`.
* **Audio.** At most eight barks and 32 combat sounds play at once, and the
  game's audio manager stops the oldest first. The warning horn is a 3D source
  at the attacked position, and the game ducks music by 6 dB while it plays.
* **Unit data at play time.** Changing a `UnitDefinition` value during play
  changes every unit that uses that asset from the next tick.

### Levels and progression

* Tier 1 is Workers and Scouts. Tier 2 (Barracks upgrade, minute 3 or later)
  adds Lancers and Rangers. Tier 3 (Foundry) adds Siege Crawlers.
* AI attack waves come at minutes 4, 8, and 12, then every 3 minutes, each
  larger than the last.
* At minute 15 an Ashstorm sweeps the valley for 60 seconds and cuts every
  sight radius by 40 percent.
* Late matches reach 2,500 units per side.
* Each faction and difficulty has bronze, silver, and gold medals: a win, a
  win under 30 minutes, and a win under 20 minutes with the Command Hall never
  below half its hit points. Records keep the fastest win per faction and
  difficulty. Winning on Hard with both factions unlocks Ironclad difficulty.

### Scoring, failure, and edge rules

* Each side has a cap of 2,500 units. A blocked production queue shows "Unit
  limit reached."
* A site cannot be placed on slopes over 15 degrees, in water, over another
  structure or prop, or where it would cut the Command Hall off from every ore
  field. The ghost turns red and names the reason.
* An order to an unreachable point moves units to the nearest reachable point
  and shows "Destination unreachable. Moving to nearest point."
* When a structure is finished, units inside its footprint move to the nearest
  navigable point outside it within 1 second.
* Enemy units inside fog are neither drawn nor selectable.
* **Rally Horn.** Every 100 units a side loses charges its Rally Horn by one
  charge, up to three charges. Sounding the Horn gives every own unit within
  40 m 30 percent more speed and damage for 10 seconds. A side whose army value
  drops below 40 percent of the opponent's gets 25 percent faster production
  until it recovers to 60 percent.
* Each destroyed caravan wagon drops 100 ore.
* A save with a CRC mismatch or truncated body reports "Save is damaged" and
  leaves the slot list intact. A format version 2 save loads with zero Rally
  Horn charges and reports migration from 2 to 3. A save of a newer format
  reports "Made by a newer version."

### Controls

| Action | Keyboard and mouse | Keyboard only |
|---|---|---|
| Pan camera | WASD, arrow keys, or pointer within 8 px of a screen edge | WASD or arrow keys |
| Rotate camera | Q and E, or middle-button drag | Q and E |
| Zoom | Mouse wheel | Page Up and Page Down |
| Move keyboard cursor | — | I, J, K, L (4 m per press, repeats at 10 Hz) |
| Select | Left click | Space at the keyboard cursor |
| Box select | Left drag | Shift+Space to start, move the cursor, Shift+Space to end |
| Add to selection | Shift with click or drag | Ctrl+Space |
| Contextual order (move, attack, gather) | Right click | Enter at the keyboard cursor |
| Attack-move | F, then left click | F, then Enter |
| Hold position | H | H |
| Stop | X | X |
| Build menu, rotate site, place site | B; R; left click | B; R; Enter |
| Sound Rally Horn | G | G |
| Assign and recall control groups | Ctrl+1 to Ctrl+9; 1 to 9 | same |
| Speed down, speed up, pause | Minus, Equals, P | same |
| Quick save, quick load | F5, F9 | same |
| Minimap jump, minimap order | Left click, right click on the minimap | — |
| Menu | Escape | Escape |

### Pause, focus, and lifecycle

Speed steps are 0.5x, 1x, 2x, and 4x, set through `Time.timeScale`. Pause (P)
sets `Time.timeScale` to 0. Camera movement, selection, and order queuing still
work while paused, and queued orders run on resume. Escape opens the menu
(Resume, Save, Load, Settings, Surrender, Quit to title). It pauses the game and
restores the previous speed on Resume. When the host cannot keep up at the
chosen speed, the HUD shows the effective speed. With pause on focus loss
enabled, losing window focus pauses the game and releases every held key and
drag. A loaded save starts paused.

### Settings

* master, music, and effects volume
* fullscreen or windowed mode, resolution, and vertical sync
* quality level (Low, Medium, or High), LOD bias (0.5 to 2.0), and grass
  density (0 to 100 percent)
* fog renderer: Compute (default) or Jobs
* edge scrolling on or off, pan speed, camera shake on or off
* faction color palette: Standard or Color-blind safe
* UI scale (100, 125, or 150 percent)
* pause on focus loss (default on)

All settings persist through `PlayerPrefs` and take effect immediately.

### What makes it fun

* **Hook.** At 0:15 a neutral supply caravan of three wagons crosses the valley
  below the player's overlook. The ten starting Scouts can intercept it within
  30 seconds of the match start, and each wagon drops ore with a coin burst and
  a chime. A new player wins a fight and a reward before building anything.
* **Mastery curve.** A new player beats Easy in about 35 minutes. A skilled
  player uses control groups, flanks for rear damage, holds high ground with
  Rangers, walls off chokepoints, and scouts through fog, and beats Hard in
  under 20 minutes.
* **Risk and reward.** The Ash Vents pay double but sit in open ground between
  both bases. A Wall Segment blocks the enemy but also blocks the player's own
  reinforcements. Siege Crawlers crush clumped armies but die fast to Scouts if
  left unescorted. The Rally Horn is stronger the more the player has lost, so
  saving it for a turning point beats firing it early, and the production
  boost keeps a losing side in the match.
* **Variety and escalation.** Tiers, growing AI waves, and the minute-15
  Ashstorm, in which armies collide blind.
* **Game feel.**
  * Every order gets a click, a voice bark from one selected unit, and an
    expanding marker ring at the target.
  * Selection circles appear under units on the frame of selection.
  * Units flash white for 0.1 s when hit.
  * Projectiles have tracers, and impacts throw sparks and dust.
  * Siege impacts within 60 m of the camera focus shake the camera.
  * Unit deaths fade out with an ember burst (Ashborne) or a shield clatter
    (Wardens).
  * The minimap pings red where the player is attacked, with a directional
    warning horn.
  * Placing a structure plays a thunk and raises a dust ring as the ground is
    cut.
* **Replay.** Medals, records, the Ironclad unlock, and a map seed and start
  position that vary every match.

## Content

### Assets

| Asset | Source format | Import settings that matter |
|---|---|---|
| `Title.unity`, `Setup.unity` | Unity YAML scene | object identity, UI references |
| `Battlefield.unity` | Unity YAML scene, at least 100,000 GameObjects | prefab instances, static flags, `NavMeshSurface` and its saved navigation data, lighting settings |
| `Terrain_0_0.unity` to `Terrain_7_7.unity` | 64 Unity YAML scenes, loaded additively | `Terrain` and `TerrainCollider`, grouping ID and auto-connect, tree and detail settings |
| Terrain data | 64 `TerrainData` `.asset` files (binary), 4 `TerrainLayer` assets | 513 by 513 heightmaps, alphamaps, trees, detail layers, wind settings |
| Unit prefabs | 5 types, each with a Warden and an Ashborne prefab variant | variant overrides of material, `UnitDefinition` reference, and team color |
| Structure prefabs | 7 structures, each with a construction-site variant | `BoxCollider`, `NavMeshObstacle` (carve, carve-only-stationary), `NavMeshModifier` |
| Prop prefabs | 40 kinds (rocks, ruins, fences, crates, wrecks) | `LODGroup` with three levels and culled, static flags, instanced materials |
| Unit, structure, and prop models | FBX | three LOD meshes per model (about 2,000, 600, and 150 triangles for units), scale factor, read/write off |
| Trees | FBX pine and ash prefabs, each with an `LODGroup` and a billboard level | tree prototypes, billboards, wind response through the URP tree shaders |
| Detail grass | two PNG grass textures and one FBX tuft mesh | grass texture and instanced detail modes |
| Textures | PNG albedo and normal maps, BC7 compression; minimap base image 1024 by 1024, sRGB, no mipmaps, clamp | sRGB, normal-map mode, compression, wrap |
| `UnitLit.shader` | HLSL for URP | GPU instancing with a per-instance team color and hit flash |
| `FogOfWar.compute`, `Minimap.compute` | compute shaders | kernels for clearing, revealing, compositing, and drawing blips |
| `FogOverlay.shader` and `FogOfWarFeature` | HLSL and a `ScriptableRendererFeature` on the Universal Renderer data asset | render pass event before transparents |
| Editor tools | C# editor scripts: `UnitDefinitionEditor` (custom Inspector), `DamageModifierDrawer` (property drawer), `MapPainterWindow` (editor window) | `[CustomEditor]`, `[CustomPropertyDrawer]`, and `EditorWindow` classes in an editor-only assembly |
| Data | 10 `UnitDefinition`, 14 `BuildingDefinition`, 4 `AIProfile`, 1 `MapDefinition` ScriptableObject | serialized values and asset references |
| Effects | Particle System prefabs for sparks, dust, embers, coins, and tracers | bursts, GPU instancing of particle meshes |
| Sounds | WAV effects and voice barks; OGG ambient wind loop | decompress-on-load and compressed-in-memory load types, 3D settings |
| Music | OGG, streaming | streaming load type, looping |
| Fonts | TrueType font with a TextMeshPro SDF font asset | SDF atlas, fallback font |
| `ProjectSettings/` | Unity YAML | layers (Terrain, Units, Structures, Props), fixed time step 0.05 s, maximum allowed time step 0.25 s, quality levels, Burst AOT settings |

### Packages

URP 14.x, AI Navigation 1.x, Burst 1.x, Mathematics 1.x, Collections (released
or verified), Input System 1.x, uGUI 1.0.x, TextMeshPro 3.0.x, and Unity Test
Framework 1.x, at the version lines CMP-002 lists. Each package resolves to
unitas's own implementation of that version line
([Packages](../architecture.md#packages)); no Unity-published package code is
vendored in the sample repository, fetched, or compiled.

### Save data

Settings, medals, records, and unlocks go in `PlayerPrefs`. Match saves are
UTF-8 JSON files under `Application.persistentDataPath/Saves/`. A save holds a
header (format version 3, game version, timestamp, CRC-32 of the body) and a
body with match time, game speed, `UnityEngine.Random` state, camera pose,
every unit (identifier, definition, side, position, rotation, hit points,
order, squad, path target), every structure (definition, pose, construction
progress, hit points, production queue), ore, unit counts, Rally Horn charges,
the AI's state, and each side's explored-fog mask. A full 5,000-unit save is
under 2 MB. The game writes a new file and replaces the old one only after the
write completes. Format version 2 lacks Rally Horn charges.

### Licences and provenance

Every asset family's source, author, and licence is recorded in
`Licenses/content-licenses.json`. Unity packages' licence terms are recorded in
the build's licence report.

## Feature Coverage

| ID | Behavior this game exercises | Scenarios |
|---|---|---|
| PRJ-001 | The project opens in place with no conversion | S1 |
| PRJ-003 | GUIDs and local identifiers stay stable in a 100,000-object scene | S1, S14 |
| PRJ-004 | FBX models with LODs, textures, grass, WAV and OGG clips, fonts | S1, S7 |
| PRJ-007 | Layers, fixed and maximum time steps, quality levels, Burst settings | S1, S11 |
| SER-001 | `UnitDefinition` and `BuildingDefinition` ScriptableObject values | S15, S16 |
| SER-002 | The persistent battlefield scene plus 64 additive terrain scenes | S1, S7 |
| SER-003 | Faction prefab variants and construction-site variants | S1, S16 |
| SER-005 | Binary `TerrainData` assets and saved navigation data | S1, S7 |
| RUN-001 | Spawning and destroying thousands of unit GameObjects | S3, S12 |
| RUN-003 | `FixedUpdate` simulation ticks, `OnApplicationFocus` | S3, S11 |
| RUN-004 | `Time.timeScale` speeds and pause, `fixedDeltaTime`, `maximumDeltaTime`, unscaled camera time | S11 |
| RUN-005 | Additive asynchronous loading and unloading of terrain scenes | S7 |
| RUN-009 | JSON saves, `Application.persistentDataPath`, seeded `UnityEngine.Random` | S12, S13 |
| RUN-010 | The custom `UnitDefinition` Inspector, the damage-modifier property drawer, and the map painter window | S15 |
| RUN-011 | `IJobParallelFor` and `IJobParallelForTransform` steering, Burst jobs, `NativeArray` and Collections containers, safety checks | S3, S4 |
| PHY-001 | `Physics.Raycast` picking, `RaycastCommand` batches for shells, `OverlapSphereNonAlloc` splash, kinematic unit colliders | S5, S8, S16 |
| NAV-001 | `NavMeshSurface` data, `CalculatePath` for squads, `NavMeshAgent` Workers, carving obstacles, runtime `UpdateNavMesh`, path states | S3, S5, S6 |
| ENV-001 | 64 connected terrains, streaming, trees, billboards, detail grass, wind | S7 |
| REN-001 | Perspective camera, `WorldToScreenPoint` and `ScreenPointToRay`, render textures for fog and minimap | S8, S9 |
| REN-002 | Instanced unit and prop materials with per-instance color, LOD meshes | S10 |
| REN-003 | `LODGroup` transitions, LOD bias, shadows, quality levels | S10 |
| REN-004 | Sparks, dust, embers, tracers | S16, S17 |
| REN-006 | URP 14.x Forward, the fog-of-war renderer feature, compute dispatches | S9 |
| REN-007 | Frame pacing with 5,000 units, vertical sync | S3 |
| UI-001 | Menus, HUD, production panels, minimap clicks, keyboard navigation | S2, S8 |
| TXT-001 | TextMeshPro HUD, messages, and results | S2 |
| INP-002 | Input System actions for keyboard, mouse, and the keyboard cursor | S2, S8 |
| AUD-001 | Barks, impacts, warning horn, music, mixer groups | S2, S17 |
| SAV-001 | Full-match saves, corruption detection, migration, settings and records | S12, S13 |
| BLD-001 | Packaged desktop players with build reports | S2 |
| BLD-002 | Headless runs of every scenario that lists the headless editor | S1, S3–S17 |
| TST-002 | Scenarios, checkpoints, approved references, input replay, performance expectations | all |
| PRF-001 | Profiling captures of jobs, worker threads, and rendering | S3 |
| PRF-002 | Scale-floor measurements on the 100,000-object scene | S14 |
| EDT-001 | Hierarchy search, selection, and parenting at scale | S14 |
| EDT-002 | Inspector multi-editing at scale and ScriptableObject edits | S14, S15 |
| EDT-005 | Undo and redo of large edits | S14 |
| AUT-002 | Paginated queries, edits, play control, and captures through the editor's MCP interface | S1, S3–S17 |
| DIA-001 | Job safety exceptions naming job and container, leak reports | S4 |
| ACC-001 | Keyboard-only play, color-blind palette, UI scale | S8 |

## Engine Behavior This Game Depends On

Only engine and package behavior, each tied to a catalog ID. Game rules are
defined under Game Design and implemented by the sample's own scripts; the
engine contains no code specific to this game. Package behavior (AI Navigation,
Burst, Collections, Input System) comes from unitas's own implementation of each
package's public API at the pinned version line
([Packages](../architecture.md#packages), CMP-002). Agents reach the game only
through the editor's generic MCP object, property, method-invocation, input,
capture, query, and profiling commands, with no game-specific command.

### Scripting and runtime

* `IJobParallelFor` and `IJobParallelForTransform` with `TransformAccessArray`
  run on worker threads; `JobHandle` dependencies complete before the main
  thread reads their output. `NativeArray` and `NativeParallelMultiHashMap`
  behave as RUN-011 defines. (RUN-011)
* `[BurstCompile]` jobs with `FloatMode.Strict` run with native Burst
  acceleration on every declared desktop target, and with Burst compilation
  disabled produce the same results within the declared mode. (RUN-011)
* In the editor and development players, reading a container before its job
  completes raises the safety exception naming the job and the container; an
  undisposed allocation is reported as a leak, with its allocation site when
  full stack traces are enabled. Release players omit these checks. (RUN-011,
  DIA-001)
* `Time.timeScale` of 0, 0.5, 1, 2, and 4 scales `FixedUpdate` ticks.
  `Time.maximumDeltaTime` caps catch-up ticks per frame. `Time.unscaledDeltaTime`
  is unaffected by the time scale. (RUN-004)
* Changing a ScriptableObject during play changes it for every reader from
  then on, and, as in Unity, the change remains on the asset after play stops.
  (SER-001)

### Project and content

* A scene with at least 100,000 GameObjects opens with every object, its prefab
  instance link, and its overrides. (SER-002, SER-003)
* Additively loaded terrain scenes keep their `TerrainData` identity. (SER-005)

### Simulation

* `NavMesh.CalculatePath` returns complete, partial, or invalid paths; 200
  calls in one tick all resolve in that tick. (NAV-001)
* `NavMeshAgent` avoids other agents and carving obstacles. A
  `NavMeshObstacle` with carve-only-stationary carves once it is stationary for
  its threshold, and agents whose paths cross it replan. (NAV-001)
* `NavMeshSurface.UpdateNavMesh` for a region changes navigation data only in
  that region; a failed update keeps the previous data. (NAV-001)
* Navigation data stays loaded when the scenes that contributed geometry
  unload. (NAV-001)
* `Physics.Raycast` from `ScreenPointToRay` hits colliders by layer mask;
  `RaycastCommand` batches and `Physics.OverlapSphereNonAlloc` report their
  results and counts as PHY-001 defines. (PHY-001, REN-001)

### Rendering and animation

* Neighboring terrains with the same grouping ID and auto-connect meet without
  gaps. Trees render with LOD transitions and billboards; detail grass renders
  instanced; grass and trees move with a `WindZone`. (ENV-001)
* Materials with GPU instancing take per-instance values from
  `MaterialPropertyBlock`; instanced and non-instanced rendering produce the
  same image, and the instanced draw count is reported. (REN-002)
* Each `LODGroup` changes level at its authored screen-relative heights times
  the LOD bias. (REN-003)
* A compute shader dispatched from script writes a random-write
  `RenderTexture`; a `ScriptableRendererFeature` at its render pass event draws
  it; a `RenderTexture` shown in a `RawImage` updates on the same frame. Every
  declared target supports compute. (REN-006, REN-001)

### Audio and video

* A one-shot started from script plays within one frame. A 3D source's
  position sets its stereo pan relative to the listener. Mixer group volumes
  set from script take effect on the next audio buffer. (AUD-001)

### Input

* Several bindings and composite controls of one action produce the same
  action. Losing window focus releases every held control. (INP-002)

### UI and text

* Menus and panels work with keyboard and pointer, with the selected element's
  visual state always shown. Text set during a tick's frame renders that frame.
  (UI-001, TXT-001)

### Player and build

* Each packaged player starts on the first scene in the build and keeps
  `PlayerPrefs` and persistent files between launches. (BLD-001, SAV-001)
* The build report lists scenes, assets, sizes, Burst targets, and the build
  identifier. (BLD-001)

### Editor and automation

* A 100,000-GameObject scene stays operable in the visual editor and through
  the editor's MCP interface as PRF-002 defines: hierarchy and asset queries,
  search, selection, property edits, undo, and save. Queries through the
  editor's MCP interface return paginated results with revision-bound cursors.
  (PRF-002, EDT-001, EDT-002, EDT-005, AUT-002)
* A custom Inspector, a property drawer, and an `EditorWindow` that paints onto
  terrain and writes to a ScriptableObject with undo draw and work in the unitas
  editor; the custom Inspector redraws when a value changes. (RUN-010, EDT-002)
* An agent starts and reads profiling captures through the editor's MCP
  interface. (PRF-001, AUT-002)

## Presentation and Performance

### Visual checkpoints

Each has a unitas-approved reference:

* the opening overlook at 0:00 with the caravan in view
* the Grand Battle state from three fixed camera poses at High quality
* fog of war with a Watchtower reveal, and the same view with fog off
* the minimap at the Grand Battle state
* a construction site with its placement ghost, valid and invalid
* the results panel with a gold medal

### Audio checkpoints

The order bark starts within one frame of the order. The warning horn comes
from the attacked side of the stereo field.

### Project-declared expectations

On the declared reference desktop class at 1080p and High quality:

* 60 frames per second or more in the Grand Battle state with all 5,000 units
  moving at 1x. No frame over 33 ms while terrain tiles stream at the maximum
  pan speed of 120 m/s.
* All jobs of one simulation tick finish within 6 ms of wall time, and the 200
  path requests of a tick take under 2 ms of main-thread time.
* Units and props draw in 400 draw calls or fewer, with instancing active.
* A navigation update after a structure finishes completes within 0.5 s.
* Match load in under 15 seconds, save in under 1.5 seconds, and load in under
  8 seconds.

Expectations are evaluated only on the declared class; other runs record the
measurement and report the expectation as unevaluable (TST-002). These are the
sample's own expectations, not the PRF-002 engine budgets.

### PRF-002 measurements

Reported in their own section of the performance report and never as sample
expectations. On `Battlefield.unity`, in the visual editor and through the
editor's MCP interface in a headless session, the game measures:

* opening the scene, cold and warm
* the first page of a hierarchy query with a page size of 1,000, and a full
  traversal by cursor
* a name search ("Rock_") and a component search (`LODGroup`)
* selecting 10,000 objects
* changing one property on 10,000 selected objects
* undo and redo of that change
* saving the scene
* entering play mode

Each measurement records the unitas version, hardware, operating system, and
target, and is compared with the release's declared PRF-002 budget for that
operation.

## Acceptance Scenarios

Outcomes use the TST-002 vocabulary: passed, failed, skipped, unevaluable, and
awaiting human review. Each perturbation names its mechanism: a TST-002
replay-stream event, a harness action outside the player, or a
development-build debug command implemented in the sample's scripts.

### S1. Open and inspect

* **Runs in:** headless editor; editor play mode.
* **Preconditions:** no import cache exists.
* **Steps:** open the project, then `Battlefield.unity`; reopen both.
* **Expected:** all assets import with no errors. The compatibility report lists
  every package with its tier and no unsupported item. The scene reports at
  least 100,000 GameObjects. Reopening yields identical GUIDs and local
  identifiers.
* **Demonstrates:** PRJ-001, PRJ-003, PRJ-004, PRJ-007, SER-002, SER-003, SER-005

### S2. Happy path

* **Runs in:** editor play mode; desktop packaged players.
* **Preconditions:** the "Weakened base" state against Easy.
* **Steps:** drive recorded input that builds a Barracks, trains 20 Lancers, and
  destroys the enemy Command Hall. Then replay a full recorded match at 4x in
  each packaged desktop player.
* **Expected:** victory appears when the Command Hall falls. The results panel
  shows time, the army-value graph, a medal, and a record. Each player's build
  report lists scenes, assets, sizes, Burst targets, and the build identifier.
* **Demonstrates:** UI-001, TXT-001, INP-002, AUD-001, BLD-001

### S3. 5,000 moving units

* **Runs in:** headless editor; development desktop packaged players.
* **Preconditions:** the "Grand Battle" state with 2,500 units per side.
* **Steps:** order every squad to cross the basin and run 60 seconds of game
  time. Capture a profile of 300 frames.
* **Expected:** every unit's position changes in every 1-second window while its
  order is active. No unit is ever more than 0.5 m from navigation data. No job
  safety exception occurs. The capture shows the steering, target-search, and
  transform jobs by name on worker threads, marks each as Burst-compiled with
  native acceleration, and shows the rendering cost of cameras, instanced
  draws, and the fog pass. The frame-rate expectation is evaluated on the
  declared class.
* **Demonstrates:** RUN-011, NAV-001, RUN-001, RUN-003, PRF-001, REN-007

### S4. Burst equivalence and job safety

* **Runs in:** editor play mode; headless editor; development and release
  desktop packaged players.
* **Preconditions:** the "Grand Battle" state with a fixed seed.
* **Steps:** run 600 ticks with Burst compilation enabled, then with it disabled
  (the editor's Burst setting, or `--burst-disable-compilation` in a player). In
  a development player and the editor, trigger the development-build debug
  commands "Read positions early" and "Leak one allocation." Repeat the debug
  commands in a release player.
* **Expected:** unit positions match within 0.0001 m, and hit points and unit
  counts match exactly. The early read raises the safety exception naming the
  steering job and its container. The leak is reported with its allocation
  site. The release player shows neither diagnostic.
* **Demonstrates:** RUN-011, DIA-001

### S5. Building and paths

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** the "Construction site" state.
* **Steps:** place a Barracks on the lot, let it finish, then destroy it.
  Separately, order a squad to the plateau top through a blocked ramp, and to a
  point in the river.
* **Expected:** within 1 second of the site becoming stationary, the squad and
  the Workers route around it. After it finishes, `NavMesh.CalculatePath`
  through its footprint does not cross it, and 1,000 sampled points outside the
  update region return the same `NavMesh.SamplePosition` results as before.
  After it is destroyed, paths cross the lot again. The plateau order returns a
  partial path and the unreachable message. The river order returns an invalid
  path and the same message, and no unit oscillates for more than 2 seconds.
* **Demonstrates:** NAV-001, PHY-001

### S6. Placement rules

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** the "Opening" state.
* **Steps:** try to place a Barracks on a 20-degree slope, in the river, over a
  ruin, and across the only route from the Command Hall to the ore fields.
* **Expected:** each attempt shows a red ghost with its reason, and no site is
  created.
* **Demonstrates:** NAV-001, PHY-001, UI-001

### S7. Terrain streaming

* **Runs in:** headless editor; desktop packaged players.
* **Preconditions:** the "Opening" state.
* **Steps:** fly the camera along a recorded route across the whole valley at
  120 m/s, while a squad walks across tiles that unload behind the camera.
* **Expected:** only tiles within the load radius are loaded at each checkpoint.
  An identity-map capture shows terrain or scene content in every pixel below
  the horizon. Neighboring tiles meet without seams. Trees switch to billboards
  at their billboard distance, and grass and trees sway in the wind. Units on
  an unloaded tile keep moving. When the tile loads, they stand within 0.05 m
  of its surface. The frame-time expectation is evaluated on the declared
  class.
* **Demonstrates:** ENV-001, SER-002, SER-005, RUN-005, PRJ-004

### S8. Box selection and picking

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** the "Grand Battle" state with a fixed camera pose.
* **Steps:** box-select a 600 by 400 px rectangle. Click one unit, one structure,
  and a terrain point. Repeat with keyboard only and the color-blind palette at
  150 percent UI scale.
* **Expected:** the selection equals the set of own, visible units whose
  `WorldToScreenPoint` falls inside the rectangle, computed independently by the
  scenario. Selection completes within one frame. Clicks select the unit and
  the structure under the pointer. The terrain move marker lands within 0.1 m
  of `Terrain.SampleHeight` at the ray's hit. Enemy units in fog are never
  selected. The keyboard-only run selects the same sets.
* **Demonstrates:** REN-001, PHY-001, INP-002, UI-001, ACC-001

### S9. Fog of war and minimap

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** the "Fog test" state.
* **Steps:** render with the Compute fog renderer, then with the Jobs renderer.
  Then start an Ashstorm (a development-build debug command).
* **Expected:** the revealed cells equal the expected discs exactly in both
  renderers. Rendering diagnostics list `FogOfWarFeature` and the compute
  dispatches. Enemy units in fog are not drawn. Minimap blips sit at the
  expected pixels. During the Ashstorm every revealed radius shrinks by 40
  percent. Captures match their approved references.
* **Demonstrates:** REN-001, REN-006

### S10. Instancing and LOD

* **Runs in:** headless editor; packaged players.
* **Preconditions:** the "Grand Battle" state.
* **Steps:** capture three fixed views with instancing enabled, then disabled on
  the unit and prop materials. Then zoom from 30 m to 220 m height at LOD bias
  1.0 and 2.0.
* **Expected:** the instanced and non-instanced captures match within the
  reference's tolerance, and the instanced draw count is reported only in the
  first. Team colors and hit flashes are correct per instance. Each `LODGroup`
  reports the level its screen height and the LOD bias select.
* **Demonstrates:** REN-002, REN-003

### S11. Speed controls

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** the "Grand Battle" state.
* **Steps:** run 10 real seconds at each speed, then pause for 5 seconds while
  panning and queuing an order. Then run at 4x with the frame time throttled
  below the expectation (a development-build debug command that adds a fixed
  delay per frame). Remove and restore window focus, through a focus event in
  the replay stream. Run the same saved state for 1,200 ticks at 1x and at 4x
  with a fixed simulated frame delta.
* **Expected:** game time advances 5, 10, 20, and 40 seconds within one tick.
  While paused, no tick runs, the camera pans, and the queued order runs on
  resume. When throttled, ticks per frame never exceed the `maximumDeltaTime`
  cap and the HUD shows the effective speed. Losing focus pauses the game. The
  1x and 4x runs end with identical unit state.
* **Demonstrates:** RUN-004, RUN-003, PRJ-007

### S12. Save and load a full match

* **Runs in:** desktop packaged players; headless editor with a play-session
  restart.
* **Preconditions:** the "Grand Battle" state at 4x.
* **Steps:** quick-save, play 30 seconds, quit to title, relaunch (a harness
  action that restarts the player, or, headless, a new play session with the
  same storage root), and load the save.
* **Expected:** the save is under 2 MB, and its duration is evaluated against
  the expectation on the declared class. After loading, unit count, structures,
  positions within 0.001 m, hit points, orders, ore, Rally Horn charges, fog
  masks, and the AI's state equal the saved values. The game loads paused. The
  next 600 ticks match a run that never saved.
* **Demonstrates:** SAV-001, RUN-009, RUN-001

### S13. Corrupt, old, and interrupted saves

* **Runs in:** desktop packaged players.
* **Preconditions:** committed fixture saves: one with one byte changed, a
  truncated save, a format version 2 save, and a format version 9 save.
* **Steps:** load each fixture. Then kill the player process during a save (a
  harness action that terminates the process while the save's write is in
  progress) and relaunch.
* **Expected:** the changed and truncated saves report "Save is damaged" and
  leave the slot list intact. The version 2 save loads with zero Rally Horn
  charges and reports migration from 2 to 3. The version 9 save reports "Made by
  a newer version." After termination, the previous save in that slot still
  loads on targets whose storage documentation says replacement is atomic.
* **Demonstrates:** SAV-001, RUN-009

### S14. Scale-floor editing

* **Runs in:** visual editor; headless editor through the editor's MCP
  interface.
* **Preconditions:** `Battlefield.unity` with at least 100,000 GameObjects.
* **Steps:** query the hierarchy in pages of 1,000, search for "Rock_" and for
  `LODGroup`, select 10,000 props, set their static flags, undo, redo, reparent
  100 props under a new group object, and save.
* **Expected:** every query is paginated, and each page reports the revision it
  belongs to. Mixed values show in the multi-object Inspector. Undo and redo
  restore exact values. The saved scene reopens in unitas with identical
  identities. Where Unity 2022.3 LTS is available, it opens the saved scene;
  otherwise that check is unevaluable. Each operation's time is recorded as a
  PRF-002 measurement, in its own report section.
* **Demonstrates:** PRF-002, EDT-001, EDT-002, EDT-005, AUT-002, PRJ-003

### S15. Unit data

* **Runs in:** editor play mode, visual and headless.
* **Preconditions:** the "Proving ground" state in the editor.
* **Steps:** during play, change the Ranger `UnitDefinition` speed from 6 to
  9 m/s in the Inspector, then change its range through the editor's MCP
  interface. Stop play. Outside play, move one ore field with the map painter
  window, and undo.
* **Expected:** every Ranger of that faction moves at 9 m/s from the next tick,
  and the new range applies to the next attack. The custom Inspector's counter
  preview (damage per second against each unit type with the counter, rear, and
  high-ground modifiers applied) redraws with the new values on the same frame.
  The asset keeps both values after play stops and shows as an unsaved asset
  change. The map painter moves the ore field, and undo restores it.
* **Demonstrates:** SER-001, RUN-010, EDT-002, AUT-002

### S16. Counters and flanks

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** the "Proving ground" state.
* **Steps:** run five fixed duels of 20 units each: Lancers against Rangers,
  Rangers against Scouts, Scouts against Siege Crawlers, Siege Crawlers against
  a clumped Lancer squad, and Rangers on high ground against Rangers below.
  Then attack a Ranger squad from behind.
* **Expected:** the stronger side wins each duel, and damage per hit matches the
  counter, rear, and high-ground modifiers exactly. Siege shells damage every
  unit within 6 m of the impact. Hit flashes and impact sparks appear on each
  hit.
* **Demonstrates:** SER-001, PHY-001, REN-004, SER-003

### S17. Hook and comeback

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** the "Opening" state, then the "Losing side" state.
* **Steps:** intercept the caravan with the starting Scouts. Then, in the
  "Losing side" state, lose 300 units and sound the Rally Horn.
* **Expected:** the caravan is in view from 0:15 and within reach before 0:30.
  Each wagon drops 100 ore with a burst and a chime. In the "Losing side" state,
  the Horn gains three charges and production speeds up 25 percent below 40
  percent army value. Sounding the Horn gives units within 40 m 30 percent more
  speed and damage for exactly 10 seconds of game time.
* **Demonstrates:** REN-004, AUD-001, RUN-004

## Manual Playtest Guidance

A person judges the following, and a headless run reports these checkpoints as
awaiting human review, never as passed:

* Whether the caravan hook makes the first minute exciting.
* Whether the two factions feel different to play and are both winnable.
* Whether counters are readable in a fight without opening the unit sheet.
* Whether the Rally Horn makes a losing match feel recoverable without making a
  winning one feel stolen.
* Whether a 5,000-unit battle stays readable, with fights, health, and fog edges
  clear at a glance.
* Whether panning, zoom, and selection feel instant.
* Whether the Ashstorm creates tension rather than frustration.
* Whether the AI's difficulty steps feel fair.

## Agent Authoring Change Requests

### CR1. Mortar Team

* **Request:** add a Mortar Team unit with a new `UnitDefinition`, Warden and
  Ashborne prefab variants, a Barracks production entry, and a 70 m indirect
  attack that ignores high ground.
* **Acceptance scenario:** in the "Proving ground" state, a Mortar Team trained
  from the Barracks hits targets at 70 m and not at 71 m, with damage per hit
  matching the counter modifiers against each class.
* **Test:** a test that checks its range and damage against each counter class.

### CR2. Bridge

* **Request:** add a Bridge structure that places a `NavMeshLink` across a
  river.
* **Acceptance scenario:** squads use the bridge when it shortens their path and
  stop using it after it is destroyed.
* **Test:** a test that compares path lengths with and without the bridge.

## Accessibility

* Every action can be performed with the keyboard through the keyboard cursor
  and hotkeys (S8).
* Menus expose accessibility names (S2).
* The color-blind palette uses blue and yellow, and the minimap draws own units
  as squares and enemy units as triangles, so color is never the only cue (S8,
  S9).
* UI scale reaches 150 percent (S8).
* Camera shake can be turned off (S17 with camera shake off).
* Every voice bark and warning horn also shows a text message in the event log
  (S17).

## Saved and Reproducible States

| State | How it is reached | Used by |
|---|---|---|
| "Opening": a new match on seed 1234, Wardens against Ashborne on Normal | save file | S6, S7, S17 |
| "Weakened base": Easy AI with its Command Hall at 10 percent | save file | S2 |
| "Grand Battle": 2,500 units per side in the central basin | save file | S3, S4, S8, S10, S11, S12 |
| "Construction site": a squad of 25 Lancers and four Workers on a path across an empty lot | save file | S5 |
| "Fog test": units at fixed positions with known sight radii | save file | S9 |
| "Proving ground": an empty basin with duel squads at fixed positions | save file | S15, S16 |
| "Losing side": the player at 35 percent of the AI's army value | save file | S17 |
| "Old save": a format version 2 save of the "Grand Battle" state | committed fixture file | S13 |

## Known Limitations and Unsupported Conditions

* A desktop host whose graphics device offers no compute shaders is not a
  declared target.

Any behavior this document requires that is not demonstrated is reported with
its real compatibility state.

## Acceptance Outcomes

* The strategy skirmish moves 5,000 units at once through Burst-compiled jobs
  over native containers, gives the same results with Burst disabled, and
  raises job safety diagnostics in development builds only. (SMP-001, RUN-011,
  DIA-001)
* Placing and removing structures carves and updates navigation data at
  runtime, squads replan around them, and unreachable orders report partial or
  invalid paths. (SMP-001, NAV-001)
* The 64-tile terrain streams with trees, grass, and wind and never shows a
  gap, and units stay grounded across loaded and unloaded tiles. (SMP-001,
  ENV-001, RUN-005)
* Box selection and click picking match the camera's screen projection and the
  physics queries exactly. (SMP-001, REN-001, PHY-001)
* Fog of war and the minimap render through compute shaders, render textures,
  and a URP renderer feature, and the Jobs renderer produces the same cells.
  (SMP-001, REN-001, REN-006)
* Units and props render with GPU instancing and LOD groups, with the same image
  as non-instanced rendering. (SMP-001, REN-002, REN-003)
* A full 5,000-unit match saves and loads with identical state, and damaged,
  old, and newer saves are each reported distinctly. (SMP-001, SAV-001)
* Speed controls and pause scale the simulation through `Time.timeScale`
  without breaking camera control or queued orders. (SMP-001, RUN-004)
* The 100,000-GameObject battlefield scene stays editable in the visual editor
  and through the editor's MCP interface, and its PRF-002 measurements are
  reported apart from the sample's own expectations. (SMP-001, PRF-002,
  EDT-001, EDT-002, AUT-002)
* Profiling captures show the game's jobs on worker threads and its rendering
  cost. (SMP-001, PRF-001)
* Counters, flanking, high ground, the Rally Horn, and the opening caravan
  behave as specified, and changes to `UnitDefinition` assets apply from the
  next tick and redraw in the custom Inspector. (SMP-001, SER-001, RUN-010)
