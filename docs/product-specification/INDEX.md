# unitas Product Specification Index

## Purpose

This directory is the detailed product specification for unitas. It defines
what the completed product does and how each feature behaves from the
perspective of developers, players, testers, and authorized automation.

Every present-tense feature statement expresses a required end state. It does
not claim that the feature is implemented, available in the current repository,
or released. Current implementation status belongs in verifiable project
status, not in this product specification.

The specifications describe observable outcomes, compatibility contracts,
states, edge cases, failures, and acceptance outcomes. The
[Engine architecture](architecture.md) and the Architecture section of each
detailed specification define how the engine is built: its layers, where state
lives, how time and work flow, the boundaries between subsystems, the
dependencies it builds on, and the performance principles every subsystem
follows. Architecture is binding: a capability whose observable behavior is
correct but whose structure violates it is incomplete. No part of the
specification prescribes source-file layout, delivery phases, schedules, or
implementation tasks.

This index defines the product-wide vision and boundaries. The documents below
expand that contract. When a summary in this index is less specific than a
linked feature specification, the detailed specification controls for that
feature. A detailed specification cannot expand the declared product boundaries
without a corresponding update to this index.

## Product Vision

unitas is an open-source game engine, runtime, and build product that is AI-
and MCP-native and C#-first. Existing Unity projects are its on-ramp: a user
opens a supported Unity 2022.3 LTS project in place, and then an authorized
agent, a developer, or both together inspect it, change it, run it, test it,
debug it, and build it without first converting it into a unitas-specific
project. Unity compatibility is the way projects arrive; it is not the
definition of the product. Each feature's compatibility promise is bounded by
its tier, and outside the Exact and Semantic tiers unitas defines its own
behavior.

unitas is agent-first:

* An agent works on a project the way a developer does: through the editor,
  the unitas IDE. The editor's Model Context Protocol (MCP) interface exposes
  every operation a developer can perform in the editor, so every product
  capability is available to authorized agents through MCP. A capability
  that a person can use but an agent cannot is incomplete. The person-only
  actions enumerated by AUT-007 (trust, grants, approvals, confirmations, and
  similar consent decisions) are the only exceptions; agents can discover and
  request them. The detailed contract is owned by
  [Automation and MCP](automation-and-mcp.md).
* The visual editor is the editor with its window. It presents and edits
  the same project model, through the same operations, that agents use
  through MCP. Its purpose is to make project and runtime state clear, to support
  direct authoring of scenes and content, and to let a developer review,
  approve, and reverse agent changes. It is a complete,
  commercial-quality authoring environment for every content type the engine
  supports.
* Script source is edited and debugged in the developer's own code editor,
  IDE, or agent. unitas provides the compilation, diagnostics, project files,
  and debugging connection that those tools need; it does not provide a
  built-in code editor.
* Every capability that runs inside the visual editor also runs in the
  headless editor, so agents and continuous integration can use the product
  without a display.
* MCP belongs to the editor only. The engine, the runtime, games made with
  unitas, and every packaged player contain no MCP interface. An agent reaches
  a running game only through an editor play session, or through the editor's
  connection to a development player.

## Product Boundaries

### Compatibility baseline

The compatibility baseline is the documented, observable behavior of Unity
2022.3 LTS, a finished release line. Projects saved by Unity 2020.3 LTS,
2021.3 LTS, or any other earlier release that Unity 2022.3 LTS can upgrade, open
by migrating their serialized forms to the 2022.3 baseline;
behavior that differs between those releases and 2022.3 is not matched
separately. Unity releases after 2022.3 LTS are outside the baseline; a
later-release feature becomes part of unitas only when a specification adopts
it explicitly. The [Compatibility contract](compatibility-contract.md) defines
what compatibility means and how it is reported.

### Compatibility tiers

Every catalog feature has exactly one tier, recorded in the
[Feature catalog](feature-catalog.md) and defined in the
[Compatibility contract](compatibility-contract.md):

* **Exact:** the feature matches Unity 2022.3 LTS exactly. Content identity
  and serialization are in this tier.
* **Semantic:** the feature behaves as Unity 2022.3 LTS does in ordinary use;
  bug-for-bug parity is not promised. The core scripting model is in this tier.
