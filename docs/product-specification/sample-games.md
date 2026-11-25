# Playable Samples and Compatibility Benchmarks

## Product Outcome

unitas includes complete playable games and focused compatibility projects that
demonstrate the engine's declared behavior. A sample is not accepted because it
opens, imports, compiles, or reaches a title screen. It has objective gameplay
scenarios that succeed in editor play mode, headless through MCP, and on
declared player targets. Agent authoring benchmarks show that an agent can
change a sample through MCP alone.

Compatibility tiers of the features this specification owns, as recorded in
the [Feature catalog](feature-catalog.md) and defined in the
[Compatibility contract](compatibility-contract.md): SMP-001 Native, CMP-001
Native. The playable samples are original unitas games; their acceptance is
defined by their own scenarios and unitas-approved references, not by
comparison with output from Unity. Only the compatibility benchmarks compare
with Unity 2022.3 LTS, and only for Exact-tier and Semantic-tier behavior.

Every sample provides controls, gameplay objective, expected presentation,
feature coverage, supported targets, accessibility information, known
limitations, and repeatable acceptance scenarios. Samples use redistributable
content with clear licensing.

## Sample-Level Requirements

Each playable sample includes:

* A complete start, gameplay, completion or failure, restart, pause, and exit
  experience.
* At least one automated happy-path scenario and one failure or recovery
  scenario.
* Manual playtest guidance for behavior that requires human judgment.
* unitas-approved reference images, as defined by TST-002 in
  [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md),
  for stable visual checkpoints; visual checkpoints of Familiar-tier and
  Native-tier features are judged against these references, never against
  Unity screenshots.
* Performance expectations stated separately for representative target classes.
* A compatibility report that identifies every feature the sample exercises.
* A saved or reproducible state for reaching important test situations quickly.
* Keyboard accessibility and at least one additional suitable input mode where
  the target supports it.

Sample performance expectations are project-declared expectations as defined
under Performance validation (TST-002) in
[Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md);
they are distinct from, and never presented as, the engine budgets of PRF-002.

## Headless and Agent Execution

Every acceptance scenario of every sample and benchmark runs in two ways: by a
person in the editor or a player, and headless through MCP by an authorized
agent or a continuous-integration invocation, as defined in
[Automation and MCP](automation-and-mcp.md) and by BLD-002 in
[Builds, platforms, XR, and networking](build-platforms-xr-networking.md). Both
ways use the same scenario definition and produce the same pass or fail outcome
and the same checkpoint evidence, including visual checkpoints captured without
a display. A scenario that requires human judgment states that requirement, and
its headless run reports the checkpoint as awaiting human review rather than as
passed.

Every scenario result records the sample revision, the unitas version, the
target, and whether it ran headless.

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

The player traverses a tile-based level, uses moving and one-way platforms
(one-way platforms use the platform effector defined by PHY-002 in
[Physics, navigation, and terrain](physics-navigation-terrain.md)), collects
items, encounters hazards, respawns at checkpoints, and reaches a goal.

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
dynamic bodies, joints, impact feedback (URP `DecalProjector` decals on URP 12.x
and 14.x, as defined by REN-006 in
[Rendering and animation](rendering-and-animation.md), and particle and mesh
feedback on the Built-in Render Pipeline and URP 10.x),
lighting, shadows, particles, spatial audio, focus behavior, and performance
profiles.

Acceptance covers focus loss and recovery, frame-rate variation, fast object
motion, interaction priority, input rebinding conflicts, pause, and representative
quality levels.

## UI and Localization Showcase

The user navigates menus, validates forms, changes settings, reads scrollable
content, interacts with modals, and completes the same flow using pointer,
keyboard, controller, and touch on the targets that offer each input mode, and
through authorized automation on every target.

The sample covers Canvas and RectTransform behavior, layout, text, font
fallback, controls, focus, navigation, safe areas, accessibility names, dynamic
scaling, multiple locales, long translations, plurals, variables, right-to-left
behavior for the showcase's locales marked right-to-left as defined by LOC-001
in [Accessibility, localization, and persistence](accessibility-localization-persistence.md),
and persistent preferences.

## Mobile Game

The player completes a short touch-driven game across declared aspect ratios and
orientations. The game survives interruption, background and foreground
transitions, touch cancellation, permitted orientation change, safe-area
changes, and restoration from saved progress.

The sample covers gestures, multi-touch, virtual controls, mobile lifecycle,
permissions the target requires, haptics on devices that expose them,
constrained quality, thermal or battery observations on targets that expose them, and mobile packaging.

