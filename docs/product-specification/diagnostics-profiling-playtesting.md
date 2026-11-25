# Diagnostics, Profiling, and Playtesting Product Specification

## Scope

This specification defines how developers, testers, and agents observe,
reproduce, debug, measure, test, and communicate project behavior, and the
performance and scale commitments unitas itself makes.

Every capability here runs in the visual editor, in headless sessions, and
through MCP with the same results. Automation rules, and the terms agent
session, play session, and headless session, are owned by AUT-004 in
[Automation and MCP](automation-and-mcp.md) and are not restated here.

### Compatibility tiers

Each feature this document owns has the tier recorded in the
[Feature catalog](feature-catalog.md), as defined in the
[Compatibility contract](compatibility-contract.md):

* Native: DIA-001, DIA-002, DIA-003, PRF-001, PRF-002, TST-001, and TST-002.
  Their behavior is defined by this specification. Unity-shaped APIs they
  observe, such as `Debug.DrawLine` and `ProfilerMarker`, keep their documented
  Unity 2022.3 LTS meaning.
* Semantic: TST-003. Project tests written for the Unity Test Framework behave
  as they do under Unity 2022.3 LTS in ordinary use.

## Unified diagnostics

A diagnostic answers what happened, where it happened, which behavior is
affected, when it happened, and what can be inspected next. It retains severity,
category, identifier, time, frame, scene, object, component, asset, script
location, build, target, device, and player connection when known. (DIA-001)

Compilation, import, serialization, runtime, rendering, physics, animation,
audio, input, navigation, package, build, platform, network, and performance
problems remain distinct. Grouping repeated findings preserves count and
individual context.

Selecting a finding navigates to the nearest responsible content. A finding with
a script location opens the configured external editor at that file and line, as
defined in [Editor and authoring](editor-and-authoring.md). Missing or
destroyed content remains identifiable by its recorded identity and location.
Each diagnostic identifier resolves to an explanation through product knowledge
(AUT-006).

The behavior of the `Debug.Log` family and of unhandled exceptions in scripts is
owned by RUN-002 in [Scripting runtime](scripting-runtime.md); this section owns how the
resulting entries are collected, grouped, filtered, and navigated.

## Debug drawing capture

Lines and rays from `Debug.DrawLine` and `Debug.DrawRay`, and gizmos drawn
through `Gizmos` in `OnDrawGizmos` and `OnDrawGizmosSelected`, are captured as
structured data in editor and headless play sessions and in edit mode. Each captured item records its
primitive kind (line, ray, cube, sphere, mesh, frustum, icon, wire or solid),
world-space coordinates after the active `Gizmos.matrix`, color, duration,
`depthTest` setting, frame, and originating object and script location.
(DIA-001)

An agent or scenario can read the captured items for a frame or time range
without rendering an image. Visual display of the same items in the scene and
game views is owned by EDT-003 in [Editor and authoring](editor-and-authoring.md).
In built players, `Debug.DrawLine` and `Debug.DrawRay` draw nothing and gizmo
callbacks are not invoked; capture is unavailable there and
is reported as such rather than as an empty result.

## Runtime inspection

During an authorized play session, a developer or agent can inspect open scenes,
hierarchies, components, values, lifecycle state, input, physics contacts,
animation states, active audio, UI focus, loaded content, logs, timing, and
memory. (DIA-001)

Inspection alone does not change the play session. A state-changing intervention
is marked in the play session's evidence so the resulting behavior is not represented as
an untouched run.

## Player crash reports, symbols, and device logs

### Crash and exception reports

Development and release players capture unhandled managed exceptions and native
crashes into a crash report. Each report records the build identifier, unitas
version, target, device model, operating system and version, the managed stack
of every managed thread, and the native stack of the crashing thread. A crash
during startup, before the first scene loads, produces a report as well.
(DIA-003)

### Symbols

Every build produces symbol files matched to its build identifier: PDB files
for Windows players, dSYM bundles for Apple targets, Android native symbols in
the form Google Play accepts for upload, and source maps for browser players.
A report from a release player is symbolicated through the visual editor, the
command line, and MCP using the symbols whose build identifier matches; a report
with no matching symbols states that and keeps its unsymbolicated addresses.