* **Familiar:** the feature imports Unity content and keeps Unity's API shape,
  while unitas defines the behavior and reports every difference.
* **Native:** the feature is unitas-defined. Rendering, the editor, and the
  agent and MCP layer are in this tier; Unity shaders and materials are
  translated, and pixel identity with Unity is not promised.

The catalog, not this summary, records the tier of each feature.

Behavior outside a feature's tier is reported with a compatibility state and is
never silently approximated.

### Packages

Behavior provided by Unity packages comes from unitas's own implementation of
each package's public API, never from the code Unity publishes for it, as
[Independent implementation](architecture.md#independent-implementation)
defines. It is in scope only for the package families and version lines that the
supported package matrix lists (CMP-002, owned by the [Compatibility
contract](compatibility-contract.md)). A package that the matrix does not list
is reported as Unknown until it is assessed; it is never assumed to be
supported. The matrix grows until it covers every package that Unity publishes
as released or verified for Unity 2022.3 LTS.

### Full Unity 2022.3 LTS content coverage

unitas supports every content family, package line in the supported package
matrix, and built-in feature that a Unity 2022.3 LTS game uses. That includes
the High Definition Render Pipeline, custom scriptable render pipelines,
Entities and Entities Graphics, Netcode for Entities, Visual Effect Graph, UI
Toolkit runtime panels, Visual Scripting, Enlighten realtime global
illumination, AssetBundles and Addressables content built by Unity, baking of
lightmaps, probes, and occlusion data inside unitas, and the drawing of custom
editor extensions. Each is owned by the specification that the
[Compatibility contract](compatibility-contract.md) names for it.

No content family is out of scope because of engine effort. Content is
reported with a state other than Supported only when it is:

* malformed, corrupt, missing, or ambiguous
* from a package line that is not yet assessed
* dependent on hardware, an entitlement, or an SDK that is absent

A gap found anywhere else is a defect, to be fixed until the content is
Supported.

### Licensing

unitas is dual licensed: it is available under the GNU General Public License
version 3 and under a separate commercial license. The obligations that each
license places on built players, and how the product reports them, are owned by
LIC-001 in [Builds, platforms, XR, and networking](build-platforms-xr-networking.md).

## How to Read the Specification

### Reading order

1. This index: the product vision and boundaries.
2. [Engine architecture](architecture.md): how the whole engine is built. Every
   detailed specification applies it.
3. [Compatibility contract](compatibility-contract.md): the baseline, the tiers,
   the compatibility states, and what counts as evidence. Every behavior
   statement is read through the tier of the feature that owns it.
4. [Automation and MCP](automation-and-mcp.md): every capability is reachable by
   agents through the editor, and a capability that agents cannot reach is
   incomplete.
5. The detailed specification of the area at hand, in full, and then the Scope
   and Architecture sections of every neighbouring specification it names.
6. The [Feature catalog](feature-catalog.md) and
   [Playable samples and benchmarks](sample-games/INDEX.md), as indexes into the
   above.

### The catalog is an index

A catalog identifier names a feature family and points to the one
specification that owns it. It is a navigation and traceability label, not a
work item, a milestone, or a unit of priority. Catalog rows are not ordered by
importance, and the short label in a row is not the scope of the feature: the
scope is everything the owning specification states for it.

### Samples and benchmarks are evidence

The playable samples, development workflows, and benchmarks show that declared
features work together in real projects. They are not an agenda, a priority
order, or the boundary of what the engine supports. No sample is more
important than another, and a behavior that no sample exercises is required
with the same weight as one that every sample exercises. The engine never
recognizes or special-cases a sample; a sample passes because the engine
implements its owning specifications.

### Where architecture lives

[Engine architecture](architecture.md) holds the rules for the whole engine.
Each detailed specification has an Architecture section, directly after its
compatibility tiers, that applies those rules to its domain: its components,
where its data lives and flows, its threading and timing, its boundaries with
neighbouring subsystems, and its domain-specific performance principles. An
Architecture section adds detail and never relaxes an engine-wide rule; when
they disagree, the Engine architecture holds. Behavior sections say what the
product does, Architecture sections say how it is built, and neither restates
the other.

### What each detailed specification answers

1. Who uses the feature and what outcome do they receive?
2. What concepts and states are visible to that user?
3. How is the domain built, and where are its boundaries with neighbouring
   domains?
4. How does the feature behave during ordinary use?
5. How does it behave at lifecycle boundaries and in edge cases?
6. What does failure look like, and what information remains available?
7. What Unity-compatible behavior is promised, at which tier?
8. What observable outcomes demonstrate the feature?

“Compatible” always refers to the observable behavior that a feature's
compatibility tier promises against the Unity 2022.3 LTS baseline. It never
means only that a file parses, a type exists, a script compiles, or a scene
opens.

## Specification Map

| Area | Detailed specification | Principal outcomes |
|---|---|---|
| Engine structure | [Engine architecture](architecture.md) | Layers, the native world, frame loop and time, concurrency, memory, the managed boundary, content pipeline, presentation, engine instances, performance principles, robustness, and dependencies |
| Agents and automation | [Automation and MCP](automation-and-mcp.md) | The primary product interface: structured parity, MCP discovery, inspection, mutation, headless sessions, agent code workflow, product knowledge, conflict handling, and evidence |
| Product compatibility | [Compatibility contract](compatibility-contract.md) | Meaning, limits, evidence, and reporting of Unity compatibility; supported package matrix; third-party content; versioning and release readiness |
| Complete feature inventory | [Feature catalog](feature-catalog.md) | Traceable inventory of every declared product feature family and its single owner |
| Projects and assets | [Projects and assets](project-and-assets.md) | Project opening and trust, asset identity, import, dependencies, packages, project settings, and content workflows |
| Editor experience | [Editor and authoring](editor-and-authoring.md) | Focused visual authoring, inspection, play mode, undo, review of agent changes, and source-control merge |
| Editor UI and UX | [Editor UI and UX](editor/INDEX.md) | Window-by-window detail of the visual editor: workspace and layouts, menus and toolbar, shortcuts, Scene view, Hierarchy, Inspector, Project window, Console, Game view and play mode, prefab workflow, animation, 2D, UI authoring, lighting and rendering tools, profiling and debugging windows, settings, and the agent-native additions |
| Serialized content | [Serialization and prefabs](serialization-and-prefabs.md) | Serialized values, references, scenes, prefabs, overrides, serialization forms, and data preservation |
| Rendering and animation | [Rendering and animation](rendering-and-animation.md) | Cameras, meshes, materials, shaders, render pipelines, 2D rendering, lighting, particles, frame presentation, animation, and cinematics |
| UI, audio, and input | [UI, audio, and input](ui-audio-input.md) | Runtime UI, immediate-mode GUI, text, event routing, device input, rebinding, audio, and video |
| C# and runtime | [Scripting runtime](scripting-runtime.md) | Compilation and assemblies, Unity object model, lifecycle, messaging, coroutines, threading, time, scenes, resources, editor-time scripts, and jobs |
| Simulation and worlds | [Physics, navigation, and terrain](physics-navigation-terrain.md) | 2D/3D physics, queries, events, characters, vehicles, navigation, terrain, and world behavior |
| Players and targets | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) | Build configuration, headless operation, scripting backends, player behavior, target coverage, licensing, installation, platform services, XR, and online play |
| Browser delivery | [Web target](web-target.md) | Browser player, loader and page API, WebAssembly memory and threads, graphics on WebGPU and WebGL 2, hosting, storage, browser input and media, web performance budgets, and browser testing |
| Quality and verification | [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md) | Diagnostics, external debugging, profiling, engine performance commitments, scenarios, project tests, replay, comparison, coverage, and tester feedback |
| Inclusive and durable games | [Accessibility, localization, and persistence](accessibility-localization-persistence.md) | Accessible editor and player features, localization, saves, settings, and data evolution |
| Demonstrable outcomes | [Playable samples and benchmarks](sample-games/INDEX.md) | Complete games, agent authoring benchmarks, and behavioral benchmarks proving declared feature combinations |

