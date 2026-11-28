# Sample 7: Twin-Stick Arena Shooter ("Hexstorm")

## Overview

The player pilots a hover drone through ten waves of enemies in a generated
arena. One stick moves, the other aims. Between waves the player picks
upgrades that combine into weapon synergies. Wave 10, the Storm, fills the
arena with 300 enemies and up to 2,000 projectiles. A run ends when the last
life is lost or wave 10 is cleared. The score goes to a leaderboard served by
a local test fixture. A run takes 8 to 12 minutes.

This is an original, redistributable unitas sample. Its drone, enemies,
arenas, effects, music, and sounds are made for unitas and are distinct from
any official Unity sample or commercial shooter. The leaderboard server is a
local test fixture shipped with the sample, not an external service. The game
is one of the demonstrations of pooling at scale, data-driven content, editor
extensions, and web requests; the acceptance of those features is owned by the
domain specifications it cites.

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

URP 14.x, Forward+ rendering path, HDR on.

### Input modes and devices

Keyboard and mouse, gamepad, and keyboard only, through an Input System 1.x
action asset with three control schemes. Menus accept the pointer, keyboard,
and gamepad.

Headless runs use synthetic devices marked synthetic; haptic and feel
checkpoints need physical hardware and are otherwise unevaluable.

## Game Design

### Core loop and flow

1. The title screen shows Play, Leaderboard, Settings, and Quit. Play offers a
   random seed or a typed seed (any 32-bit integer).
2. The seed generates the arena. The drone spawns at the center with three
   lives of 5 hull points each and the Pulse weapon.
3. Each wave announces itself, then spawns enemy groups from six portals. A
   wave ends when every enemy of it is destroyed.
4. After waves 2, 4, 6, and 8, the game pauses on an upgrade screen with
   three cards. The player picks one. Every fourth wave adds a weapon.
5. Clearing wave 10 or losing the last life opens the results screen: score,
   wave reached, kills, best multiplier, and medal. The player enters three
   initials, and the score posts to the leaderboard.
6. Results offer Retry Same Seed, New Seed, and Quit to Title. Retry
   regenerates the same arena and returns every pooled object to its pool.
7. Quit on the title exits the player.

### Rules and mechanics

* **Dash** moves 4 m in 0.15 s with 0.3 s of invulnerability and recharges in
  1.2 s.
* **Weapons.** Pulse fires 12 shots per second at 40 m/s, 1 damage each.
  Scatter fires 5 pellets in a 30-degree spread 3 times per second, 2 damage
  each. Arc fires a bolt that chains to 3 more enemies within 4 m, 3 damage,
  and shocks each for 1.5 s. The drone gains Scatter after wave 4 and Arc
  after wave 8.
* **Enemies.** Swarmers rush the drone (2 hit points). Spitters keep 8 m away
  and fire rings of 12 projectiles (6 hit points). Brutes charge and shrug off
  knockback (30 hit points). Splitters burst into three Swarmers (8 hit
  points). The Warden mini-boss in wave 5 has four armor plates that must
  break before its core takes damage.
* **Data.** Wave content comes from `WaveDefinition` assets, weapons from
  `WeaponDefinition`, enemies from `EnemyDefinition`, and upgrades from
  `UpgradeDefinition`. All are ScriptableObjects. The game's `OnValidate`
  clamps group counts to 1 to 300 and delays to 0 or more, and warns when a
  wave's peak live enemy count exceeds 300.
* **Pools.** All projectiles come from one pool of 2,000 and all enemies from
  per-type pools totaling 300, prewarmed while the arena loads. After warm-up
  the game instantiates nothing. A fire request with an empty pool is dropped
  and counted. A spawn with no free enemy slot waits for one.
* **Hits.** Projectiles detect hits with sphere casts on the Enemies, Player,
  and Walls layers each fixed step (1/60 s). Hits call
  `SendMessage("ApplyDamage", damage, SendMessageOptions.DontRequireReceiver)`.
  Wave start calls `BroadcastMessage("OnWaveStarted", index)` on the arena
  root. Warden plates call `SendMessageUpwards("OnPlateBroken")`. Spawn groups
  use `Invoke` with their delay. `EnemyHealth.onDeath` lists its listeners in
  the order score, gem drop, impulse.