Players send crash reports only to destinations that the project configures.
No destination is configured by default; a player with no destination sends no
report off the device.

### Device logs

Logs are collected from a connected player as a live stream, or from a copied
log file: the desktop `Player.log` locations of each desktop target, Android
logcat, the iOS device console, and the browser console. Collected lines become
structured entries with time, severity, source (player, engine, or operating
system), and, for player entries, the originating message as defined by the log
APIs of RUN-002 in [Scripting runtime](scripting-runtime.md). People and agents
read the same entries, filtered by player, severity, and time range.

## External code tools and script debugging

unitas has no built-in code editor. Script source is edited and debugged in the
developer's external editor or IDE, or by an agent, using the project files and
debugging connection defined here. (DIA-002)

### Generated project files

unitas generates a solution file (`.sln`) and one C# project file (`.csproj`)
per compiled assembly: each predefined assembly (`Assembly-CSharp`,
`Assembly-CSharp-firstpass`, `Assembly-CSharp-Editor`,
`Assembly-CSharp-Editor-firstpass`) that has sources, and each assembly
definition. Each project file lists the same source files, scripting define
symbols, assembly and precompiled references, language version, unsafe-code
setting, and, in trusted projects, the same Roslyn analyzers and source
generators that unitas compilation uses, as defined by RUN-007 in
[Scripting runtime](scripting-runtime.md). References to the UnityEngine and
UnityEditor API resolve to reference assemblies that describe the
unitas-compatible surface.

* An external IDE or language server that loads the generated files reports the
  same compiler diagnostics (identifier, file, line, column) that unitas reports
  for the same sources.
* The files are regenerated when the assembly layout, defines, references, or
  target change, without a manual step, and their content depends only on the
  project and the selected target, not on the machine.
* The files are written to the project root with one `.csproj` per assembly
  named after the assembly, so source-control ignore rules that exclude
  `*.csproj` and `*.sln` at the project root exclude them; they are
  generated content as defined by SER-005 in
  [Serialization and prefabs](serialization-and-prefabs.md).
* Generation can be turned off per user. Opening a project never requires the
  files to exist.

### Debugging through the Debug Adapter Protocol

unitas provides a Debug Adapter Protocol (DAP) endpoint. It listens only on the
loopback interface unless network connections are enabled as defined by AUT-004
in [Automation and MCP](automation-and-mcp.md), and it requires a credential that
unitas issues to the person or agent session. Attaching, setting breakpoints, and
reading values require the read-project scope; evaluating expressions, setting
values, and attaching to edit-time script execution require the
execute-project-code scope (AUT-007). A DAP client attaches to:

* the editor's play session and edit-time script execution (editor scripts,
  `[InitializeOnLoad]` code, and commands);
* headless play sessions, including each play session that an agent session
  starts (AUT-004);
* development players built with script debugging enabled, including a player
  started to wait for a debugger before running project code.

An attached client can:

* set line breakpoints, conditional breakpoints, hit-count breakpoints, and
  logpoints, and break on thrown or unhandled exceptions;
* pause, continue, step in, step over, and step out;
* read the call stack, including coroutine and `async` method frames mapped to
  their source lines;
* read and expand locals, arguments, `this`, static fields, and evaluated
  expressions; an evaluation that could have side effects is marked as such;
* see Unity objects with their name, type, and instance ID; a destroyed Unity
  object is shown as destroyed (it compares equal to `null` through the
  `UnityEngine.Object` equality operator) while its managed fields remain readable.

While a breakpoint holds execution, the frame loop does not advance. After
resuming, the next frame's `Time.deltaTime` is limited by
`Time.maximumDeltaTime`.

A breakpoint that cannot bind (for example in a file not part of any compiled
assembly, or in code stripped from the player) is reported as unbound with the
reason. A release player, or a target whose scripting backend cannot support
managed debugging, reports debugging as unavailable at build time with the
reason, using the Platform-limited state where the limit belongs to the target.

Several clients can attach to the same play session or player. Breakpoints from every attached
client apply, each attributed to its client, and every client is notified of
each stop.

### Debugging for agents

The same debugging operations are available to agents as MCP tools: attach,
set and clear breakpoints, continue, step, read stack frames and values,
evaluate expressions, and detach. A stop is delivered to the agent as a
notification with the stopping location and reason. An agent and a person can
debug the same play session at once under the multiple-client rule above.

