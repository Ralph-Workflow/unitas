# Feature Catalog

## Purpose

This catalog gives every promised feature family a stable identifier, a tier,
and one owning specification. It is an index for navigation and acceptance
reporting. It is not a work list, a milestone plan, or a priority order: rows
are grouped by area, not ranked, and a row's short label is not the feature's
scope. The scope is everything the owning specification states. The
[Engine architecture](architecture.md) has no identifier because it binds every
row.

Each feature has exactly one owning specification and exactly one compatibility
tier (Exact, Semantic, Familiar, or Native), defined in the
[Compatibility contract](compatibility-contract.md). Other documents reference
the owner instead of restating its behavior. Every acceptance outcome in a
detailed specification names the identifiers it demonstrates.

## Project and Content

| ID | Feature | Tier | Owning specification |
|---|---|---|---|
| PRJ-001 | Open and recognize existing Unity projects | Exact | [Projects and assets](project-and-assets.md) |
| PRJ-002 | Browse, search, label, preview, move, and delete assets | Native | [Projects and assets](project-and-assets.md) |
| PRJ-003 | Preserve GUID, local identifier, and sub-asset references | Exact | [Serialization and prefabs](serialization-and-prefabs.md) |
| PRJ-004 | Import pipeline, import state, and source-asset families | Semantic | [Projects and assets](project-and-assets.md) |
| PRJ-005 | Inspect dependencies and reverse dependencies | Exact | [Projects and assets](project-and-assets.md) |
| PRJ-006 | Resolve package manifests, registries, and project extensions | Semantic | [Projects and assets](project-and-assets.md) |
| PRJ-007 | Represent project settings and target-specific effects | Exact | [Projects and assets](project-and-assets.md) |
| PRJ-008 | Project trust and safe opening of untrusted projects | Native | [Projects and assets](project-and-assets.md) |
| SER-001 | Read, preserve, edit, and save supported serialized values | Exact | [Serialization and prefabs](serialization-and-prefabs.md) |
| SER-002 | Author and load scenes, including additive scene sets | Exact | [Serialization and prefabs](serialization-and-prefabs.md) |
| SER-003 | Author prefab sources, instances, nesting, variants, and overrides | Exact | [Serialization and prefabs](serialization-and-prefabs.md) |
| SER-004 | Preserve missing, unknown, cyclic, and cross-scene references accurately | Exact | [Serialization and prefabs](serialization-and-prefabs.md) |
| SER-005 | Text, binary, and mixed serialization forms and generated caches | Exact | [Serialization and prefabs](serialization-and-prefabs.md) |

## Editor and Authoring

| ID | Feature | Tier | Owning specification |
|---|---|---|---|
| EDT-001 | Hierarchy, selection, parenting, activation, tags, and layers | Native | [Editor and authoring](editor-and-authoring.md) |
| EDT-002 | Inspector and serialized property editing | Native | [Editor and authoring](editor-and-authoring.md) |
| EDT-003 | 2D and 3D scene navigation, handles, snapping, and visualization | Native | [Editor and authoring](editor-and-authoring.md) |
| EDT-004 | Game view, play, pause, step, and edit-state restoration | Native | [Editor and authoring](editor-and-authoring.md) |
| EDT-005 | Undo, redo, unsaved-state handling, and recovery | Native | [Editor and authoring](editor-and-authoring.md) |
| EDT-006 | Workspace views, commands, shortcuts, drag and drop, and multi-editing | Native | [Editor and authoring](editor-and-authoring.md) |
| EDT-007 | External-change, source-control, and semantic merge workflows | Native | [Editor and authoring](editor-and-authoring.md) |
| EDT-008 | Review, approval, and reversal of agent change sets | Native | [Editor and authoring](editor-and-authoring.md) |

## Runtime and Simulation