* **Procedural arenas.** The seed sets `Random.InitState`. The arena is a
  hexagonal floor of radius 28 m built from 1 m hex cells as one runtime
  `Mesh` with 32-bit indices and more than 65,535 vertices, plus 6 to 12
  pillars, 3 to 6 cover walls, and a boundary wall, each with a `MeshCollider`
  from its generated mesh. Spawn order, portal choice, and upgrade offers come
  from the same seed. The overlay shows a layout hash computed from the
  generated vertex data.
* **Feedback.** Hit flash and dissolve values reach each enemy through
  `MaterialPropertyBlock`. The hit-stop sets `Time.timeScale` to 0.05 for
  40 ms of unscaled time on Brute and Warden kills. Rumble: low 0.3 and high
  0.6 for 0.15 s on a hit, 0.8 and 0.8 for 0.4 s on a life lost; `ResetHaptics`
  runs on quit. Each Cinemachine impulse's amplitude is multiplied by the shake
  setting.
* **Leaderboard client.** The client uses `UnityWebRequest` with a 5-second
  timeout and distinguishes success, connection error, protocol error with its
  code, and data processing error, showing "Leaderboard offline", "Server
  error (500)", "Leaderboard data unreadable", or a timeout. Bodies are
  `JsonUtility` JSON wrapped in an object. Unsent scores join
  `pending_scores.json` and post once when the server answers.
* **Debug overlay.** The IMGUI overlay is a `GUILayout.Window` showing frame
  time, live and pooled counts, dropped fire requests, seed, layout hash, wave,
  and multiplier. It has God Mode and Show Hitboxes toggles, a time-scale
  slider (0.1 to 2.0), and Skip Wave, Spawn 50 Swarmers, and Fill Overdrive
  buttons. `DragWindow` moves it. A click that the overlay uses never fires the
  weapon.
* **Editor tools.** A custom `WeaponDefinition` inspector shows a DPS preview, a
  projectile-pattern preview, and a synergy warning. The Wave Designer
  `EditorWindow` draws a timeline of spawn groups that can be dragged to change
  their delays. Both are built with IMGUI (`EditorGUILayout`). The commands
  `Arena/Waves/Validate All Waves`, `Arena/Waves/New Wave From Template`, and
  `Arena/Waves/Preview Wave`, and the context menus `Sort Spawn Groups By
  Delay` and `Double All Counts`, are project editor commands.

### Levels and progression

| Waves | Content |
|---|---|
| 1 to 4 | each introduces one enemy type; wave 1 is 24 Swarmers |
| 5 | the Warden |
| 6 to 9 | every type at rising counts |
| 10 | the Storm: 300 enemies and up to 2,000 projectiles |

Bronze for reaching wave 5, Silver for clearing wave 10, and Gold for clearing
it without losing a life. Endless mode unlocks after the first wave 10 clear
and repeats wave 10 with 10% more health each loop.

### Scoring, failure, and edge rules

* Kills within 1.5 s of each other raise the multiplier one step, from x1 to
  x8. Taking damage resets it to x1.
* An enemy projectile that passes within 0.5 m of the drone without hitting is
  a graze and adds 5% to the Overdrive meter. A full meter gives 6 seconds of
  double fire rate and a x2 score bonus.
* Gems worth 10 points each drop where enemies die and vanish after 6 seconds.
* No enemy or projectile leaves the arena or passes through a wall.
* An enemy hit by several projectiles in one step dies once, scores once, and
  returns to its pool once.
* The leaderboard never blocks play. When it fails, the score stays in the
  local pending queue.

### Controls

| Action | Keyboard and mouse | Gamepad | Keyboard only |
|---|---|---|---|
| Move | WASD | Left stick | WASD |
| Aim | Mouse pointer | Right stick | Arrow keys (8 directions) |
| Fire | Left mouse button (hold) | Right trigger, or right stick past 0.5 | Fires while aiming |
| Dash | Space or right mouse button | South button | Space |
| Overdrive | F | North button | F |
| Next and previous weapon | Mouse wheel, Q and E | Right and left shoulder | Q and E |
| Pause | Escape or P | Start | Escape |
| Debug overlay | F3 | View button (hold 1 s) | F3 |
| Menus: navigate, confirm, back | Pointer, Enter, Escape | D-pad, South, East | Arrows, Enter, Escape |

