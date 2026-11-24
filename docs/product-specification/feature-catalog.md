# Feature Catalog

## Purpose

This catalog gives every promised product family a stable identifier and an
owning detailed specification. Identifiers support navigation and acceptance
reporting; they are not implementation milestones.

Each feature has exactly one owning specification. Other documents reference
the owner instead of restating its behavior. Every acceptance outcome in a
detailed specification names the identifiers it demonstrates.

## Project and Content

| ID | Feature | Owning specification |
|---|---|---|
| PRJ-001 | Open and recognize existing Unity projects | [Projects and assets](project-and-assets.md) |
| PRJ-002 | Browse, search, label, preview, move, and delete assets | [Projects and assets](project-and-assets.md) |
| PRJ-003 | Preserve GUID, local identifier, and sub-asset references | [Serialization and prefabs](serialization-and-prefabs.md) |
| PRJ-004 | Import textures, models, animation, and audio | [Projects and assets](project-and-assets.md) |
| PRJ-005 | Inspect dependencies and reverse dependencies | [Projects and assets](project-and-assets.md) |
| PRJ-006 | Resolve package manifests, registries, and project extensions | [Projects and assets](project-and-assets.md) |
| PRJ-007 | Represent project settings and target-specific effects | [Projects and assets](project-and-assets.md) |
| PRJ-008 | Project trust and safe opening of untrusted projects | [Projects and assets](project-and-assets.md) |
| SER-001 | Read, preserve, edit, and save supported serialized values | [Serialization and prefabs](serialization-and-prefabs.md) |
| SER-002 | Author and load scenes, including additive scene sets | [Serialization and prefabs](serialization-and-prefabs.md) |
| SER-003 | Author prefab sources, instances, nesting, variants, and overrides | [Serialization and prefabs](serialization-and-prefabs.md) |
| SER-004 | Preserve missing, unknown, cyclic, and cross-scene references accurately | [Serialization and prefabs](serialization-and-prefabs.md) |
| SER-005 | Text, binary, and mixed serialization forms and generated caches | [Serialization and prefabs](serialization-and-prefabs.md) |

## Editor and Authoring

| ID | Feature | Owning specification |
|---|---|---|
| EDT-001 | Hierarchy, selection, parenting, activation, tags, and layers | [Editor and authoring](editor-and-authoring.md) |
| EDT-002 | Inspector and serialized property editing | [Editor and authoring](editor-and-authoring.md) |
| EDT-003 | 2D and 3D scene navigation, handles, snapping, and visualization | [Editor and authoring](editor-and-authoring.md) |
| EDT-004 | Game view, play, pause, step, and edit-state restoration | [Editor and authoring](editor-and-authoring.md) |
| EDT-005 | Undo, redo, unsaved-state handling, and recovery | [Editor and authoring](editor-and-authoring.md) |
| EDT-006 | Commands, shortcuts, drag and drop, and multi-editing | [Editor and authoring](editor-and-authoring.md) |
| EDT-007 | External-change, source-control, and semantic merge workflows | [Editor and authoring](editor-and-authoring.md) |
| EDT-008 | Review, approval, and reversal of agent change sets | [Editor and authoring](editor-and-authoring.md) |

## Runtime and Simulation

| ID | Feature | Owning specification |
|---|---|---|
| RUN-001 | GameObject, Component, Transform, identity, and lifetime | [Scripting runtime](scripting-runtime.md) |
| RUN-002 | C# scripting profile and UnityEngine-compatible APIs | [Scripting runtime](scripting-runtime.md) |
| RUN-003 | MonoBehaviour lifecycle, messages, attributes, and activation behavior | [Scripting runtime](scripting-runtime.md) |
| RUN-004 | Coroutines, asynchronous operations, threading, and time | [Scripting runtime](scripting-runtime.md) |
| RUN-005 | Runtime scene management and persistent objects | [Scripting runtime](scripting-runtime.md) |
| RUN-006 | Resource loading, residency, release, and content catalogs | [Scripting runtime](scripting-runtime.md) |
| RUN-007 | Script compilation, assemblies, defines, and plugins | [Scripting runtime](scripting-runtime.md) |
| RUN-008 | Serialized events, messages, and delayed invocation | [Scripting runtime](scripting-runtime.md) |
| RUN-009 | Web requests, file locations, JSON utilities, and randomness | [Scripting runtime](scripting-runtime.md) |
| RUN-010 | Editor-time scripts and content hooks | [Scripting runtime](scripting-runtime.md) |
| RUN-011 | Jobs, Burst-compiled code, and native collections | [Scripting runtime](scripting-runtime.md) |
| PHY-001 | 3D bodies, colliders, joints, queries, and events | [Physics, navigation, and terrain](physics-navigation-terrain.md) |
| PHY-002 | 2D bodies, colliders, effectors, joints, queries, and events | [Physics, navigation, and terrain](physics-navigation-terrain.md) |
| PHY-003 | Character movement and grounding | [Physics, navigation, and terrain](physics-navigation-terrain.md) |
| PHY-004 | Wheels, articulations, cloth, and simulation control | [Physics, navigation, and terrain](physics-navigation-terrain.md) |
| NAV-001 | Walkable worlds, navigation data building, paths, agents, obstacles, and links | [Physics, navigation, and terrain](physics-navigation-terrain.md) |
| ENV-001 | Terrain, vegetation, large worlds, and environment behavior | [Physics, navigation, and terrain](physics-navigation-terrain.md) |

