# Sample 7: Twin-Stick Arena Shooter

## Overview

The player pilots a hover drone through ten waves of enemies in a generated
arena. One stick moves, the other aims. Between waves the player picks
upgrades that combine into weapon synergies. Wave 10, the Storm, fills the
arena with 300 enemies and up to 2,000 projectiles. A run ends when the last
life is lost or wave 10 is cleared. The score goes to a local online
leaderboard. A run takes 8 to 12 minutes.

This is an original, redistributable unitas sample. Its drone, enemies,
arenas, effects, music, and sounds are made for unitas and are distinct from
any official Unity sample or commercial shooter. The leaderboard server is a
local test fixture shipped with the sample, not an external service.

Compatibility tiers follow the [Feature catalog](../feature-catalog.md). This
document owns the acceptance of this game under SMP-001.

## Targets and Input Modes

* Targets: macOS, Windows, and Linux desktop players, plus the editor's play
  mode and the headless editor.
* Render pipeline: URP 14.x, Forward+ rendering path, HDR on.
* Input modes: keyboard and mouse, gamepad, and keyboard only, through an
  Input System 1.x action asset with three control schemes. Menus accept the
  pointer, keyboard, and gamepad.
* Gamepad rumble needs a gamepad with motors. Without one, the rumble
  checkpoint reports the device's explicit status and is not evaluable, never
  passed. Headless runs use synthetic devices, marked synthetic.

## Game Design

### Core loop

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

Dash moves 4 m in 0.15 s with 0.3 s of invulnerability and recharges in 1.2 s.

### Weapons, enemies, and waves

* Pulse fires 12 shots per second at 40 m/s, 1 damage each. Scatter fires 5
  pellets in a 30-degree spread 3 times per second, 2 damage each. Arc fires a
  bolt that chains to 3 more enemies within 4 m, 3 damage, and shocks each for
  1.5 s. The drone gains Scatter after wave 4 and Arc after wave 8.
* Swarmers rush the drone (2 hit points). Spitters keep 8 m away and fire
  rings of 12 projectiles (6 hit points). Brutes charge and shrug off
  knockback (30 hit points). Splitters burst into three Swarmers (8 hit
  points). The Warden mini-boss in wave 5 has four armor plates that must
  break before its core takes damage.
* Wave content comes from `WaveDefinition` assets, weapons from
  `WeaponDefinition`, enemies from `EnemyDefinition`, and upgrades from
  `UpgradeDefinition`. All are ScriptableObjects.
* All projectiles come from one pool of 2,000 and all enemies from per-type
  pools totaling 300, prewarmed while the arena loads. After warm-up the game
  instantiates nothing. A fire request with an empty pool is dropped and
  counted. A spawn with no free enemy slot waits for one.

### Procedural arenas

The seed sets `Random.InitState`. The arena is a hexagonal floor of radius
28 m built from 1 m hex cells as one runtime `Mesh` with 32-bit indices and
more than 65,535 vertices, plus 6 to 12 pillars, 3 to 6 cover walls, and a
boundary wall, each with a `MeshCollider` from its generated mesh. Spawn
order, portal choice, and upgrade offers come from the same seed. The overlay
shows a layout hash computed from the generated vertex data.

### What makes it fun

* **Hook.** Wave 1 starts 3 seconds after Play with 24 Swarmers that pop in
  one Pulse hit, with a flash, a dissolve, and a gem burst each. The first
  multiplier step arrives within 10 seconds.
* **Mastery curve.** Kills within 1.5 s of each other raise the multiplier one
  step, from x1 to x8. Taking damage resets it to x1. A skilled player holds
  x8 through whole waves, dashes through Spitter rings, and clears wave 10
  without losing a life.
* **Risk and reward.** An enemy projectile that passes within 0.5 m of the
  drone without hitting is a graze and adds 5% to the Overdrive meter. A full
  meter gives 6 seconds of double fire rate and a x2 score bonus. Gems worth
  10 points each drop where enemies die and vanish after 6 seconds, so
  collecting them means flying into the fight.
