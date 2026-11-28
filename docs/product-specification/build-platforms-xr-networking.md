# Builds, Platforms, XR, and Networking Product Specification

## Scope

This specification defines the user-visible build product, headless and
continuous-integration operation, scripting backends and code stripping, player
behavior, platform integration, XR behavior, the supported networked-game
experience, deployment to devices and development-player connections, content
builds and post-release content updates, product licensing as it applies to
shipped players, product installation, updates, and target toolchains, and the
product's own data flows and privacy behavior. The browser delivery of PLT-001
is detailed in [Web target](web-target.md).

Every capability in this specification is available to authorized agents
through structured automation and MCP as well as to people, under the rules
owned by [Automation and MCP](automation-and-mcp.md). Package-provided behavior
(XR, networking, content delivery, input) is in scope only for the package
families and version lines listed in the supported package matrix (CMP-002) of
the [Compatibility contract](compatibility-contract.md).

Compatibility tiers of the features this specification owns, as recorded in
the [Feature catalog](feature-catalog.md) and defined in the
[Compatibility contract](compatibility-contract.md): BLD-001 Native, BLD-002
Native, BLD-003 Native, BLD-004 Native, BLD-005 Native, PLT-001 Native, XR-001
Familiar, NET-001 Familiar, LIC-001 Native, PRD-001 Native, PRD-002 Native.
Unity build settings, player settings, and package content are imported and
honored as this specification defines; the behavior is unitas-defined.

## Architecture

This section applies [Engine architecture](architecture.md) to builds, players, and targets.

### Players and the platform layer

* There is one player per target and configuration: development, release, and dedicated
  server. It holds the engine layers and the .NET host, contains no game, and runs the cooked
  project bundle the build pairs it with. On targets that permit just-in-time compilation, the
  player's code is identical for every project. On ahead-of-time targets, the build links the
  project's compiled managed code with that same engine code. Engine-code stripping removes
  whole modules the project does not use and never changes a module it keeps.
* Each target implements the platform layer once: window and presentation surface, GPU
  adapter, input devices, audio device, file system and storage, threads and timers, sockets,
  permissions, and process lifecycle. Every capability query answers supported, denied, or
  absent, and the `Application`, `Screen`, `SystemInfo`, and `Input` members report those
  answers. Nothing above the platform layer branches on the target.
* Window, focus, lifecycle, and device events enter queues that the PlayerLoop drains at its
  defined phases on the main thread. The target's frame source (display refresh, the
  browser's animation frame, or the XR runtime's frame wait) paces the loop and never runs
  engine or script work outside it.
* .NET is hosted in process on every target: the just-in-time runtime where the target
  permits it, ahead-of-time code plus an interpreter for code generated at runtime where it
  does not, and the .NET WebAssembly runtime in the browser. Bindings and managed behavior are
  the same in every mode; only code generation differs.
* A dedicated server player is the same player with no presentation surface, GPU device, or
  audio device. Rendering and audio components keep their state and API values, no render
  proxies or voices are derived from them, and an operation that needs output reaches the
  absent device, which reports it.

### Builds and content

* A build is one editor command, whether started from the visual editor, the editor's MCP
  interface, the command line, or `BuildPipeline.BuildPlayer`. It has three stages:
  * **Cook:** target-specific artifacts (texture formats, shader forms, variant stripping)
    come from the same content-hash cache the editor uses.
  * **Compile:** scripts compile with the target's defines, are stripped, and on
    ahead-of-time targets are compiled to native code.
  * **Package:** cooked bundles, managed code, and the player are assembled into the target's
    artifact. External toolchains (Gradle, Xcode, signing, notarization, and console SDK
    tools) run as separate processes.
* Each stage is keyed by the digests of its inputs, toolchain versions included. The build
  manifest records those keys, so builds are reproducible, comparable, and resumable after
  cancellation or failure. Independent work runs in parallel, and no stage blocks the
  editor's main thread.
