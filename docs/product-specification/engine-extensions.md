# Engine Extensions

## Scope

This specification defines the capabilities unitas offers beyond Unity 2022.3
LTS: large-world rendering, dynamic lighting, upscaling, world streaming, a
shared content cache, live code reload, multi-instance play, collaborative
editing, and unified tracing. It owns EXT-001 through EXT-010 in the
[Feature catalog](feature-catalog.md).

Each extension exists so that projects can go further than Unity 2022.3
allows, without changing how a Unity project behaves. The rules in
[Compatibility rules](#compatibility-rules) bind every extension, including
extensions that later specifications add.

Every capability here is available in the visual editor, headless, and
through the editor's MCP interface, as defined in
[Automation and MCP](automation-and-mcp.md).

### Compatibility tiers

EXT-001 through EXT-010 are Native: unitas defines their behavior, and no Unity
behavior is promised for them because Unity 2022.3 LTS has none.

## Compatibility rules

* **Off by default.** Every extension is off in a project until the project
  enables it. A Unity project opened in unitas behaves as its owning
  specifications define, with no extension in effect.
* **Stored outside Unity's files.** Enabling an extension and its settings are
  unitas-specific project content (SER-005 in
  [Serialization and prefabs](serialization-and-prefabs.md)). unitas never
  rewrites a Unity-authored asset, setting, or `.meta` file to enable an
  extension, and Unity 2022.3 opens the project and behaves as it would
  without the extension.
* **Additive APIs.** Script APIs for extensions live in `Unitas.*` namespaces in
  assemblies that unitas supplies. unitas defines the `UNITAS` scripting symbol
  in every compilation, so project code that guards extension calls with it
  compiles unchanged in Unity 2022.3.
* **Reported.** The project compatibility report, build results, and product
  knowledge list every extension a project enables and every asset it affects.
* **Never a fallback for compatibility.** An extension never stands in for a
  Unity behavior that a specification requires. A required behavior is
  implemented as its owning specification defines whether or not an
  extension exists.
* **Independently implemented.** Extensions are built from public research and
  public descriptions of techniques, under the
  [Independent implementation](architecture.md#independent-implementation)
  rules. Those rules apply to every other engine, including engines whose
  source code is available under license: unitas never reads, copies, or
  derives from that source.

## Architecture

Extensions are subsystems and editor commands like any other, and follow the
[Engine architecture](architecture.md).

* **Rendering extensions** are passes and resources in the one render graph.
  They read the same render proxies, GPU scene buffer, and visibility results
  as the rest of rendering. They are enabled per render pipeline asset, per
  camera, or per quality level, so a scene can mix extension and standard
  rendering.
* **Capability tiers.** Each rendering extension states the device
  capabilities it needs, such as compute, 64-bit atomics, or hardware ray
  tracing. On a device that lacks them, the extension reports that, and
  rendering uses the standard path for that content. That path is the same
  one a project without the extension uses, not a second implementation of
  the extension.
* **Content extensions** are additional import outputs. An importer writes
  the extension's cooked data next to the standard cooked artifact, under
  the same content-hash key rules, so turning an extension off never needs a
  reimport.
* **Editor extensions** of this document are commands in the editor's command
  layer, and their state is command records, so undo, review, audit, and MCP
  apply to them unchanged.

## Virtualized geometry

A mesh marked for virtualized geometry, through an import setting, renders
with per-cluster level of detail chosen on the GPU each frame, so triangle
cost follows the screen resolution rather than the source triangle count.
(EXT-001)

* At import, the mesh is split into clusters, and a hierarchy of simplified
  cluster groups is built with error bounds that keep group boundaries
  watertight.
* Each frame, the GPU selects the clusters whose projected error is below one
  pixel, culls them against the frustum and the previous frame's depth, and
  draws them through indirect draws, with small triangles rasterized in a
  compute pass where the device supports it.
* Cluster data streams on demand from the cooked artifact, within a resident
  memory budget that the project sets.
* A virtualized mesh keeps its `Mesh` asset, collider, and script-visible data.
  Scripts, physics, and lightmapping see the original mesh.

## Dynamic global illumination and reflections

A render pipeline asset can enable dynamic global illumination, which lights
scenes with diffuse bounce light and glossy reflections computed each frame,
so moving lights and changing geometry need no baking. (EXT-002)

* Indirect lighting is traced against a simplified scene representation built
  and updated from render proxies through change tracking. Hardware ray
  tracing is used where the device offers it, and screen-space and
  distance-field tracing where it does not.
* Results are cached in world space and in screen space and accumulated over
  frames, so per-frame cost follows the screen resolution and the amount of
  change.
* Baked lightmaps, light probes, and reflection probes that the project
  contains keep working. The project chooses, per volume, whether dynamic
  global illumination replaces or adds to them.

## Virtual shadow maps

Directional, spot, and point lights can use virtual shadow maps: very large
shadow textures that are paged, so only the pages that visible pixels sample
are rendered, and cached pages are redrawn only where casters moved.
(EXT-003)

## Temporal upscaling

Cameras can render at a lower internal resolution and reconstruct the output
resolution with a temporal upscaler that uses depth, motion vectors, and
history, with jittered projection and history rejection for disocclusion. The
render scale, the quality mode, and the resulting sharpness are per-camera
settings, and the upscaler composes with the post-processing chain at a
defined point. (EXT-004)

## World streaming

A scene can be partitioned into a spatial grid of cells. Each cell holds the
objects whose bounds it contains, and cells load and unload asynchronously
around one or more streaming sources, such as the player camera, with
per-cell levels of detail for distant content. (EXT-005)

* Partitioning is an editor command that writes the cells as ordinary additive
  scenes, so the project stays openable in Unity 2022.3 as a set of scenes.
* Loading and unloading cells use the runtime scene management of RUN-005 in
  [Scripting runtime](scripting-runtime.md), time-sliced so no frame stalls.
* Objects that the project marks as always loaded stay in a persistent scene.
* Large-world precision follows [Large worlds](physics-navigation-terrain.md#large-worlds).

## Shared content cache

Cooked import results can be shared between machines through a shared
content cache, so a developer or a continuous-integration job reuses results
that any other machine has already produced for the same inputs. (EXT-006)

* The cache is keyed by the same content hashes as the local import cache, so
  a shared result is identical to a local one.
* A shared cache is a network location or service that the team configures.
  Reads never block the editor's main thread, and a cache that is unavailable
  only means local import.
* Each import result records whether it came from the local cache, the shared
  cache, or a fresh import.

## Live code reload

During play mode, a change to a method body is applied to the running play
session without a domain reload, preserving every object's state. (EXT-007)

* Changes that the hosted .NET runtime can apply in place, such as edited
  method bodies, added methods, and added fields on new instances, apply
  within the next frame.
* Changes it cannot apply, such as a changed type layout of existing
  instances, are reported naming the type, and the project's normal reload
  behavior applies.
* Live code reload is a choice in the editor's script-reload settings. The
  default reload behavior is the one Unity 2022.3 defines.

## Multi-instance play

A play session can start several game instances at once: any combination of
server, host, and clients, each in its own worker, with its own window or
headless, its own input, and its own player identity. (EXT-008)

* The editor shows each instance's game view and console, and selection in
  the Hierarchy can follow any instance.
* Instances connect to each other over the networking stack of NET-001 in
  [Builds, platforms, XR, and networking](build-platforms-xr-networking.md),
  with optional simulated latency, loss, and jitter per instance.
* Agents start, drive, and observe each instance through the editor's MCP
  interface.

## Collaborative editing

Several people, and agents, can edit one project at the same time from
different machines. (EXT-009)

* Participants join a session hosted by one editor. Every participant's edits
  are command records that the host orders and replicates, so all
  participants see the same project state.
* Each participant has their own selection, camera, and undo stream, and sees
  the others' presence, selections, and locks.
* Saving writes the project's ordinary files. A participant who leaves the
  session keeps a working copy of the project as of that moment.
* Conflicts are resolved by the same revision checks that resolve conflicts
  between agent sessions (AUT-004 in [Automation and MCP](automation-and-mcp.md)).

## Unified tracing

One trace records the editor instance, its workers, and connected development
players on a single timeline, including CPU work per thread, GPU passes,
loading, memory, network traffic, and custom markers. (EXT-010)

* Tracing streams to a file or to the editor while the session runs, with a
  per-frame cost the profiler states.
* A trace can be reopened later and compared with another trace of the same
  project.
* The profiler of PRF-001 in
  [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md)
  reads the same instrumentation; unified tracing adds the cross-process
  timeline and long recordings.

## Acceptance outcomes

1. A Unity 2022.3 project opened in unitas with no extension enabled produces
   the same scenario results, captures, and build results as the same project
   before any extension existed, and enabling and then disabling every
   extension leaves every Unity-authored file byte-for-byte unchanged.
   (EXT-001, EXT-002, EXT-003, EXT-004, EXT-005, EXT-006, EXT-007, EXT-008,
   EXT-009, EXT-010)
2. A scene of meshes with a combined billion source triangles, marked for
   virtualized geometry, renders within the release's declared frame-time
   budget on the reference desktop, with resident cluster memory within the
   project's budget. (EXT-001)
3. Moving the only light in a room lit by dynamic global illumination changes
   the bounce lighting within the frames the setting declares, with no baking.
   (EXT-002)
4. A directional light with virtual shadow maps shades a scene with no
   cascade seams, and a static scene redraws no shadow pages from frame to
   frame. (EXT-003)
5. A camera at half internal resolution with temporal upscaling meets its
   visual validation reference within the declared tolerance, and moving
   objects show no ghosting beyond it. (EXT-004)
6. A partitioned world streams cells around a moving camera with no frame
   exceeding the declared budget, and the partitioned project opens in Unity
   2022.3 as a set of additive scenes. (EXT-005)
7. A second machine opening the same project revision with a shared content
   cache imports no asset that the first machine already imported, and its
   import results report the shared cache as their source. (EXT-006)
8. Editing a method body during play applies on the next frame and keeps
   every object's field values, and an edit that changes a type's layout is
   reported and falls back to the project's reload behavior. (EXT-007)
9. A play session with one server and three clients, started through the
   editor's MCP interface, replicates a spawned object to all clients, and each
   instance's console is distinct. (EXT-008)
10. Two people editing one scene from two machines see each other's changes
    and selections, undo only their own changes, and save one consistent set
    of files. (EXT-009)
11. A unified trace of an editor play session with a connected development
    player shows both processes on one timeline, and reopening it later shows
    the same data. (EXT-010)
