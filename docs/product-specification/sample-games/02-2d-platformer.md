# Sample 2: Two-Dimensional Platformer ("Wick's Lantern")

## Overview

The player guides Wick, a small lantern-bearing clockwork figure, through
three tile-based levels. Wick runs, jumps, rides moving platforms and
conveyor belts, lands on one-way platforms, chains ember pickups, avoids
hazards, respawns at checkpoints, and rings the goal bell at the end of each
level. A first run takes 10 to 15 minutes; a medal run of one level takes 2 to
5 minutes. The tone is calm and warm, with pixel art and soft 2D lighting.

This is an original, redistributable unitas sample. Its character, art,
music, and level design are unitas content and are not derived from any Unity
sample or third-party game. It proves the 2D feature set, from sprite and
tilemap import through 2D physics, skeletal sprite animation, 2D lighting, and
checkpoint saves, on desktop and in a browser.

Compatibility tiers follow the [Feature catalog](../feature-catalog.md). This
document owns the acceptance of this game under SMP-001.

## Targets and Input Modes

* Targets: macOS, Windows, and Linux desktop players, a browser player
  (Unity's WebGL build target, delivered as a browser bundle and run in a
  WebGL 2-class browser), the editor's play mode, and the headless editor.
* Render pipeline: URP 14.x with the 2D Renderer only.
* Input modes: keyboard, gamepad, and mouse (menus only), through one Input
  System 1.x action asset. On the browser target the gamepad is used when the
  browser exposes it, and the capability report states whether it does.
* Declared aspect ratios: 16:9, 16:10, 21:9, and 4:3.

## Game Design

### Core loop

1. The title shows Continue (only when a save exists), New Game, Level Select,
   Night Run (once unlocked), Settings, and Quit. On the browser target it
   first shows "Press any key" until a user gesture has resumed audio.
2. A level starts with Wick at the level start and the best-run ghost, when
   one exists, beside him. The HUD shows three hearts, embers out of the
   level total, the glow score, the chain counter, and the elapsed time.
3. Touching a checkpoint lantern lights it, writes the checkpoint save file,
   refills hearts to three, and makes it the respawn point.
4. Touching a hazard costs one heart and respawns Wick at the last lit
   checkpoint, or at the level start. Losing the last heart shows the "Out of
   light" panel with Retry Level and Quit to Title.
5. Ringing the bell ends the level. The completion panel shows embers, time,
   glow score, hearts lost, the medal earned, and records beaten, with Next
   Level, Replay, and Quit to Title. After level 3 it shows the run summary.

### Levels

| Level | Name | Tiles | Embers (gold) | Checkpoints | Par time | New elements |
|---|---|---|---|---|---|---|
| 1 | Rootway | 220 x 40 | 20 (2) | 2 | 2:30 | running, jumping, one-way stacks, pits |
| 2 | Lantern Mill | 260 x 60 | 25 (3) | 3 | 3:30 | moving platforms, conveyors, darkness lit by 2D lights |
| 3 | Thorn Ridge | 300 x 80 | 30 (4) | 3 | 4:30 | thorns, beetles, crumbling bridge, masked secret passage, final timed climb |

Each room has a camera confiner polygon. Level 2 has two rooms and level 3
three, joined by doorways where the camera cuts.

### Movement and hazards

* Run speed 6 units per second, reached in 0.1 s; jump height 3.5 units;
  gravity scale 3; fall speed capped at 18 units per second. Coyote time and
  jump buffering are 0.1 s each. Releasing jump early cuts the remaining rise
  to 40 percent.
* One-way platforms are 4 pixels thick. A body moving upward from below passes
  through. A body arriving from above or from either side collides and can
  stand on it. Holding down and pressing jump while standing on a one-way
  platform drops the player through it. The game's script turns off the
  player's collision with that platform for 0.25 s, so the passive one-way
  rule never lets anything through from above.
* Conveyors move standing bodies at 3 units per second. Moving platforms follow
  waypoints at 2 units per second and carry standing bodies.
* Hazards: thorn tiles (trigger tilemap), pits (a trigger strip 4 units below
  each pit), and beetles that patrol between two markers at 1.5 units per
  second. Each contact costs one heart.