### Pause, focus, and lifecycle

Pause sets `Time.timeScale` to zero, pauses rumble with `PauseHaptics`, shows
Resume, Restart, Settings, and Quit to Title, and keeps menu sounds playing.
Losing window focus pauses the game.

### Settings

Master, music, and effects volume, display mode, resolution, vertical sync,
quality (Low, Medium, High), screen shake (0 to 100%), rumble on or off,
auto-fire, reduced motion, upgrade card text size (100% or 150%), and
rebinding. They persist through `PlayerPrefs` and apply immediately.

### What makes it fun

* **Hook.** Wave 1 starts 3 seconds after Play with 24 Swarmers that pop in
  one Pulse hit, with a flash, a dissolve, and a gem burst each. The first
  multiplier step arrives within 10 seconds.
* **Mastery curve.** A skilled player holds x8 through whole waves, dashes
  through Spitter rings, and clears wave 10 without losing a life.
* **Risk and reward.** Grazing fills Overdrive, and gems pull the player into
  the fight. Each upgrade screen offers three of twelve cards, such as
  Overcharge (Pulse shots pierce one enemy), Ricochet (Scatter pellets bounce
  once off walls), Conductor (Pulse deals double damage to shocked enemies),
  and Magnet (gem pickup radius from 1 m to 4 m). Two cards that name each
  other form a synergy, marked on the card. The offers come from the seed, so a
  seed can be shared and replanned.
* **Variety and escalation.** The wave table escalates to the Storm, and each
  arena seed changes the cover, so routes differ.
* **Game feel.**
  * Hit flash for 0.08 s and a dissolve over 0.6 s on death.
  * Explosions with spark and smoke sub-emitters, and bloom on every shot.
  * A Cinemachine impulse whose amplitude scales with what exploded.
  * A 40 ms hit-stop on Brute and Warden kills.
  * A chromatic aberration pulse for 0.2 s when the drone is hit.
  * Rumble on hits and deaths.
  * A multiplier chime that rises one semitone per step.
* **Replay.** Medals, local best score per seed, a top-10 leaderboard, and
  Endless mode.

## Content

### Assets

| Asset | Source format | Import settings that matter |
|---|---|---|
| `Title.unity`, `Arena.unity` | Unity YAML scene | prefab instances, URP Volume, Cinemachine rig |
| Drone, enemy, and projectile prefabs | Unity YAML prefabs and variants | `Splitter` as a variant of `Swarmer`, serialized UnityEvents |
| Drone and enemy models | FBX, Read/Write off | mesh compression, no rig |
| `Dissolve.shadergraph`, `HitFlash.shadergraph`, `Noise.shadersubgraph` | Shader Graph 14.x | exposed `_DissolveAmount`, `_EdgeColor`, `_FlashAmount`, sub graph reference |
| Materials | `.mat` on the graphs and URP Lit | GPU instancing on projectile materials |
| Explosion, muzzle, and spark effects | Particle System prefabs | sub-emitters on birth and death, trails module, world space |
| Weapon, enemy, wave, and upgrade data | ScriptableObject `.asset` files | serialized lists, enum fields, object references |
| `WaveDesignerWindow.cs`, `WeaponDefinitionEditor.cs` | Editor scripts in an editor-only assembly | `[MenuItem]` and `[ContextMenu]` commands |
| Sound effects | WAV 44.1 kHz | Decompress On Load, priority per source |
| Music | OGG Vorbis | Streaming load type, looping |
| `ShooterMixer.mixer` | Unity YAML audio mixer | groups Music, Weapons, Explosions, Player, UI; compressor on Effects |
| `ShooterControls.inputactions` | Input action asset | three control schemes, processors |
| Fonts | TrueType with TextMeshPro SDF assets | SDF atlas, fallback font |
| `Tests/EditMode`, `Tests/PlayMode` | Assembly definitions with test references | `UNITY_INCLUDE_TESTS` constraint |
| `ProjectSettings/` | Unity YAML | layers, physics matrix, 32 real and 256 virtual voices |

