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
* **Storage:** components of the same type are stored together so subsystems
  iterate them in bulk. Looking an object up by identity is constant-time and
  needs no global lock.
* **Hierarchy and transforms:** each transform caches its world matrix. A
  change to a local transform marks that transform and its descendants dirty.
  World matrices are recomputed once per frame, before the systems that read
  them run, and are never recomputed per query.
* **Change tracking:** the world records what changed in each frame: objects
  created and destroyed, components added and removed, and values written. A
  subsystem keeps its own derived state, such as render proxies, physics
  bodies, audio voices, and navigation agents, and updates it from those
  changes. It never rebuilds derived state from the whole world each frame.
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
  by value or by span.
* An API that touches many objects, such as transform access arrays, physics
  query results, mesh data, or native containers, has a bulk form that crosses
  the boundary once per batch rather than once per element.
* Native code calls managed code only to dispatch script callbacks and
  events. Engine work never waits on managed code.
* The player hosts .NET in its own process. Managed code runs with
  just-in-time compilation where the target permits it, and with
  ahead-of-time compilation where the target forbids it, such as browsers,
  iOS, and consoles. Roslyn compiles project and package scripts under Unity
  2022.3's language profile into collectible load contexts, so the editor
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
  previews, and captures.
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