### What makes it fun

* **Hook.** The level 1 start is dark. Within 3 s Wick's lantern flickers on
  and reveals an arc of five embers over the first gap. Collecting them in one
  jump plays a rising five-note chime and shows "Chain x5". By second 30 the
  player has ridden the first one-way stack and seen a gold ember glinting
  above thorns.
* **Mastery curve.** An ember collected within 1.0 s of the previous one
  extends the chain. Each ember scores 10 glow points times the chain length,
  capped at x10; a gold ember scores 50 times the chain. A better player keeps
  momentum, uses coyote time and buffered jumps to link chains across gaps,
  rides conveyors in their direction, and finds the one shortcut per level
  through the masked passage or a hidden one-way stack.
* **Risk and reward.** Gold embers sit above thorns or on beetle routes.
  Jumping over a checkpoint lantern without touching it leaves it unlit and
  adds a x1.5 Daring bonus to the level's glow score, but a hit then sends Wick
  back to the previous lit checkpoint. Slowing down to stay safe breaks the
  chain.
* **Variety and escalation.** Each level adds the elements in the levels
  table. Level 3 ends with a 45-second timed climb in which the crumbling
  bridge, beetles, conveyors, and one-way stacks appear together.
* **Juice.** Landing squashes Wick to 1.2 by 0.8 scale for 0.08 s and plays a
  dust puff. A hurt pauses gameplay for 0.05 s (hit-stop), shakes the camera
  by 2 pixels for 0.15 s when screen shake is on, and swaps the face to
  `Hurt`. Embers pop with a sprite-swap burst and a chime one semitone higher
  per chain step. A lighting checkpoint flares its 2D light from 1 to 3
  intensity over 0.3 s. The camera looks ahead 1.5 units in the run direction.
* **Replay reasons.** Each level awards Bronze for ringing the bell, Silver
  for every ember, and Gold for every ember under par time. A Lantern badge
  marks a run with no heart lost. Best time and best glow score are recorded
  per level. The best run is saved as a ghost and replays as a translucent
  Wick on the next attempt. Gold on all three levels unlocks Night Run (global
  light 0.1, only ember and lantern lights) and the `Brass` Wick skin.

### Controls

| Action | Keyboard | Gamepad | Mouse |
|---|---|---|---|
| Move | A and D, or Left and Right arrows | Left stick, D-pad | — |
| Jump | Space, W, or Up arrow | South button | — |
| Interact (ring bell) | E | West button | — |
| Pause | Escape or P | Start | — |
| Navigate menus | Arrow keys, Tab | D-pad, left stick | Pointer |
| Confirm / Back | Enter or Space / Escape | South / East button | Left click / right click |

Every gameplay binding can be rebound from Settings > Controls.

### Pause, restart, and exit

Pause sets `Time.timeScale` to zero, shows Resume, Restart from Checkpoint,
Restart Level, Settings, and Quit to Title, and moves the music to a lowpass
snapshot over 0.3 s while menu sounds play unfiltered. Restart Level resets
every transient value and deletes the level's checkpoint save. Quit to Title
keeps it. Quit on the title exits the desktop player; the browser player
hides Quit. Losing window or page focus pauses the game, and it stays paused
until the player resumes.

### Settings

Master, music, and effects volume; fullscreen mode; resolution from
`Screen.resolutions`; vertical sync; pixel snapping; screen shake; ghost on or
off; skin; and control bindings. All persist through `PlayerPrefs` and apply
immediately. In the browser, fullscreen requires the click that sets it, and
resolution follows the page canvas size.

## Content