### Packages

URP 14.x, Shader Graph 14.x, Cinemachine 2.x, Input System 1.x, uGUI 1.0.x,
TextMeshPro 3.0.x, and Unity Test Framework 1.x, at the version lines CMP-002
lists. Packages are resolved from their registry when the project opens and are
never vendored in the sample repository.

### Save data

Settings, local records, and medals in `PlayerPrefs`; `pending_scores.json`
under `Application.persistentDataPath`.

### Licences and provenance

Every asset family's source, author, and licence is recorded in
`Licenses/content-licenses.json`. Unity packages' licence terms are recorded in
the build's licence report.

## Feature Coverage

| ID | Behavior this game exercises | Scenarios |
|---|---|---|
| PRJ-001 | Open in place | S1 |
| PRJ-003 | Stable identities | S1 |
| PRJ-004 | Models and audio | S1 |
| PRJ-007 | Voice and physics settings | S1 |
| SER-001 | ScriptableObject data, serialized events | S1, S10 |
| SER-003 | Prefab variants | S1 |
| EDT-002 | Custom-inspector and default-inspector edits of ScriptableObject data | S10, S11 |
| RUN-001 | Pooling by activation, 2,000 projectiles and 300 enemies, no instantiation after warm-up | S3, S16 |
| RUN-002 | Scripts | S1 |
| RUN-003 | Lifecycle on pool reuse | S2 |
| RUN-004 | `Time.timeScale` pause and hit-stop | S5, S6 |
| RUN-008 | UnityEvents, `SendMessage`, `BroadcastMessage`, `SendMessageUpwards`, `Invoke` | S14 |
| RUN-009 | Seeded randomness, UnityWebRequest, JsonUtility, pending queue file | S4, S8, S9 |
| RUN-010 | Editor commands, `OnValidate`, the custom inspector and the Wave Designer window drawn and operable | S1, S11 |
| PHY-001 | Mesh colliders from generated meshes, sphere casts, layer matrix | S3, S4 |
| REN-002 | Runtime meshes with 32-bit indices, `MaterialPropertyBlock` on graph materials | S4, S7 |
| REN-003 | URP bloom, tonemapping, vignette, chromatic aberration | S7 |
| REN-004 | Explosions with sub-emitters, projectile trails, muzzle flashes | S7 |
| REN-006 | Shader Graph dissolve and hit flash, Forward+, GPU instancing | S7 |
| CIN-001 | Cinemachine impulse sources and the impulse listener extension | S13 |
| INP-002 | Three control schemes, rumble with explicit status, rebinding | S2, S13 |
| AUD-001 | Voice limiting by priority, mixer compressor, streamed music | S13 |
| UI-001 | Menus, upgrade cards, HUD, results and initials entry | S2, S6 |
| TXT-001 | HUD and card text | S2 |
| UI-002 | IMGUI debug overlay with window, toggles, slider, and buttons | S12 |
| SAV-001 | Settings, local records, medals, pending scores | S9 |
| TST-003 | EditMode and PlayMode tests through the Unity Test Framework | S15 |
| PRF-001 | Peak-load profile with project-declared expectations reported apart from engine budgets | S3 |
| ACC-001 | Keyboard-only aiming, auto-fire, shake slider, reduced motion, card text size | S16 |
| BLD-001 | Desktop players | S16 |
| BLD-002 | Headless editor runs | S1–S15 |
| TST-002 | Replay, captures | all |
| AUT-002 | The editor's MCP interface | S1–S15 |

## Engine Behavior This Game Depends On

Only engine and package behavior, each tied to a catalog ID. Game rules are
defined under Game Design and implemented by the sample's own scripts; the
engine contains no code specific to this game. Package behavior (Cinemachine,
Shader Graph, Input System, Unity Test Framework) comes from the package's own
code at the pinned version line, compiled and run unmodified (CMP-002). Agents
reach the game only through the editor's generic MCP object, property,
method-invocation, input, capture, and editor-command commands, with no
game-specific command.

### Scripting and runtime