## Multiplayer Arena

Using Netcode for GameObjects within the version line that CMP-002 in the
[Compatibility contract](compatibility-contract.md) lists, multiple players can host or join a
short session, move and act, observe consistent scoring, handle late joining,
disconnect and reconnect within declared rules, finish the match, and reject
incompatible versions clearly.

Scenarios distinguish game-rule correctness from transport and synchronization
behavior. They cover latency, loss, host departure, duplicate messages, stale
state, ownership, local multi-instance testing including headless instances,
and participant-specific logs.

## XR Interaction Room

On declared XR targets, the player sees correctly scaled stereo content, uses
tracked input, selects and grabs objects, interacts with world-space UI,
experiences spatial audio and, on devices that expose them, haptics, changes comfort options,
and recovers from tracking or focus interruption.

The sample distinguishes device capability differences and never reports a
feature as passing on hardware that does not expose it.

## Compatibility Benchmark Projects

Focused benchmark projects isolate behavior whose correctness is difficult to
infer from a complete game. Benchmarks of Exact-tier and Semantic-tier behavior
compare unitas with observations of Unity 2022.3 LTS; that set covers
serialization and references, prefabs and overrides, C# APIs and lifecycle
order, time and coroutines, scene loading, physics contacts and queries, asset
and resource loading, project tests, and save evolution.

Benchmarks of Familiar-tier and Native-tier behavior, including shader and
material behavior, animation transitions, UI layout and event routing, input
phases, audio timing, and target lifecycle, check the behavior that the owning
unitas specification defines against unitas-approved references. They are
never compared with Unity output, and a difference from Unity in them is a
listed difference, not a benchmark failure.

Each benchmark states its input project, tier, reference (Unity 2022.3 LTS for
Exact-tier and Semantic-tier benchmarks, the owning unitas specification and
its approved references otherwise), preconditions, observable expected
behavior, permitted variation, unsupported conditions, and result evidence. A
benchmark remains unchanged when unitas fails it; the result reports the
incompatibility rather than changing the expectation to match the engine. Each
benchmark result records the unitas version and, for Exact-tier and
Semantic-tier benchmarks, the Unity 2022.3 LTS release it was compared
against.

An upstream Unity project is included only when its license permits
redistribution. Otherwise, the benchmark records acquisition prerequisites and
expected behavior without copying protected content. An original unitas sample
and an unmodified upstream compatibility benchmark remain explicitly distinct.

## Agent Authoring Benchmark

The Agent Authoring Benchmark demonstrates that an authorized agent using only
MCP can make a specified gameplay change to a sample and carry it through
compilation, testing, play, and developer review.

Each benchmark run starts from:

* a named playable sample at a recorded revision, with its existing acceptance
  scenarios passing;
* a written change request with an acceptance scenario that defines success;
* an agent session connected through MCP, as defined in
  [Automation and MCP](automation-and-mcp.md), with no other access to the
  project and no human edits during the run.

The reference change request for the Roll-a-Ball-style sample is: add a second
pickup type that awards a different score value, has a distinct material and
collection sound, appears at specified positions in the arena, and shows a
score-feedback message in the UI when collected, together with a test that
checks the awarded score. Every other playable sample has at least one change
request of comparable scope, such as a new hazard in the two-dimensional
platformer or a new interactable in the first-person interaction sample.

During the run the agent inspects the project, creates or edits scripts,
assets, prefabs, and scene content, requests compilation and resolves its
diagnostics, adds and runs project tests (TST-003), runs the sample's scenarios
and the change request's scenario (TST-002), and plays the sample to capture
evidence. The developer then reviews the session's change set (EDT-008) in
[Editor and authoring](editor-and-authoring.md) and accepts or rejects it.

A run succeeds only when every criterion is observed:

* the project compiles with no errors, and every warning introduced by the
  change is listed in the run result;
* the tests the agent added and the project's existing tests pass;
* the change request's acceptance scenario passes in editor play mode and in a
  player on at least one declared target;
* every acceptance scenario that passed at the starting revision still passes;
* the change set contains only changes attributed to the agent session, each
  shown as a semantic property-level difference for content and a text
  difference for scripts, with no unrelated serialized changes;
* the changed files remain Unity-readable under the Exact-tier round-trip
  expectations of the [Compatibility contract](compatibility-contract.md), and,
  where Unity 2022.3 LTS is available, the changed project opens in it with the
  new scenes, prefabs, serialized values, and references intact;
* accepting the change set keeps the change, and rejecting it returns the
  project to its starting content.