| Asset | Source format | What matters on import |
|---|---|---|
| `Title.unity` and three level scenes | Unity YAML scene | grids, tilemaps, prefab instances, 2D lights, references |
| `Wick.psb` and `Wick.prefab` | Photoshop large document through 2D PSD Importer | 14 layers to sprites, generated hierarchy, bones, weights, `SpriteSkin`, 16 pixels per unit |
| `Ember.prefab`, `GoldEmber.prefab` | prefab and prefab variant | variant overrides of sprite, light color, and score |
| `Checkpoint.prefab` | prefab nesting `Lantern.prefab` | nested prefab, trigger, 2D light reference |
| `MovingPlatform.prefab`, `MovingPlatformVertical.prefab` | prefab and variant | overrides of waypoints, speed, and sprite |
| `Beetle.prefab`, `GoalBell.prefab` | Unity YAML prefab | Kinematic body, sprite-swap clips |
| Tile sheets | PNG, Sprite Mode Multiple, 16 x 16 | point filter, no mipmaps, no compression, custom physics shapes on slopes |
| Normal maps, UI sprites | PNG | `_NormalMap` secondary textures; nine-slice borders of 4 pixels |
| `World.spriteatlasv2`, `UI.spriteatlasv2` | Sprite Atlas v2 | packed sprites, included in build |
| Tiles | `RuleTile`, `RuleOverrideTile`, `AnimatedTile` (2D Tilemap Extras) | neighbor rules, collider type per rule, animation speed |
| `Wick.controller`, clips | Animator controller, `.anim` | states, 1D blend tree, events, sprite-swap curves |
| `Wick.spriteLib`, `WickBrass.spriteLib` | Sprite Library assets | face labels `Calm`, `Hurt`, `Cheer`; Brass skin |
| Effects, chimes, music | WAV, OGG | decompress-on-load chimes in 10 pitch steps, streamed looping music |
| `Main.mixer` | Audio Mixer | Master, Music, Effects, UI groups; `Play` and `Paused` snapshots |
| Pixel font | TrueType with TextMeshPro bitmap font asset | point filter, fallback to an SDF font asset |
| `Controls.inputactions` | Input System action asset | Gameplay and UI maps, Keyboard and Gamepad schemes |
| `ProjectSettings/` | Unity YAML | sorting layers, 2D physics, layer matrix, quality, time step 0.02 s |

Sorting layers, back to front: Background, Parallax, Tiles, Props, Characters,
Foreground, Effects. Packages: URP 14.x; 2D Sprite, 2D Tilemap Editor, 2D
Tilemap Extras, 2D Animation, 2D Pixel Perfect, and 2D PSD Importer at their
released or verified versions; Cinemachine 2.x; Input System 1.x; uGUI 1.0.x;
TextMeshPro 3.0.x.

## Feature Coverage

| ID | Behavior this game exercises | Scenarios |
|---|---|---|
| PRJ-001 | The project opens in place with no conversion | S1 |
| PRJ-003 | Sprite sub-asset, tile, and PSB sprite identities stay stable | S1, S15 |
| PRJ-004 | Sprite sheets, secondary normal maps, PSB import, WAV and OGG, fonts | S1, S5 |
| PRJ-006 | The 2D, Cinemachine, Input System, uGUI, and TextMeshPro packages resolve | S1 |
| PRJ-007 | Sorting layers, 2D physics settings, layer matrix, quality, time step | S1, S4 |
| SER-001, SER-002, SER-003 | Scene load, nested prefabs, variants, serialized waypoints | S1, S2, S15 |
| RUN-001, RUN-002, RUN-003 | Object lifetime, compiled scripts, `FixedUpdate`, 2D and focus messages | S2, S6 |
| RUN-004 | `Time.timeScale`, hit-stop, fixed time steps, coroutines | S4, S6, S12 |
| RUN-005 | Level loading, restart, return to title | S6 |
| RUN-009 | Save and ghost files under `Application.persistentDataPath`, `JsonUtility` | S7, S13, S14 |
| REN-001 | Orthographic camera, resolution and aspect changes | S8, S9 |
| REN-005 | Sprites, atlases, nine-slice, sorting layers, Sorting Groups, sprite masks, rule tiles, 2D lights, Pixel Perfect Camera, PSB character, skinned sprites | S1, S5, S9, S11, S14 |
| REN-006 | URP 14.x 2D Renderer data and light blend styles | S5 |
| REN-007 | Vertical sync, fullscreen modes, frame pacing | S4, S9 |
| ANI-001 | Sprite-swap clips, Animator states, blend tree, events, 2D Animation rig | S2, S11 |
| CIN-001 | Cinemachine 2.x camera with 2D confiner, lookahead, Pixel Perfect extension | S8 |
| PHY-002 | `Rigidbody2D`, composite tilemap collider, platform and surface effectors, casts, 2D messages | S2, S3, S4, S6 |
| INP-002 | Action asset, keyboard and gamepad schemes, rebinding, device changes | S2, S10 |
| UI-001 | Menus, HUD, focus, navigation, scaling across aspect ratios | S2, S6, S9, S10 |
| TXT-001 | TextMeshPro bitmap and SDF text with fallback | S2, S9 |
| AUD-001 | Chimes, streamed music, mixer groups and snapshots, browser audio activation | S2, S12, S13 |
| SAV-001 | Checkpoint saves, records, ghosts, settings and bindings, corrupt-save handling | S6, S7, S10, S13, S14 |
| PLT-001 | Browser loading, focus, resize, fullscreen, storage, and refresh | S13 |
| BLD-001 | Desktop players and a browser bundle with build reports | S16 |
| BLD-002 | Headless runs of every scenario | all |
| TST-002 | Scenarios, checkpoints, approved references, input replay | all |
| EDT-004 | Play, stop, and edit-state restoration | S15 |
| AUT-002 | Agents play, inspect physics and sort order, and capture frames | all |