## Profiling

As a developer, I can inspect frame duration and contributions from scripting,
rendering, physics, animation, audio, navigation, asset activity, networking,
player work, and editor work. I can distinguish CPU duration, waiting, frame
pacing, and target-present behavior when the target exposes them. (PRF-001)

Memory views identify major allocations, retained objects, loaded assets,
growth, and peak use. Rendering views identify cameras, renderers, geometry,
materials, textures, passes, and frame cost. Audio and physics views identify
active voices, bodies, contacts, and simulation expense.

Custom samples from `Profiler.BeginSample` and `Profiler.EndSample`,
`ProfilerMarker`, and `CustomSampler` appear under their names in the thread
and frame where they ran. In players, they are recorded only in development
builds.

Profiling runs in the editor, headless, and against connected development
players. An agent can start and stop a capture and read it as structured data.

A capture records project, build, target, device, quality settings, scene,
time range, and whether collection changed performance materially. Captures from
different contexts are not presented as directly equivalent without showing
their differences.

## Engine performance and scale commitments

unitas states the performance it delivers as per-release budgets with published
measurements.
(PRF-002)

### Scale floor

Projects with at least 100,000 assets and scenes with at least 100,000
GameObjects open and remain operable through automation and the visual editor.
Operable means that hierarchy and asset queries, search, selection, property
edits, undo, and save on such content complete within the release's declared
budgets, and that queries return paginated results rather than the whole
project.

### Declared budgets

Each release declares budgets, measured on named reference projects with
recorded hardware, operating system, and target, for:

* project open, cold (no unitas caches) and warm;
* incremental reimport after a single asset change;
* latency from a saved script change to a play-ready state;
* entering play mode;
* visual editor responsiveness to input and MCP request latency for
  representative inspection and edit requests;
* runtime frame-time overhead relative to the Unity reference player on the
  benchmark projects (CMP-001 in [Playable samples and benchmarks](sample-games/INDEX.md)),
  same target and hardware;
* player startup time;
* editor, headless, and player memory overhead;
* hierarchy and asset queries, search, selection, property edits, undo, and
  save on the scale-floor reference projects;
* build time and player size.

Budgets and their measurements are published with the release and record the
unitas version. A measurement that exceeds its budget is a regression; a
release with an unresolved regression cannot claim the affected feature family
generally available on that target, as defined by CMP-004 in the
[Compatibility contract](compatibility-contract.md). Numbers measured on one
reference project, machine, or target are never presented as applying to
another.

## Playtests

As a developer, I can begin a playtest at the normal entry point, a selected
scene, or an approved checkpoint. A playtest identifies project state, build,
target, quality, display, locale, input devices, controlled random seed when the
game exposes one, start and end time, and editor or player environment.
(TST-001)

A tester can play without authoring access and submit notes, screenshots, short
recordings, logs, exceptions, performance warnings, and current game location.
Feedback remains associated with the exact build and device. Startup and crash
reports remain possible even if the main menu is never reached.

## Structured scenarios

A scenario describes its initial scene or save state, player setup, required
actions, checkpoints, and expected observable outcomes. Outcomes can cover
object state, scenes, UI, input, animation, audio, physics, navigation, time,
logs, debug drawing, rendered images, performance, persistence, networking, or
completion. (TST-002)

A scenario is unitas-specific project content (SER-005 in
[Serialization and prefabs](serialization-and-prefabs.md)) stored in a versioned
text format whose schema is published through product knowledge (AUT-006).
People, and agent sessions holding the edit-content scope, create and edit
scenarios; an agent session's changes to scenarios are items of its change set
(EDT-008). A reference image or checkpoint expectation is approved when a
person, or an agent session holding the review scope, accepts the change-set
item that adds or changes it; an unapproved reference is never used to pass a
comparison.

A result records:

* setup success or failure;
* steps performed and their timing;
* checkpoints passed, failed, skipped, or unevaluable;
* expected and observed values;
* relevant messages, exceptions, captures, and runtime state;
* environmental differences and interventions;
* repeatability under the recorded conditions.

The same scenario can be performed by a person, an authorized agent, headless,
or against a built player. A partial run is never reported as a full pass.

## Input recording and replay