* **Synergies and choices.** Each upgrade screen offers three of twelve
  cards, such as Overcharge (Pulse shots pierce one enemy), Ricochet (Scatter
  pellets bounce once off walls), Conductor (Pulse deals double damage to
  shocked enemies), and Magnet (gem pickup radius from 1 m to 4 m). Two cards
  that name each other form a synergy, marked on the card. The offers come
  from the seed, so a seed can be shared and replanned.
* **Variety and escalation.** Waves 1 to 4 each introduce one enemy type.
  Wave 5 is the Warden. Waves 6 to 9 mix every type at rising counts, and
  wave 10 peaks at 300 enemies. Each arena seed changes the cover, so routes
  differ.
* **Juice.**
  * Hit flash for 0.08 s and a dissolve over 0.6 s on death.
  * Explosions with spark and smoke sub-emitters, and bloom on every shot.
  * A Cinemachine impulse whose amplitude scales with what exploded.
  * A 40 ms hit-stop on Brute and Warden kills.
  * A chromatic aberration pulse when the drone is hit.
  * Rumble on hits and deaths.
  * A multiplier chime that rises one semitone per step.
* **Replay reasons.** Bronze for reaching wave 5, Silver for clearing wave 10,
  and Gold for clearing it without losing a life. Local best score per seed,
  a top-10 leaderboard, and Endless mode, which unlocks after the first wave
  10 clear and repeats wave 10 with 10% more health each loop.

### Pause and settings

Pause sets `Time.timeScale` to zero, pauses rumble with `PauseHaptics`, shows
Resume, Restart, Settings, and Quit to Title, and keeps menu sounds playing.
Settings are master, music, and effects volume, display mode, resolution,
vertical sync, quality (Low, Medium, High), screen shake (0 to 100%), rumble
on or off, auto-fire, reduced motion, and rebinding. They persist through
`PlayerPrefs` and apply immediately.

### Failure and edge rules

* No enemy or projectile leaves the arena or passes through a wall.
* An enemy hit by several projectiles in one step dies once, scores once, and
  returns to its pool once.
* The leaderboard never blocks play. When it fails, the score stays in a local
  pending queue.
* Losing window focus pauses the game.

## Content

| Asset | Source format | What matters on import |
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

Packages: URP 14.x, Shader Graph 14.x, Cinemachine 2.x, Input System 1.x,
uGUI 1.0.x, TextMeshPro 3.0.x, and Unity Test Framework 1.x.

## Feature Coverage

| ID | Behavior this game exercises | Scenarios |
|---|---|---|
| PRJ-001, PRJ-003, PRJ-004, PRJ-007 | Open in place, stable identities, models and audio, voice and physics settings | S1 |
| SER-001, SER-003 | ScriptableObject data, prefab variants, serialized events | S1, S10 |
| EDT-002 | Custom-inspector and default-inspector edits of ScriptableObject data | S10, S11 |
| RUN-001 | Pooling by activation, 2,000 projectiles and 300 enemies, no instantiation after warm-up | S3, S16 |
| RUN-002, RUN-003, RUN-004 | Scripts, lifecycle on pool reuse, `Time.timeScale` pause and hit-stop | S2, S5 |
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
| UI-001, TXT-001 | Menus, upgrade cards, HUD, results and initials entry | S2, S6 |
| UI-002 | IMGUI debug overlay with window, toggles, slider, and buttons | S12 |
| SAV-001 | Settings, local records, medals, pending scores | S9 |
| TST-003 | EditMode and PlayMode tests through the Unity Test Framework | S15 |
| PRF-002 | Peak-load expectations reported apart from the engine budgets | S3 |
| ACC-001 | Keyboard-only aiming, auto-fire, shake slider, reduced motion | S16 |
| BLD-001, BLD-002, TST-002, AUT-002 | Desktop players, headless editor runs, replay, captures, the editor's MCP interface | all |

## Engine Behavior This Game Depends On

### Scripting and runtime

* `SetActive(false)` on return to a pool and `SetActive(true)` on reuse run
  `OnDisable` and `OnEnable` every time, so pooled objects reset their state.
* `UnityEvent` listeners on `EnemyHealth.onDeath` run in serialized order:
  score, gem drop, impulse. A listener with a missing target is reported and
  skipped, and the others still run.
