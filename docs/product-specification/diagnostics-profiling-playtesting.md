# Diagnostics, Profiling, and Playtesting Product Specification

## Scope

This specification defines how developers, testers, and agents observe,
reproduce, evaluate, and communicate project behavior.

## Unified diagnostics

A diagnostic answers what happened, where it happened, which behavior is
affected, when it happened, and what can be inspected next. It retains severity,
category, time, frame, scene, object, component, asset, script location, build,
target, device, and player connection where available.

Compilation, import, serialization, runtime, rendering, physics, animation,
audio, input, navigation, package, build, platform, network, and performance
problems remain distinct. Grouping repeated findings preserves count and
individual context.

Selecting a finding navigates to the nearest responsible content. Missing or
destroyed content remains identifiable by its recorded identity and location.

## Runtime inspection

During an authorized session, a developer or agent can inspect open scenes,
hierarchies, components, values, lifecycle state, input, physics contacts,
animation states, active audio, UI focus, loaded content, logs, timing, and
memory.

Inspection alone does not change the session. A state-changing intervention is
marked in the session evidence so the resulting behavior is not represented as
an untouched run.

## Profiling

As a developer, I can inspect frame duration and contributions from scripting,
rendering, physics, animation, audio, navigation, asset activity, networking,
player work, and editor work. I can distinguish CPU duration, waiting, frame
pacing, and target-present behavior where exposed.

Memory views identify major allocations, retained objects, loaded assets,
growth, and peak use. Rendering views identify cameras, renderers, geometry,
materials, textures, passes, and frame cost. Audio and physics views identify
active voices, bodies, contacts, and simulation expense.

A capture records project, build, target, device, quality settings, scene,
time range, and whether collection changed performance materially. Captures from
different contexts are not presented as directly equivalent without showing
their differences.

## Playtest sessions

As a developer, I can begin a playtest at the normal entry point, a selected
scene, or an approved checkpoint. A session identifies project state, build,
target, quality, display, locale, input devices, controlled random seed when the
game exposes one, start and end time, and editor or player environment.

A tester can play without authoring access and submit notes, screenshots, short
recordings, logs, exceptions, performance warnings, and current game location.
Feedback remains associated with the exact build and device. Startup and crash
reports remain possible even if the main menu is never reached.

## Structured scenarios

A scenario describes its initial scene or save state, player setup, required
actions, checkpoints, and expected observable outcomes. Outcomes can cover
object state, scenes, UI, input, animation, audio, physics, navigation, time,
logs, rendered images, performance, persistence, networking, or completion.

A result records:

* setup success or failure;
* steps performed and their timing;
* checkpoints passed, failed, skipped, or unevaluable;
* expected and observed values;
* relevant messages, exceptions, captures, and runtime state;
* environmental differences and interventions;
* repeatability under the recorded conditions.

The same scenario can be performed by a person, an authorized agent, or against
a built player. A partial run is never reported as a full pass.

## Input recording and replay

A tester can record input values and transitions with device identity, timing,
scene changes, focus, pause, and relevant application events. Replay attempts
the same interaction under declared compatible conditions.

A replay reports the first observed divergence and distinguishes an input
mismatch from a game-state mismatch. Uncontrolled network state, external
services, wall-clock time, platform scheduling, or project randomness prevents
an unconditional determinism claim and is recorded as a limitation. Even with
recorded input, cross-platform physics, floating-point, rendering, and device
behavior are not presumed deterministic.

## Visual validation

Rendered frames and sequences can be compared with approved references at
declared target, resolution, aspect, quality, scene, camera, time, and state.
Results identify changed regions, magnitude, permitted tolerances, and whether
the comparison was evaluable.

Expected platform or temporal variation can be declared narrowly. A broad
tolerance that hides missing objects, shaders, text, lighting, UI, or camera
output is not represented as a match.

## Audio validation

A playtest can observe expected clips, cues, timing, channels, levels, looping,
spatial placement, listener behavior, and silence. It distinguishes sound that
never started, was inaudible, was routed elsewhere, was virtualized, or stopped
early. Absence of an exception is not evidence of correct sound.

## Performance validation

Projects can state performance expectations per target and gameplay scenario
for frame time, frame pacing, loading pauses, memory, allocation growth,
rendering, physics, scripts, audio, networking, thermal state, and battery-
relevant behavior where measurable.

A failure identifies the interval, frequency, severity, target, and contributing
objects, systems, or assets when known. Averages do not conceal individual
stalls or sustained degradation.

## Coverage view

The project shows which scenes, gameplay flows, targets, devices, input schemes,
display configurations, locales, accessibility modes, save migrations, network
conditions, and major features have named playtest evidence.

Coverage means a scenario ran under recorded conditions. It does not imply that
loaded content, executed lines, or a single platform proves product correctness.

## Sample-game validation

Each official sample has controls, supported targets, expected behavior, known
limitations, win or completion state, failure and restart behavior, and
objective scenarios. A sample passes only when gameplay behavior succeeds in
editor play and its declared built players.

The sample portfolio covers at least a physics-based introductory game, a 3D
character game, a 2D platformer, a first-person interaction game, a UI-heavy
application, mobile touch behavior, and applicable networking or XR behavior.

## Acceptance stories

1. A tester report identifies the exact build, device, location, input context,
   evidence, and observed failure.
2. A recorded scenario distinguishes setup failure, failed expectation,
   unevaluable expectation, and full pass.
3. A replay identifies its first divergence and does not overclaim determinism.
4. Visual, audio, and performance failures link back to relevant project state.
5. Coverage communicates actual executed scenarios, not a marketing percentage.