* A content build runs the cook and package stages and writes bundles and catalogs instead
  of a player. Unity-built bundles are converted to cooked form on first load, as the
  [Engine architecture](architecture.md#content-pipeline) defines.
* Headless and continuous-integration operation is the headless editor running the same
  commands. The command line maps arguments to commands and results to output and exit
  status; there is no separate build tool.

### Devices and development players

* Deployment is one device interface in the editor, with one adapter per target's device
  tooling. Discovery, install, launch, and removal are commands on it.
* A development player has one connection endpoint. It carries the profiler, log-stream,
  debugger, and remote-control channels, authenticated by the build's pairing credential. In
  the browser, where a page cannot listen for connections, the player opens the connection to
  the editor. The editor is the only client; agents reach the player through the editor's MCP
  interface. A release player contains no endpoint code at all, not a disabled one.

### XR

* The XR subsystem owns the session, runtime, tracking spaces, poses, and haptics, and
  reaches the device through the OpenXR loader in the platform layer. The XR packages'
  managed code runs as shipped. unitas provides the native provider that code calls, in place
  of the package's native library. `XRDisplaySubsystem`, `XRInputSubsystem`, and the XR Plugin
  Management loaders are bindings over this subsystem.
* Interaction profiles and hand joints become Input System devices through the input
  subsystem. Render views (poses, fields of view, and the runtime's swapchain images) go to
  the render graph's stereo path. The XR subsystem never draws.
* On an XR target, the runtime's frame wait paces the PlayerLoop. Each frame publishes the
  runtime's predicted display time; simulation reads poses predicted for that time, and
  submission re-reads head and eye poses as late as the runtime allows. Session state changes
  become lifecycle messages at loop phases.
* The editor, headless sessions, and scenarios run the same subsystem against a simulated
  runtime whose poses, views, and input the session declares. That runtime is always reported
  as simulated.

### Networking

* Layers, bottom up: platform sockets (UDP and TCP on native targets, WebSocket and WebRTC
  data channels in the browser), Unity Transport, a netcode package, then the game. Each layer
  uses only the one below. The packages' managed code runs as shipped. unitas provides the
  sockets, jobs, and engine APIs it calls.
* Socket I/O is asynchronous and never blocks the main thread. Sends and receives are drained
  at defined loop phases, and netcode reads time only from the time the loop publishes.
* Server, host, client, and thin client are roles of the same player. Several instances,
  headless or not, run on one machine with distinct identities. Every packet, message, and
  replicated change is attributed to a connection and participant in diagnostics. The network
  simulator tags what it alters, and release builds contain no simulator.

## Build configuration

As a developer or authorized agent, I can select scenes and order, product
identity, semantic or platform version, icons, orientation or window behavior,
displays, quality, development diagnostics, scripting backend, managed code
stripping level, scripting compatibility, content choices, signing, and
target-specific options. Unity Player Settings and Build Settings stored in the
project are read and honored as the starting configuration; their storage is
owned by PRJ-007 in [Projects and assets](project-and-assets.md).

Every option states its current value, default, valid choices, affected targets,
and player-visible effect. Invalid combinations are reported before distribution
with the setting and consequence identified.

Development and release builds are visibly distinct. A release build never
contains development-only access merely because an earlier build enabled it.

### Build hosts

Every desktop target builds on every supported host. A developer on macOS
builds Windows and Linux players, a developer on Windows builds macOS and Linux
players, and a developer on Linux builds Windows and macOS players, with either
scripting backend, without a second machine or a virtual machine. The linkers,
platform libraries, and other tools a cross-host build needs install with the
target's support, from sources whose licenses permit that. Authenticode
signing of Windows players runs on every host, and macOS notarization runs on
every host through the notarization service with the developer's credentials.

A target whose platform holder requires its own host tooling, such as Xcode for
iOS, tvOS, and visionOS-class devices or a console vendor's SDK, states that
requirement in the release's host-target matrix (CMP-005 in the
[Compatibility contract](compatibility-contract.md)). On any other host, the
build produces every artifact up to the step that needs that tooling, such as
the generated Xcode project, and reports the remaining step and the host it
needs. A build never claims a step that it did not run.

## Build results

A successful build produces the artifacts expected by the target, including a
desktop application, Android package or app bundle, iOS application project or
package, browser bundle, XR application, or licensed-console package as
applicable.

Every result records:

* target, architecture, configuration, product and version;
* the unitas version that produced it, as defined by CMP-004 in the
  [Compatibility contract](compatibility-contract.md);
* included scenes and effective player settings;
* scripting backend, managed code stripping level, and ahead-of-time
  compilation findings;
* success, failure, cancellation, and elapsed time;
* errors and warnings linked to content or settings;
* output location and primary launch artifact;
* total size, useful size categories, and major contributors;
* presence of development diagnostics;
* signing, notarization, and store packaging status;
* the license that governs the included unitas player components and the
  obligations recorded for it (see [Licensing](#licensing));
* the third-party components included in the player and their notices;
* project behavior that is Platform-limited on the target because the target
  lacks a capability, and content reported Unknown or invalid.

Cancellation leaves no artifact presented as a successful player. Repeated
builds with the same project state, build choices, toolchain, dependency
versions, and target environment have no unexplained behavioral or content
differences. Changes in external content used by a build are recorded.

## Build identity, reproducibility, and long builds

Every build has a unique build identifier, which the player exposes as
`Application.buildGUID`. Every build produces a build manifest that records
digests of its inputs (project content, settings, packages, and plug-ins),
toolchain versions, the unitas version, the build choices, and digests of every
output it produced. Comparing the manifests of two builds names each differing
input and each differing output. The manifest includes a machine-readable
software bill of materials listing the unitas components and the third-party
components included in the player, with their versions and licenses.

A build that encounters a content failure fails, and its result reports every
failure found during the build, not only the first. After a build is cancelled,
interrupted, or fails, the next build reuses completed results whose recorded
inputs are unchanged and states which work it redid and why.

## Release hardening

A release player contains no automation, remote-control, debugger, or profiler
endpoint. It embeds no absolute paths of the build host and no account names
beyond those present in the project's own content. It loads native libraries
only from its own installation locations. Beyond those endpoints, which a
release player never contains, any other development-only capability in a
release build is named in the build result.

## Native project generation

Android builds honor the custom templates in `Assets/Plugins/Android`
(`AndroidManifest.xml`, `mainTemplate.gradle`, `launcherTemplate.gradle`,
`baseProjectTemplate.gradle`, `gradleTemplate.properties`, and
`settingsTemplate.gradle`) and `.androidlib` folders, merge the manifests of
the project, its plug-ins, and its libraries, and can export a Gradle project
instead of a finished package. `IPostGenerateGradleAndroidProject` callbacks
run on the generated Gradle project before it is built.

iOS builds produce an Xcode project that `UnityEditor.iOS.Xcode` APIs
(`PBXProject`, `PlistDocument`, and `ProjectCapabilityManager`) can modify from
`[PostProcessBuild]` methods and `IPostprocessBuildWithReport` callbacks.

Every template, template placeholder, and `UnityEditor.Android` and
`UnityEditor.iOS.Xcode` member that Unity 2022.3 LTS defines is honored. A
template that is malformed, or that uses a placeholder Unity 2022.3 LTS does
not define, is reported in the build result, naming the file and the field;
it is never silently ignored.

## Signing, notarization, and store packaging

Each target reports its signing requirement and current signing state:
unsigned, signed for development, signed for distribution, or signing failed
with the reason. Android builds identify whether a debug or a release keystore
and which key alias were used. Apple targets identify the signing identity,
provisioning profile, and entitlements used. macOS distribution builds report
notarization as not requested, submitted, accepted, accepted and stapled, or
rejected with the notarization service's findings. Windows builds report
whether an Authenticode signature was applied.

Each target reports the store packaging form it produced (for example an
Android App Bundle or APK, an Xcode project or signed archive, a macOS
application bundle or installer package) and whether that form is the one the
target's store requires for submission.

Signing secrets such as keystore passwords and private keys never appear in
build results, logs, diagnostics, or automation output. An agent can use signing
configuration that the developer has made available to it, and cannot read the
secrets themselves. A build never submits to a store implicitly, and a signed or
packaged artifact is never presented as accepted by a store, platform holder,
or notarization service unless that service reported acceptance.

### PC storefronts and Steam

Windows, macOS, and Linux players are produced in the folder layout that PC
storefronts ingest, one folder per target and architecture, so that each folder
can be uploaded as a Steam depot or the equivalent unit of another storefront
without rearrangement. A game that uses the Steamworks API does so through its
own Steamworks binding package and the Steamworks SDK's native libraries, which
load as precompiled native plugins (RUN-001 in
[Scripting runtime](scripting-runtime.md)). The Steam overlay, Steam Input,
and the other client features that hook the game process work, because the
player presents through the graphics APIs and process model those features
support. A development build places the application identifier file the
project declares next to the executable; a release build omits it.

Uploading is a separate, explicit step. The editor runs the storefront's own
upload tool, such as SteamPipe for Steam or the tool another storefront
provides, with credentials the developer configured, and reports the
storefront's result. Uploading is a person-only action
([People and person-only actions](automation-and-mcp.md#people-and-person-only-actions)):
an agent can prepare the build and the upload configuration and request the
upload. The build result states which license governs the player and its
obligations for the chosen storefront, as [Licensing](#licensing) defines.

## Headless command-line operation and continuous integration

unitas runs every project operation from the command line on a machine with no
display, no visual editor, and no interactive session. The command line drives
the headless editor defined in [Automation and MCP](automation-and-mcp.md) and
performs the same editor operations; it is not a second implementation. A single invocation can:

* open a project and import or reimport its assets;
* compile project scripts and report diagnostics;
* run project tests, as defined by TST-003 in
  [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md);
* run structured scenarios, as defined by TST-002 in the same specification;
* produce player builds for one or more targets;
* execute a named static editor method in the project, corresponding to Unity's
  `-executeMethod`, with editor-time script behavior as defined by RUN-010 in
  [Scripting runtime](scripting-runtime.md).

Every Unity 2022.3 LTS editor command-line argument that selects a project,
an operation, a build, a test option, or logging is accepted, and each selects
the corresponding operation or option defined in this section. These include
`-batchmode`, `-nographics`, `-quit`, `-projectPath`, `-buildTarget`,
`-standaloneBuildSubtarget`, the per-target player build arguments (such as
`-buildWindows64Player`, `-buildOSXUniversalPlayer`, and
`-buildLinux64Player`), `-executeMethod`, `-logFile`, `-stackTraceLogType`,
`-runTests`, `-testPlatform`, `-testResults`, `-testFilter`,
`-testCategory`, `-assemblyNames`, `-accept-apiupdate`, and
`-disable-assembly-updater`. Unity license arguments (such as `-serial`,
`-username`, `-password`, and `-returnlicense`) are accepted without effect,
because unitas needs no Unity license, and the output states that they were
ignored. An argument that Unity 2022.3 LTS does not define is reported by
name; it is never silently ignored.

In the editor, `-nographics` removes the window and presentation surface.
Offscreen rendering for captures, visual checkpoints, and tests remains, and a
session with no GPU adapter, hardware or software, reports that.

Every invocation produces machine-readable output that states each requested
operation, its outcome, its diagnostics with locations, and the location of its
artifacts and logs, using the same result shapes that automation returns. Logs
are written to the requested location with each entry attributed to the
operation that produced it.

The exit status is zero only when every requested operation succeeded. Distinct
documented nonzero statuses identify invalid invocation, a project that could
not be opened, a project that is not trusted for code execution, compilation
failure, test or scenario failure, build failure, missing toolchain or
entitlement, and cancellation or timeout. A terminated or timed-out invocation
leaves no artifact presented as successful and reports the operation that was
interrupted.

A headless run of a project that is not trusted performs only operations that
execute no project code and reports each operation it refused. Trust is granted
only by the user, through the mechanism owned by PRJ-008 in
[Projects and assets](project-and-assets.md). The trust argument defined by
PRJ-008 is the only invocation input that trusts a project.

Builds started from project scripts through `BuildPipeline.BuildPlayer`, and
build callbacks such as `IPreprocessBuildWithReport` and
`IPostprocessBuildWithReport`, behave as defined by RUN-010 in
[Scripting runtime](scripting-runtime.md); their results are reported with the
same build result as any other build.

unitas-owned caches can be preserved and restored between continuous-
integration runs. A restored cache whose recorded inputs match the project
state is reused; a stale, incompatible, or corrupt cache is detected, rebuilt,
and reported. A run using a restored cache produces the same results and
artifacts as a run from a clean state.

Concurrent invocations, on different projects or on the same project, follow
the headless session and same-project concurrency rules owned by AUT-004 in
[Automation and MCP](automation-and-mcp.md).

## Scripting backends, ahead-of-time compilation, and code stripping

Managed code runs with just-in-time compilation on targets that permit it, and
with ahead-of-time compilation on targets that forbid just-in-time compilation,
including iOS, licensed consoles, and browser delivery. The project's Unity
scripting backend setting (Mono or IL2CPP) is honored: Mono selects
just-in-time compilation and IL2CPP selects ahead-of-time compilation. Selecting
a backend that the target does not permit is reported as an invalid
configuration before the build starts.

Ahead-of-time targets run the same managed code with the same behavior as
just-in-time targets. A game developer moving a project from a desktop target
to iOS, a console, or the browser changes no code to keep it running:

* **Generics:** every generic instantiation runs, including instantiations
  that no code names at build time. Full generic sharing covers generic
  virtual and interface methods, instantiations over value types and
  reference types created through `MakeGenericType` and `MakeGenericMethod`,
  and generic types that deserializers and dependency-injection containers
  construct at runtime. A shared instantiation has the same results as one
  compiled for its exact type arguments, and the build result lists the
  instantiations it shares as a performance finding.
* **Reflection-based serializers:** `XmlSerializer`, `DataContractSerializer`,
  `BinaryFormatter`, `System.Text.Json` and Newtonsoft Json in their
  reflection modes, and serializers that generate code at runtime produce the
  same output and read the same input as on just-in-time targets.
* **Dynamic code:** `System.Reflection.Emit`, `DynamicMethod`, compiled
  expression trees, and the C# `dynamic` keyword run on every target. Where
  the target forbids generating native code at runtime, the code runs through
  an interpreter with the same results, and the build result lists each
  statically detectable use, naming the type and member, as a performance
  finding. `RuntimeFeature.IsDynamicCodeSupported` reports true on every
  target. `RuntimeFeature.IsDynamicCodeCompiled` reports whether that code is
  compiled to native code or interpreted, so libraries that choose a faster
  path for interpreted targets keep working.
* **Reflection:** reflection over types, members, attributes, and generic
  definitions returns the same results on every target, subject only to the
  project's stripping settings below.

The only managed-code failures specific to ahead-of-time targets are those
that the project's own stripping settings cause. Unity 2022.3 LTS raises
`ExecutionEngineException` or `NotSupportedException` for some of these
operations under IL2CPP. In unitas, code that catches those exceptions to
choose a fallback path keeps working, because the guarded operation
succeeds.

The Unity 2022.3 LTS managed stripping levels (Disabled, Minimal, Low, Medium,
and High) remove unused managed code with equivalent
observable effect. `link.xml` files in the project and its packages,
`[Preserve]`, `[AlwaysLinkAssembly]`, and `[RequireAttributeUsages]` are honored.
The build result reports the stripping level used and the assemblies, types,
and members removed. In development builds, access to a stripped type or
member, including access through reflection by name, raises an exception that
names the member, the stripping level, and the preservation mechanisms that
would retain it.

Burst-compiled code behavior is owned by RUN-011 in
[Scripting runtime](scripting-runtime.md); whether native acceleration is
available on a target is reported as a performance property of the build.

## Target families

Targets are Windows x86-64, x86, and ARM64, Universal Windows Platform
x86-64 and ARM64, macOS Apple Silicon and Intel, Linux x86-64 and AArch64,
Android ARM64, ARMv7, and x86-64 (including Android TV and ChromeOS devices),
iOS ARM64 and the iOS simulator, tvOS ARM64, browser delivery, Linux-based
handheld devices (for example Steam Deck), XR devices reached through the XR
Plugin Management providers that CMP-002 lists, and embedded Linux. Licensed
console environments (PlayStation, Xbox, and Nintendo Switch families) and
QNX are supported when the platform holder's SDK is installed and the user
holds the platform holder's entitlement. Every target delivers the full set of
feature families that its hardware can run; a family is never withheld from a
target because of engine work.

Compatibility reporting distinguishes targets and capabilities derived from the
Unity 2022.3 LTS baseline from unitas extensions such as WebGPU, visionOS-
class devices, and architecture variants that Unity 2022.3 LTS does not offer.
An extension is not presented as a Unity 2022.3 LTS target.

A target carries one of the compatibility states defined in the
[Compatibility contract](compatibility-contract.md); an Externally unavailable
target states whether the cause is the current host (for example an Apple
target on a host without the required Apple toolchain), a missing toolchain, or
a missing user entitlement.
Proprietary platform access is never implied by the existence of a target name.

Targets without a section of their own follow the sections named here:
embedded Linux follows the [Linux target](#linux-target); Linux-based handheld
devices follow the Linux target as well; Android-based standalone XR headsets
follow the [Android target](#android-target) and
[XR and spatial behavior](#xr-and-spatial-behavior); visionOS-class devices
follow the [iOS and tvOS targets](#ios-and-tvos-targets) and XR and spatial
behavior; QNX follows [Console targets](#console-targets) for SDK and
entitlement rules and reports its packaging, graphics API, lifecycle, and file
locations in the build result.

## Dedicated server players

Windows, Linux, and macOS dedicated server players build from the Dedicated
Server target and run with no rendering or audio output. A project migrated
from Unity 2020.3 LTS with the Server Build option enabled opens with the
Dedicated Server target selected. A server build defines `UNITY_SERVER` and
applies server-build asset stripping. A server player behaves as if launched
with `-batchmode -nographics`, whether or not those arguments are given.
Rendering and audio components keep their serialized state and API values.
Only operations that need a GPU or audio device, such as drawing, readback, and
playback, are reported as Platform-limited, once per API, naming it.

## Player lifecycle

The player starts in the configured first scene and exposes startup failures.
Focus, pause, resume, low-memory notification (`Application.lowMemory`) on
targets that provide it, quit requests, and shutdown follow the target's
application lifecycle and deliver the Unity lifecycle messages
(`OnApplicationFocus`, `OnApplicationPause`, `OnApplicationQuit`) defined by
RUN-003 in [Scripting runtime](scripting-runtime.md).

Window, display, full-screen, safe-area, orientation, input-device, cursor,
clipboard, file-selection, deep-link, notification, permission, vibration,
haptic, and controller features accurately report support and denial.
Unavailable capabilities return explicit state rather than apparent success.

Editor play and built-player behavior remain equivalent for every runtime
feature except where an editor-only capability is explicitly identified.

## Web target

The Web target, the browser delivery of PLT-001, is detailed in
[Web target](web-target.md). PLT-001 remains owned by this specification. The
Web target document owns no catalog feature; its acceptance outcomes
demonstrate PLT-001 together with the features they name, and every rule of
this specification applies to the Web target unless that document states what
the browser changes.

## Windows target

The Windows target covers the Windows standalone player and the Universal
Windows Platform (UWP) player.

### Windows standalone

* **Architectures:** x86-64, x86, and ARM64. A standalone ARM64 player is a
  unitas extension, because Unity 2022.3 LTS offers ARM64 on Windows only
  through UWP. `-buildWindows64Player` and `-buildWindowsPlayer` select the
  x86-64 and x86 players.
* **Packaging:** a folder with the executable named after the product, its
  `<Product>_Data` folder, and the files the player needs. The Copy PDB files
  and Create Visual Studio Solution build options are honored. Store and
  launcher packaging is done by the developer's tools, outside the build.
* **Signing:** Authenticode signing is reported as defined in
  [Signing, notarization, and store packaging](#signing-notarization-and-store-packaging).
* **Graphics APIs:** the project's Graphics APIs list (Direct3D 11, the
  default, Direct3D 12, Vulkan, and OpenGL Core) is honored in its order, with
  fallback to the next entry when one fails to start. Each entry is served by a
  render-graph backend: Direct3D 12, Vulkan, and OpenGL Core by their own
  backends, and Direct3D 11 by the Direct3D 12 backend at Direct3D 11-class
  feature levels. `SystemInfo.graphicsDeviceType` returns the type of the
  requested entry, and the startup report names the requested entry, the
  backend that serves it, and the reason any earlier entry was skipped. Direct3D 12 ray tracing is
  available on hardware that offers it.
* **Lifecycle:** minimizing delivers focus and pause messages unless Run In
  Background is on. Closing the window calls `Application.wantsToQuit`, then
  `OnApplicationQuit`. System shutdown and sign-out deliver `OnApplicationQuit`
  within the time Windows allows. A graphics device removal or driver reset is
  recovered with rendering resumed, or reported as an unrecoverable graphics
  error naming the cause.
* **Windowing and display:** Exclusive Fullscreen, Fullscreen Window,
  Maximized Window, and Windowed modes; per-monitor DPI awareness; multiple
  displays; HDR output; and window resizing behave as REN-007 in
  [Rendering and animation](rendering-and-animation.md) defines. Alt+Enter
  switches modes when Allow Fullscreen Switch is on.
* **Input:** XInput and Windows.Gaming.Input controllers, raw HID devices,
  pen and touch, the mouse with raw movement, and IME text input reach the
  legacy Input Manager and the Input System.
* **File locations:** `Application.persistentDataPath` is
  `%USERPROFILE%\AppData\LocalLow\<company>\<product>`, where `Player.log` is
  written as well. `PlayerPrefs` are stored in the registry under
  `HKCU\Software\<company>\<product>`, as in Unity.
* **Platform services:** store, launcher, and platform SDKs that the project
  integrates through native plug-ins run when their SDK and account are
  present. They are reported as defined in
  [Online and platform services](#online-and-platform-services).
* **Player Settings:** Fullscreen Mode, Default Is Native Resolution,
  Resizable Window, Visible In Background, Allow Fullscreen Switch, Force
  Single Instance, Use Player Log, Capture Single Screen, Run In Background,
  the DXGI flip model swap chain for Direct3D 11, and the D3D12 and Vulkan
  options are honored.

### Universal Windows Platform

* **Architectures:** x86-64 and ARM64.
* **Packaging:** a Visual Studio solution of the XAML or Direct3D build type,
  from which the developer's tools produce an MSIX package. The build result
  records the solution's location and the package form it produces.
* **Signing and store:** package signing and the Microsoft Store form are
  reported as defined in
  [Signing, notarization, and store packaging](#signing-notarization-and-store-packaging).
* **Capabilities:** the capabilities declared in Publishing Settings are
  written to `Package.appxmanifest`. A use of a capability the manifest does
  not declare returns the platform's denial.
* **Lifecycle:** suspension delivers `OnApplicationPause(true)` within the
  deadline Windows sets. Resumption delivers the reverse. A terminated
  suspended app starts fresh at its next launch.
* **File locations:** `Application.persistentDataPath` is the package's
  `LocalState` folder.
* **APIs:** `UnityEngine.WSA` members and `ENABLE_WINMD_SUPPORT` code behave as
  in Unity.

## macOS target

* **Architectures:** Apple Silicon, Intel, and universal binaries that hold
  both, selected by the Architecture build setting.
  `-buildOSXUniversalPlayer` builds the universal player.
* **Packaging:** an application bundle (`.app`), or an Xcode project when
  Create Xcode Project is on. The bundle identifier, version, build number,
  and category from Player Settings are written to `Info.plist`.
* **Signing, hardened runtime, and notarization:** signing identity,
  entitlements, and notarization are reported as defined in
  [Signing, notarization, and store packaging](#signing-notarization-and-store-packaging).
  Distribution builds sign with the hardened runtime. The entitlements a Mono
  player needs for just-in-time compilation are added, and the build result
  names them.
* **Sandbox and Mac App Store:** with Mac App Store Validation on, the bundle
  is sandboxed and packaged in the form the Mac App Store requires. The
  sandbox entitlements the project declares, such as network client and
  server, camera, and microphone, gate those capabilities. An access the
  sandbox refuses returns the platform's denial. Usage descriptions for
  protected resources are written from Player Settings.
* **Graphics:** Metal. Every rendering feature in
  [Rendering and animation](rendering-and-animation.md) runs on Metal.
* **Lifecycle:** hiding and unhiding the app, switching Spaces, and focus
  changes deliver focus and pause messages. Command-Q calls
  `Application.wantsToQuit`, then `OnApplicationQuit`.
* **Windowing and display:** Fullscreen Window, Maximized Window, and Windowed
  modes. macOS has no exclusive fullscreen, so that request reports the
  resolved mode. Retina scaling follows the Mac Retina Support setting.
  `Screen.safeArea` excludes the camera housing on displays that have one.
  Variable refresh rate, multiple displays, and HDR output work as REN-007
  defines.
* **Input:** keyboards, the mouse and trackpad, controllers that the system's
  game controller support recognizes, and IME text input.
* **File locations:** `Application.persistentDataPath` is
  `~/Library/Application Support/<company>/<product>`, or the equivalent
  inside the app's container when sandboxed. `PlayerPrefs` use
  `~/Library/Preferences/unity.<company>.<product>.plist`, and `Player.log`
  is written under `~/Library/Logs/<company>/<product>`, as in Unity.
* **Platform services:** Game Center through the `Social` API, and other SDKs
  the project integrates, run when their SDK, entitlement, and account are
  present.
* **Player Settings:** Bundle Identifier, Build, Category, Mac App Store
  Validation, Mac Retina Support, Use Player Log, Fullscreen Mode, Resizable
  Window, and Run In Background are honored.

## Linux target

* **Architectures:** x86-64 and AArch64. AArch64 is a unitas extension.
  `-buildLinux64Player` builds the x86-64 player.
* **Packaging:** a folder with the executable (`<Product>.x86_64` or the
  AArch64 equivalent) and its `<Product>_Data` folder, in the form a
  distribution channel such as a Steam depot expects.
* **Signing:** Linux players are unsigned unless the distribution channel
  signs them. The build result reports this as the target's signing state.
* **Graphics:** Vulkan and OpenGL Core, in the order of the Graphics APIs list,
  with the same fallback and reporting as Windows.
* **Display servers:** Wayland and X11. A native Wayland session is a unitas
  extension. Under Wayland, a project setting runs the player through the X11
  compatibility layer instead. The startup report names the display server in
  use. Fractional scaling, multiple displays, and fullscreen modes behave as
  REN-007 defines.
* **Lifecycle:** focus, pause, and quit follow the desktop rules above.
  System suspend and resume deliver `OnApplicationPause`.
* **Input:** keyboards, the mouse, touch, gamepads through the system's input
  devices, and IME text input through the desktop's input method.
* **File locations:** `Application.persistentDataPath`, `PlayerPrefs`, and
  `Player.log` are under `~/.config/unity3d/<company>/<product>`, as in Unity.
* **Steam Deck and handhelds:** the player runs natively under the Steam
  Linux runtime and the Steam Deck compositor at the device's resolution. The
  built-in controls, touch screen, and suspend and resume work. The Steam
  on-screen keyboard and other Steam services run when Steamworks is
  integrated and its account is present. Other Linux handhelds receive the
  same treatment for their controls and displays.
* **Player Settings:** the Linux resolution, fullscreen, and Run In Background
  settings are honored.

## Mobile behavior

These rules apply to the Android and iOS players. Target-specific behavior
follows in [Android target](#android-target) and
[iOS and tvOS targets](#ios-and-tvos-targets).

Mobile players handle safe areas, orientation, touch cancellation, application
backgrounding and foregrounding, interruption, permissions, constrained memory,
and supported device changes. A tester can determine whether a pause came from
game logic, operating-system interruption, or focus loss.

* **Safe area and cutouts:** `Screen.safeArea` and `Screen.cutouts` reflect the
  device and update when orientation changes.
* **Orientation:** allowed orientations and auto-rotation from Player
  Settings apply. Rotation changes `Screen.orientation` and the screen size
  from the next frame.
* **Memory:** operating-system memory warnings raise `Application.lowMemory`.
  An app that the system ends in the background starts fresh at its next
  launch, and its saved data remains as SAV-001 defines.
* **Permissions:** runtime permission requests return granted, denied, or
  denied permanently, and each state is observable.
* **Thermal state:** thermal and low-power state are reported where the
  operating system exposes them.
* **Embedding:** a project can be built as a library that a native app hosts,
  as Unity 2022.3 LTS's Unity as a Library does, with the same lifecycle
  messages.

## Android target

* **ABIs:** ARM64, ARMv7, and x86-64, selected by Target Architectures. Split
  APKs by target architecture is honored.
* **Packaging:** an APK, an Android App Bundle (AAB), or an exported Gradle
  project, as defined in [Native project generation](#native-project-generation).
  Play Asset Delivery packs (install-time, fast-follow, and on-demand) and
  Split Application Binary are honored for app bundles.
* **Signing and store:** keystore and alias reporting is defined in
  [Signing, notarization, and store packaging](#signing-notarization-and-store-packaging).
  The build reports whether it meets Google Play's current requirements: the
  target API level (as CMP-005 declares), 64-bit code for every included
  32-bit ABI, the app bundle form for new apps, and support for 16 KB memory
  pages. A requirement the build does not meet is reported as store
  submission blocked, while local and development builds stay available.
* **Graphics:** Vulkan and OpenGL ES 3.x, in the order of the Graphics APIs
  list. A device on which Vulkan fails to start falls back to the next API,
  and the startup report names the device and the cause.
* **Lifecycle:** activity pause and resume deliver focus and pause messages.
  Trim-memory notifications raise `Application.lowMemory`. The Activity and
  GameActivity application entry points both deliver the same lifecycle. The
  back button reaches the game as the Escape key and the Input System back
  control, and `Input.backButtonLeavesApp` keeps its Unity meaning.
* **Windowing and display:** multi-window and resizable windows (Resizable
  Window), foldable posture and size changes, Render Outside Safe Area,
  Optimized Frame Pacing, Target DPI resolution scaling, and the device's
  refresh rates through `Screen.resolutions` all apply.
* **Input:** multi-touch, stylus, gamepads, hardware keyboards and mice, and
  sensors. Android TV remotes and D-pads drive UI navigation. On ChromeOS, mouse,
  keyboard, and touchpad input arrive as desktop input when ChromeOS Input
  Emulation is off.
* **File locations:** `Application.persistentDataPath` follows the Write
  Permission setting (internal or external storage), as in Unity.
  `PlayerPrefs` use the app's shared preferences, and player logs go to
  logcat.
* **Platform services:** Google Play services, billing, and other SDKs that the
  project integrates through plug-ins or packages run when their SDK and
  account are present. `UnityEngine.Android.Permission` requests runtime
  permissions.
* **Android TV and ChromeOS:** Android TV Compatibility adds the TV launcher
  entry and banner. ChromeOS devices run the ARM64 or x86-64 player as their
  processor requires.
* **Player Settings:** Package Name, Version and Bundle Version Code, Minimum
  and Target API Level, Install Location, Internet Access, Write Permission,
  Filter Touches When Obscured, Sustained Performance Mode, Low Accuracy
  Location, Texture Compression Formats for app bundles (ASTC and ETC2), and
  the settings named above are honored.

## iOS and tvOS targets

* **Packaging:** an Xcode project, as defined in
  [Native project generation](#native-project-generation). The project builds
  for devices or for the simulator, as the Target SDK setting selects. On Apple
  Silicon hosts, simulator builds are ARM64.
* **Signing and App Store:** signing identity, provisioning profile, and
  entitlements are reported as defined in
  [Signing, notarization, and store packaging](#signing-notarization-and-store-packaging).
  The build reports whether it meets the App Store's current requirements: the
  minimum Xcode and SDK versions (as CMP-005 declares), the privacy manifest
  (PRD-002), usage descriptions for every protected resource the game uses,
  required icons, and a launch screen. A requirement the build does not meet
  is reported as store submission blocked.
* **Graphics:** Metal. Every rendering feature in
  [Rendering and animation](rendering-and-animation.md) runs on Metal.
* **Lifecycle:** resigning active, entering the background, and returning
  deliver focus and pause messages. Behavior In Background (Suspend, Exit, and
  Custom) is honored. Memory warnings raise `Application.lowMemory`. Audio
  session interruptions, such as calls, behave as AUD-001 defines.
* **Windowing and display:** safe areas, ProMotion refresh rates through
  `Screen.resolutions`, Requires Fullscreen, status bar visibility, home
  indicator auto-hide, deferred system gesture edges, and external displays
  all apply.
* **Input:** multi-touch, Apple Pencil, game controllers that the system's game
  controller support recognizes, and hardware keyboards and pointers on iPad.
  On tvOS, the Siri Remote acts as a gamepad. `UnityEngine.tvOS.Remote`
  settings and Require Extended Game Controller keep their Unity meaning.
* **File locations:** `Application.persistentDataPath` is the app's
  `Documents` folder and `temporaryCachePath` is `Library/Caches`, as in Unity.
  `PlayerPrefs` use the app's user defaults.
  `UnityEngine.iOS.Device.SetNoBackupFlag` excludes a file from backup. On tvOS,
  the operating system may purge local storage other than user defaults. The
  player reports that storage class.
* **Platform services:** Game Center through the `Social` API, in-app
  purchases, push notifications, and on-demand resources run when their
  entitlement and account are present.
* **Player Settings:** Bundle Identifier, Build, Target minimum iOS Version,
  Target Device, Target SDK, the camera, microphone, and location usage
  descriptions, Requires Persistent WiFi, Supported URL schemes, Accelerometer
  Frequency, Mute Other Audio Sources, Prepare iOS for Recording, Force iOS
  Speakers when Recording, Render Extra Frame on Pause, and the settings named
  above are honored.

## Console targets

PlayStation, Xbox, and Nintendo Switch family targets are supported when the
platform holder's SDK is installed and the user holds the platform holder's
entitlement. Without them, the target is Externally unavailable with the cause
named, as defined in [Target families](#target-families). This section states
only publicly known expectations and implies no access to any platform
holder's SDK, documentation, or programs.

* **Packaging:** a console build produces the package form that the installed
  SDK's tools define, and deploys to a development kit as BLD-004 defines.
* **Signing and submission:** signing and packaging state are reported as
  defined in
  [Signing, notarization, and store packaging](#signing-notarization-and-store-packaging).
  A console build is never presented as having passed the platform holder's
  certification. Distribution requires the commercial license, as defined in
  [Licensing](#licensing).
* **Lifecycle:** the platform's suspend, resume, constrained, and quit events
  deliver the Unity lifecycle messages (RUN-003).
* **Display and input:** the platform's display modes and controllers reach
  the game through the same Unity APIs as on other targets. Signed-in users and
  controller pairing are reported to the game where the platform exposes them.
* **File locations and services:** save storage, user accounts, achievements,
  and other platform services follow the SDK and are reported as defined in
  [Online and platform services](#online-and-platform-services).
* **Settings:** console-specific Player Settings stored in the project are
  preserved and honored as the installed SDK defines. Their values are never
  written to build results, logs, or diagnostics that leave the developer's
  machine.

## XR and spatial behavior

XR behavior is provided through XR Plugin Management, the OpenXR provider
plug-in (`com.unity.xr.openxr`), XR Hands (`com.unity.xr.hands`), and the XR
Interaction Toolkit within the version lines that CMP-002 lists. An XR
provider plug-in is reported against its own package and version; a provider
that the matrix does not list is Unknown. XR content imports with its
identity, and the public types and members of those packages keep their Unity
2022.3 LTS names and signatures; the behavior is the behavior this section
defines, and every known difference from Unity is listed in the API reference
owned by AUT-006 in [Automation and MCP](automation-and-mcp.md).

The managed code of each listed XR package runs as shipped. unitas provides the
engine APIs and native subsystems that code calls, including an OpenXR provider
that takes the place of the package's native provider library.

XR exposes tracked head and controller poses, tracking state, declared actions,
device capabilities, render views, recentering, interaction, the haptics each
device offers, and focus state.

Projects distinguish unavailable hardware, initialization failure, denied
permission, lost or limited tracking, features the device or runtime does not
offer, and application interruption. World scale, seated or standing mode,
locomotion, comfort choices, world-space UI, selection, grabbing, and spatial
audio have inspectable state.

XR playtests record device, runtime, refresh behavior, frame timing, tracking
events, input capabilities, comfort settings, and failures. A desktop preview is
not represented as proof of real-device correctness.

### OpenXR

As a developer targeting OpenXR devices, I select the OpenXR loader per target
in XR Plugin Management, and the player starts its XR session on the device's
active OpenXR runtime.

* **Runtime and session:** the player reports the active runtime's name,
  version, and the OpenXR API version it negotiated. The OpenXR session states
  (idle, ready, synchronized, visible, focused, stopping, loss pending, and
  exiting) are observable. Visible and focused drive `OnApplicationFocus` and
  `OnApplicationPause` (PLT-001). Loss pending ends the session and reports
  it; the player restarts the session when the runtime becomes available again.
* **Features and feature groups:** the `OpenXRFeature` set enabled per target
  in OpenXR settings decides which OpenXR extensions the player requests. The
  same holds for project-supplied `OpenXRFeature` subclasses, which run their
  hooks in a trusted project (PRJ-008). An enabled feature whose extension the
  runtime does not offer stays disabled. It reports the feature and the
  missing extension, and the rest of the session continues.
* **Interaction profiles:** each enabled interaction profile (for example
  Khronos Simple Controller, Oculus Touch, Valve Index, HTC Vive, Microsoft
  Motion Controller, Eye Gaze, and Hand Interaction) produces the Input System
  devices and controls that the profile defines (INP-002). The runtime's
  current binding for each hand is observable. Haptic impulses sent to a
  controller reach the runtime with their amplitude, duration, and frequency.
* **Views and spaces:** the runtime's preferred view configuration defines the
  render views, which are drawn as the Stereo rendering section of
  [Rendering and animation](rendering-and-animation.md) defines, with depth
  submitted where the runtime accepts it. The tracking origin mode (device, floor, or unbounded where
  the runtime offers it) and `XRInputSubsystem.TryRecenter` behave as the
  device reports them. Play area boundary points are readable when the runtime
  provides them.
* **Frame timing:** rendered views use head and eye poses predicted for the
  frame's display time that the runtime reports. Each late or missed frame is
  counted and reported per session with its duration. The refresh rates the
  runtime offers are listed, the project's choice among them is applied, and
  the active rate is observable.
* **Failures:** no active runtime, a runtime that refuses to create a session,
  and a runtime that does not offer a required view configuration each
  produce a distinct initialization failure that names the runtime and the
  cause. With the XR Plugin Management setting to initialize on startup off,
  the project starts the loader by script, and the failure is returned to that
  script.

### Hand tracking

As a developer of hand-tracked interaction, I read tracked hands through XR
Hands on any device whose runtime exposes hand tracking.

* `XRHandSubsystem` provides a left and a right `XRHand`. Each hand reports
  its tracking state, its root pose, and the pose and tracking state of each
  `XRHandJointID` joint (wrist, palm, and four or five joints per finger).
  Each joint's radius and velocities are reported where the runtime provides
  them.
* `updatedHands` is raised for each hand update with its update type (dynamic
  or before render) and flags that name which hands and joints changed. A hand
  that loses tracking reports not tracked with its last pose, and it reports
  tracked again when it is reacquired.
* Hand shape and hand pose assets (`XRHandShape`, `XRHandPose`) evaluate
  against the current joints with their authored tolerances. Gestures built on
  them report detection and loss.
* The OpenXR Hand Interaction profile and the hand-tracking aim data make pinch,
  poke, grab, and aim usable by the XR Interaction Toolkit's interactors.
* A device without hand tracking reports that hand tracking is absent, and the
  subsystem is not started; projects that offer controller input continue
  with controllers.

### AR Foundation

As a developer of an augmented-reality app, I use AR Foundation with the Apple
ARKit and Google ARCore providers within the version lines CMP-002 lists, on
devices that expose AR tracking.

* **Session:** `ARSession` reports its state (unsupported, checking
  availability, needs install, ready, session initializing, and session
  tracking) and the reason tracking is limited. Installing or updating the
  device's AR services is offered where the provider supports it.
* **Trackables:** planes, point clouds, anchors, tracked images, tracked
  faces, and environment probes are added, updated, and removed through their
  managers with the provider's tracking state, and raycasts against planes,
  feature points, and depth return hits with their trackable and pose.
* **Camera and environment:** the camera background renders through the
  render graph in every listed render pipeline, and light estimation and
  occlusion values are reported where the provider offers them.
* **Capabilities:** each subsystem descriptor states what the device and
  provider support. A manager whose subsystem the device lacks is reported as
  Platform-limited, naming the device, provider, and feature, and the rest of
  the session continues.
* **Editor and headless:** sessions in the editor and in headless play
  sessions run against a simulated environment whose trackables the session
  declares, and that environment is always reported as simulated.

## Networking behavior

Networked play is provided through Netcode for GameObjects, Netcode for
Entities (`com.unity.netcode`), and Unity Transport (`com.unity.transport`),
within the version lines that CMP-002 lists. Netcode content imports with its
identity, and the API shape below keeps its Unity 2022.3 LTS names and
signatures; the behavior is the behavior this section defines, and every known
difference from Unity is listed in the AUT-006 API reference.

The managed code of each listed networking package runs as shipped. unitas
provides the sockets, jobs, and engine APIs that code calls.

When a supported networking package and version is declared for the project, a
game can represent sessions, connections, player identity, ownership, messages,
synchronized state, remote actions, timeouts, disconnection, reconnection, late
joining, and incompatible versions.

As a player, I receive distinct outcomes for successful connection, refused
connection, unreachable endpoint, full session, authentication failure, version
mismatch, timeout, host departure, and deliberate disconnection.

As a developer, I can associate logs, objects, state changes, traffic, and
errors with the correct participant. Multiple local test instances, including
headless instances, retain distinct identity. Simulated latency, jitter, loss,
duplication, and reordering, including those that the Unity Transport
simulator applies, are visibly marked as test conditions.

Network success is separate from local gameplay success. A locally correct
action is not reported as synchronized until the remote observation that the
scenario or test declares occurs. Conflicts, authority rejection, and stale
state are explicit.

### Netcode for GameObjects

* **`NetworkManager` start modes and connection approval:** `StartHost` runs a
  server and a local client in one instance, `StartServer` runs a server with
  no local client, and `StartClient` connects to a server. A start call while
  the instance is already running returns `false` and is reported. `Shutdown`
  disconnects every client and despawns every spawned object. When connection
  approval is enabled, no client's player object is spawned until the approval
  callback approves it; a rejected client receives the refused-connection
  outcome with the reason the callback supplied.
* **`NetworkObject` spawn, despawn, and ownership:** only the server spawns
  and despawns network objects; a client call raises an exception naming the
  object. A spawned object appears on every connected client, and on each
  late-joining client, with the same `NetworkObjectId`. `ChangeOwnership` and
  `RemoveOwnership` deliver `OnLostOwnership` to the previous owner and
  `OnGainedOwnership` to the new owner. `OnNetworkSpawn` and `OnNetworkDespawn`
  run on every participant that observes the spawn or despawn.
* **`NetworkVariable` permissions:** a write by a participant that the
  variable's write permission (server or owner) does not allow raises an
  exception and sends nothing. A participant that the read permission
  (everyone or owner) excludes never receives the value. `OnValueChanged` runs
  on each receiving participant with the previous and new values, and a
  late-joining client receives the current value when the object spawns for it.
* **`ServerRpc` and `ClientRpc` delivery:** a `ServerRpc` runs only on the
  server; with `RequireOwnership` set, a call from a non-owner is rejected and
  reported, naming the RPC and the caller. A `ClientRpc` runs on every client,
  or on the clients that its `ClientRpcParams` target. Reliable RPCs sent on the
  same object arrive in the order they were sent.
* **`NetworkTransform` and `NetworkAnimator`:** the authoritative participant
  (the server, or the owner when the component declares owner authority)
  changes the synchronized position, rotation, and scale axes, and other
  participants receive interpolated values. A change made by a non-authoritative
  participant is overwritten by the next authoritative update and reported in
  development builds. `NetworkAnimator` replicates the authoritative animator's
  parameters, triggers, and state changes.
* **Network scene management:** the server loads and unloads network scenes;
  every client loads the same scenes and reports completion through scene
  events, and a late-joining client synchronizes the loaded scenes before it
  receives spawned objects in them.
* **Unity Transport:** the configured address and port select the endpoint, and
  the transport's connection and disconnect timeouts produce the timeout
  outcome.

### Netcode for Entities

As a developer of a server-authoritative, predicted game built on Entities
(RUN-011 in [Scripting runtime](scripting-runtime.md)), I use Netcode for
Entities in client and server worlds.

* **Worlds and bootstrap:** `ClientServerBootstrap` creates client, server,
  thin-client, and local simulation worlds as the project's bootstrap
  requests. In the editor, the Multiplayer PlayMode Tools settings decide the
  play mode type and the number of thin clients. A connection is an entity with
  `NetworkStreamConnection`. It receives `NetworkId` when the server accepts
  it, and `NetworkStreamInGame` marks it as in game.
* **Ghosts:** a prefab with ghost authoring becomes a ghost. The server
  serializes every `[GhostField]` of its ghost components into snapshots,
  using each field's quantization, smoothing, and send-data rules. Clients
  receive interpolated ghosts and predicted ghosts according to the ghost's
  supported and default modes, and owner-predicted ghosts are predicted on
  the owner's client. `GhostOwner` identifies the owner. Ghost relevancy
  rules, importance scaling, and the snapshot size limit decide which ghosts
  each connection receives, and a ghost that leaves relevancy is despawned on
  that client.
* **Time and prediction:** `NetworkTime` reports the server tick, interpolation
  tick, and prediction tick on clients. `PredictedSimulationSystemGroup`
  rolls predicted ghosts back to the last received snapshot and resimulates
  to the current prediction tick, with partial ticks where the frame calls
  for them. A misprediction is corrected in the next resimulation, and
  prediction-error smoothing applies.
* **Commands, inputs, and RPCs:** `IInputComponentData` and `ICommandData`
  carry each client's input for each tick to the server and to prediction.
  `InputEvent` fields count discrete presses exactly once. `IRpcCommand`
  RPCs are delivered reliably in send order to the server or to the targeted
  connections, and an RPC that is not registered on the receiver is reported
  with its type.
* **Physics and lag compensation:** predicted physics runs inside the
  prediction loop. With lag compensation enabled, server queries against
  `PhysicsWorldHistorySingleton` use the collision world at the tick the
  client saw.
* **Failures:** a protocol version mismatch between client and server, which
  covers the netcode version, the game version, and the RPC and component
  collection hashes, refuses the connection with the version-mismatch outcome.
  Ghosts that do not fit in a snapshot are sent in later snapshots in order
  of importance, and development builds report the deferral.

### Unity Transport

As a developer, I use Unity Transport directly or through either netcode
package.

* **Driver and connections:** `NetworkDriver` binds, listens, connects, and
  disconnects with IPv4 and IPv6 `NetworkEndpoint` values. `ScheduleUpdate`
  and `PopEvent` deliver connect, data, and disconnect events per connection,
  and each disconnect carries its reason (timeout, closed by remote, refused,
  maximum connection attempts reached, or protocol error).
* **Settings:** `NetworkSettings` sets connect timeout, maximum connection
  attempts, disconnect timeout, heartbeat timeout, and send and receive queue
  sizes, and each value produces the outcome it names.
* **Pipelines:** `ReliableSequencedPipelineStage` delivers every message once,
  in order, within its window size, and reports a full window to the sender.
  `UnreliableSequencedPipelineStage` drops messages that arrive after a newer
  one, and `FragmentationPipelineStage` splits and reassembles messages up to
  its payload capacity. Stages combine in the order the pipeline declares.
* **Network simulator:** `SimulatorPipelineStage`, configured through
  `WithSimulatorStageParameters`, applies its packet delay, packet jitter,
  packet drop interval or percentage, packet duplication percentage, and
  fuzzing to each packet that passes through it. Netcode for GameObjects'
  `UnityTransport.SetDebugSimulatorParameters` applies delay, jitter, and drop
  rate the same way in development builds. Every simulated condition is
  marked as a test condition in logs, profiles, and playtest records, and a
  release build contains no active simulator stage.
* **Secure and relayed connections:** `WithSecureServerParameters` and
  `WithSecureClientParameters` establish encrypted connections with the
  configured certificates, and a certificate mismatch refuses the connection
  with the authentication-failure outcome. `RelayServerData` connects through
  a relay server, and an expired or invalid relay allocation produces the
  service-failure outcome (see
  [Online and platform services](#online-and-platform-services)).
* **Browser targets:** on browser targets, the driver uses WebSocket
  connections, and a browser player connects to any server that listens with
  the WebSocket network interface. Because browsers offer no raw UDP, a UDP
  endpoint configured in a browser player reports Platform-limited, naming
  the endpoint.

## Online and platform services

Identity, matchmaking, relay, cloud storage, achievements, commerce, friends,
entitlements, and platform services are reported per service and target. The
product distinguishes unavailable integration, user cancellation, offline
state, permission denial, authentication expiration, rate limits, service
failure, and incompatible account state.

* **Outcomes:** every service call returns one of those outcomes, distinct from
  each other and from success, and the outcome is attributed in diagnostics to
  the service, the call, and the participant that made it.
* **Offline:** a game that loses connectivity receives the offline outcome for
  every pending and new call, keeps running, and receives a connectivity
  change when it returns.
* **Inspection:** for each service a project uses, the editor and automation
  show whether the integration is configured for the target and whether the
  project holds credentials for it, without revealing them.

A project request never creates a claim that the developer owns a service,
license, signing identity, or platform entitlement.

The Unity Gaming Services SDKs (`com.unity.services.*`, including
Authentication, Lobby, Relay, Cloud Save, and Analytics) in the version lines
CMP-002 lists run as shipped. With a service account and credentials
configured for the project, their calls reach the services. Without them,
calls return the services' own unavailable errors, which are reported as the
unavailable-integration outcome. Their data flows follow
[Product data flows, telemetry, and privacy](#product-data-flows-telemetry-and-privacy).

## Deployment, device connection, and development players

A developer or authorized agent can deploy, install, launch, and remove a built
player on a selected connected device or on the local host: an Android device
or emulator, an iOS or tvOS device or simulator, a standalone XR headset, a
Linux-based handheld reached over the local network, a desktop host, a local
browser, or a licensed console development kit when the platform holder's SDK
is installed.
Device discovery lists each device with its name, target, architecture,
operating system version, and connection state, and states why a listed device
is unusable, such as being locked, unauthorized for debugging, on an
incompatible operating system version, or missing a required provisioning
profile. A failed deployment names the device, the step that failed, and the
reason.

A development player opens its profiler, debugger, log-stream, and
remote-control connections only when it was built with those options. It
accepts a connection only from a client that holds the pairing credential
generated by that build. The credential is recorded in the build result, is
readable by people and by agent sessions that hold the build permission scope,
and is never written to player logs. The player reports each accepted and
refused connection. Profiler and log-stream connections additionally require
the read-project scope, and debugger and remote-control connections require the
execute-project-code scope, as owned by AUT-007 in
[Automation and MCP](automation-and-mcp.md). Deployment operations are
available through MCP under the deploy permission scope that AUT-007 defines.

## Content builds and post-release content updates

Addressables groups build for the selected target using the project's
Addressables profiles and their build and load paths. Updating a previous
content build uses that build's recorded content state to produce only the
changed bundles and a new content catalog. Every content build result states
which shipped player versions can load the content it produced, and records
the unitas version, the target, and the bundles and catalog it produced.

A unitas player loads AssetBundles and Addressables content built by Unity
2022.3 LTS and by the earlier Unity releases whose projects unitas migrates
(SER-005 in [Serialization and prefabs](serialization-and-prefabs.md)), for
the player's target, as well as content built by unitas. A Unity-built bundle
is converted to cooked form when it is first loaded, by the same importer code
the editor uses for that format, including its shader translation, as the
[Engine architecture](architecture.md#content-pipeline) defines. The result is
cached on the device by the bundle's content hash, and later loads read only
the cooked result. A shipped Unity game
can therefore move to a unitas player and keep its existing content delivery:

* **Bundle archives:** uncompressed, LZ4, LZ4HC, and LZMA bundle archives load
  through every `AssetBundle` loading form, including file, memory, stream,
  file offset, and `UnityWebRequestAssetBundle` downloads with cache, version,
  hash, and CRC checks. A CRC mismatch fails the load with the expected and
  found values.
* **Serialized content:** objects in a bundle load with the same values,
  references, and identities they have in the Unity player, whether the bundle
  carries type trees or was built without them. Script references resolve to
  the player's script types by assembly, namespace, and class name. A script
  type absent from the player leaves a missing-script record, as SER-004
  defines.
* **Shaders and materials:** shaders in Unity-built bundles are translated,
  during that conversion, by the same translation that REN-006 in
  [Rendering and animation](rendering-and-animation.md) applies to project
  shaders at import. Each variant that the bundle carries is available, and a variant
  the bundle stripped is reported by shader and keyword set.
* **Manifests, dependencies, and catalogs:** `AssetBundleManifest` content,
  bundle dependencies, and Addressables content catalogs in the forms that
  the listed Addressables line writes load and resolve. This includes remote
  catalogs with their hash files and catalogs produced by content updates.
  Dependencies resolve by bundle name across Unity-built and unitas-built
  bundles in the same player.
* **Failures:** a bundle built for another target, a bundle built by a Unity
  release outside the supported lines, a truncated or corrupt archive, and a
  catalog that names a missing bundle each report the affected bundle by name
  with the reason, and that content is not loaded partially.

A player that encounters content built for an incompatible unitas player
version reports each affected bundle by name with the reason, and does not
load that content partially. Runtime loading behavior is owned by RUN-006 in
[Scripting runtime](scripting-runtime.md).

## Licensing

unitas is dual licensed: it is available under the GNU General Public License
version 3 (GPLv3) and under a separate commercial license.

The editor, the command line, and automation each report which license governs
the current installation and project, and where that determination comes from.
A commercial license is reported as governing only when the developer has
configured a commercial license that unitas recognizes for that project;
otherwise GPLv3 governs. Absence or expiry of a configured commercial license is
reported before a build starts.

Every build result records which license governs the unitas player components
included in that player and the obligations the product associates with it. For
a GPLv3 build, the result states that the player is conveyed under the GPLv3
as a work that includes the unitas components; that the obligation to provide
Corresponding Source extends to the whole conveyed work, including project code
and content compiled or linked into it, except separate works merely
aggregated with it; and how the build makes that source available or where it
is offered. The player includes the GPLv3 license text. The result also states
that the GPLv3 distribution of unitas carries no linking exception and that
the commercial license is the route for distributing a player without these
obligations. For a commercially licensed build,
the result identifies the commercial license that governs the included unitas
components.

Each release publishes a licensing table that states, for every target, its
status with respect to GPLv3 distribution. For the targets that the release's
licensing table lists as conflicting with GPLv3 distribution (at minimum
licensed console environments, and each store whose terms the table records as
adding restrictions on recipients), the build result and target status state that
distribution requires the commercial license. A development build for
such a target is not blocked on that basis.

Every player includes the notices and license texts of the third-party
components unitas includes in it, and the build result lists those components
with their licenses. On browser targets the notices are delivered alongside the
bundle.

unitas never claims ownership of, or rights in, the developer's own content and
code. Every license report states that it is product information and not legal
advice, and identifies the license texts it refers to.

## Installation, updates, and toolchains

unitas installs on Windows, macOS, and Linux host systems, on the host
architectures each release declares. Multiple unitas versions install side by
side, and each project opens with the version the user or invocation selects.

A project records the unitas version last used to save it as unitas-specific
project content, without altering any Unity-readable file; that storage location is owned
by SER-005 in [Serialization and prefabs](serialization-and-prefabs.md).
Opening a project with a different unitas version than the recorded one states
both versions before any change is saved, and never silently migrates
unitas-owned project state to a newer form.

Releases are published on a stable channel and a preview channel. Preview
installations are visibly marked in the editor, the command line, build results,
and automation, and a preview version is never installed when the user selected
the stable channel.

An update never applies during an open editor, headless, or agent session; it is
applied only when no session is using that installation. The previously
installed version can be restored, and a restored version behaves as it did
before the update. Installation and update packages are verified before they
are applied; a failed verification or an interrupted update leaves the previous
installation usable. An offline installation package installs every selected
component without network access, and an unreachable update service never
prevents use of an installed version.

### Getting unitas

* **Installers:** each release ships signed installers for every supported host
  operating system: an Authenticode-signed Windows installer, a signed and
  notarized macOS installer, and Linux packages, plus archives that run
  without installation. The same releases are published through each host's
  common package managers.
* **Launcher:** a launcher application lists the developer's projects with the
  unitas version each uses, installs and removes unitas versions and the
  support for each build target, opens each project with its version, creates
  projects from templates, and configures licenses. Every launcher operation
  is also a command-line operation, for continuous integration.
* **Source:** the complete source of every release is public under the GPLv3,
  with each release tagged. Building from that source with the documented
  toolchain produces an installation that behaves identically to the released
  one, and release binaries are reproducible from their tagged source.
* **Continuous integration:** each release publishes container images of the
  headless editor for Linux hosts, with target support preinstalled per image.
* **Licenses:** a commercial license, where a developer holds one, is
  configured in the launcher, the editor, or the command line, and governs as
  [Licensing](#licensing) defines.

Target toolchains, including the Android SDK, NDK, and JDK, Xcode on macOS
hosts, the browser build toolchain, and the .NET toolchain, are located when
already installed or acquired with the user's consent. Each located toolchain
is validated against the version range the unitas release requires. A missing
or incompatible toolchain produces an error that names the tool, the version
found, the required range, and the action that resolves it. Accepting a
toolchain vendor's license terms is a user action; an agent can inspect
installed versions and toolchain state and can request acquisition, and cannot
accept those terms.

## Product data flows, telemetry, and privacy

A player contains no telemetry, analytics, or network connection that neither
the game nor the build's development options requested. The editor, including its MCP interface, and the command line
send project content only to MCP clients and external tools that a person authorized
(AUT-004, AUT-007 in [Automation and MCP](automation-and-mcp.md)) and to
devices and services that a person directed, such as deployment targets,
package registries, and crash-report destinations. unitas never sends project
content to its publisher or to any other party on its own initiative. Usage reporting and crash reporting for
unitas itself are off until a person enables them; when enabled, each lists the data it sends
and its destination, and neither ever blocks offline use. Apple target builds
include a privacy manifest that covers the required-reason APIs used by the
included unitas components.

## Acceptance outcomes

1. A configured project produces the expected artifact for each declared target
   or a precise target-specific incompatibility report, and the build result
   records every item listed under build results. (BLD-001, PLT-001)
2. Two builds from the same project state, choices, toolchain, and dependency
   versions show no unexplained differences, and a cancelled build leaves no
   artifact presented as a successful player. (BLD-001)
3. A macOS distribution build reports its signing identity and notarization
   status, an Android release build reports its keystore and alias, and no
   signing secret appears in any result, log, or automation output. (BLD-001)
4. On a machine with no display, one command-line invocation opens a project,
   compiles it, runs its tests and scenarios, executes a static editor method,
   and builds a player, producing machine-readable output and an exit status
   that identifies the first failed operation. (BLD-002, TST-002, TST-003)
5. A continuous-integration run that restores a cache from an earlier run
   produces the same results and artifacts as a clean run, and a corrupt cache
   is rebuilt and reported. (BLD-002)
6. Two command-line invocations on the same project at once follow the AUT-004
   same-project concurrency rules, and neither corrupts the project. (BLD-002,
   AUT-004)
7. An iOS build runs code that uses `System.Reflection.Emit`, `dynamic`, a
   compiled expression tree, and a generic virtual method instantiated only at
   runtime with the results they have under just-in-time compilation, and its
   build result lists the `Reflection.Emit` use by type and member as a
   performance finding; a development build at High stripping that reflects on
   a stripped member raises an exception naming the member and the
   preservation mechanisms; preserving it with `link.xml` removes the
   exception. (BLD-003)
8. A built player launches the configured scene and delivers focus, pause,
   permission, and shutdown messages in the order its target's section
   defines. (PLT-001)
9. Mobile and browser lifecycle changes, including backgrounding, page refresh,
   storage quota exhaustion, and lost graphics or audio context, each report
   the state change with the data preserved or the reason it was rejected.
   (PLT-001)
10. A target without the required host toolchain or user entitlement reports
    Externally unavailable with the specific cause rather than an engine
    failure. (PLT-001, PRD-001)
11. An XR play session reports whether it runs on a device or a simulated
    runtime, reports tracking loss and recovery, lists each input capability
    the device offers, and counts late and missed frames; a simulated session is
    never reported as device evidence. (XR-001)
12. The [multiplayer arena sample](sample-games/11-multiplayer-arena.md),
    which uses Netcode for GameObjects, distinguishes connection,
    synchronization, gameplay, and service failures and identifies the affected
    participant across local and headless instances. (NET-001)
13. A GPLv3 build's result records that the player is conveyed under the GPLv3,
    that the Corresponding Source obligation covers the whole conveyed work
    including project code compiled into it, that no linking exception applies,
    and the included third-party notices; a console build states that distribution requires the
    commercial license; and every license report states it is not legal advice.
    (LIC-001)
14. A build with a recognized commercial license records that license as
    governing the included unitas components, and expiry of that license is
    reported before the next build starts. (LIC-001)
15. Two unitas versions installed side by side each open the same project, which
    records the version used without changing any Unity-readable file, and an
    update that arrives during an open session is applied only after the session
    ends and can be rolled back. (PRD-001)
16. An offline installation on a host without network access installs and runs,
    and an Android build on a host with an incompatible JDK reports the tool,
    found version, required range, and resolving action. (PRD-001)
17. An agent using only MCP configures, builds, and inspects the result of a
    player build, queries license and toolchain state, and is refused when it
    attempts to accept a toolchain vendor's license terms. (BLD-001, LIC-001,
    PRD-001, AUT-001)
18. Two builds from identical inputs produce manifests whose comparison shows no
    differing inputs or outputs; changing one texture makes the comparison name
    that input and each output it changed, and each player reports its own
    `Application.buildGUID`. (BLD-001)
19. A build with three broken assets fails and reports all three; the next build
    after the fix reuses unchanged completed work and states what it redid.
    (BLD-001)
20. A release player contains no automation, debugger, or profiler endpoint and
    no build-host absolute paths, and any other development-only capability in
    a release build is named in the build result. (BLD-001)
21. A headless invocation without the PRJ-008 trust argument refuses to run a
    project's static editor method, and a project build started through
    `BuildPipeline.BuildPlayer` produces the same build result as a command-line
    build. (BLD-002, PRJ-008, RUN-010)
22. A Linux dedicated server player, launched without arguments, behaves as if
    launched with `-batchmode -nographics`, defines `UNITY_SERVER`, keeps a
    renderer's material and bounds readable, and reports its first draw call as
    Platform-limited once. (PLT-001)
23. A target listed in the release's licensing table as conflicting with GPLv3
    distribution states that distribution requires the commercial license.
    (LIC-001)
24. An agent lists connected devices, sees an Android device reported as
    unauthorized for debugging, then deploys, launches, and removes a player on
    an emulator; a development player refuses a profiler connection from a
    client without the pairing credential recorded in its build result, accepts
    one from an agent session holding the build and read-project scopes, and
    never writes the credential to its logs. (BLD-004, AUT-007)
25. After a player ships, a content update produces only the changed bundles and
    a new catalog that the shipped player loads; Addressables content and a
    remote catalog built by Unity 2022.3 LTS for the same target load in a
    unitas player with the same values and references; content built for an
    incompatible player version or for another target is reported by bundle
    name and reason and is not loaded partially. (BLD-005, RUN-006)
26. A release player built from a project that makes no network requests opens
    no network connection, the editor and its MCP interface send project content only to
    authorized MCP clients and to destinations a person directed, unitas crash
    reporting stays off until a person enables it, and an iOS
    build includes a privacy manifest for the included unitas components.
    (PRD-002)
27. An Android build honors a custom `mainTemplate.gradle` and an `.androidlib`
    folder, runs `IPostGenerateGradleAndroidProject`, and can export the Gradle
    project; an iOS build's `[PostProcessBuild]` method adds a capability
    through `ProjectCapabilityManager`; a malformed template field is reported
    by file and field. (BLD-001)
28. With connection approval enabled, a rejected client receives the refused
    outcome with the callback's reason, a client's attempt to spawn an object
    raises an exception naming it, a non-owner `ServerRpc` with
    `RequireOwnership` is rejected and reported, and a late-joining client
    receives current `NetworkVariable` values and loaded network scenes before
    spawned objects. (NET-001)
29. A project's XR Interaction Toolkit content imports with its identity, and
    each known XR behavior difference from Unity is listed in the AUT-006 API
    reference. (XR-001, AUT-006)
30. A player using the OpenXR loader starts a session on the active runtime,
    reports the runtime's name and version and each session state change,
    produces Input System devices for the Oculus Touch and Hand Interaction
    profiles, and reports an enabled feature whose extension the runtime lacks
    while the session continues; with no active runtime, initialization fails
    with a cause naming the runtime. (XR-001, INP-002)
31. On a device with hand tracking, XR Hands reports both hands' joint poses
    each update, a hand that leaves view reports not tracked and is reacquired,
    an `XRHandShape` for a pinch is detected, and the XR Interaction Toolkit
    grabs an object with a pinch; on a device without hand tracking, the
    absence is reported and controllers continue to work. (XR-001)
32. A Netcode for Entities game with a server, two clients, and a thin client
    in the editor spawns a predicted ghost owned by one client, replicates its
    `[GhostField]` values to the other client as an interpolated ghost,
    corrects a misprediction on resimulation, and refuses a client whose
    protocol version differs with the version-mismatch outcome. (NET-001,
    RUN-011)
33. A Unity Transport driver with a reliable sequenced pipeline and a
    simulator stage set to 100 ms delay, 20 ms jitter, and 5 percent drop
    delivers every reliable message once and in order, its logs mark the
    simulated conditions as test conditions, and the release build contains no
    active simulator stage; a browser player connects to the same server over
    WebSocket, as the [Web target](web-target.md) defines. (NET-001, PLT-001)
34. A Windows x86 build, an Android ARMv7 build, a tvOS build, and a Universal
    Windows Platform build each produce their target's artifact, and a console
    target on a host with the platform holder's SDK installed builds and
    deploys to a development kit. (PLT-001, BLD-001, BLD-004)
35. A macOS dedicated server player behaves as if launched with
    `-batchmode -nographics` and defines `UNITY_SERVER`. (PLT-001)
36. An iOS player and a browser player each deserialize a save file with
    `XmlSerializer` and with `BinaryFormatter` into a generic container type
    created through `MakeGenericType`, and produce the same objects and the
    same bytes on reserialization as a desktop just-in-time player. (BLD-003)
37. A console player built at Disabled stripping runs a dependency-injection
    container that emits constructors with `DynamicMethod` and resolves a
    generic service over a value type that no code names at build time.
    `RuntimeFeature.IsDynamicCodeSupported` reports true and
    `RuntimeFeature.IsDynamicCodeCompiled` reports false, and the build result
    lists the emitted code and shared instantiation as performance findings.
    (BLD-003)
38. The same project built at High stripping without a `link.xml` entry for the
    container's resolved type fails in a development build. The failure is
    the stripping exception naming the member, the stripping level, and the
    preservation mechanisms, and it is the only ahead-of-time failure the
    build or player reports. (BLD-003)
39. A Windows x86-64 build whose Graphics APIs list is Direct3D 12 then
    Direct3D 11 runs on Direct3D 12. On a device where Direct3D 12 fails to
    start, it runs its Direct3D 11 entry on the Direct3D 12 backend at a
    Direct3D 11-class feature level, and the startup report names the entry,
    the backend, and the reason. Its `PlayerPrefs` are in the
    registry and its `persistentDataPath` is under `AppData\LocalLow`, as in
    Unity. A standalone ARM64 build runs natively and is reported as a unitas
    extension. (PLT-001, BLD-001)
40. A UWP build produces a Visual Studio solution whose manifest declares the
    capabilities set in Publishing Settings. Suspending the app delivers
    `OnApplicationPause(true)` within Windows's deadline, and a use of an
    undeclared capability returns the platform's denial. (PLT-001, BLD-001)
41. A universal macOS build runs natively on Apple Silicon and on Intel. With
    the hardened runtime and a distribution identity, it reports its
    entitlements and notarization state. With Mac App Store Validation on,
    the sandboxed app writes saves inside its container and a network access
    that the entitlements do not allow is denied. (PLT-001, BLD-001)
42. A macOS player in Fullscreen Window mode on a display with a camera
    housing reports a `Screen.safeArea` that excludes it, and Command-Q calls
    `Application.wantsToQuit` before `OnApplicationQuit`. (PLT-001, REN-007)
43. A Linux x86-64 build runs under a native Wayland session and under X11, and
    its startup report names the display server. Its Vulkan device fails on a
    test machine and it falls back to OpenGL Core with the reason named. An
    AArch64 build runs natively and is reported as a unitas extension.
    (PLT-001, BLD-001)
44. A Linux build deployed to a Steam Deck over the local network runs at the
    device's resolution with its built-in controls and touch screen, survives
    suspend and resume with `OnApplicationPause`, and stores its saves under
    `~/.config/unity3d/<company>/<product>`. (PLT-001, BLD-004)
45. An Android app bundle with ARM64 and ARMv7 reports whether it meets Google
    Play's target API level, 64-bit, and 16 KB page-size requirements, and a
    build from a release that does not meet the target API level reports
    store submission blocked while a local APK still builds. The same project
    exports a Gradle project. (PLT-001, BLD-001)
46. An Android player on a device where Vulkan fails falls back to OpenGL ES 3
    and names the device and cause. A trim-memory notification raises
    `Application.lowMemory`. On Android TV, the remote's D-pad navigates the
    game's UI, and on a ChromeOS device the x86-64 player receives mouse and
    keyboard input as desktop input. (PLT-001, INP-002)
47. An iOS build produces an Xcode project that builds for a device and for
    the simulator. It reports whether it meets the App Store's Xcode, SDK,
    privacy manifest, usage description, and launch screen requirements, and
    a missing microphone usage description is reported before submission.
    (PLT-001, BLD-001, PRD-002)
48. An iOS player with Behavior In Background set to Suspend delivers
    `OnApplicationPause(true)` when backgrounded and resumes with its state. A
    memory warning raises `Application.lowMemory`. A tvOS player reads the
    Siri Remote as a gamepad and honors `UnityEngine.tvOS.Remote` settings.
    (PLT-001, INP-002)
49. On a host without a console platform holder's SDK, each console target is
    Externally unavailable with the missing SDK or entitlement named, and no
    console capability is implied. (PLT-001, BLD-001)
50. On a host with a console platform holder's SDK and the user's
    entitlement, a console build deploys to a development kit, delivers
    suspend and resume as lifecycle messages, is never presented as
    certified, and states that distribution requires the commercial
    license. Its console-specific settings do not appear in diagnostics that
    leave the machine. (PLT-001, BLD-004, LIC-001)
51. An AR Foundation app on a device with ARKit or ARCore reports
    its session state, detects a horizontal plane, places an anchor with a
    raycast against it, and keeps the anchor after tracking is limited and
    recovered; on a device without image tracking, the image manager is
    reported as Platform-limited and the session continues. (XR-001)
52. A game using Unity Gaming Services Authentication and Cloud Save
    with no service credentials configured receives the unavailable-integration
    outcome for each call and keeps running; with network access removed while
    a save is pending, it receives the offline outcome and a connectivity
    change when access returns. (NET-001, PRD-002)
53. On a macOS host, a developer builds the same project as a Windows x86-64
    player with each scripting backend, Authenticode-signed with the
    developer's certificate. The player runs on Windows with the same scenario
    results as a player built on a Windows host, and the build results of the
    two hosts differ only in their host fields. (BLD-001, BLD-003, PLT-001)
54. A Windows, macOS, and Linux build of one project each produces one folder
    per target and architecture that uploads as a Steam depot without
    rearrangement. A game using a Steamworks binding package initializes the
    Steam client API, unlocks an achievement, and shows the Steam overlay in a
    development build. An agent's upload request waits for a person, and the
    person's approved upload reports SteamPipe's result. (BLD-001, PLT-001,
    AUT-007)
55. The launcher installs a unitas version with Windows, macOS, and Linux
    target support, opens a project with that version, and a second version
    installs beside it; an installation built from the release's tagged source
    produces a player identical to the one the released installation produces
    for the same project. (PRD-001)