* Projectile hits call `SendMessage("ApplyDamage", damage,
  SendMessageOptions.DontRequireReceiver)`. Wave start calls
  `BroadcastMessage("OnWaveStarted", index)` on the arena root. Warden plates
  call `SendMessageUpwards("OnPlateBroken")`. Spawn groups use `Invoke` with
  their delay, so a time scale of zero holds them.
* The hit-stop sets `Time.timeScale` to 0.05 for 40 ms of unscaled time.
* The same seed gives the same `Random` sequence, so the arena, spawns, and
  upgrade offers repeat.
* The leaderboard client uses `UnityWebRequest` with a 5-second timeout and
  tells success, connection error, protocol error with its code, and data
  processing error apart. Bodies are `JsonUtility` JSON wrapped in an object,
  because top-level arrays are not supported.

### Editor and content

* The custom `WeaponDefinition` inspector draws in the unitas editor. It shows
  a DPS preview, a projectile-pattern preview, and a synergy warning. The Wave
  Designer `EditorWindow` draws a timeline of spawn groups that can be dragged
  to change their delays. Both are built with IMGUI (`EditorGUILayout`), and
  each control in them can be operated by an agent through the editor's MCP interface.
* The commands `Arena/Waves/Validate All Waves`, `Arena/Waves/New Wave From
  Template`, and `Arena/Waves/Preview Wave`, and the context menus `Sort Spawn
  Groups By Delay` and `Double All Counts`, run from the command palette and
  through the editor's MCP interface with the same content effect as in Unity.
* `OnValidate` clamps group counts to 1 to 300 and delays to 0 or more, and
  warns when a wave's peak live enemy count exceeds 300.
* Changing a ScriptableObject during editor play persists after play and shows
  as an unsaved change. In a player, the change is lost on restart.

### Simulation

* Projectiles detect hits with sphere casts on the Enemies, Player, and Walls
  layers each fixed step (1/60 s), so no 40 m/s projectile passes a 0.5 m
  wall.
* Generated `MeshCollider`s block enemies and the drone on the frame the arena
  finishes generating.

### Rendering

* The generated floor renders the data the script supplied. Assigning its
  vertex count to a 16-bit mesh fails with an error naming the mesh.
* Hit flash and dissolve values reach each enemy through
  `MaterialPropertyBlock`, so 300 enemies create no material instances.
* Explosion sub-emitters spawn sparks on birth and smoke on death. Projectile
  trails last 0.12 s. Particle counts under a fixed seed match in preview,
  play mode, and player.
* A global Volume applies bloom (threshold 0.9), ACES tonemapping, and
  vignette. A local override applies chromatic aberration for 0.2 s on a hit.

### Audio and input

* With 32 real voices, music (priority 0) and player-hit sounds (16) never
  virtualize, and weapon fire (200) virtualizes first. Virtualized and
  stopped sources are reported. A compressor on the Effects group keeps the
  output under 0 dBFS.
* `SetMotorSpeeds` drives rumble: low 0.3 and high 0.6 for 0.15 s on a hit,
  0.8 and 0.8 for 0.4 s on a life lost. `ResetHaptics` runs on quit.
* The impulse listener on the top-down virtual camera shakes it by each
  source's amplitude times the shake setting. The camera's confiner keeps the
  view inside the arena.

### UI and overlay

* The IMGUI overlay is a `GUILayout.Window` showing frame time, live and
  pooled counts, dropped fire requests, seed, layout hash, wave, and
  multiplier. It has God Mode and Show Hitboxes toggles, a time-scale slider
  (0.1 to 2.0), and Skip Wave, Spawn 50 Swarmers, and Fill Overdrive buttons.
  `DragWindow` moves it.
* A click that the overlay uses never fires the weapon.

### Player, tests, and automation

* The local server fixture listens on `http://127.0.0.1:8090` and can answer
  normally, with HTTP 500, with malformed JSON, after 8 seconds, or not at all.
* Project tests run in the editor, headless, and through the editor's MCP interface, with NUnit 3
  XML results.

## Presentation and Performance

* Visual checkpoints, each with a unitas-approved reference: the title, the
  arena for seed 1234, a Swarmer mid-dissolve, a hit flash, a Brute explosion
  with sub-emitters, the upgrade screen, the overlay, and wave 10 at peak.