A tester can record input values and transitions with device identity, timing,
scene changes, focus, pause, and relevant application events. Replay attempts
the same interaction under declared compatible conditions. (TST-002)

A replay reports the first observed divergence and distinguishes an input
mismatch from a game-state mismatch. Uncontrolled network state, external
services, wall-clock time, platform scheduling, or project randomness prevents
an unconditional determinism claim and is recorded as a limitation. Even with
recorded input, cross-platform physics, floating-point, rendering, and device
behavior are not presumed deterministic.

## Visual validation

Visual references are unitas-approved captures produced by unitas rendering, not
Unity screenshots; rendering is Native tier and pixel identity with Unity is not
promised. Rendered frames and sequences can be compared with approved references
at
declared target, resolution, aspect, quality, scene, camera, time, and state.
Results identify changed regions, magnitude, permitted tolerances, and whether
the comparison was evaluable. (TST-002)

Expected platform or temporal variation can be declared per region, metric, and
target. A tolerance that hides missing objects, shaders, text, lighting, UI, or
camera output is not represented as a match.

## Audio validation

A playtest can observe expected clips, cues, timing, channels, levels, looping,
spatial placement, listener behavior, and silence. It distinguishes sound that
never started, was inaudible, was routed elsewhere, was virtualized, or stopped
early. Absence of an exception is not evidence of correct sound. (TST-002)

## Performance validation

Projects can state performance expectations per target and gameplay scenario
for frame time, frame pacing, loading pauses, memory, allocation growth,
rendering, physics, scripts, audio, networking, thermal state, and battery-
relevant behavior when the target exposes them. (TST-002)

A failure identifies the interval, frequency, severity, target, and contributing
objects, systems, or assets when known. Averages do not conceal individual
stalls or sustained degradation. Project-declared expectations are distinct from
the engine budgets of PRF-002.

## Project tests

Projects' Unity Test Framework tests are discovered and run. (TST-003)

### Discovery

* EditMode tests are discovered in test assemblies that include only the Editor
  platform; PlayMode tests in test assemblies that include runtime platforms.
  Test assemblies are recognized through their assembly definition's reference
  to the test runner and NUnit, the `UNITY_INCLUDE_TESTS` define constraint, and
  the older `optionalUnityReferences: TestAssemblies` form.
* Discovery lists each test's full name, assembly, mode, categories, and
  parameters, and reports tests that cannot run (for example a compile error in
  the test assembly) with the reason.

### Supported test behavior

* NUnit attributes: `[Test]`, `[TestCase]`, `[TestCaseSource]`, `[Values]`,
  `[ValueSource]`, `[TestFixture]`, `[SetUp]`, `[TearDown]`, `[OneTimeSetUp]`,
  `[OneTimeTearDown]`, `[Category]`, `[Ignore]`, `[Explicit]`, `[Timeout]`,
  `[Repeat]`, and `[Retry]`, and the NUnit assertion and constraint model.
* Unity Test Framework attributes: `[UnityTest]` coroutine tests yielding
  `null`, `WaitForSeconds`, `WaitForFixedUpdate`, `WaitForEndOfFrame`, other
  `YieldInstruction`s, and `CustomYieldInstruction`s; `[UnitySetUp]` and
  `[UnityTearDown]`; `[UnityPlatform]`; `[PrebuildSetup]` and
  `[PostBuildCleanup]`.
* `LogAssert.Expect` and `LogAssert.NoUnexpectedReceived`; an unexpected error
  or exception log fails the test, as in Unity, unless
  `LogAssert.ignoreFailingMessages` is set.
* PlayMode tests run in a play session that starts from a fresh test scene and
  leaves edit-time state unchanged afterwards.

### Running and results

Tests run in the visual editor, headless, through the command line defined by
BLD-002 in [Builds, platforms, XR, and networking](build-platforms-xr-networking.md),
and through MCP. PlayMode tests also run on a development player for a target:
unitas builds the test player, runs it, and collects the results. A target that
cannot host a test player reports that run as Platform-limited.

* Runs can be filtered by test name, full-name pattern, assembly, category, and
  mode, and a run can be cancelled.
* Results are produced in the NUnit 3 XML result format, and also as structured
  data through MCP, with each test's outcome (passed, failed, skipped,
  inconclusive, or not run), duration, message, stack trace with source
  locations, and captured log output.