### Start here

Each row names where to begin for an area of the engine and which documents
share its boundaries. Read the Architecture section of every document in the
row before changing behavior that crosses a boundary.

| Working on | Read first | Shares boundaries with |
|---|---|---|
| Core world, frame loop, jobs, memory | [Engine architecture](architecture.md), [Scripting runtime](scripting-runtime.md) | every subsystem |
| C# and the Unity API | [Scripting runtime](scripting-runtime.md) | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md), [Serialization and prefabs](serialization-and-prefabs.md) |
| Assets, import, serialized content | [Projects and assets](project-and-assets.md), [Serialization and prefabs](serialization-and-prefabs.md) | [Rendering and animation](rendering-and-animation.md), [UI, audio, and input](ui-audio-input.md), [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) |
| Rendering, 2D, animation, particles | [Rendering and animation](rendering-and-animation.md) | [UI, audio, and input](ui-audio-input.md), [Physics, navigation, and terrain](physics-navigation-terrain.md), [Web target](web-target.md) |
| UI, text, audio, video, input | [UI, audio, and input](ui-audio-input.md) | [Accessibility, localization, and persistence](accessibility-localization-persistence.md), [Rendering and animation](rendering-and-animation.md) |
| Physics, navigation, terrain | [Physics, navigation, and terrain](physics-navigation-terrain.md) | [Scripting runtime](scripting-runtime.md), [Rendering and animation](rendering-and-animation.md) |
| The editor | [Editor and authoring](editor-and-authoring.md), [Editor UI and UX](editor/INDEX.md) | [Automation and MCP](automation-and-mcp.md) and the authoring requirements of every specification |
| Agents and MCP | [Automation and MCP](automation-and-mcp.md) | [Editor and authoring](editor-and-authoring.md) and every specification |
| Builds, players, targets, XR, networking | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md), [Web target](web-target.md) | [Scripting runtime](scripting-runtime.md), [Compatibility contract](compatibility-contract.md) |
| Diagnostics, performance, tests | [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md) | every subsystem |
| Compatibility reporting, packages | [Compatibility contract](compatibility-contract.md), [Feature catalog](feature-catalog.md) | every specification |