* Audio checkpoints: the multiplier chime steps, the player-hit sound audible
  at peak load, and no output above 0 dBFS.
* Project-declared expectations on the reference desktop class at 1080p, High
  quality: 60 frames per second or more with 2,000 projectiles and 300
  enemies, 99th-percentile frame time under 20 ms, no managed allocation per
  frame after warm-up, and arena generation under 0.5 s. The performance
  report shows these apart from the PRF-002 engine budgets and never labels
  them as engine budgets.

## Acceptance Scenarios

Every scenario runs in editor play mode, in a packaged player, and headless
through the editor's MCP interface, unless it says otherwise.

* **S1. Open and inspect.** Precondition: no cache. Open the project.
  Expected: assets import without errors, the custom inspector and the Wave
  Designer window draw, and reopening yields identical identities. (PRJ-001, PRJ-003, PRJ-004, PRJ-007, SER-001, RUN-010)
* **S2. Happy path.** Seed 1234, keyboard and mouse, recorded input for waves
  1 to 3. Expected: the score, kills, and best multiplier equal the recorded
  values exactly, the upgrade screen appears after wave 2 with the seed's
  three cards, and HUD text updates on the frame of each kill. (RUN-003,
  RUN-009, INP-002, UI-001, TXT-001)
* **S3. Peak load.** Load the saved wave 10 state with God Mode on. Expected:
  live projectiles reach 2,000 and enemies 300, never more. Instantiation
  after warm-up is zero, and the dropped-fire counter rises once the pool is
  empty. No projectile leaves the arena. The performance report shows the
  declared expectations separately from PRF-002. (RUN-001, PHY-001, PRF-002,
  TST-002)
* **S4. Seeded arenas.** Generate seeds 1234 and 1235 twice each. Expected:
  each seed repeats its layout hash, vertex count, and first 20 spawn
  positions. The two seeds differ. The floor mesh has more than 65,535
  vertices with 32-bit indices, and the drone cannot pass any generated wall.
  (RUN-009, REN-002, PHY-001)
* **S5. Combo, graze, and Overdrive.** In the saved graze-test state, kill 8
  Swarmers 1 s apart, take one hit, then fly so that 20 projectiles pass within
  0.5 m. Expected: the multiplier climbs to x8, resets to x1 on the hit, and
  each step raises the chime. The meter fills, and Overdrive doubles fire rate
  and score for 6 s. (RUN-004, AUD-001, UI-001)
* **S6. Upgrades and synergy.** Seed 1234, pick Overcharge and then
  Conductor, and fire Pulse at a shocked Brute. Expected: the synergy marker
  shows on the second card, Pulse shots pierce one enemy, and damage on the
  shocked Brute doubles. Hit-stop lasts 40 ms on its death. (SER-001, RUN-004,
  UI-001)
* **S7. Effects and materials.** Capture each visual checkpoint at every
  quality level. Expected: matches within tolerance, with dissolve, hit flash,
  sub-emitters, trails, bloom, and chromatic aberration present. Residency
  shows no per-enemy material instance. (REN-002, REN-003, REN-004, REN-006,
  TST-002)
* **S8. Leaderboard.** Server in normal mode. Finish a run and post "ABC".
  Expected: the POST body round-trips through `JsonUtility`, the top-10 list
  shows "ABC" at its rank, and diagnostics record the request with method,
  status 200, and timing. (RUN-009)
* **S9. Leaderboard failures.** Repeat S8 with the server down, then in HTTP
  500 mode, then malformed JSON, then 8-second delay, then normal. Expected:
  "Leaderboard offline", "Server error (500)", "Leaderboard data unreadable",
  and a timeout after 5 s, each without blocking play. Each score joins
  `pending_scores.json`, and the normal run posts all of them once. (RUN-009,
  SAV-001)
* **S10. Data-driven content.** In the editor, set Pulse fire rate to 6 and
  Swarmer count in wave 1 to 10, then play. Expected: Pulse fires 6 shots per
  second, and wave 1 spawns 10 Swarmers. A change made during play stays
  after play as an unsaved change. Editor and headless only. (SER-001,
  EDT-002)