* A run that stops early (cancellation, crash, timeout, or lost player) reports
  which tests completed and never reports the remainder as passed.

## Coverage view

The project shows which scenes, gameplay flows, targets, devices, input schemes,
display configurations, locales, accessibility modes, save migrations, network
conditions, and major features have named playtest, scenario, or test evidence.
(TST-002)

Coverage means a scenario or test ran under recorded conditions. It does not
imply that loaded content, executed lines, or a single platform proves product
correctness.

## Sample-game validation

The official samples, their acceptance scenarios, and their pass criteria are
owned by SMP-001 in [Playable samples and benchmarks](sample-games/INDEX.md). Their
scenarios are structured scenarios as defined in this document (TST-002).

## Acceptance stories

1. Selecting a grouped runtime exception shows its count, each occurrence's
   frame and object, and opens the configured external editor at the throwing
   line. (DIA-001)
2. A scenario reads the `Debug.DrawRay` items a script drew during a frame,
   headless, with their world coordinates and originating object. (DIA-001)
3. An external IDE opening the generated solution reports the same compile
   error, at the same file, line, and column, that unitas reports. (DIA-002)
4. A DAP client attached to a development player stops at a conditional
   breakpoint inside a coroutine, shows the coroutine frame at its source line,
   and displays a destroyed referenced object as destroyed. (DIA-002)
5. An agent sets a breakpoint through MCP in an isolated headless play session,
   receives the stop notification, reads locals, and continues. (DIA-002)
6. A profile capture from a development player shows a `ProfilerMarker` sample
   by name and records the build, device, and quality settings. (PRF-001)
7. A capture from the editor and one from a player are shown with their
   context differences rather than as directly equivalent. (PRF-001)
8. A release publishes budgets and measurements for every listed metric on its
   named reference projects, and a project with 100,000 assets opens and accepts
   paginated hierarchy queries and property edits through MCP within those
   budgets. (PRF-002)
9. A measured regression beyond a budget prevents the affected family from
   being claimed generally available on that target. (PRF-002)
10. A tester report identifies the exact build, device, location, input context,
    evidence, and observed failure. (TST-001)
11. A recorded scenario distinguishes setup failure, failed expectation,
    unevaluable expectation, and full pass. (TST-002)
12. A replay identifies its first divergence and does not overclaim determinism.
    (TST-002)
13. Visual, audio, and performance failures link back to relevant project state.
    (TST-002)
14. Coverage communicates actual executed scenarios and tests, not a marketing
    percentage. (TST-002)
15. A project's EditMode and PlayMode tests, including a `[UnityTest]` and a
    `[TestCase]`-parametrized test, run headless filtered by category and produce
    an NUnit 3 XML result whose outcomes match the same run in the editor.
    (TST-003)
16. A test that logs an unexpected error fails, and the same test passes when it
    declares the log with `LogAssert.Expect`. (TST-003)
17. A cancelled test run reports which tests completed and does not report the
    remainder as passed. (TST-003)
18. A native crash in a release Android player produces a report with its build
    identifier, device, and stacks, which an agent symbolicates through MCP
    with that build's symbols. (DIA-003)
19. A browser player's unhandled exception is mapped to source locations through
    the build's source maps, and a report with no matching symbols states that.
    (DIA-003)
20. A player built with no configured report destination sends no crash report
    off the device. (DIA-003)
21. Logs streamed from a connected iOS development player and from a copied
    Windows `Player.log` both appear as structured entries filterable by
    severity and time. (DIA-003)
22. The published budgets include hierarchy and asset queries, search,
    selection, property edits, undo, and save on the scale-floor reference
    projects. (PRF-002)
23. A DAP client on another machine cannot connect while network connections are
    disabled, and an agent session holding only the read-project scope can set
    breakpoints and read values but receives a permission outcome when it
    evaluates an expression. (DIA-002)
24. A scenario file round-trips through its published schema, an agent session's
    edit to it appears as a change-set item, and a visual reference it adds is
    not used for comparison until the item is accepted. (TST-002)
25. A visual comparison of a translated Unity material passes against a
    unitas-approved capture without reference to any Unity screenshot.
    (TST-002)
