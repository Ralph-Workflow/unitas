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

## How to Read the Specification

Each feature document answers the same questions:

1. Who uses the feature and what outcome do they receive?
2. What concepts and states are visible to that user?
3. How does the feature behave during ordinary use?
4. How does it behave at lifecycle boundaries and in edge cases?
5. What does failure look like, and what information remains available?
6. What Unity-compatible behavior is promised?
7. What observable stories establish that the feature is complete?

“Compatible” always refers to observable behavior within the declared Unity
2020.3 LTS through 2022.3 LTS scope. It never means only that a file parses, a
type exists, a script compiles, or a scene opens.

## Specification Map

| Area | Detailed specification | Principal outcomes |
|---|---|---|
| Product compatibility | [Compatibility contract](compatibility-contract.md) | Meaning, limits, evidence, and reporting of Unity compatibility |
| Complete feature inventory | [Feature catalog](feature-catalog.md) | Traceable inventory of every declared product feature family |
| Projects and assets | [Projects and assets](project-and-assets.md) | Project opening, asset identity, import, dependency, search, and content workflows |
| Editor experience | [Editor and authoring](editor-and-authoring.md) | Hierarchy, Inspector, scene view, play mode, undo, workspace, and collaboration |
| Serialized content | [Serialization and prefabs](serialization-and-prefabs.md) | Serialized values, references, scenes, prefabs, overrides, and data preservation |
| Rendering and animation | [Rendering and animation](rendering-and-animation.md) | Cameras, meshes, materials, shaders, lighting, particles, animation, and cinematics |
| UI, audio, and input | [UI, audio, and input](ui-audio-input.md) | Runtime UI, text, event routing, device input, rebinding, audio, and media |
| C# and runtime | [Scripting runtime](scripting-runtime.md) | Unity object model, C# behavior, lifecycle, coroutines, time, scenes, and resources |
| Simulation and worlds | [Physics, navigation, and terrain](physics-navigation-terrain.md) | 2D/3D physics, queries, events, characters, navigation, terrain, and world behavior |
| Players and targets | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) | Build configuration, player behavior, target coverage, platform services, XR, and online play |
| Quality and verification | [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md) | Debugging, profiling, scenarios, replay, comparison, coverage, and tester feedback |
| Inclusive and durable games | [Accessibility, localization, and persistence](accessibility-localization-persistence.md) | Accessible editor and player features, locale behavior, saves, settings, and data evolution |
| Demonstrable outcomes | [Playable samples and benchmarks](sample-games.md) | Complete games and behavioral benchmarks proving declared feature combinations |
| Agents and automation | [Automation and MCP](automation-and-mcp.md) | Structured feature parity, inspection, mutation, conflict handling, and automated evidence |

## Coverage Rules

Every declared product feature appears in at least one row above. Every detailed
document links its acceptance outcomes to the feature catalog.
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
