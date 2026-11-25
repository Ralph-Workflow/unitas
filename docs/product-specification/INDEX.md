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
states, edge cases, failures, and acceptance stories. They do not prescribe
architecture, source-code organization, dependencies, algorithms, toolchains,
delivery phases, or implementation tasks.

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

* **Exact:** content identity and serialization (scenes, prefabs and
  overrides, GUIDs and local identifiers, materials, project settings, and
  round-trip saving) match Unity 2022.3 LTS exactly.
* **Semantic:** the core scripting model, physics bodies, callbacks, and
  queries, scene loading, and resource loading behave as Unity 2022.3 LTS does
  in ordinary use; bug-for-bug parity is not promised.
* **Familiar:** UI, text, animation, audio, video, input, particles,
  navigation, terrain, localization, XR, and networking import Unity content
  and keep Unity's API shape, while unitas defines the behavior and reports
  every difference.
* **Native:** rendering, the editor, builds and platforms, diagnostics, and
  the agent and MCP layer are unitas-defined. Unity shaders and materials are
  translated; pixel identity with Unity is not promised.

Behavior outside a feature's tier is reported with a compatibility state and is
never silently approximated.

### Packages

Behavior provided by Unity packages is in scope only for the package families
and version lines that the supported package matrix lists (CMP-002, owned by
the [Compatibility contract](compatibility-contract.md)). A package that the
matrix does not list is reported as Unknown until it is assessed; it is never
assumed to be supported. The matrix grows until it covers every package that
Unity publishes as released or verified for Unity 2022.3 LTS.

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

Each feature document answers the same questions:

1. Who uses the feature and what outcome do they receive?
2. What concepts and states are visible to that user?
3. How does the feature behave during ordinary use?
4. How does it behave at lifecycle boundaries and in edge cases?
5. What does failure look like, and what information remains available?
6. What Unity-compatible behavior is promised?
7. What observable stories establish that the feature is complete?

“Compatible” always refers to the observable behavior that a feature's
compatibility tier promises against the Unity 2022.3 LTS baseline. It never
means only that a file parses, a type exists, a script compiles, or a scene
opens.

## Specification Map

| Area | Detailed specification | Principal outcomes |
|---|---|---|
| Agents and automation | [Automation and MCP](automation-and-mcp.md) | The primary product interface: structured parity, MCP discovery, inspection, mutation, headless sessions, agent code workflow, product knowledge, conflict handling, and evidence |
| Product compatibility | [Compatibility contract](compatibility-contract.md) | Meaning, limits, evidence, and reporting of Unity compatibility; supported package matrix; third-party content; versioning and release readiness |
| Complete feature inventory | [Feature catalog](feature-catalog.md) | Traceable inventory of every declared product feature family and its single owner |
| Projects and assets | [Projects and assets](project-and-assets.md) | Project opening and trust, asset identity, import, dependencies, packages, project settings, and content workflows |
| Editor experience | [Editor and authoring](editor-and-authoring.md) | Focused visual authoring, inspection, play mode, undo, review of agent changes, and source-control merge |
| Serialized content | [Serialization and prefabs](serialization-and-prefabs.md) | Serialized values, references, scenes, prefabs, overrides, serialization forms, and data preservation |
| Rendering and animation | [Rendering and animation](rendering-and-animation.md) | Cameras, meshes, materials, shaders, render pipelines, 2D rendering, lighting, particles, frame presentation, animation, and cinematics |
| UI, audio, and input | [UI, audio, and input](ui-audio-input.md) | Runtime UI, immediate-mode GUI, text, event routing, device input, rebinding, audio, and video |
| C# and runtime | [Scripting runtime](scripting-runtime.md) | Compilation and assemblies, Unity object model, lifecycle, messaging, coroutines, threading, time, scenes, resources, editor-time scripts, and jobs |
| Simulation and worlds | [Physics, navigation, and terrain](physics-navigation-terrain.md) | 2D/3D physics, queries, events, characters, vehicles, navigation, terrain, and world behavior |
| Players and targets | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) | Build configuration, headless operation, scripting backends, player behavior, target coverage, licensing, installation, platform services, XR, and online play |
| Quality and verification | [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md) | Diagnostics, external debugging, profiling, engine performance commitments, scenarios, project tests, replay, comparison, coverage, and tester feedback |
| Inclusive and durable games | [Accessibility, localization, and persistence](accessibility-localization-persistence.md) | Accessible editor and player features, localization, saves, settings, and data evolution |
| Demonstrable outcomes | [Playable samples and benchmarks](sample-games/INDEX.md) | Complete games, agent authoring benchmarks, and behavioral benchmarks proving declared feature combinations |

## Coverage Rules

Every declared product feature appears in at least one row above and has exactly
one identifier in the [Feature catalog](feature-catalog.md). Every acceptance
outcome in a detailed document names, in parentheses, the catalog identifiers it
demonstrates, and every catalog identifier appears in at least one acceptance
outcome of its owning document.
A feature that spans documents has one owning specification and cross-references
from related documents rather than competing definitions.

The catalog uses stable feature identifiers. These identifiers describe product
requirements, not implementation work items. A feature remains incomplete when
its observable acceptance outcomes are unmet, even if internal components with
matching names exist.

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
limitations, identify its failure states, and find at least one acceptance story
or benchmark that can demonstrate the claim without relying on private
implementation knowledge.