## Engine Behavior This Game Depends On

### Scripting and runtime

* `OnTriggerEnter2D` fires once per ember, checkpoint, hazard, and bell
  contact, after the physics step and before `Update`. `OnCollisionEnter2D`
  reports each landing with a normal pointing away from the surface.
* Clearing `Rigidbody2D.simulated` removes Wick's contacts for the 0.5 s
  respawn fade. `Time.timeScale = 0` stops `FixedUpdate`, platforms, beetles,
  and scaled-time animation; hit-stop holds it at 0 for 0.05 s of unscaled
  time while menu animation continues.

### Project and content

* `Wick.psb` imports 14 sprites and a hierarchy named after its layers, and
  reimporting keeps sprite identities, bones, and weights. Tile slices keep
  names, pivots, and physics shapes. Variants override every instance.

### Simulation

* Wick is a Dynamic `Rigidbody2D` with continuous collision detection,
  interpolation, frozen rotation, and a 0.75 by 1.375 unit capsule.
* The ground `TilemapCollider2D` is used by a `CompositeCollider2D` (outline,
  Static body). Crossing seams changes vertical velocity by at most 0.05
  units per second and never stops Wick.
* One-way platforms use `PlatformEffector2D` with a 200 degree surface arc
  and side friction off; an upward pass from below collides with nothing and
  raises no collision event. Conveyors use `SurfaceEffector2D`. Kinematic
  platforms move by `MovePosition` in `FixedUpdate`, and a rider drifts less
  than 1 pixel (0.0625 units) per waypoint leg.
* Ground checks use `Physics2D.BoxCast` with a `ContactFilter2D` on the Ground
  and OneWay layers and normals between 45 and 135 degrees. The layer matrix
  keeps beetles from colliding with embers and checkpoints.

### Rendering and animation

* Draw order follows sorting layer, order in layer, Sorting Group, and the
  transparency sort axis (0, 1, 0); Wick's 14 parts sort as one unit. The
  ground tilemap renders in Chunk mode and the prop tilemap in Individual
  mode, so tall props sort against Wick by height.
* Rule tiles choose sprite and collider type per cell, and the level 3 rule
  override tile swaps sprites while keeping rules. Nine-slice signs and panels
  keep their 4-pixel borders at every width.
* In the secret passage a 3-unit `SpriteMask` on the lantern hides rock faces
  (Visible Outside Mask) and reveals glyph platforms (Visible Inside Mask).
* 2D lights: a global light (1.0 outdoors, 0.25 in the mill and caves, 0.1 in
  Night Run), a 4-unit lantern spot light, freeform window lights, sprite
  lights on embers, and `ShadowCaster2D` pillars. Background sprites outside
  the lantern's target sorting layers receive only global light.
* The Pixel Perfect Camera (URP 2D variant) uses 16 assets pixels per unit, a
  320 by 180 reference, and upscale render texture. Zoom is 3 at 1024 x 768;
  4 at 1280 x 720 and 1366 x 768; 6 at 1920 x 1080 and 2560 x 1080; 8 at
  2560 x 1440; and 12 at 3840 x 2160.