## Coverage Rules

Every declared product feature appears in at least one row above and has exactly
one identifier in the [Feature catalog](feature-catalog.md). Every acceptance
outcome in a detailed document names, in parentheses, the catalog identifiers it
demonstrates, and every catalog identifier appears in at least one acceptance
outcome of its owning document.
A feature that spans documents has one owning specification and cross-references
from related documents rather than competing definitions.

A detail document, such as [Web target](web-target.md) or a document in the
[Editor UI and UX](editor/INDEX.md) folder, owns no catalog identifier. Its
Scope names the features it details and the specification that owns them, and
its acceptance outcomes demonstrate those features. It adds detail to its
owning specification and never contradicts it.

The catalog uses stable feature identifiers. These identifiers describe product
requirements, not implementation work items. A feature is complete only when
every behavior its owning specification states holds on every declared target,
in the editor, headless, and in packaged players, and when it is built as the
[Engine architecture](architecture.md) and the owning Architecture section
require. Acceptance outcomes are representative observations of that behavior,
not its whole. Internal components with matching names never make a feature
complete.

## Accuracy Rules

* Unity names identify compatibility targets and do not imply sponsorship,
  certification, or ownership by Unity Technologies.
* A feature claim states the supported behavior and relevant limitations.
* Target-specific behavior is not generalized to all platforms.
* Proprietary SDK access, platform approval, external services, copyrighted
  sample content, and third-party entitlements are never implied by engine
  support.
* A sample or benchmark proves only the behaviors its scenarios observe.
* Unsupported, partially supported, platform-limited, and unknown behavior is
  distinguishable from supported behavior.
* Exact parity is not claimed for unspecified floating-point, timing, driver,
  operating-system, hardware, or external-service variation.
* Package-provided Unity features are scoped by compatible package and version;
  they are not implied to be built into every Unity editor version.
* Additional unitas targets and formats are product extensions, not claims that
  the same capability was a stock feature of every baseline Unity release.

## Cross-Cutting Acceptance

The specification corpus is complete when a reader can begin with this index,
locate every declared feature, determine its observable behavior and
limitations, identify its failure states, determine how it is built from the
Engine architecture and its owning Architecture section, and find at least one
acceptance outcome or benchmark that can demonstrate the claim without relying
on private implementation knowledge.