* `SetActive(false)` and `SetActive(true)` run `OnDisable` and `OnEnable` every
  time. (RUN-001, RUN-003)
* `UnityEvent` listeners run in serialized order; a listener with a missing
  target is reported by owner, field, and method and skipped, and the others
  still run. (RUN-008)
* `SendMessage` with `DontRequireReceiver`, `BroadcastMessage`,
  `SendMessageUpwards`, and `Invoke` behave as RUN-008 defines; a time scale of
  zero holds pending `Invoke` calls. (RUN-008)
* `Time.timeScale` set to 0.05 slows scaled time while unscaled time measures
  the hit-stop. (RUN-004)
* The same seed gives the same `Random` sequence. `UnityWebRequest` reports
  success, connection error, protocol error with its code, data processing
  error, and timeout distinctly. `JsonUtility` round-trips object-wrapped
  arrays and rejects a top-level array. (RUN-009)

### Project and content

* Changing a ScriptableObject during editor play persists after play and shows
  as an unsaved change. In a player, the change is lost on restart. (SER-001)

### Simulation

* Sphere casts against a layer mask each fixed step at 1/60 s find a 40 m/s
  sphere's hits against a 0.5 m wall. (PHY-001)
* A `MeshCollider` built from a runtime mesh blocks bodies from the frame it is
  assigned. (PHY-001)

### Rendering and animation

* A runtime `Mesh` renders the vertex data the script supplies; assigning more
  than 65,535 vertices to a mesh with 16-bit indices fails with an error naming
  the mesh. (REN-002)
* Values set through `MaterialPropertyBlock` reach Shader Graph materials
  without creating material instances, as residency inspection shows. (REN-002,
  REN-006)
* Sub-emitters on birth and death, the trails module, and world-space
  simulation behave as REN-004 defines; particle counts under a fixed seed
  match in preview, play mode, and player on the same target. (REN-004)
* A global Volume with bloom, ACES tonemapping, and vignette, and a local
  override weighted from script, render as REN-003 defines. (REN-003)

### Audio and video

* With 32 real voices, sources virtualize in priority order (higher priority
  values first); virtualized and stopped sources are reported. A compressor on
  a mixer group keeps that group's output under its threshold. (AUD-001)

### Input

* `SetMotorSpeeds`, `PauseHaptics`, and `ResetHaptics` drive rumble on devices
  with motors and return an explicit status on others. (INP-002)

### UI and text

* `GUILayout.Window` with `DragWindow`, toggles, a slider, and buttons behaves
  as UI-002 defines; an event the IMGUI window uses is marked used and is not
  delivered to the Input System actions. (UI-002)

### Player and build

* The build report lists scenes and sizes and excludes editor-only assemblies.
  (BLD-001)
* The local server fixture listens on `http://127.0.0.1:8090` and can answer
  normally, with HTTP 500, with malformed JSON, after 8 seconds, or not at all.
  (RUN-009)

### Editor and automation

* A custom inspector and an `EditorWindow` built with IMGUI draw in the unitas
  editor, and each control in them can be operated by a person and by an agent
  through the editor's MCP interface. `[MenuItem]` and `[ContextMenu]` commands
  run from the command palette and through the editor's MCP interface with the
  same content effect as in Unity, and their edits are undoable. (RUN-010,
  EDT-002)
* The impulse listener extension shakes its virtual camera by each source's
  amplitude; the confiner keeps the view inside its bounds. (CIN-001)
* Project tests run in the editor, headless, and through the editor's MCP
  interface, with NUnit 3 XML results. (TST-003)

## Presentation and Performance

### Visual checkpoints

Each has a unitas-approved reference: the title, the arena for seed 1234, a
Swarmer mid-dissolve, a hit flash, a Brute explosion with sub-emitters, the
upgrade screen, the overlay, and wave 10 at peak.

### Audio checkpoints

The multiplier chime steps, the player-hit sound audible at peak load, and no
output above 0 dBFS.

### Project-declared expectations