* Wick's Animator has Idle, Run, Jump, Fall, Land, Hurt, and Cheer states and
  a 1D blend tree on `Speed`; run frames fire footstep events at normalized
  times 0.25 and 0.75. Bones deform the mesh by skin weights, a limb solver
  plants the feet on slopes up to 30 degrees, and the sprite resolver shows
  `Hurt` on the frame a heart is lost. Embers, flames, dust, and beetles use
  12 frames per second sprite-swap clips.

### Audio

* Effects and footsteps start within one frame of their events, and each chain
  chime plays the pitch step of the chain length. Music loops without a gap.
  In the browser no sound plays before the first user gesture.

### Input

* Keyboard and gamepad give the same movement and jump timing, and a gamepad
  connecting or disconnecting drops or duplicates no jump. Rebinding excludes
  the mouse, cancels with Escape or East, times out after 5 s, and names both
  actions when a binding conflicts.

### UI and text

* The canvas uses Scale With Screen Size (640 by 360 reference, match 0.5). At
  every declared aspect ratio no HUD element overlaps another, and HUD text is
  at least 16 screen pixels tall at 1280 x 720. Every menu works with
  keyboard, gamepad, and pointer, with visible focus.

### Player and build

* Each player starts on the title and restores settings and saves. The browser
  player reports loading progress, startup, storage availability, and audio
  activation as PLT-001 defines.

### Editor and automation

* Stopping play restores edited scene values, including bridge tiles removed
  at runtime. Through the editor's MCP interface an agent reads Wick's position, velocity, grounded
  state, hearts, embers, chain, glow score, medal, and checkpoint, and any
  renderer's effective sort order.

## Presentation and Performance

* Visual checkpoints with unitas-approved references: the title; each level
  start; the mill with lantern light and pillar shadows; the secret passage
  under the mask; Wick mid-hurt with the `Hurt` face; the ghost beside Wick;
  the completion panel with a Gold medal; and the HUD at each aspect ratio.
* Audio checkpoints: a footstep within one frame of its event; chain chimes
  rising one semitone per step; the pause snapshot at its cutoff after 0.3 s.
* Project-declared expectations: 60 frames per second at 1920 x 1080 on the
  declared reference desktop class and at a 1280 x 720 canvas on the declared
  reference browser class; level load under 1.5 s on desktop and under 4 s in
  the browser with the bundle cached. These are the sample's own
  expectations, not the PRF-002 engine budgets.

## Acceptance Scenarios

Every scenario runs in editor play mode, in a packaged desktop player, in the
browser player, and headless through the editor's MCP interface, unless it says otherwise.

* **S1. Open and inspect.** No cache exists. Open the project.
  * Expected: all assets import with no errors, every package is reported
    Supported with its tier, `Wick.psb` yields 14 sprites, and reopening yields
    identical GUIDs and local identifiers.
  * (PRJ-001, PRJ-003, PRJ-004, PRJ-006, PRJ-007, SER-003)
* **S2. Happy path.** Start a new game and replay the recorded input that
  completes all three levels with every ember.
  * Expected: embers reach 20, 25, and 30; each checkpoint lights once; each
    bell ends its level; the run summary appears; the Animator passes through
    every state.
  * (RUN-003, PHY-002, ANI-001, INP-002, UI-001, TXT-001, AUD-001)
* **S3. One-way platforms.** From the saved state beside the level 1 stack, jump
  up through three stacked one-way platforms, land on the top one, then walk
  into a platform's end at platform height.
  * Expected: Wick passes upward with no collision event during the pass, stands
    on top with zero vertical velocity, and is stopped by the side contact.
  * (PHY-002)
* **S4. Edges and frame rate.** At 30, 60, 144, and 240 frames per second and
  fixed steps of 0.02 s and 0.01 s, run across a 40-tile floor, stand with 1
  pixel of the capsule over a ledge, and fall 12 units onto a tile corner.
  * Expected: no stop or vertical spike on seams, Wick stays grounded on the
    ledge, and no fall tunnels.
  * (PHY-002, RUN-004, REN-007, PRJ-007)