| ID | Feature | Tier | Owning specification |
|---|---|---|---|
| RUN-001 | GameObject, Component, Transform, identity, and lifetime | Semantic | [Scripting runtime](scripting-runtime.md) |
| RUN-002 | C# scripting profile, UnityEngine-compatible APIs, and Visual Scripting | Semantic | [Scripting runtime](scripting-runtime.md) |
| RUN-003 | MonoBehaviour lifecycle, messages, attributes, and activation behavior | Semantic | [Scripting runtime](scripting-runtime.md) |
| RUN-004 | Coroutines, asynchronous operations, threading, and time | Semantic | [Scripting runtime](scripting-runtime.md) |
| RUN-005 | Runtime scene management and persistent objects | Semantic | [Scripting runtime](scripting-runtime.md) |
| RUN-006 | Resource loading, residency, release, and content catalogs | Semantic | [Scripting runtime](scripting-runtime.md) |
| RUN-007 | Script compilation, assemblies, defines, and plugins | Semantic | [Scripting runtime](scripting-runtime.md) |
| RUN-008 | Serialized events, messages, and delayed invocation | Semantic | [Scripting runtime](scripting-runtime.md) |
| RUN-009 | Web requests, sockets, file locations, JSON utilities, and randomness | Semantic | [Scripting runtime](scripting-runtime.md) |
| RUN-010 | Editor-time scripts and content hooks | Semantic | [Scripting runtime](scripting-runtime.md) |
| RUN-011 | Jobs, Burst-compiled code, native collections, and Entities | Semantic | [Scripting runtime](scripting-runtime.md) |
| PHY-001 | 3D bodies, colliders, joints, queries, and events | Semantic | [Physics, navigation, and terrain](physics-navigation-terrain.md) |
| PHY-002 | 2D bodies, colliders, effectors, joints, queries, and events | Semantic | [Physics, navigation, and terrain](physics-navigation-terrain.md) |
| PHY-003 | Character movement and grounding | Semantic | [Physics, navigation, and terrain](physics-navigation-terrain.md) |
| PHY-004 | Wheels, articulations, cloth, and simulation control | Familiar | [Physics, navigation, and terrain](physics-navigation-terrain.md) |
| NAV-001 | Walkable worlds, navigation data building, paths, agents, obstacles, and links | Familiar | [Physics, navigation, and terrain](physics-navigation-terrain.md) |
| ENV-001 | Terrain, vegetation, large worlds, and environment behavior | Familiar | [Physics, navigation, and terrain](physics-navigation-terrain.md) |

## Presentation and Interaction

| ID | Feature | Tier | Owning specification |
|---|---|---|---|
| REN-001 | Cameras, viewports, displays, and render targets | Native | [Rendering and animation](rendering-and-animation.md) |
| REN-002 | Meshes, skinning, blend shapes, materials, textures, and shaders | Native | [Rendering and animation](rendering-and-animation.md) |
| REN-003 | Lighting, shadows, baked data, environment, post-processing, and quality behavior | Native | [Rendering and animation](rendering-and-animation.md) |
| REN-004 | Particles, trails, and visual effects | Familiar | [Rendering and animation](rendering-and-animation.md) |
| REN-005 | Sprites, atlases, sorting, masks, and tilemaps | Native | [Rendering and animation](rendering-and-animation.md) |
| REN-006 | Render pipelines, shader graphs, and scripted rendering | Native | [Rendering and animation](rendering-and-animation.md) |
| REN-007 | Frame presentation, frame rate, and display output | Native | [Rendering and animation](rendering-and-animation.md) |
| ANI-001 | Clips, curves, events, controllers, states, and blend trees | Familiar | [Rendering and animation](rendering-and-animation.md) |
| ANI-002 | Avatars, humanoid retargeting, inverse kinematics, and playable graphs | Familiar | [Rendering and animation](rendering-and-animation.md) |
| CIN-001 | Camera behavior and time-based cinematic sequencing | Familiar | [Rendering and animation](rendering-and-animation.md) |
| UI-001 | Canvas, layout, controls, events, focus, and navigation | Familiar | [UI, audio, and input](ui-audio-input.md) |
| UI-002 | Immediate-mode runtime GUI | Familiar | [UI, audio, and input](ui-audio-input.md) |
| TXT-001 | Fonts, Unicode text, fallback, and rich presentation | Familiar | [UI, audio, and input](ui-audio-input.md) |
| INP-001 | Legacy input behavior | Familiar | [UI, audio, and input](ui-audio-input.md) |
| INP-002 | Action-based input, devices, players, and rebinding | Familiar | [UI, audio, and input](ui-audio-input.md) |
| AUD-001 | Clips, sources, listeners, spatial audio, filters, and mixing | Familiar | [UI, audio, and input](ui-audio-input.md) |
| MED-001 | Video playback and synchronized media state | Familiar | [UI, audio, and input](ui-audio-input.md) |

## Distribution, Services, and Inclusion