The run result records the sample revision, the change request, the unitas
version, the agent client as it identified itself, elapsed time, the number of
MCP requests, the diagnostics encountered, and the number of human
interventions, which is zero for a successful run. A failed run is classified as
a product failure when unitas lacked an MCP capability, returned an incorrect
result, or produced misleading diagnostics, and as an agent failure when the
product behaved as specified. A missing MCP capability is reported as a parity
failure of AUT-001 naming the capability.

## Acceptance Outcomes

* A new user can open, play, understand, alter, and build every sample for its
  declared targets. (SMP-001, BLD-001)
* Each feature claim in a sample maps to a catalog identifier and an observed
  scenario outcome. (SMP-001)
* Every acceptance scenario of every sample produces the same outcome and
  checkpoint evidence when a person runs it in the editor and when an agent runs
  it headless through MCP. (SMP-001, AUT-004, TST-002)
* The Roll-a-Ball-style sample collects every pickup exactly once at different
  frame rates, keeps the ball inside its boundaries, and completes the same
  scenario with keyboard and gamepad in editor and player. (SMP-001, PHY-001,
  INP-002, UI-001)
* The two-dimensional platformer passes through one-way platforms only from
  below, keeps sprite order correct, and restores checkpoints identically.
  (SMP-001, PHY-002, REN-005, SAV-001)
* The third-person adventure resumes from a save with the same area, character
  state, and objective progress, and changes locale during play without losing
  state. (SMP-001, ANI-001, PHY-003, NAV-001, LOC-001, SAV-001)
* The first-person interaction sample recovers from focus loss, resolves input
  rebinding conflicts, and keeps fast-moving objects from passing through
  colliders. (SMP-001, INP-002, PHY-001)
* The UI and localization showcase completes the same flow with pointer,
  keyboard, controller, touch, and automation, including right-to-left
  layouts. (SMP-001, UI-001, TXT-001, LOC-001, ACC-001)
* The mobile game survives interruption, backgrounding, and orientation change
  and restores saved progress. (SMP-001, PLT-001, SAV-001)
* The multiplayer arena distinguishes game-rule failures from transport and
  synchronization failures and attributes logs to participants. (SMP-001,
  NET-001)
* The XR interaction room never reports a feature as passing on hardware that
  does not expose it. (SMP-001, XR-001)
* A sample's performance report presents its project-declared expectations
  separately from the PRF-002 engine budgets and never labels them as engine
  budgets. (SMP-001, TST-002, PRF-002)
* The first-person interaction sample shows `DecalProjector` impact decals on
  URP 12.x and 14.x and particle and mesh feedback on the Built-in Render
  Pipeline and URP 10.x. (SMP-001, REN-006)
* A failing sample identifies the first failed behavior without erasing later
  evidence. (SMP-001, TST-002)
* Visual appeal never substitutes for behavioral coverage, and broad feature
  count never substitutes for a complete playable loop. (SMP-001)
* An agent using only MCP completes the Roll-a-Ball-style pickup change request,
  with compilation, tests, the new scenario, and all prior scenarios passing,
  and the developer accepts its change set; rejecting the same change set
  returns the project to its starting content. (SMP-001, AUT-005, TST-003,
  EDT-008)
* A failed Agent Authoring Benchmark run is classified as a product failure or
  an agent failure, and a missing MCP capability is reported by name. (SMP-001,
  AUT-001)
* A compatibility benchmark that unitas fails keeps its expectation unchanged,
  reports the incompatibility with its evidence, and records the unitas version
  and, for an Exact-tier or Semantic-tier benchmark, the Unity 2022.3 LTS
  release it was compared against. (CMP-001, CMP-004)
* A Semantic-tier benchmark of coroutine and lifecycle order compares unitas
  with Unity 2022.3 LTS observations, while a Familiar-tier benchmark of UI
  layout passes against its unitas-approved reference even where its layout
  differs from Unity's, and that difference is listed. (CMP-001, RUN-003,
  UI-001)
* A sample's visual checkpoint is judged against its unitas-approved reference
  image, and no sample acceptance scenario requires output identical to Unity.
  (SMP-001, TST-002)
* An upstream Unity project whose license does not permit redistribution is
  represented by acquisition prerequisites and expected behavior without copied
  content. (CMP-001)
* For each release, every generally available feature family in that release's
  feature catalog maps to at least one named sample scenario on a declared
  supported target; focused benchmarks cover boundary and failure behavior.
  (SMP-001, CMP-001, CMP-004)