## Presentation and Interaction

| ID | Feature | Owning specification |
|---|---|---|
| REN-001 | Cameras, viewports, displays, and render targets | [Rendering and animation](rendering-and-animation.md) |
| REN-002 | Meshes, skinning, blend shapes, materials, textures, and shaders | [Rendering and animation](rendering-and-animation.md) |
| REN-003 | Lighting, shadows, baked data, environment, post-processing, and quality behavior | [Rendering and animation](rendering-and-animation.md) |
| REN-004 | Particles, trails, and visual effects | [Rendering and animation](rendering-and-animation.md) |
| REN-005 | Sprites, atlases, sorting, masks, and tilemaps | [Rendering and animation](rendering-and-animation.md) |
| REN-006 | Render pipelines, shader graphs, and scripted rendering | [Rendering and animation](rendering-and-animation.md) |
| REN-007 | Frame presentation, frame rate, and display output | [Rendering and animation](rendering-and-animation.md) |
| ANI-001 | Clips, curves, events, controllers, states, and blend trees | [Rendering and animation](rendering-and-animation.md) |
| ANI-002 | Avatars, humanoid retargeting, inverse kinematics, and playable graphs | [Rendering and animation](rendering-and-animation.md) |
| CIN-001 | Camera behavior and time-based cinematic sequencing | [Rendering and animation](rendering-and-animation.md) |
| UI-001 | Canvas, layout, controls, events, focus, and navigation | [UI, audio, and input](ui-audio-input.md) |
| UI-002 | Immediate-mode runtime GUI | [UI, audio, and input](ui-audio-input.md) |
| TXT-001 | Fonts, Unicode text, fallback, and rich presentation | [UI, audio, and input](ui-audio-input.md) |
| INP-001 | Legacy input behavior | [UI, audio, and input](ui-audio-input.md) |
| INP-002 | Action-based input, devices, players, and rebinding | [UI, audio, and input](ui-audio-input.md) |
| AUD-001 | Clips, sources, listeners, spatial audio, filters, and mixing | [UI, audio, and input](ui-audio-input.md) |
| MED-001 | Video playback and synchronized media state | [UI, audio, and input](ui-audio-input.md) |

## Distribution, Services, and Inclusion

| ID | Feature | Owning specification |
|---|---|---|
| BLD-001 | Configure, validate, create, and report player builds | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) |
| BLD-002 | Headless command-line operation and continuous integration | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) |
| BLD-003 | Scripting backends, ahead-of-time compilation, and code stripping | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) |
| PLT-001 | Desktop, mobile, browser, handheld, and console behavior | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) |
| XR-001 | Tracking, views, interaction, haptics, focus, and comfort | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) |
| NET-001 | Sessions, connections, ownership, synchronization, and failure | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) |
| LIC-001 | Dual licensing of the product and license obligations of shipped players | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) |
| PRD-001 | Installation, updates, and target toolchain acquisition | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) |
| LOC-001 | Localized strings, assets, locale changes, and layout | [Accessibility, localization, and persistence](accessibility-localization-persistence.md) |
| ACC-001 | Accessible editor and game experiences | [Accessibility, localization, and persistence](accessibility-localization-persistence.md) |
| SAV-001 | Player settings, save data, corruption, and migration | [Accessibility, localization, and persistence](accessibility-localization-persistence.md) |

## Evidence, Samples, and Automation

| ID | Feature | Owning specification |
|---|---|---|
| DIA-001 | Logs, exceptions, diagnostics, and source navigation | [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md) |
| DIA-002 | Script debugging and code intelligence through external tools and agents | [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md) |
| PRF-001 | CPU, frame, rendering, memory, physics, audio, and target profiling | [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md) |
| PRF-002 | Engine performance and scale commitments | [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md) |
| TST-001 | Manual playtest sessions and tester feedback | [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md) |
| TST-002 | Structured scenarios, input replay, comparison, and coverage | [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md) |
| TST-003 | Project unit and integration tests | [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md) |
| SMP-001 | Complete playable sample suite | [Playable samples and benchmarks](sample-games.md) |
| CMP-001 | Behavioral compatibility benchmark suite | [Playable samples and benchmarks](sample-games.md) |
| CMP-002 | Supported package and version matrix | [Compatibility contract](compatibility-contract.md) |
| CMP-003 | Third-party and asset-store content | [Compatibility contract](compatibility-contract.md) |
| CMP-004 | Product versioning, release readiness, and interface stability | [Compatibility contract](compatibility-contract.md) |
| AUT-001 | Structured feature parity for authorized agents | [Automation and MCP](automation-and-mcp.md) |
| AUT-002 | MCP discovery, inspection, editing, play control, and evidence | [Automation and MCP](automation-and-mcp.md) |
| AUT-003 | Human-agent shared context and conflict behavior | [Automation and MCP](automation-and-mcp.md) |
| AUT-004 | Headless and concurrent agent sessions | [Automation and MCP](automation-and-mcp.md) |
| AUT-005 | Agent code workflow: author, compile, diagnose, test, and fix | [Automation and MCP](automation-and-mcp.md) |
| AUT-006 | Agent-accessible product knowledge and documentation | [Automation and MCP](automation-and-mcp.md) |