* **S11. Editor commands.** Give one wave a group with delay -2 and another a
  peak of 320 enemies, then run Validate All Waves and Sort Spawn Groups By
  Delay. Expected: `OnValidate` clamps the delay to 0, validation reports the
  320-enemy wave by name, sorting reorders the groups, and undo restores them.
  Then open the Wave Designer window, drag a spawn group from 4 s to 6 s, and
  change a weapon's fire rate in the custom inspector, once as a person and once
  as an agent through the editor's MCP interface. Expected: both edits apply, their previews update,
  and undo reverts each. Editor and headless only. (RUN-010, SER-001, EDT-002)
* **S12. Debug overlay.** Open the overlay, drag it, toggle God Mode, set time
  scale 0.5, and click Spawn 50 Swarmers over the arena. Expected: the window
  moves, the drone takes no damage, game time runs at half speed, 50 Swarmers
  spawn, and the click does not fire the weapon. (UI-002)
* **S13. Shake, rumble, and sound.** At peak load, capture camera position,
  rumble calls, and audio. Repeat with shake at 0%. Expected: impulses move
  the camera in proportion to amplitude, and not at all at 0%. Rumble runs the
  stated speeds on a motor gamepad and reports status on one without motors.
  Music and player-hit sounds stay audible, and weapon fire is the first
  virtualized. (CIN-001, INP-002, AUD-001)
* **S14. Events and messages.** Kill a Swarmer, a Splitter, and a Warden
  plate. Then remove the gem dropper target from a Brute's `onDeath` and kill
  it. Expected: listeners run in order. `OnWaveStarted` reaches every
  spawner, and the Warden receives `OnPlateBroken`. The broken listener is
  reported by owner, field, and method, and score and impulse still run.
  (RUN-008)
* **S15. Project tests.** Run all EditMode and PlayMode tests headless.
  Expected: wave validation, seeded arena hashes, JSON round-trips, combo
  rules, pool caps, single death per enemy, and leaderboard handling pass.
  Results come as NUnit 3 XML and through the editor's MCP interface. (TST-003)
* **S16. Loss, restart, and package.** Build each desktop player. Lose all
  lives on wave 3 using keyboard only, then Retry Same Seed. Expected:
  results show no medal. Retry regenerates the
  same layout hash, every pool reports full availability, and no enemy or
  projectile remains. The build report lists scenes and sizes and excludes the
  editor assembly. (RUN-001, BLD-001, ACC-001)

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

Add a fourth enemy type, the Mine Layer, defined by a new `EnemyDefinition`,
which drops stationary mines that explode after 2 seconds, with its own
dissolve color and a wave 7 appearance. Include a test that checks a mine's
damage radius.

A second change request adds an upgrade card, Split Shot, that makes Arc fork
into two chains, with a scenario proving that six enemies take damage from one
bolt.

## Accessibility

Every action works with the keyboard alone, including 8-direction aiming and
auto-fire. All controls can be rebound. The shake slider and reduced motion
remove camera shake, hit-stop, and chromatic aberration. Enemy types differ
by silhouette as well as color. Upgrade card text scales to 150%.

## Saved and Reproducible States

* Seed 1234 at wave 10 peak with God Mode on.
* Graze test: one Spitter ring firing at a stationary drone.
* Upgrade screen after wave 2 for seed 1234.
* Results screen with three pending leaderboard scores.

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
  apart from the PRF-002 budgets. (SMP-001, RUN-001, PRF-002, TST-002)
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
  and the custom inspector and the Wave Designer window draw and edit them for a
  person in the visual editor and for an agent through the editor's MCP interface. (SMP-001, SER-001,
  RUN-010, EDT-002)
* The IMGUI overlay controls the game and consumes its own clicks. (SMP-001,
  UI-002)
* Impulse shake, rumble, and voice limiting behave as stated under peak load.
  (SMP-001, CIN-001, INP-002, AUD-001)
* UnityEvents and the SendMessage family deliver in order, and a broken
  listener is reported without stopping the others. (SMP-001, RUN-008)
* The project's EditMode and PlayMode tests pass headless with NUnit 3 XML
  results, and each desktop player is packaged with a build report. (SMP-001,
  TST-003, BLD-001)
