# Engine Architecture

## Scope

This document defines how unitas is built: the layers of the engine, where
state lives, how time and work flow through a frame, and the principles every
subsystem follows. It is binding. A capability whose observable behavior is
correct but whose structure violates this document is incomplete.

It describes the end state of the whole engine at once. It is not a work list,
a sequence, or a priority order, and no section of it is more urgent than
another. Each detailed specification has an Architecture section that applies
these rules to its own domain. Those sections add detail; they never relax a
rule stated here. When a detailed specification and this document disagree,
this document holds and the detailed specification is corrected.

The rest of the specification says *what* the product does. This document and
the Architecture sections say *how* it is built. Neither restates the other.

## Independent implementation

unitas runs Unity 2022.3 projects the way Wine runs Windows programs: it
reimplements everything those projects rely on and contains none of the
original implementation.

### What unitas never uses

unitas never uses, loads, runs, links, embeds, or redistributes any
implementation published by Unity Technologies. That covers:

* the Unity Editor and player binaries;
* the implementation of the `UnityEngine` and `UnityEditor` APIs;
* every Unity-published package, both its C# code and its native libraries;
* Unity's built-in shaders and shader includes;
* the Burst compiler and IL2CPP;
* Unity's native XR, platform, and plugin libraries;
* the code of Unity's services SDKs.

Each of these is a unitas implementation, written independently and shipped as
part of the unitas distribution.

The exclusion covers code published by Unity Technologies only, including
Unity's own native plugins and libraries. A project's third-party code loads
and runs as it does in Unity: third-party C# packages, third-party native
plugins such as the Steamworks SDK, FMOD, or a platform vendor's SDK, and
Asset Store content that Unity did not publish.

### What unitas keeps

unitas keeps only the names and formats that projects depend on, because
existing projects refer to them and must compile and load unchanged:

* namespaces, type names, member names, and signatures;
* package identifiers, version lines, and assembly names;
* menu paths and attribute names;
* serialized type names, field names, and file formats.

These are required identifiers. Keeping them states compatibility and implies
no affiliation with, or endorsement by, their owner.

### Packages

* unitas implements the public API of each package in the supported package
  matrix (CMP-002 in the [Compatibility contract](compatibility-contract.md))
  as part of the engine distribution.
* A project's manifest entry for a supported Unity-published package resolves
  to unitas's implementation of the listed version line. That implementation
  has the same package identifier, assembly names, namespaces, serialized type
  names, and asset formats as the published package.
* unitas never downloads, compiles, loads, or redistributes the code that
  Unity publishes for those packages. When a project embeds such a package
  under `Packages/`, that copy stays on disk untouched, is reported, and is
  never compiled; unitas's implementation is used instead.
* A Unity-published package outside the supported package matrix is reported
  Unknown, and its code is never compiled in its place.
* Packages that are not published under Unity's terms, such as third-party,
  scoped-registry, git, and project-authored packages, compile as ordinary
  project code.
* Code marked `[BurstCompile]` is compiled by unitas's own compiler, and
  ahead-of-time compilation is unitas's own toolchain on the hosted .NET
  runtime.

### Unity-authored files inside a project

Some Unity-authored files arrive inside a project's own folders: the resources
a package imports into `Assets/` (for example TextMeshPro's Essential
Resources), the samples a package copies into the project, and Unity's
standard assets. unitas treats them by what they are:

* **Code and shaders:** scripts, shaders, shader includes, and compute
  shaders that Unity authored are never compiled or run. unitas recognizes
  them by package origin, GUID, and name, and uses its own implementation
  under the same names and GUIDs, so every reference to them resolves and
  behaves as the project expects. The project's copy stays on disk untouched,
  and the project compatibility report lists each substitution.
* **Data:** fonts, textures, meshes, and other data assets are read as
  ordinary project content when their license permits it, such as fonts under
  the SIL Open Font License. Data whose license does not permit use outside
  Unity is replaced by unitas's own equivalent under the same GUID, and the
  substitution is reported.
* **unitas's own copies:** when unitas imports such resources into a project,
  for example on the first use of TextMeshPro, it writes its own files under
  the GUIDs and paths the project expects.

### Unity-built content