| ID | Feature | Tier | Owning specification |
|---|---|---|---|
| BLD-001 | Configure, validate, create, and report player builds | Native | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) |
| BLD-002 | Headless command-line operation and continuous integration | Native | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) |
| BLD-003 | Scripting backends, ahead-of-time compilation, and code stripping | Native | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) |
| BLD-004 | Deployment, device connection, and development-player channels | Native | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) |
| BLD-005 | Content builds and post-release content updates | Native | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) |
| PLT-001 | Desktop, mobile, browser, handheld, and console behavior | Native | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) |
| XR-001 | Tracking, views, interaction, haptics, focus, and comfort | Familiar | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) |
| NET-001 | Sessions, connections, ownership, synchronization, and failure | Familiar | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) |
| LIC-001 | Dual licensing of the product and license obligations of shipped players | Native | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) |
| PRD-001 | Installation, updates, and target toolchain acquisition | Native | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) |
| PRD-002 | Product data flows, telemetry, and privacy | Native | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) |
| LOC-001 | Localized strings, assets, locale changes, and layout | Familiar | [Accessibility, localization, and persistence](accessibility-localization-persistence.md) |
| ACC-001 | Accessible editor and game experiences | Native | [Accessibility, localization, and persistence](accessibility-localization-persistence.md) |
| SAV-001 | Player settings, save data, corruption, and migration | Semantic | [Accessibility, localization, and persistence](accessibility-localization-persistence.md) |

## Evidence, Samples, and Automation

| ID | Feature | Tier | Owning specification |
|---|---|---|---|
| DIA-001 | Logs, exceptions, diagnostics, and source navigation | Native | [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md) |
| DIA-002 | Script debugging and code intelligence through external tools and agents | Native | [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md) |
| DIA-003 | Player crash reports, symbols, and device logs | Native | [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md) |
| PRF-001 | CPU, frame, rendering, memory, physics, audio, and target profiling | Native | [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md) |
| PRF-002 | Engine performance and scale commitments | Native | [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md) |
| TST-001 | Manual playtest sessions and tester feedback | Native | [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md) |
| TST-002 | Structured scenarios, input replay, comparison, and coverage | Native | [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md) |
| TST-003 | Project unit and integration tests | Semantic | [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md) |
| SMP-001 | Complete playable sample suite | Native | [Playable samples and benchmarks](sample-games/INDEX.md) |
| CMP-001 | Behavioral compatibility benchmark suite | Native | [Playable samples and benchmarks](sample-games/INDEX.md) |
| CMP-002 | Supported package and version matrix | Native | [Compatibility contract](compatibility-contract.md) |
| CMP-003 | Third-party and asset-store content | Native | [Compatibility contract](compatibility-contract.md) |
| CMP-004 | Product versioning, release readiness, and interface stability | Native | [Compatibility contract](compatibility-contract.md) |
| CMP-005 | Support lifecycle, host requirements, and store requirements | Native | [Compatibility contract](compatibility-contract.md) |
| AUT-001 | Structured feature parity for authorized agents | Native | [Automation and MCP](automation-and-mcp.md) |
| AUT-002 | MCP discovery, inspection, editing, play control, and evidence | Native | [Automation and MCP](automation-and-mcp.md) |
| AUT-003 | Human-agent shared context and conflict behavior | Native | [Automation and MCP](automation-and-mcp.md) |
| AUT-004 | Headless and concurrent agent sessions | Native | [Automation and MCP](automation-and-mcp.md) |
| AUT-005 | Agent code workflow: author, compile, diagnose, test, and fix | Native | [Automation and MCP](automation-and-mcp.md) |
| AUT-006 | Agent-accessible product knowledge and documentation | Native | [Automation and MCP](automation-and-mcp.md) |
| AUT-007 | Agent permission scopes, grants, and confirmations | Native | [Automation and MCP](automation-and-mcp.md) |
| AUT-008 | Agent audit log and provenance | Native | [Automation and MCP](automation-and-mcp.md) |

## Engine Extensions

Capabilities beyond Unity 2022.3 LTS, each off by default and bound by the
compatibility rules of [Engine extensions](engine-extensions.md).

| ID | Feature | Tier | Owning specification |
|---|---|---|---|
| EXT-001 | Virtualized geometry | Native | [Engine extensions](engine-extensions.md) |
| EXT-002 | Dynamic global illumination and reflections | Native | [Engine extensions](engine-extensions.md) |
| EXT-003 | Virtual shadow maps | Native | [Engine extensions](engine-extensions.md) |
| EXT-004 | Temporal upscaling | Native | [Engine extensions](engine-extensions.md) |
| EXT-005 | World streaming | Native | [Engine extensions](engine-extensions.md) |
| EXT-006 | Shared content cache | Native | [Engine extensions](engine-extensions.md) |
| EXT-007 | Live code reload | Native | [Engine extensions](engine-extensions.md) |
| EXT-008 | Multi-instance play | Native | [Engine extensions](engine-extensions.md) |
| EXT-009 | Collaborative editing | Native | [Engine extensions](engine-extensions.md) |
| EXT-010 | Unified tracing | Native | [Engine extensions](engine-extensions.md) |