* **S5. Sprite order and lighting.** At every visual checkpoint, capture a frame
  and read each renderer's effective order.
  * Expected: the order follows the sorting rules, Wick never interleaves with
    props, the lantern light falls off within 4 units, pillars cast shadows, and
    each capture matches its approved reference.
  * (REN-005, REN-006, PRJ-004, TST-002)
* **S6. Hazard, respawn, and failure.** In level 3, light checkpoint 2, collect
  three embers, and touch a thorn tile; then lose all hearts and choose Retry
  Level.
  * Expected: one heart is lost, Wick respawns at checkpoint 2 with zero
    velocity after 0.5 s, the three embers reappear, and platforms and beetles
    return to the phases recorded when the checkpoint lit; repeating the hit and
    input yields the same state snapshot. "Out of light" appears at zero hearts,
    and Retry gives 3 hearts, 0 embers, timer 0, unlit checkpoints, and no
    object from the previous load.
  * (SAV-001, PHY-002, RUN-001, RUN-004, RUN-005, UI-001)
* **S7. Checkpoint restoration across launches.** Light checkpoint 2 of level 2,
  quit, relaunch, and Continue. Then truncate the save file and relaunch.
  * Expected: Continue restores the level, checkpoint, hearts, embers, glow
    score, and time of the checkpoint snapshot; the truncated file is reported
    as corrupt, not absent, the title offers New Game, and the file stays on
    disk.
  * (SAV-001, RUN-009)
* **S8. Camera containment.** Replay a path into every corner of every room at
  16:9, 16:10, 21:9, and 4:3.
  * Expected: the view never shows space outside the room's confiner, the camera
    cuts at doorways with no blend, and lookahead never pushes the view past the
    confiner.
  * (CIN-001, REN-001)
* **S9. Resolution changes.** Switch the desktop player through 1024 x 768, 1280
  x 720, 1366 x 768, 1920 x 1080, 2560 x 1080, and 2560 x 1440, windowed and
  fullscreen, then drag-resize; resize the browser page to the same sizes.
  * Expected: each change applies from the next frame, the zoom equals the
    documented value, snapped sprites stay on the pixel grid, and the HUD passes
    its overlap and size checks.
  * (REN-005, REN-007, UI-001, TXT-001)
* **S10. Input equivalence and rebinding.** Complete level 1 with keyboard only,
  then gamepad only, using the pointer in menus. Rebind Jump to K, try to bind
  Move Left to K, cancel, and relaunch. Disconnect the gamepad mid-jump and
  reconnect it.
  * Expected: both runs reach the bell with the same embers; the conflict prompt
    names both actions; K survives relaunch; no input is dropped or duplicated.
  * (INP-002, UI-001, SAV-001)
* **S11. Character animation.** From the saved slope state, walk, run, jump,
  land, and take a hit.
  * Expected: blend weights follow `Speed`, footsteps fire at the stated times,
    feet stay on the slope in Idle, and the face is `Hurt` on the frame of the
    hit.
  * (ANI-001, REN-005)
* **S12. Chain and juice.** In level 1, collect the opening five-ember arc 0.6 s
  apart, wait 1.1 s, collect one more, then land from 6 units and take a hit.
  * Expected: the chain reads x1 to x5 with glow 10, 20, 30, 40, 50; chimes rise
    one semitone per step; the sixth ember resets the chain to x1; the landing
    squash lasts 0.08 s; hit-stop lasts 0.05 s of unscaled time; and the 2-pixel
    shake occurs only with screen shake on.
  * (RUN-004, AUD-001, REN-005)
* **S13. Browser lifecycle.** Browser player only. Load the page, press a key,
  light a checkpoint, switch tabs, return, and refresh. Then fill storage to its
  quota and light another checkpoint.
  * Expected: progress reaches 100 percent before the title; audio starts after
    the key press; leaving the tab pauses; after refresh Continue restores the
    checkpoint; the quota-failed write shows "Checkpoint not saved" and the
    prior save stays intact.
  * (PLT-001, SAV-001, AUD-001, RUN-009)