Content that Unity built, such as AssetBundles, Addressables content,
lightmaps, probes, and other baked data, is input. unitas reads it with its
own code, as the [Content pipeline](#content-pipeline) defines, and never with
Unity's code. Test fixtures that a licensed Unity Editor produced are
permitted as input content, with the producing Unity release recorded.

### Clean-room development

* **Sources of behavior:** Unity's public documentation, published file
  formats, and black-box observation of a licensed Unity Editor or player.
  Observation records reference behavior and runs the compatibility
  benchmarks, as Wine's conformance tests run on Windows.
* **Never a source:** Unity's source code in any form, including the C#
  reference source, package source, built-in shader source, and any
  decompiled or disassembled binary. Observing what Unity does is permitted;
  reading how Unity's code does it is not.
* Contributors follow the [independent implementation rules](../../CONTRIBUTING.md#independent-implementation)
  and the [trademark rules](../../TRADEMARKS.md).

## Layers

unitas is one engine with five layers. Each layer depends only on the layers
below it.

1. **Platform:** windows and surfaces, input devices, audio devices, the GPU
   device, files, threads, timers, sockets, and process services for each
   target. Everything above it is target-independent.
2. **Core:** the native world, the frame loop, the job system, memory, and
   change tracking.
3. **Subsystems:** assets and serialization, rendering, UI and text, audio and
   video, input, physics, animation, particles, navigation, terrain,
   networking, and XR. Each one reads and writes the world through the core
   and never through another subsystem's internals.
4. **Scripting:** C# on hosted .NET, with the `UnityEngine` and `UnityEditor`
   APIs as bindings over the native layers.
5. **Player:** the application that hosts the other layers on a target and
   runs whatever cooked project it is given.

The editor is a unitas project that runs on the player. It is not a layer of
its own and has no access to the engine that a game lacks.

The native layers are written in Rust. Scripting, the `UnityEngine` and
`UnityEditor` APIs, and the editor are written in C#.

## The native world

Every scene object, component, and runtime asset instance exists once, in the
core's native world.

* **Identity:** each object has a stable native identity. A managed
  `UnityEngine.Object` is a thin handle that holds that identity and nothing
  else. Managed code keeps no copy of scene or engine state.
* **Storage:** components of the same type are stored together in dense
  arrays so subsystems iterate them in bulk. A native identity is a
  generational handle: an index into a slot table plus a generation, so
  looking an object up is one indexed read that needs no lock and no hash, and
  a stale handle is detected rather than misread. Adding and removing
  components keeps the arrays dense, and its cost does not grow with the size
  of the world.
* **Hierarchy and transforms:** each root object's hierarchy is stored as one
  contiguous block in depth-first order, holding local transforms, cached
  world matrices, and dirty bits. A change to a local transform marks that
  transform and its descendants dirty. World matrices of dirty transforms are
  recomputed once per frame, before the systems that read them run, as jobs
  that run in parallel across hierarchies; they are never recomputed per
  query. A read of a dirty transform's world value between those points
  updates only that transform's dirty ancestors.
* **Math:** vector, matrix, quaternion, and bounds math uses SIMD types from a
  mature math library on every target that has SIMD, including WebAssembly
  SIMD in browsers.
* **Change tracking:** the world records what changed in each frame: objects
  created and destroyed, components added and removed, and values written.
  Each component array keeps a change version per element and a list of the
  elements changed since each consumer last read it, so a consumer finds its
  changes in time proportional to their number, without scanning and without
  allocating. A subsystem keeps its own derived state, such as render proxies,
  physics bodies, audio voices, and navigation agents, and updates it from
  those changes. It never rebuilds derived state from the whole world each
  frame.
* **Lifetime:** destruction is deferred to defined points of the frame, as
  Unity defines. A handle to a destroyed object compares equal to null and
  raises Unity's exception when used. It never reaches freed memory.

## The frame loop and time

* The core runs a single PlayerLoop with Unity 2022.3's phases in Unity's
  order, in the editor, headless, and in every player. Subsystems register
  their work into its phases. Nothing else advances time.
* Fixed-step work, such as physics, runs zero or more times per rendered
  frame, as the project's time settings define. Rendering, animation, UI, and
  audio scheduling read the time that the loop publishes for the frame.
* Script callbacks are dispatched from the loop in Unity's order. The loop
  iterates the scripts that implement each callback; it never calls into every
  script to find out.
* Headless sessions and captured frame rates drive the same loop with a
  controlled clock. There is no second loop for tests or tools.
* **Frame pipelining:** while the main thread simulates frame N+1, render
  threads record and submit frame N and the GPU executes frame N-1 or N. The
  number of frames queued ahead of the display follows
  `QualitySettings.maxQueuedFrames` and the target's presentation mode, so
  input-to-display latency is bounded and stated. The main thread waits on
  the GPU only where an API is synchronous by contract.

## Concurrency

* **Main thread:** script callbacks, the Unity APIs that Unity restricts to the
  main thread, and changes to the structure of the world run on the main
  thread.
* **Job system:** a work-stealing job system with one worker per available
  core serves engine work and the C# Job System alike. Subsystems split bulk
  work, such as transform updates, culling, animation, skinning, particles,
  physics islands, and navigation, into jobs with declared read and write sets.
  The job system finishes them before the frame needs their results.
* **Rendering:** the main thread extracts each frame's render data. Render
  threads record and submit GPU work for that frame while the main thread
  simulates the next one.
* **Audio:** the mixer runs on the audio device's thread. It takes no locks,
  allocates no memory, and waits on no other thread. It receives commands
  through lock-free queues.
* **I/O and loading:** file, network, decompression, decoding, and GPU upload
  work run asynchronously. The main thread never blocks on I/O.
* **Single-threaded targets:** on a target without threads, such as a browser
  page that is not cross-origin isolated, the same jobs run inline on the main
  thread with the same results. Only the parallelism changes.

## Memory

* Memory is allocated by lifetime: per-frame scratch memory is reset at the end
  of each frame, job-scoped memory is released when its job finishes, and
  persistent memory is owned by the world or by a subsystem.
* Steady-state play allocates no persistent native memory and no managed
  memory per frame inside the engine. Growth comes from content the game loads
  or creates.
* General-purpose native allocation goes through a scalable allocator with
  per-thread caches, so jobs on many cores allocate without contending.
* The managed heap uses the hosted .NET runtime's generational, concurrent
  garbage collector. Collections of short-lived objects do not scan the whole
  heap, and full collections run in the background where the target has
  threads. Engine APIs that Unity documents as non-allocating allocate
  nothing, and main-thread pause times are measured against the release's
  declared budgets.
* GPU resources are reference-counted by the content that uses them, are
  uploaded asynchronously, and are released when the last user goes away.
* Native code that needs unchecked memory access, such as foreign-function
  calls, .NET hosting, native collections, GPU and audio buffers, SIMD, or
  platform APIs, keeps it inside small, named modules behind safe interfaces.
  Each such module states the invariants it relies on.

## The managed boundary

* The bindings between C# and the native layers are generated from one
  description of each native API. They are never written by hand per member.
* A binding call resolves its handle directly to the native object. It takes
  no global lock, allocates nothing on either side, and passes blittable data
  by value or by span. Calls go through direct unmanaged function pointers,
  not marshalling stubs. An accessor that cannot block, allocate, or call back
  into managed code, such as reading a transform's position, crosses without
  a garbage-collector transition, so its cost is that of a native function
  call.
* Script callbacks are dispatched in batches: for each PlayerLoop phase, native
  code passes the list of behaviours to call in one transition, and a managed
  dispatcher calls each one directly, with no reflection and no per-call
  transition.
* An API that touches many objects, such as transform access arrays, physics
  query results, mesh data, or native containers, has a bulk form that crosses
  the boundary once per batch rather than once per element.
* Native code calls managed code only to dispatch script callbacks and
  events. Engine work never waits on managed code.
* The player hosts .NET in its own process. Managed code runs with
  just-in-time compilation where the target permits it, and with
  ahead-of-time compilation where the target forbids it, such as browsers,
  iOS, and consoles. Roslyn compiles project scripts and the scripts of
  packages not published by Unity under Unity 2022.3's language profile into collectible load contexts, so the editor
  reloads code without restarting the process.

## Content pipeline

* **Import:** importers turn Unity source assets into cooked artifacts. Each
  artifact is keyed by a hash of its inputs: the source content, its metadata,
  its import settings, the importer version, and its dependencies. Importing
  runs in parallel and incrementally, and imports again only what an input
  change can affect.
* **Identity:** a GUID and local file identifier name a piece of content the
  same way in the editor, in cooked data, and in the player.
* **Cooked data:** cooked artifacts are laid out to be loaded directly into
  the world and onto the GPU, with no parsing of Unity source formats at
  runtime. Players load only cooked data. Code that reads Unity source formats
  belongs to importers alone.
* **Bundles and streaming:** a build packages cooked artifacts into bundles.
  The player streams them asynchronously, so content loads while the game
  runs.
* **Unity-built content:** AssetBundles and Addressables content that Unity
  itself built are converted to cooked form when a player first loads them, by
  the same importer code the editor uses for that format, shader translation
  included. The result is cached on the device by the bundle's content hash,
  and later loads read only the cooked result. This is the only reading of a
  Unity format at runtime.

## Presentation

* A render graph on `wgpu` draws everything: games, the editor, UI, text,
  previews, and captures. The render graph, not `wgpu`, is the rendering
  abstraction. Where a backend's native API offers a capability that `wgpu`
  does not expose, such as asynchronous compute and copy queues, ray tracing,
  mesh shaders, bindless resource tables, or pipeline libraries, the render
  graph's backend for that API uses it through `wgpu`'s native-API interop,
  inside the same graph. This is the one implementation of that capability on
  that backend, not a second renderer.
* **GPU-driven drawing:** render proxies live in a persistent GPU scene buffer
  updated only where change tracking reports changes. On devices with compute,
  culling against the frustum and a hierarchical depth buffer runs on the GPU,
  and visible objects are drawn through indirect multi-draw calls grouped by
  pipeline. CPU cost per frame therefore follows the number of distinct
  pipelines and changed objects, not the number of visible objects. Devices
  without compute use the same proxies with CPU culling jobs and instancing.
* **Parallel recording:** passes and draw lists are recorded on several render
  threads at once, into separate command encoders that submit in graph order.
* **Pipelines:** each device's compiled pipelines are kept in a persistent
  cache keyed by device and driver, warm-up creates them before first use, and
  no frame waits on compiling a pipeline that warm-up or a previous run
  created.
* Shaders are translated at import by a real shader compiler toolchain into
  the forms each graphics API needs. Pipelines are compiled ahead of first use
  and cached.
* Headless sessions and captures run the same render graph offscreen. There is
  no CPU renderer and no second rendering path for tests.

## Engine instances and topology

* **Player:** one generic player per target and configuration (development,
  release, dedicated server) runs any cooked project bundle. It contains no
  game and no default content. Where the target permits just-in-time
  compilation, the player's code is identical for every project. Where it
  requires ahead-of-time compilation, the build links the project's compiled
  managed code with that same engine code; engine-code stripping removes whole
  modules the project does not use and never changes a module it keeps.
* **Editor:** the editor is a unitas project running on the player. The editor
  instance runs the editor's own scripts and holds the project model, and it
  never loads or runs project code. Project code runs in workers: separate
  operating-system processes, each an engine instance with its own live world,
  that host edit-time scripts, editor extensions, compilation with its
  analyzers and source generators, play mode, and tests. Each isolated play
  session and each isolated proposal environment is its own worker. A crashing
  or hanging worker never takes the editor down. The
  [Editor and authoring](editor-and-authoring.md#architecture) specification
  defines how the two cooperate.
* **Commands:** each editor command is implemented once, in the editor
  instance. The visual editor, the editor's MCP interface, and the command line
  are front ends over the same commands, and none of them holds state or logic
  of its own.
* **MCP:** only the editor has an MCP interface. The engine, the runtime, the
  `UnityEngine` API, the player, and every game build contain no MCP code or
  dependency. An agent reaches a running game only through the editor.
* **No editor-only hooks:** a capability the editor needs becomes an engine
  capability that games can use as well.
* **No external driver:** no standalone .NET program drives the engine: not the
  editor, not a scenario or playtest runner, and not a tool. A .NET test project
  may load the native library to unit-test bindings, and that is all.

## Games and samples

* The engine runs games and never contains them. No engine, player, build, or
  tool code knows a particular game, and the player has no default or built-in
  game. A game proves the engine only when the packaged player and the headless
  editor run the game's own scripts and content.
* Samples are original content written the way developers write games for Unity
  2022.3, in its on-disk formats, so that Unity itself could open them. A sample
  never works around a gap in unitas; the gap is closed in the engine.

## Performance principles

* **Cost follows change and visibility.** Per-frame work scales with what
  changed and what is visible, never with the size of the world. An object
  that neither changes nor is visible in a frame adds no measurable cost to
  that frame.
* **Draw like things together.** Rendering culls what cannot be seen. It keeps
  per-object GPU data persistent between frames and updates only what changed.
  It draws objects that share a mesh and material as instances, and batches
  compatible draws, so draw-call counts track distinct materials and meshes
  rather than object counts.
* **Optimized engine builds.** Release engine binaries are built with
  link-time optimization, a single code-generation unit per crate, and
  profile-guided optimization trained on the benchmark projects. Release
  players abort on an unrecoverable engine fault rather than unwinding.
  Development players keep the checks and symbols that diagnostics need.
* **Comparable to Unity.** Frame time on the benchmark projects is measured
  against Unity 2022.3's own player on the same target and hardware, observed
  as a black box, and the declared budgets bound unitas's overhead relative to
  it.
* **Measure everything.** Every subsystem carries profiler markers that cost
  nothing when profiling is off. Engine performance is the frame time of the
  packaged player, measured against the budgets that the
  [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md)
  specification declares.

## Robustness

* Malformed content, missing references, script exceptions, device loss, lost
  graphics contexts, and window resizes never crash the engine. A failure is
  contained to the object, asset, or subsystem it belongs to.
* Diagnostics name the asset, object, component, or member at fault.

## Dependencies

Solved infrastructure comes from mature, maintained libraries whose licenses
are compatible with GPL-3.0: physics solvers, image, audio, and video codecs,
font shaping, shader compilers, .NET hosting, and serialization. unitas
integrates them behind its own interfaces, so a library can be replaced
without changing behavior.

## One implementation per capability

Each capability has one implementation, used in the editor, headless, and the
player on every target. There are no fallbacks, parallel paths, or legacy
modes. When a better implementation replaces an older one, the older one is
removed.