On the reference desktop class at 1080p, High quality: 60 frames per second or
more with 2,000 projectiles and 300 enemies, 99th-percentile frame time under
20 ms, no managed allocation per frame after warm-up, and arena generation
under 0.5 s. Expectations are evaluated only on the declared class; other runs
record the measurement and report the expectation as unevaluable (TST-002). The
performance report shows these apart from the PRF-002 engine budgets and never
labels them as engine budgets.

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
* **Steps:** open the project; open the custom inspector and the Wave Designer
  window; reopen the project.
* **Expected:** assets import without errors, the custom inspector and the Wave
  Designer window draw, and reopening yields identical identities.
* **Demonstrates:** PRJ-001, PRJ-003, PRJ-004, PRJ-007, SER-001, RUN-010

### S2. Happy path

* **Runs in:** editor play mode; headless editor; desktop packaged players.
* **Preconditions:** seed 1234, keyboard and mouse.
* **Steps:** replay recorded input for waves 1 to 3.
* **Expected:** the score, kills, and best multiplier equal the recorded values
  exactly, the upgrade screen appears after wave 2 with the seed's three cards,
  and HUD text updates on the frame of each kill.
* **Demonstrates:** RUN-003, RUN-009, INP-002, UI-001, TXT-001

### S3. Peak load

* **Runs in:** headless editor; development desktop packaged players.
* **Preconditions:** the saved state "seed 1234 at wave 10 peak with God Mode
  on".
* **Steps:** play 60 s with a profiler capture.
* **Expected:** live projectiles reach 2,000 and enemies 300, never more.
  Instantiation after warm-up is zero, and the dropped-fire counter rises once
  the pool is empty. No projectile leaves the arena. The project-declared
  expectations are evaluated on the declared class and reported unevaluable
  elsewhere, and the performance report shows them apart from PRF-002.
* **Demonstrates:** RUN-001, PHY-001, PRF-001, TST-002

### S4. Seeded arenas

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** none.
* **Steps:** generate seeds 1234 and 1235 twice each.
* **Expected:** each seed repeats its layout hash, vertex count, and first 20
  spawn positions. The two seeds differ. The floor mesh has more than 65,535
  vertices with 32-bit indices, and the drone cannot pass any generated wall.
* **Demonstrates:** RUN-009, REN-002, PHY-001

### S5. Combo, graze, and Overdrive

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** the saved graze-test state.
* **Steps:** kill 8 Swarmers 1 s apart, take one hit, then fly so that 20
  projectiles pass within 0.5 m, and use Overdrive.
* **Expected:** the multiplier climbs to x8, resets to x1 on the hit, and each
  step raises the chime. The meter fills, and Overdrive doubles fire rate and
  score for 6 s.
* **Demonstrates:** RUN-004, AUD-001, UI-001

### S6. Upgrades and synergy

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** the saved state "upgrade screen after wave 2 for seed
  1234".
* **Steps:** pick Overcharge and then Conductor, and fire Pulse at a shocked
  Brute.
* **Expected:** the synergy marker shows on the second card, Pulse shots pierce
  one enemy, and damage on the shocked Brute doubles. Hit-stop lasts 40 ms of
  unscaled time on its death.
* **Demonstrates:** SER-001, RUN-004, UI-001

### S7. Effects and materials

* **Runs in:** headless editor; packaged players.
* **Preconditions:** each saved state needed by a checkpoint.
* **Steps:** capture each visual checkpoint at every quality level, with
  residency inspection.
* **Expected:** matches within tolerance, with dissolve, hit flash,
  sub-emitters, trails, bloom, and chromatic aberration present. Residency
  shows no per-enemy material instance.
* **Demonstrates:** REN-002, REN-003, REN-004, REN-006, TST-002

### S8. Leaderboard

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** the server fixture in normal mode.
* **Steps:** finish a run and post "ABC".
* **Expected:** the POST body round-trips through `JsonUtility`, the top-10
  list shows "ABC" at its rank, and diagnostics record the request with
  method, status 200, and timing.
* **Demonstrates:** RUN-009

### S9. Leaderboard failures

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** the server fixture.
* **Steps:** repeat S8 with the fixture down, then in HTTP 500 mode, then
  malformed JSON, then 8-second delay, then normal (harness actions on the
  fixture).
* **Expected:** "Leaderboard offline", "Server error (500)", "Leaderboard data
  unreadable", and a timeout after 5 s, each without blocking play. Each score
  joins `pending_scores.json`, and the normal run posts all of them once.