* **S14. Medals, ghosts, and unlocks.** From a fresh profile, replay a level 1
  run with every ember in 2:20 and no hit, then start level 1 again. Then replay
  all-ember runs of levels 2 and 3 under par.
  * Expected: level 1 awards Gold and the Lantern badge and records 2:20; on the
    next attempt the ghost is within 1 pixel of the recorded run's positions at
    matching times; a 2:40 run awards Silver and keeps the 2:20 record; Gold on
    all levels unlocks Night Run and the Brass skin, and both survive relaunch.
  * (SAV-001, RUN-009, REN-005)
* **S15. Edit and restore.** In the editor, paint three rule tiles, change a
  platform waypoint, enter play mode, let the bridge crumble, and stop.
  * Expected: the tiles and waypoint persist, the bridge tiles return, and the
    saved scene opens in Unity 2022.3 LTS with the same values.
  * (EDT-004, SER-001, PRJ-003)
* **S16. Package.** Build each desktop player and the browser bundle.
  * Expected: each launches, completes the S2 replay, and its build report lists
    scenes, atlases, assets, sizes, and the build identifier.
  * (BLD-001)

## Manual Playtest Guidance

A person judges the following, and a headless run reports these checkpoints as
awaiting human review, never as passed:

* Whether the jump feels responsive and fair with coyote time and buffering.
* Whether the lantern reveal and the first ember arc make the opening 30
  seconds compelling.
* Whether chains feel rewarding enough to take risks for.
* Whether gold embers and the Daring bonus create real choices.
* Whether level 3's timed climb feels like an escalation, not a spike.
* Whether the ghost and a near-miss of Gold make the player want to retry.
  Playtesters record whether they replayed a level without being asked.
* Whether the lighting mood and the pixel art read as intended after
  resolution changes.

## Agent Authoring Change Requests

Add a new hazard: a stalactite in level 3 that drops a droplet every 2 s,
costs one heart on contact, splashes with a sprite-swap clip and a sound, and
does not damage Wick during the respawn fade. It comes with a test that checks
one heart per contact and none during the fade, and a scenario that passes
under it twice.

A second request adds a bounce pad rule tile that launches Wick 6 units up,
with a scenario proving the same height at 30 and 240 frames per second.

## Accessibility

Every action works with the keyboard alone and with a gamepad alone, and every
gameplay action can be rebound. Menus expose accessibility names. HUD text
meets contrast requirements against its panel. Turning off screen shake also
stops hit-stop shake and fast parallax. Hazards differ by shape as well as
color, and chain feedback is shown as text as well as sound.

## Saved and Reproducible States

* Level 1 beside the one-way stack, and on the 30-degree slope.
* Level 2 at checkpoint 2 with 12 embers.
* Level 3 at checkpoint 2 with one heart, beside the thorn field.
* Level 3 inside the secret passage.
* A profile with Gold on levels 1 and 2 and a recorded ghost for each.

## Known Limitations and Unsupported Conditions

On a browser that does not expose gamepads, gamepad scenarios are reported as
not applicable on that target, never as passed. Any behavior this document
requires that is not demonstrated is reported with its real compatibility
state.

## Acceptance Outcomes

* The two-dimensional platformer passes through one-way platforms only from
  below, keeps sprite order correct, and restores checkpoints identically.
  (SMP-001, PHY-002, REN-005, SAV-001)
* Wick crosses tile seams and stands on edges without snagging or tunneling at
  every declared frame rate and fixed time step. (SMP-001, PHY-002, RUN-004)
* The camera stays inside its confiner, and the Pixel Perfect zoom and HUD stay
  correct and readable at every declared aspect ratio and resolution change.
  (SMP-001, CIN-001, REN-005, UI-001)
* The game completes with keyboard only and with a controller only, and a
  rebound control survives a relaunch. (SMP-001, INP-002, SAV-001)
* The PSB character animates, deforms, and swaps sprites on the frames the
  Animator and sprite resolver select. (SMP-001, ANI-001, REN-005)
* Chains, medals, records, ghosts, and unlocks follow their stated thresholds
  and persist across launches. (SMP-001, SAV-001, AUD-001)
* A corrupt checkpoint save is reported as corrupt, not absent, and a browser
  refresh keeps the last durable checkpoint. (SMP-001, SAV-001, PLT-001)
* Each desktop player and the browser bundle is packaged with a build report
  and completes the replayed happy path. (SMP-001, BLD-001)
