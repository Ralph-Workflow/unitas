# Playable Samples and Compatibility Benchmarks

## Product Outcome

unitas includes complete playable games and focused compatibility projects that
demonstrate the engine's declared behavior. A sample is not accepted because it
opens, imports, compiles, or reaches a title screen. It has objective gameplay
scenarios that succeed in editor play mode and declared player targets.

Every sample provides controls, gameplay objective, expected presentation,
feature coverage, supported targets, accessibility information, known
limitations, and repeatable acceptance scenarios. Samples use redistributable
content with clear licensing.

## Sample-Level Requirements

Each playable sample includes:

* A complete start, gameplay, completion or failure, restart, pause, and exit
  experience appropriate to its genre.
* At least one automated happy-path scenario and one failure or recovery
  scenario.
* Manual playtest guidance for behavior that requires human judgment.
* Expected screenshots or other references for stable visual checkpoints.
* Performance expectations stated separately for representative target classes.
* A compatibility report that identifies every feature the sample exercises.
* A saved or reproducible state for reaching important test situations quickly.
* Keyboard accessibility and at least one additional suitable input mode where
  the target supports it.

## Roll-a-Ball-Style unitas Sample

This is an original, redistributable unitas sample inspired by the familiar
rolling-ball learning scenario. It is distinct from any official Unity sample
and makes no claim to redistribute or modify Unity-owned sample content.

The player moves a physics-controlled ball through a contained arena, collects
all pickups exactly once, sees score progression, receives completion feedback,
and can restart.

The sample demonstrates project opening, scene serialization, prefabs,
GameObjects, transforms, scripts, lifecycle, input, cameras, rigid bodies,
colliders, triggers, materials, lighting, UI, audio, builds, and playtesting.

Acceptance stories include collecting items at different frame rates, colliding
without passing through boundaries, losing and regaining input focus, finishing
with keyboard and gamepad, restarting without retained transient state, and
completing the same scenario in editor and player.

## Two-Dimensional Platformer

The player traverses a tile-based level, uses moving and one-way platforms,
collects items, encounters hazards, respawns at checkpoints, and reaches a goal.

It demonstrates sprites, atlases, animation, sorting, tilemaps, orthographic
cameras, 2D physics and queries, UI, audio, input, save checkpoints, resolution
changes, and pixel-oriented presentation.

Acceptance includes stable collision at platform edges, correct sprite order,
camera containment, deterministic checkpoint restoration, equivalent keyboard
and controller completion, and readable UI across declared aspect ratios.

## Third-Person Adventure

The player explores a short environment, walks, runs, jumps, uses moving
platforms, interacts with objects, reaches checkpoints, changes areas, saves,
loads, pauses, changes settings, and completes an objective.

It demonstrates imported skinned characters, animation controllers, blend
trees, root motion where declared, character collision, camera following,
action-based input, terrain, navigation, particles, spatial audio, scene
transitions, localization, and persistence.

Acceptance covers slope and step behavior, falling and respawning, device
changes during play, save-and-resume, locale changes, pause semantics, animation
transitions, and player/editor equivalence.

## First-Person Interaction

The player navigates a compact physics space, manipulates objects, uses switches
and doors, follows spatial sound, changes graphics and controls, and completes a
short puzzle.

It demonstrates pointer look, cursor capture, rebinding, physics queries,
dynamic bodies, joints, decals or equivalent impact feedback where supported,
lighting, shadows, particles, spatial audio, focus behavior, and performance
profiles.

Acceptance covers focus loss and recovery, frame-rate variation, fast object
motion, interaction priority, input rebinding conflicts, pause, and representative
quality levels.

## UI and Localization Showcase

The user navigates menus, validates forms, changes settings, reads scrollable
content, interacts with modals, and completes the same flow using pointer,
keyboard, controller, touch, and authorized automation where available.

The sample covers Canvas and RectTransform behavior, layout, text, font
fallback, controls, focus, navigation, safe areas, accessibility names, dynamic
scaling, multiple locales, long translations, plurals, variables, right-to-left
behavior within declared support, and persistent preferences.

## Mobile Game

The player completes a short touch-driven game across declared aspect ratios and
orientations. The game survives interruption, background and foreground
transitions, touch cancellation, permitted orientation change, safe-area
changes, and restoration from saved progress.

The sample covers gestures, multi-touch, virtual controls, mobile lifecycle,
permissions where required, haptics where available, constrained quality,
thermal or battery observations where exposed, and mobile packaging.

## Multiplayer Arena

When networking is declared supported, multiple players can host or join a
short session, move and act, observe consistent scoring, handle late joining,
disconnect and reconnect within declared rules, finish the match, and reject
incompatible versions clearly.

Scenarios distinguish game-rule correctness from transport and synchronization
behavior. They cover latency, loss, host departure, duplicate messages, stale
state, ownership, local multi-instance testing, and participant-specific logs.

## XR Interaction Room

On declared XR targets, the player sees correctly scaled stereo content, uses
tracked input, selects and grabs objects, interacts with world-space UI,
experiences spatial audio and haptics where available, changes comfort options,
and recovers from tracking or focus interruption.

The sample distinguishes device capability differences and never reports a
feature as passing on hardware that does not expose it.

## Compatibility Benchmark Projects

Focused benchmark projects isolate behavior whose correctness is difficult to
infer from a complete game. The set covers serialization and references,
prefabs and overrides, C# APIs and lifecycle order, time and coroutines, scene
loading, shader and material behavior, animation transitions, physics contacts
and queries, UI layout and event routing, input phases, audio timing, asset
loading, save evolution, and target lifecycle.

Each benchmark states its input project, Unity reference version, preconditions,
observable expected behavior, permitted variation, unsupported conditions, and
result evidence. A benchmark remains unchanged when unitas fails it; the result
reports the incompatibility rather than changing the expectation to match the
engine.

An upstream Unity project is included only when its license permits
redistribution. Otherwise, the benchmark records acquisition prerequisites and
expected behavior without copying protected content. An original unitas sample
and an unmodified upstream compatibility benchmark remain explicitly distinct.

## Acceptance Outcomes

* A new user can open, play, understand, alter, and build every sample for its
  declared targets.
* Each feature claim in a sample maps to a catalog identifier and an observed
  scenario outcome.
* A failing sample identifies the first failed behavior without erasing later
  evidence.
* Visual appeal never substitutes for behavioral coverage, and broad feature
  count never substitutes for a complete playable loop.
* For each release, every generally available feature family in that release's
  feature catalog maps to at least one named sample scenario on a declared
  supported target; focused benchmarks cover boundary and failure behavior.