* **Demonstrates:** RUN-009, SAV-001

### S10. Data-driven content

* **Runs in:** editor play mode, visual and headless.
* **Preconditions:** the project open in the editor.
* **Steps:** set Pulse fire rate to 6 and Swarmer count in wave 1 to 10 in the
  Inspector, then play; change a value during play and stop.
* **Expected:** Pulse fires 6 shots per second, and wave 1 spawns 10 Swarmers.
  The change made during play stays after play as an unsaved change.
* **Demonstrates:** SER-001, EDT-002

### S11. Editor commands

* **Runs in:** editor, visual and headless.
* **Preconditions:** the project open in the editor.
* **Steps:** give one wave a group with delay -2 and another a peak of 320
  enemies, then run Validate All Waves and Sort Spawn Groups By Delay, and
  undo. Then open the Wave Designer window, drag a spawn group from 4 s to 6 s,
  and change a weapon's fire rate in the custom inspector, once as a person
  and once as an agent through the editor's MCP interface, and undo each.
* **Expected:** `OnValidate` clamps the delay to 0, validation reports the
  320-enemy wave by name, sorting reorders the groups, and undo restores them.
  Both editor-window and inspector edits apply, their previews update, and
  undo reverts each.
* **Demonstrates:** RUN-010, SER-001, EDT-002

### S12. Debug overlay

* **Runs in:** editor play mode; headless editor; development packaged players.
* **Preconditions:** a development build; Arena in play.
* **Steps:** open the overlay, drag it, toggle God Mode, set time scale 0.5,
  and click Spawn 50 Swarmers over the arena.
* **Expected:** the window moves, the drone takes no damage, game time runs at
  half speed, 50 Swarmers spawn, and the click does not fire the weapon.
* **Demonstrates:** UI-002

### S13. Shake, rumble, and sound

* **Runs in:** editor play mode; headless editor; packaged players. With
  synthetic devices the rumble feel checkpoint is unevaluable.
* **Preconditions:** the saved state "seed 1234 at wave 10 peak".
* **Steps:** capture camera position, rumble calls, and audio. Repeat with
  shake at 0%, and with a gamepad that reports no motors.
* **Expected:** impulses move the camera in proportion to amplitude, and not at
  all at 0%. Rumble commands carry the stated speeds on a motor gamepad and
  return status on one without motors. Music and player-hit sounds stay
  audible, and weapon fire is the first virtualized.
* **Demonstrates:** CIN-001, INP-002, AUD-001

### S14. Events and messages

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** wave 5 started.
* **Steps:** kill a Swarmer, a Splitter, and a Warden plate. Then remove the gem
  dropper target from a Brute's `onDeath` (a development-build debug command
  implemented in the sample's scripts) and kill it.
* **Expected:** listeners run in order. `OnWaveStarted` reaches every spawner,
  and the Warden receives `OnPlateBroken`. The broken listener is reported by
  owner, field, and method, and score and impulse still run.
* **Demonstrates:** RUN-008

### S15. Project tests

* **Runs in:** headless editor.
* **Preconditions:** none.
* **Steps:** run all EditMode and PlayMode tests headless.
* **Expected:** wave validation, seeded arena hashes, JSON round-trips, combo
  rules, pool caps, single death per enemy, and leaderboard handling pass.
  Results come as NUnit 3 XML and through the editor's MCP interface.
* **Demonstrates:** TST-003

### S16. Loss, restart, and package

* **Runs in:** headless editor (build) and desktop packaged players.
* **Preconditions:** none.
* **Steps:** build each desktop player. Lose all lives on wave 3 using keyboard
  only with auto-fire and 150% card text, then Retry Same Seed.
* **Expected:** results show no medal. Retry regenerates the same layout hash,
  every pool reports full availability, and no enemy or projectile remains.
  Card text is 150% of its default size. The build report lists scenes and
  sizes and excludes the editor assembly.
* **Demonstrates:** RUN-001, BLD-001, ACC-001

## Manual Playtest Guidance

A person judges the following, and a headless run reports these checkpoints as
awaiting human review, never as passed:

* Whether the first wave feels powerful within 10 seconds.
* Whether grazing feels like a deliberate, tempting risk.
* Whether upgrade choices feel distinct and synergies feel worth planning.
* Whether wave 10 stays readable with 2,000 projectiles on screen.
* Whether hits, kills, and hit-stop feel punchy without hurting readability.
* Whether a lost x8 multiplier makes the player want to retry.

## Agent Authoring Change Requests

### CR1. Mine Layer

* **Request:** add an enemy type, the Mine Layer, defined by a new
  `EnemyDefinition`, which drops stationary mines that explode after 2 seconds,
  with its own dissolve color and a wave 7 appearance.
* **Acceptance scenario:** in wave 7 a Mine Layer drops mines; each mine
  explodes 2 s after landing and damages the drone only inside its radius.
* **Test:** a test that checks a mine's damage radius.

### CR2. Split Shot

* **Request:** add an upgrade card, Split Shot, that makes Arc fork into two
  chains.
* **Acceptance scenario:** with Split Shot, one Arc bolt fired into a group of
  eight enemies damages six of them.
* **Test:** a test that counts enemies damaged by one bolt.

## Accessibility

* Every action works with the keyboard alone, including 8-direction aiming and
  auto-fire (S16).
* All controls can be rebound (S2).
* The shake slider and reduced motion remove camera shake, hit-stop, and
  chromatic aberration (S13).
* Enemy types differ by silhouette as well as color (S7).
* Upgrade card text scales to 150% (S16).

## Saved and Reproducible States

| State | How it is reached | Used by |
|---|---|---|
| Seed 1234 at wave 10 peak with God Mode on | save file | S3, S13 |
| Graze test: one Spitter ring firing at a stationary drone | save file | S5 |
| Upgrade screen after wave 2 for seed 1234 | save file | S6 |
| Results screen with three pending leaderboard scores | save file | S9 |

## Known Limitations and Unsupported Conditions

* The leaderboard talks only to the local test fixture over cleartext HTTP on
  `127.0.0.1`. This is a design choice of the sample, not an engine limit.

Any behavior that this document requires and that is not demonstrated is
reported with its real compatibility state, as a defect to fix.

## Acceptance Outcomes

* The arena shooter plays ten waves in editor and player, with the same score
  for the same seed and input, and Retry leaves no live object from the
  previous run. (SMP-001, RUN-001, RUN-009)
* Pools hold at 2,000 projectiles and 300 enemies with no instantiation after
  warm-up, and the peak-load report keeps its project-declared expectations
  apart from the PRF-002 budgets. (SMP-001, RUN-001, PRF-001, TST-002)
* The multiplier, grazing, Overdrive, upgrades, and synergies follow their
  stated rules. (SMP-001, RUN-004, SER-001, UI-001)
* Dissolve and hit-flash graphs, explosion sub-emitters, trails, and URP
  post-processing match their approved references with no per-enemy material
  instance. (SMP-001, REN-002, REN-003, REN-004, REN-006)
* Seeded arenas built with the runtime Mesh API repeat for a seed and block
  movement with generated colliders. (SMP-001, RUN-009, REN-002, PHY-001)
* The leaderboard posts and lists scores through the local fixture, and
  offline, error, malformed, and slow responses keep the score queued without
  blocking play. (SMP-001, RUN-009, SAV-001)
* Wave and weapon ScriptableObjects drive play, editor commands validate them,
  and the custom inspector and the Wave Designer window draw and edit them for
  a person in the visual editor and for an agent through the editor's MCP
  interface. (SMP-001, SER-001, RUN-010, EDT-002)
* The IMGUI overlay controls the game and consumes its own clicks. (SMP-001,
  UI-002)
* Impulse shake, rumble, and voice limiting behave as stated under peak load.
  (SMP-001, CIN-001, INP-002, AUD-001)
* UnityEvents and the SendMessage family deliver in order, and a broken
  listener is reported without stopping the others. (SMP-001, RUN-008)
* The project's EditMode and PlayMode tests pass headless with NUnit 3 XML
  results, and each desktop player is packaged with a build report. (SMP-001,
  TST-003, BLD-001)
