# Builds, Platforms, XR, and Networking Product Specification

## Scope

This specification defines the user-visible build product, headless and
continuous-integration operation, scripting backends and code stripping, player
behavior, platform integration, XR behavior, the supported networked-game
experience, deployment to devices and development-player connections, content
builds and post-release content updates, product licensing as it applies to
shipped players, product installation, updates, and target toolchains, and the
product's own data flows and privacy behavior.

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
only from its own installation locations. When a release build contains any
development-only capability, the build result names that capability.

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

## Dedicated server players

Windows, Linux, and macOS dedicated server players build from the Dedicated
Server target and run with no rendering or audio output. A project migrated
from Unity 2020.3 LTS with the Server Build option enabled opens with the
Dedicated Server target selected. A server build defines `UNITY_SERVER`, applies server-build
asset stripping, and runs with `-batchmode -nographics`. Graphics and audio API
use in a server player is reported as Platform-limited, naming the API.

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

The Web target is Unity 2022.3 LTS's WebGL build target. It produces a browser
bundle: a WebAssembly player, its loader, its data, and a page that hosts it.
A project that Unity 2022.3 LTS builds for WebGL builds for the Web target with
its WebGL Player Settings, Build Settings, templates, and plug-ins honored. The
`UNITY_WEBGL` define is set, and `Application.platform` returns
`RuntimePlatform.WebGLPlayer`.

Behavior that Unity 2022.3 LTS offers on WebGL is the baseline. Behavior that
Unity 2022.3 LTS lacks there, such as WebGPU, managed threads, WebAssembly SIMD,
and script debugging, is a unitas extension, and the build result names each
extension a build uses. A limitation of Unity's WebGL player is never carried
over as a limitation of the Web target. Only a capability that the browser,
the operating system, or the device does not offer is reported as
Platform-limited, naming the browser, the version, and the missing capability.

The browser player contains no MCP interface. An agent reaches a running
browser player only through the editor's connection to a development player
(BLD-004).

### Session and page lifecycle

A browser player reports, to the page and to diagnostics:

* loading progress and each startup phase;
* successful startup;
* a missing browser capability, naming it;
* storage availability and persistence;
* whether audio is permitted, and that the next gesture enables it;
* focus changes, visibility changes, resizing, and fullscreen changes;
* unrecoverable startup and runtime errors, with their cause.

Browser security and permission restrictions are visible to the game and to
diagnostics. A permission the person denies returns an explicit status, never
apparent success.

Page refresh and navigation away end the player session. Data the game wrote to
persistent storage before that point remains available to the next session.
During page hide and unload, the player delivers `OnApplicationPause(true)` and
`OnApplicationQuit` to the extent the browser runs page code at that time, and
it finishes pending persistent writes as SAV-001 in
[Accessibility, localization, and persistence](accessibility-localization-persistence.md)
defines. A page restored from the browser's back-forward cache resumes the same
session and delivers `OnApplicationFocus` and `OnApplicationPause(false)`.

Offline state and its changes are reported to the game, through
`Application.internetReachability`, and to diagnostics.

`Application.Quit` stops the player, releases its memory, and leaves the page
open, as in Unity. The page's `Quit` call does the same (see
[Loader and page API](#loader-and-page-api)).

### Graphics APIs and selection

WebGL 2 is the Unity 2022.3 LTS baseline graphics API for the Web target.
WebGPU is a unitas extension. The WebGL target's Graphics APIs list in Player
Settings includes both.

* With Auto Graphics API on, the player tries WebGPU first and WebGL 2 second.
* With a manual list, the player tries the listed APIs in their order and
  never uses an API that the list omits.
* The player falls back to the next API when the browser does not offer the
  API, when the browser refuses an adapter or device, when the adapter lacks a
  feature or limit the build requires, or when device creation fails. The
  startup report names the API tried, the reason it was not used, and the API
  selected.
* The WebGL Power Preference Player setting (Default, Low Power, and High
  Performance) is passed to the browser's adapter and context requests.
* `SystemInfo.graphicsDeviceType` returns `OpenGLES3` under WebGL 2, as in
  Unity. `SystemInfo.graphicsDeviceVersion` and `graphicsDeviceName` name the
  active API, its version, and the adapter. Under WebGPU, these values identify
  WebGPU, which is reported as a unitas extension.
* When neither API is available, startup fails before the engine loads its
  data, and the page receives an error that names the missing API. This is a
  real browser absence. It covers a browser with WebGL disabled and a GPU the
  browser blocks.

### Render pipelines and features on the web

Projects authored for the Built-in Render Pipeline and URP 14.x render on both
WebGPU and WebGL 2. HDRP 14.x and project-defined scriptable render pipelines
render on both as well, which is a unitas extension: Unity 2022.3 LTS does not
offer them on WebGL. The pipeline rules are owned by REN-006 in
[Rendering and animation](rendering-and-animation.md).

On both APIs, every rendering feature in that specification is available,
following its graphics API rule: a feature that the API lacks but the device
can perform is provided by other means.

* **Built-in Render Pipeline:** the forward, deferred, and legacy vertex-lit
  paths, realtime and baked lighting, shadows, Linear and Gamma color space,
  MSAA, HDR, GPU instancing, and Built-in post-processing render on both APIs.
* **URP 14.x:** the Forward, Forward+, and Deferred paths, camera stacking,
  the 2D Renderer, render scale, HDR, MSAA, decals, SSAO, renderer features,
  and the SRP Batcher render on both APIs.
* **Compute-dependent features:** `ComputeShader` dispatch, Visual Effect
  Graph, GPU culling, GPU skinning, and indirect drawing run natively on
  WebGPU. On WebGL 2, which has no compute stage, they run through an
  equivalent path with the same results, so no feature disappears on WebGL 2.
  `SystemInfo.supportsComputeShaders` returns true on both APIs. A per-target
  setting keeps Unity's WebGL value of false for projects that choose a
  fallback path from it. Any quality difference between the two APIs, such as
  a lower particle capacity, is reported per feature and asset.
* **Hardware ray tracing:** browsers offer no ray-tracing access, so ray-traced
  effects are Platform-limited on both APIs. The report names the effect and
  what renders instead.

Each feature's provision on the active API (direct, provided by other means,
or Platform-limited) is inspectable as REN-006 defines.

### Capability detection

At startup the player detects and reports:

* the graphics API, adapter, limits, optional features, and compressed texture
  formats;
* cross-origin isolation, thread availability, and the worker count;
* WebAssembly SIMD, exception handling, and 64-bit memory support;
* whether the page is a secure context;
* storage quota, persistence, and whether storage is session-only;
* audio state, gamepad access, fullscreen, pointer lock, keyboard lock, and
  clipboard access;
* the device pixel ratio, touch support, and device memory and processor count
  where the browser reports them.

`SystemInfo` members (`maxTextureSize`, `SupportsTextureFormat`,
`SupportsRenderTextureFormat`, `processorCount`, `systemMemorySize`,
`operatingSystem`, `deviceType`, and the rest) return values derived from this
detection. A value the browser hides is reported as unknown in diagnostics, and
the member returns Unity's documented fallback for it. The full capability
report is available in development diagnostics and through the editor's
connection to a development player.

### Lost graphics context

When the browser loses the WebGL 2 context or the WebGPU device, the player
stops rendering and keeps running game logic. When the browser restores the
context or provides a new device, the player re-creates every GPU resource and
resumes rendering on the next frame. Render texture contents are lost, as Unity
documents for device loss: `RenderTexture.IsCreated` returns false until the
texture is used or created again. Every other texture, mesh, buffer, and shader
renders as before.

If the browser does not restore the context, or re-creation fails, the player
reports an unrecoverable graphics error to the page and to diagnostics, naming
the cause. It stops rendering without freezing the page. Each loss and
restoration is recorded with its time and duration.

### Texture compression

The Web target supports ASTC, ETC2, and DXT (BC) texture compression, chosen per
browser and device.

* The Player Settings Texture Compression format and the Build Settings
  Texture Compression override (Use Player Settings, DXT, ETC2, and ASTC) are
  honored, along with per-texture format overrides for the WebGL target.
* **Multi-format builds:** a build option includes texture data in several
  formats in one build. At startup the loader detects the formats the device
  supports and downloads only the data for the best one, in the order the
  project sets. The default order is ASTC, then DXT, then ETC2.
* **Runtime selection:** a texture whose format the device cannot sample is
  decompressed at load to an uncompressed format that holds every channel. The
  player reports the texture, the format, and the extra memory.
* **Supercompressed textures:** Crunch-compressed textures, and KTX2 textures
  with Basis Universal supercompression from a package that CMP-002 lists,
  transcode at load to the best format the device offers.
* **Reporting:** the build result lists each texture's format per included
  format set. At runtime the player reports the format set selected, each
  transcoded or decompressed texture, and the memory it uses.

### WebAssembly player

Managed code on the Web target runs with ahead-of-time compilation (BLD-003).
Every managed-code guarantee in
[Scripting backends, ahead-of-time compilation, and code stripping](#scripting-backends-ahead-of-time-compilation-and-code-stripping)
holds in the browser.

#### Code size and stripping

* The managed stripping level, `link.xml`, and the preservation attributes
  work as defined above. Strip Engine Code removes engine modules the project
  does not use, and the build result lists the modules kept and the reason
  each one is kept.
* The Build Settings Code Optimization choices (Shorter Build Time, Runtime
  Speed, Runtime Speed with LTO, Disk Size, and Disk Size with LTO) and the
  IL2CPP Code Generation setting are honored. The size report states the
  effect of the choice used on code size, measured against the build's
  previous Code Optimization choice when one exists.
* The browser compiles the module while it downloads, and instantiates it as
  soon as compilation completes. Data download continues in parallel.

#### Memory

* The Initial Memory Size and Maximum Memory Size settings, and Memory Growth
  Mode (None, Linear, and Geometric) with its linear step, geometric step, and
  geometric cap, are honored.
* With growth mode None, the heap stays at its initial size.
* Heaps above 4 GB are a unitas extension, available in browsers that offer
  64-bit WebAssembly memory. Elsewhere, a Maximum Memory Size above the
  browser's limit is reported at build time and at startup, naming the limit.
* The current heap size, the managed heap, and the heap high-water mark are
  reported in development diagnostics and through the profiler (PRF-001).

#### Out of memory

When an allocation cannot be satisfied, the player reports out of memory to the
page and to diagnostics. The report states the requested size, the current
heap size, the configured maximum, the browser's limit, and the high-water
mark. Development builds add the largest managed and native allocations. The
page stays responsive and shows the report through the template. A managed
allocation failure raises `OutOfMemoryException` in managed code first, as on
desktop, so a game that catches it can recover.

#### Threads

Browser threads behave as RUN-004 in [Scripting runtime](scripting-runtime.md)
defines.

* **Cross-origin isolated page:** explicitly created threads, the thread pool,
  `Task.Run` work, timers, blocking waits, and the job system run on worker
  threads in parallel. Native plug-in code and engine work use them as well.
* **Page not isolated:** the player still starts. The startup report states
  that browser threading is unavailable and names the missing response
  headers. Thread creation and blocking waits fail with Unity's browser
  behavior, jobs run on the main thread with the same results, and the lack of
  parallelism is reported as a performance property.
* **Unity behavior kept:** the per-target setting that keeps Unity's browser
  threading behavior is honored, and the build result states which behavior
  the build uses.

The headers that isolation requires are listed in
[Hosting and server headers](#hosting-and-server-headers).

#### SIMD

WebAssembly SIMD is a unitas extension, on by default. Managed code, engine
code, and Burst-compiled code (RUN-011 in
[Scripting runtime](scripting-runtime.md)) use SIMD instructions where the
browser offers them. When a browser in the release's declared support matrix
lacks SIMD, the build includes a module without SIMD and the loader selects
it. The startup report states which module runs. A build setting turns SIMD
off.

#### Exceptions

The Enable Exceptions setting (None, Explicitly Thrown Exceptions Only, Full
Without Stacktrace, and Full With Stacktrace) selects diagnostic detail, not
correctness.

* Under every value, managed exceptions are thrown and caught as on desktop.
  This includes the null-reference, array-bounds, cast, and arithmetic
  exceptions that the runtime detects.
* Full With Stacktrace fills `Exception.StackTrace` with managed frames in
  release builds. The other values leave it empty in release builds.
  Development builds always include managed frames.
* The size report states the code size of the value used.
* WebAssembly Arithmetic Exceptions keeps its Unity meaning. Throw, the
  default, raises `DivideByZeroException` and `OverflowException` as on
  desktop. Ignore produces the value the WebAssembly operation returns, and
  the build result reports Ignore as a difference from desktop behavior.

#### Debug symbols and stack traces

* The Debug Symbols setting (Off, External, and Embedded) is honored. External
  writes a separate symbols file that the page does not download.
* Every build produces source maps and symbol files matched to its build
  identifier (DIA-003 in
  [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md)).
* In development builds, and in release builds with Embedded symbols, browser
  console stack traces show managed type, method, file, and line.
* For release builds with External or Off, a stack trace that a crash report
  or the console captured is symbolicated through the editor, the command line,
  and the editor's MCP interface.

### Build output and templates

A Web build writes a folder that holds the page (`index.html`), a `Build` folder
with the loader, framework, code, and data files, `StreamingAssets`, the
template's `TemplateData`, the third-party notices (LIC-001), and a header
manifest. The header manifest lists every file with the response headers it
requires. The build also writes server configuration for Apache, nginx, and IIS
that applies those headers.

WebGL templates:

* **Built-in templates:** the Default template shows the product name, a
  loading bar, a fullscreen button, and the warning banner, with no Unity
  branding. The Minimal template shows only the canvas. The Progressive Web App
  template adds a web app manifest and offline support.
* **Project templates:** every folder in `Assets/WebGLTemplates` with an
  `index.html` is a selectable template. Its `thumbnail.png` appears in the
  template picker, and its other files are copied to the build.
* **Selection:** `PlayerSettings.WebGL.template` values in the
  `APPLICATION:<name>` and `PROJECT:<name>` forms select the template, as in
  Unity.
* **Template variables:** the built-in variables (`COMPANY_NAME`,
  `PRODUCT_NAME`, `PRODUCT_VERSION`, `WIDTH`, `HEIGHT`, `BACKGROUND_COLOR`,
  `SPLASH_SCREEN_STYLE`, `UNITY_VERSION`, `DEVELOPMENT_PLAYER`,
  `DECOMPRESSION_FALLBACK`, `LOADER_FILENAME`, `FRAMEWORK_FILENAME`,
  `CODE_FILENAME`, `DATA_FILENAME`, `MEMORY_FILENAME`, `SYMBOLS_FILENAME`, and
  `BACKGROUND_FILENAME`) expand in `{{{ }}}` tags, and JavaScript expressions in
  tags and `#if`, `#else`, and `#endif` directives evaluate as in Unity.
  `UNITY_VERSION` expands to the value `Application.unityVersion` returns.
* **Custom variables:** any other variable a template uses appears as a custom
  template variable in Player Settings, and its value is stored with the
  project's Player Settings.
* **Template errors:** a template with an unknown directive, an unbalanced
  `#if`, or an expression that fails to evaluate is reported in the build
  result, naming the file, the line, and the cause. It is never silently
  ignored.

### Loader and page API

The page starts the player with `createUnityInstance(canvas, config,
onProgress)`, as in Unity.

* **Config:** `dataUrl`, `frameworkUrl`, `codeUrl`, `memoryUrl`, `symbolsUrl`,
  `streamingAssetsUrl`, `companyName`, `productName`, `productVersion`,
  `devicePixelRatio`, `matchWebGLToCanvasSize`, `webglContextAttributes`,
  `cacheControl`, and `showBanner` keep their Unity meaning. An unknown config
  key is reported in the console in development builds.
* **Progress:** `onProgress` receives values from 0 to 1 that never decrease.
  It reaches 1 when the first scene is ready.
* **Result:** the returned promise resolves to the instance once the player has
  started. It rejects with a message that names the cause on any startup
  failure, including an unsupported browser, a missing graphics API, a file
  that failed to download or decompress, and out of memory.
* **Instance:** `SetFullscreen(1)` and `SetFullscreen(0)` enter and leave
  fullscreen, subject to the gesture rule below. `SendMessage(objectName,
  methodName, value)` calls a method on a named GameObject, as in Unity.
  `Quit()` returns a promise that resolves once the player has stopped and
  released its memory. After `Quit()`, the same canvas can host a new
  instance.
* **Warnings:** `showBanner` receives warnings and errors with their severity.
  The Default template displays them.

### Compression, caching, and file names

* **Compression Format:** Brotli, Gzip, and Disabled are honored. Brotli
  produces the smallest transfer.
* **Decompression Fallback:** with the fallback on, the loader decompresses
  files that arrive without a matching `Content-Encoding` header. Development
  builds warn that the header is missing and state the startup cost. With the
  fallback off, a compressed file that arrives without the header fails
  startup with an error that names the file and the header the server must
  send.
* **Brotli over plain HTTP:** browsers accept Brotli content encoding only on
  secure origins. A Brotli build served over plain HTTP without the fallback is
  reported with that cause.
* **Data Caching:** with Data Caching on, downloaded build files are stored in
  the browser's IndexedDB cache, and later visits load them from the cache.
  `cacheControl` decides per URL whether a file is revalidated, treated as
  immutable, or not cached. A cache the browser has evicted or that is
  corrupt is detected and downloaded again.
* **Name Files As Hashes:** file names become digests of their content, so a
  host can mark them immutable and a new build never collides with a cached
  file from an earlier build.

### Remote content and Addressables

Addressables content and AssetBundles for the Web target build and load as
defined in
[Content builds and post-release content updates](#content-builds-and-post-release-content-updates)
and RUN-006.

* Remote load paths that point at a CDN load catalogs and bundles from it. The
  `Caching` API stores downloaded bundles in IndexedDB.
* A bundle request that the browser blocks for cross-origin reasons is
  reported, naming the URL, the CDN's missing response header, and the
  cross-origin rule.
* On a cross-origin isolated page, content from another origin must carry a
  cross-origin resource policy or CORS headers. A missing header is reported
  in the same way.
* Download size queries, dependency downloads, and catalog update checks work
  as on other targets.

### Hosting and server headers

The header manifest states every header the build needs. Development builds
check the headers they receive at startup and report each missing or wrong
header, naming the file and the expected value.

| Header | Files | Value |
|---|---|---|
| `Content-Encoding` | Brotli files (`.br`) | `br` |
| `Content-Encoding` | Gzip files (`.gz`) | `gzip` |
| `Content-Type` | `.wasm` | `application/wasm` |
| `Content-Type` | `.js` | `application/javascript` |
| `Content-Type` | `.data` and bundles | `application/octet-stream` |
| `Content-Type` | `.symbols.json` | `application/json` |
| `Cross-Origin-Opener-Policy` | the page | `same-origin` |
| `Cross-Origin-Embedder-Policy` | the page | `require-corp` or `credentialless` |
| `Cross-Origin-Resource-Policy` | files from another origin | `cross-origin` |

* With Decompression Fallback on, compressed files use the `.unityweb`
  extension. Serving them with the `Content-Encoding` of the build's
  compression format skips decompression in the loader.
* Without `application/wasm`, the browser cannot compile while downloading.
  The player still starts, and development builds report the slower startup
  and name the header.
* Without the two cross-origin isolation headers on the page, the player runs
  without browser threads, as described in [Threads](#threads).

#### Local development server

The editor serves Web builds from a local development server that sends every
header in the manifest, including the cross-origin isolation headers and
`Content-Encoding`. Build And Run uses it.

* The server picks a free port and reports the URL.
* Serving to other devices on the local network, for testing on phones and
  tablets, is opt-in. It serves over HTTPS with a development certificate,
  because WebGPU, browser threads, and the clipboard require a secure context
  on any host other than the local machine. Trusting that certificate on a
  device is the person's decision.
* The server runs in the headless editor as well. Agents start, inspect, and
  stop it through the editor's MCP interface under the deploy scope (AUT-007).

#### Embedding and game portals

* **Iframes:** a build embedded in an iframe works when the embedding page
  grants the permissions the game uses: fullscreen, autoplay, gamepad,
  clipboard, pointer lock, and cross-origin isolation. The player reports each
  permission the embedding page's policy denies. Keyboard input reaches the
  player once the person clicks the frame.
* **Portals:** a build option produces output for portals that host the game
  on their own domain, accept a single archive, and do not let the developer
  set response headers. That output uses relative paths only, uses the
  decompression fallback, and needs no server configuration. The build result
  states which features depend on headers the portal may not send, such as
  browser threads.
* **Portal SDKs:** a portal's JavaScript SDK is reached through JavaScript
  interop.

### JavaScript interop

* **Plug-ins:** `.jslib` files in the project and its packages are included in
  Web builds as RUN-007 in [Scripting runtime](scripting-runtime.md) defines.
  Their `mergeInto(LibraryManager.library, ...)` functions, `$`-prefixed
  dependencies, `__deps` entries, and `autoAddDeps` resolve as in Unity.
  `.jspre` files run before the player starts, in a stable order that the
  build result lists.
* **Calling JavaScript from C#:** `[DllImport("__Internal")]` declarations call
  jslib functions with `int`, `float`, `double`, `bool`, string, and array
  arguments, and return numbers, booleans, and strings.
* **Calling C# from JavaScript:** `SendMessage` from a jslib or the page calls
  a method with no argument, a string, or a number. Managed callbacks passed as
  function pointers and marked `[MonoPInvokeCallback]` are called from
  JavaScript through the `makeDynCall` and `dynCall` forms that Unity 2022.3
  LTS documents.
* **Marshalling:** strings convert with `UTF8ToString`, `stringToUTF8`, and
  `lengthBytesUTF8`. A string returned to C# is allocated with `_malloc`.
  Arrays pass as a pointer and a length and are read through the `HEAP8`,
  `HEAPU8`, `HEAP16`, `HEAPU16`, `HEAP32`, `HEAPU32`, `HEAPF32`, and `HEAPF64`
  views. These views stay valid after the heap grows and when memory is shared
  between threads.
* **Module:** `Module` is reachable from jslib code and, as the instance's
  `Module`, from the page.
* **Threads:** a jslib function called from a managed background thread runs on
  the page's main thread, where the page's document is reachable, and the
  calling thread waits for its result.

Errors in each direction:

* A JavaScript exception thrown in a jslib function raises an exception in the
  calling C# code. The exception names the jslib function and carries the
  JavaScript message and stack. The player keeps running.
* A `DllImport` that names no included jslib function fails the build, naming
  the declaration and the missing function.
* `SendMessage` to an object or method that does not exist logs an error that
  names both, as in Unity.
* An exception thrown by the C# method that `SendMessage` called is logged with
  its managed stack and does not stop the player.
* An exception thrown by a managed callback that JavaScript called reaches the
  JavaScript caller as an error that names the managed exception type and
  message.

### Canvas, display, and fullscreen

* **Canvas size:** the player renders at the canvas's CSS size multiplied by
  the device pixel ratio. `config.devicePixelRatio` overrides the ratio, and a
  project setting caps it, for performance on high-density screens.
  `Screen.width`, `Screen.height`, and `Screen.dpi` report the result.
* **Resizing:** resizing the canvas or the window changes `Screen.width` and
  `Screen.height` from the next frame. With `matchWebGLToCanvasSize` off,
  `Screen.SetResolution` sets the render resolution and the browser scales it
  to the canvas.
* **Fullscreen:** `Screen.fullScreen`, `Screen.fullScreenMode`, and
  `SetFullscreen` request fullscreen. Browsers grant fullscreen only during a
  user gesture, so a request made from game code applies at the next click,
  touch, or key press on the canvas, as in Unity. Until then, the request is
  reported as pending. Leaving fullscreen with Escape or a browser control
  updates `Screen.fullScreen`. A browser that offers no element fullscreen,
  such as Safari on iPhone, reports fullscreen as Platform-limited.
* **Pointer lock:** `Cursor.lockState` follows the same gesture rule.
  `WebGLInput.stickyCursorLock` keeps its Unity meaning. A lock request that
  the browser refuses, for example right after the person pressed Escape, is
  reported and retried at the next gesture.
* **Cursor:** `Cursor.SetCursor` and `Cursor.visible` apply to the canvas.

### Keyboard and focus

* `WebGLInput.captureAllKeyboardInput` keeps its Unity meaning. When it is
  true, the player receives keyboard input from the whole page. When it is
  false, the player receives keys only while the canvas has focus, so HTML
  elements on the page receive their own input.
* Clicking the canvas gives it focus. Focus loss delivers
  `OnApplicationFocus(false)`, and keys held at that moment are released.
* Browsers reserve some shortcuts, such as closing a tab. These never reach the
  player, and they are listed in product knowledge (AUT-006). In fullscreen, in
  browsers that offer keyboard lock, a project setting captures Escape and
  other system keys.

### Gamepads, touch, sensors, and mobile browsers

* **Gamepads:** gamepads come from the browser's Gamepad API for the legacy
  Input Manager (INP-001) and the Input System (INP-002), with device
  recognition as UI, audio, and input defines. A gamepad appears once the
  person presses one of its buttons, as browsers require. Rumble works where
  the browser offers vibration for that gamepad. Light bars and other HID
  outputs work through the browser's HID access where the browser offers it
  and the person grants it. Elsewhere they are reported as Platform-limited.
* **Touch:** multi-touch reaches `Input.touches` and the Input System
  `Touchscreen`, including touch cancellation when the browser takes over a
  gesture. Templates stop page scrolling and zooming on the canvas.
  `Input.mousePresent` and `Input.touchSupported` reflect the device.
* **Sensors:** accelerometer, gyroscope, and attitude data come from the
  browser's motion and orientation events. On browsers that require
  permission, such as Safari on iOS, the first sensor use asks for it at the
  next gesture, and a denial returns an explicit status.
* **Orientation and safe area:** `Screen.orientation` reports the current
  orientation. An orientation lock applies where the browser allows it, such
  as fullscreen in Chrome on Android, and is reported as Platform-limited
  elsewhere. `Screen.safeArea` reflects the browser's safe-area insets.
* **Vibration:** `Handheld.Vibrate` works where the browser offers vibration,
  and returns an explicit status elsewhere.
* **Mobile browsers:** Safari on iOS and iPadOS and Chrome on Android are
  supported browsers. On these devices the player applies the mobile memory
  limit the browser enforces. It reports the limit at startup and keeps the
  out-of-memory report above. Browsers on iOS and iPadOS that use Safari's
  engine behave as Safari.

### Text input and clipboard

* **Virtual keyboard:** on touch devices, focusing a uGUI, TextMeshPro, UI
  Toolkit, or IMGUI text field opens the browser's on-screen keyboard. The
  keyboard follows the field's content type, multiline setting, and secure
  setting where the browser allows. Text, selection, composition, submission,
  and cancellation flow back to the field. `TouchScreenKeyboard` behaves as UI,
  audio, and input defines, and `WebGLInput.mobileKeyboardSupport` keeps its
  Unity meaning.
* **IME:** composition works on desktop browsers as UI, audio, and input
  defines.
* **Clipboard:** clipboard access behaves as UI-001 in
  [UI, audio, and input](ui-audio-input.md) defines for the browser target.
  Writing happens within the gesture that triggered the copy, where the
  browser requires one.

### Visibility and background tabs

* Hiding the page, by switching tabs or minimizing, delivers
  `OnApplicationFocus(false)` and, with Run In Background off,
  `OnApplicationPause(true)`. Showing it again delivers the reverse.
* With Run In Background on, the player keeps running in a hidden tab at the
  rate the browser allows. Browsers throttle hidden tabs and may stop their
  frames entirely. The player reports the throttling.
* Returning from a throttled or stopped state never produces a burst of
  catch-up frames. `Time.deltaTime` is bounded by `Time.maximumDeltaTime`,
  fixed updates are bounded as Unity defines, and game time continues from
  where it stopped without a jump. `Time.realtimeSinceStartup` reports real
  elapsed time.
* Audio follows the pause state. Network connections stay open while the
  browser allows.

### Audio and video on the web

Audio plays through the browser's Web Audio output. Every AUD-001 behavior in
[UI, audio, and input](ui-audio-input.md) applies: clip load types
(Decompress On Load, Compressed In Memory, and Streaming), mixer groups,
snapshots and effects, filters, `OnAudioFilterRead`, spatial audio, and
`GetSpectrumData` and `GetOutputData`. These are unitas extensions where Unity
2022.3 LTS's WebGL player lacks them.

* **Autoplay unlock:** browsers start audio only after a user gesture. Until
  the first click, touch, or key press on the page, audio is reported as not
  permitted, and sources that play are tracked as playing silently. The first
  gesture enables audio, and sources continue from their current time. On
  Safari for iOS, the unlock uses the gesture that the browser accepts for
  audio, including a touch ending.
* **Suspension:** a context that the browser suspends later resumes after the
  next gesture. The game can observe whether audio is currently permitted.
* **Device mute:** the iOS ring/silent switch affects browser audio as the
  operating system decides. The player reports that state where the browser
  exposes it.
* **Compressed audio:** every encoding AUD-001 lists decodes in every supported
  browser, including encodings the browser's own decoder lacks.
* **Latency:** the output latency the browser reports is shown in audio
  inspection and diagnostics. Where the browser reports none, the latency is
  reported as unknown.
* **Microphone:** microphone capture behaves as UI, audio, and input defines
  for the browser target.

Video follows MED-001.

* `VideoPlayer` plays `VideoClip` assets and URL sources on the Web target.
  Unity 2022.3 LTS's WebGL player plays only URLs, so `VideoClip` playback is
  a unitas extension.
* Every codec and container MED-001 lists plays in every supported browser,
  including codecs the browser's own decoder lacks. The decoding path in use
  is reported as a performance property.
* A cross-origin video URL whose server sends no CORS headers cannot be read
  into a texture in a browser. It is reported as Platform-limited, naming the
  URL and the missing header.
* Video with sound starts under the same autoplay rule as audio.

### Storage

* **PlayerPrefs:** `PlayerPrefs` is stored in the page origin's IndexedDB
  storage, with no fixed size limit, as SAV-001 defines. Reads and writes in
  game code are synchronous, as in Unity. `PlayerPrefs.Save` returns after the
  preferences are durable.
* **Files:** `Application.persistentDataPath` returns the path form Unity
  2022.3 LTS returns for WebGL, scoped to the page's origin. Files under it use
  `System.IO` synchronously. A closed file is copied to IndexedDB
  automatically and reported as pending until the copy completes, as SAV-001
  defines.
* **Quota:** the player reports the quota and usage the browser grants. A
  write that exceeds the quota fails in the calling code with the .NET
  exception for a full disk, naming the path, never with apparent success.
* **Persistence:** a project setting asks the browser to make storage
  persistent. Whether the browser granted that is observable. Storage that the
  browser may evict is reported as best-effort.
* **Private browsing:** where private browsing makes storage session-only or
  smaller, the player reports storage as session-only at startup, and writes
  behave as above within that storage.
* **Origin scope:** builds served from different origins never share
  storage.

### Networking on the web

* **Web requests:** `UnityWebRequest` runs over the browser's fetch and
  `XMLHttpRequest` APIs, with the RUN-009 behaviors in
  [Scripting runtime](scripting-runtime.md). Upload and download progress,
  streaming download handlers, timeouts, and `Abort` work. A request blocked
  by CORS returns `ConnectionError`, and diagnostics name the URL and the
  missing response header. An HTTP request from an HTTPS page is reported as
  mixed content. Forbidden headers are reported as RUN-009 defines.
* **WebSocket:** `ClientWebSocket` and the Unity Transport WebSocket interface
  connect to WebSocket servers, as RUN-009 and
  [Unity Transport](#unity-transport) define. A `ws://` URL from an HTTPS page
  is reported as mixed content.
* **WebRTC:** a game that opens WebRTC data channels, through JavaScript
  interop or a package that CMP-002 lists, receives their messages in managed
  code in order. Connection state changes are reported.
* **No raw sockets:** browsers give pages no raw TCP or UDP access. This is a
  real browser absence, and socket use is reported as RUN-009 defines.

### Browser support matrix

Each release declares, under CMP-005 in the
[Compatibility contract](compatibility-contract.md), the minimum browser
versions for the Web target. The matrix covers at least:

| Browser | Platforms | Versions |
|---|---|---|
| Chrome | Windows, macOS, Linux, ChromeOS, Android | current stable and previous major |
| Edge | Windows, macOS | current stable and previous major |
| Firefox | Windows, macOS, Linux, Android | current stable, previous major, and current Extended Support Release |
| Safari | macOS, iOS, iPadOS | current major and previous major |

For each browser and platform, the matrix states WebGPU, browser threads,
SIMD, 64-bit memory, compressed texture formats, audio, gamepad, fullscreen,
pointer lock, clipboard, and storage persistence.

On a browser below the declared minimum, the loader does not start the player.
The page receives an error that names the browser, its version, and the
minimum, and the Default template shows it. A project setting lets the player
start anyway, with a warning banner. A browser that meets the minimum but lacks
an optional capability, such as WebGPU or threads, starts with the fallback
defined above and is not treated as unsupported.

### Web performance and optimization

The Web target is optimized as far as the browser allows. The outcomes below
are measurable, and the reference-project measurements are published with
each release.

#### Rendering

When WebGL 2 is active, rendering uses a path designed for WebGL 2's
constraints, never a slow generic fallback.

* **Batching and instancing:** on the reference projects, draw-call and batch
  counts under WebGL 2 and WebGPU equal those of the same project's Android
  OpenGL ES 3 player.
* **Shader warm-up:** shader programs and pipelines that the build's shader
  variant collections and its first scene need are compiled at load, and the
  startup report states the warm-up time. `ShaderVariantCollection.WarmUp` and
  `Shader.WarmupAllShaders` are honored.
* **Compilation hitches:** on the reference projects, no frame during gameplay
  stalls longer than 4 ms for shader or pipeline compilation. Development
  builds report each variant that was first compiled during gameplay, naming
  the shader, pass, and keywords.
* **Texture uploads:** no texture upload stalls a frame for more than 2 ms.
* **Supercompressed textures:** these transcode to the best format the device
  offers, as described in [Texture compression](#texture-compression).
* **Compute-dependent features:** these run on both APIs, as described in
  [Render pipelines and features on the web](#render-pipelines-and-features-on-the-web).

#### WebAssembly

* **Code Optimization:** each choice has a measured effect on code size and
  frame time, stated in the size report.
* **Streaming:** the module compiles and instantiates while data downloads.
* **SIMD:** used where the browser offers it.
* **Garbage collection:** with incremental garbage collection, which is on by
  default and honors `GarbageCollector.GCMode`, main-thread collection pauses
  are 2 ms or less at the 99th percentile on the reference projects.
* **Memory:** during steady-state play after load, the heap does not grow. In a
  replayed session (TST-002) that loops the same gameplay, the heap size after
  the last loop equals the size after the first loop.
* **High-water mark:** the heap high-water mark is reported in development
  diagnostics, in the profiler, and in out-of-memory reports.

#### Startup and size

* **Progressive loading:** a build option splits build data by scene and by
  Addressables group. The first interactive frame waits only for the content
  the first scene needs. Later scenes download in build order after the first
  frame. A load of a scene that has not arrived waits for its content, and
  `LoadSceneAsync` progress includes that download. The wait is reported in
  development diagnostics.
* **Startup report:** the startup report states the time spent in download,
  compilation, instantiation, engine start, first scene load, and shader
  warm-up, and the time to the first interactive frame.
* **Size report:** every Web build produces a download-size report. It breaks
  the transfer into code (the WebAssembly module and framework), data
  (scenes, assets, and resources), streaming assets, and Addressables
  content. It lists sizes before and after compression, per assembly, per
  engine module, and per asset, with the largest contributors first. It also
  lists the change from the previous build of the same target.

#### Engine web budgets

These are engine budgets, declared and measured alongside the PRF-002 budgets
in
[Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md)
under PRF-002's measurement and regression rules. A measurement beyond a
budget is a visible regression.

* The Roll-a-Ball-style sample's compressed transfer is no more than 15 MB.
* Its first interactive frame arrives within 3 s on a cold cache over a
  50 Mbps connection, and within 1 s on a warm cache, on the reference desktop.
* It runs at 60 fps at 1080p on the reference integrated-GPU laptop in Chrome,
  Edge, Firefox, and Safari.
* It runs at 60 fps on the reference mid-range phone in Chrome on Android and
  Safari on iOS.
* The 2D platformer sample's heap stays under 256 MB.

#### Project budgets

A project declares its own web budgets: compressed transfer size, initial
download size, heap size, and time to first interactive frame. Each build
result and each headless browser test run reports every project budget as met
or exceeded. A project setting makes an exceeded budget fail the build.
Project budgets are separate from the engine budgets of PRF-002.

#### Frame pacing

* Frames are driven by the browser's animation frame callback. With
  `Application.targetFrameRate` at -1, the player presents on every display
  refresh, including on high-refresh displays. A positive value paces frames
  below the refresh rate, as REN-007 in
  [Rendering and animation](rendering-and-animation.md) defines.
* Background-tab throttling is handled as described in
  [Visibility and background tabs](#visibility-and-background-tabs).
* The device pixel ratio cap and resolution scale settings (URP render scale
  and dynamic resolution) are available for performance, and their effect is
  shown in the profiler.

#### Web build profile

The editor has a web build profile view. For a Web build, it reports size per
assembly, engine module, and asset, flags the largest contributors, and
recommends stripping, Code Optimization, compression, and texture format
settings with their estimated savings. Agents use it as an editor command
through the editor's MCP interface (AUT-002). The browser player itself
carries no such interface.

### Web diagnostics

* **Console forwarding:** `Debug.Log`, `Debug.LogWarning`, `Debug.LogError`,
  and exceptions reach the browser console with matching severity. Browser
  console messages, including graphics API errors and page script errors,
  reach the editor's device log stream (DIA-003) when the editor is connected.
* **Development builds:** Autoconnect Profiler, Deep Profiling Support, and
  Show Diagnostics Overlay are honored. A development browser player connects
  to the editor's profiler (PRF-001) and to script debugging (DIA-002) with
  breakpoints and stepping, under the pairing rules of
  [Deployment, device connection, and development players](#deployment-device-connection-and-development-players).
  Script debugging is a unitas extension.
* **Crash and exception reports:** unhandled managed exceptions, WebAssembly
  traps, out-of-memory failures, and unrecoverable graphics errors produce
  DIA-003 reports. Each report carries the build identifier, the browser and
  its version, the operating system, the graphics API and adapter, and mapped
  stacks. Reports go only to a destination the project configures, and the
  page's error path receives each one.

### Automated browser testing

The headless editor and the command line (BLD-002) run project tests (TST-003)
and structured scenarios (TST-002) in a Web build inside browsers that the
editor launches and controls with no display. The Unity test-platform argument
for WebGL selects this mode.

* The invocation selects one or more installed browsers (Chromium-based,
  Firefox, and WebKit-based) and whether the page is cross-origin isolated.
* The editor serves the build with its development server and collects test
  results, logs, console output, crash reports, startup reports, and project
  budget results into the same machine-readable output and `-testResults` file
  as other test runs.
* Each result records the browser, its version, the graphics API, and whether
  rendering ran on a GPU or a software rasterizer.
* A requested browser that is not installed is reported as a missing toolchain
  with its distinct exit status.

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
* **Graphics APIs:** Direct3D 11 (the default), Direct3D 12, Vulkan, and
  OpenGL Core, in the order of the project's Graphics APIs list, with fallback
  to the next API when one fails to start. The startup report names the API
  chosen and the reason any earlier one was skipped. Direct3D 12 ray tracing is
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
* **Render modes and spaces:** multi-pass and single-pass instanced rendering,
  depth submission, and the runtime's preferred view configuration render
  every eye view. The tracking origin mode (device, floor, or unbounded where
  the runtime offers it) and `XRInputSubsystem.TryRecenter` behave as the
  device reports them. Play area boundary points are readable when the runtime
  provides them.
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

## Networking behavior

Networked play is provided through Netcode for GameObjects, Netcode for
Entities (`com.unity.netcode`), and Unity Transport (`com.unity.transport`),
within the version lines that CMP-002 lists. Netcode content imports with its
identity, and the API shape below keeps its Unity 2022.3 LTS names and
signatures; the behavior is the behavior this section defines, and every known
difference from Unity is listed in the AUT-006 API reference.

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
action is not reported as synchronized until the declared remote observation
occurs. Conflicts, authority rejection, and stale state are explicit.

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

A project request never creates a claim that the developer owns a service,
license, signing identity, or platform entitlement.

Input, Addressables content delivery, localization, networking, and XR
capabilities that originate in separately versioned Unity packages are reported
against the specific package and version line listed in CMP-002. Project use of
one version is not evidence that every version or provider behaves identically.

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
the player's target, as well as content built by unitas. A shipped Unity game
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
* **Shaders and materials:** shaders in Unity-built bundles render through the
  same translation that REN-006 in
  [Rendering and animation](rendering-and-animation.md) applies to project
  shaders. Each variant that the bundle carries is available, and a variant
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

A player contains no telemetry, analytics, or network connection that the game
itself did not request. The editor, including its MCP interface, and the command line
send project content only to MCP clients and external tools that a person authorized
(AUT-004, AUT-007 in [Automation and MCP](automation-and-mcp.md)) and to
devices and services that a person directed, such as deployment targets,
package registries, and crash-report destinations. unitas never sends project
content to its publisher or to any other party on its own initiative. Usage reporting and crash reporting for
unitas itself are off until a person enables them; when enabled, each lists the data it sends
and its destination, and neither ever blocks offline use. Apple target builds
include a privacy manifest that covers the required-reason APIs used by the
included unitas components.

## Acceptance stories

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
8. A built player launches the configured scene and handles focus, pause,
   permissions, and shutdown correctly on its platform. (PLT-001)
9. Mobile and browser lifecycle changes, including backgrounding, page refresh,
   storage quota exhaustion, and lost graphics or audio context, preserve or
   reject game state in a defined, inspectable way. (PLT-001)
10. A target without the required host toolchain or user entitlement reports
    Externally unavailable with the specific cause rather than an engine
    failure. (PLT-001, PRD-001)
11. XR play distinguishes real hardware state, tracking loss, input capability,
    and performance from desktop approximation. (XR-001)
12. A multiplayer sample using Netcode for GameObjects distinguishes connection,
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
    no build-host absolute paths, and a release build that contains a
    development-only capability names it in the build result. (BLD-001)
21. A headless invocation without the PRJ-008 trust argument refuses to run a
    project's static editor method, and a project build started through
    `BuildPipeline.BuildPlayer` produces the same build result as a command-line
    build. (BLD-002, PRJ-008, RUN-010)
22. A Linux dedicated server player runs with `-batchmode -nographics`, defines
    `UNITY_SERVER`, and reports a graphics API call as Platform-limited.
    (PLT-001)
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
26. A player built from a project that makes no network requests opens no
    network connection, the editor and its MCP interface send project content only to
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
    WebSocket. (NET-001, PLT-001)
34. A Windows x86 build, an Android ARMv7 build, a tvOS build, and a Universal
    Windows Platform build each produce their target's artifact, and a console
    target on a host with the platform holder's SDK installed builds and
    deploys to a development kit. (PLT-001, BLD-001, BLD-004)
35. A macOS dedicated server player runs with `-batchmode -nographics` and
    defines `UNITY_SERVER`. (PLT-001)
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
39. A Web build with Auto Graphics API on runs on WebGPU in a browser that
    offers it. In a browser without WebGPU, and in one whose adapter lacks a
    required feature, the same build runs on WebGL 2, and the startup report
    names the API tried, the reason it was skipped, and the API selected. A
    build whose Graphics APIs list holds only WebGL 2 never uses WebGPU.
    (PLT-001, BLD-001)
40. A Web build served with the cross-origin isolation headers runs a thread
    that blocks on its result and an `IJobParallelFor` on workers with the
    same results as a desktop build. Served without the headers, it starts,
    reports that browser threading is unavailable and names the missing
    headers, and runs the jobs on the main thread with the same results.
    (PLT-001, RUN-004, RUN-011)
41. A Brotli build with Decompression Fallback off starts when served with
    `Content-Encoding: br`, and fails without the header with an error naming
    the file and the header. The same project with the fallback on starts
    without the header, and its development build warns about the missing
    header. A build served without `application/wasm` starts, and its
    development build names that header. (PLT-001, BLD-001)
42. A jslib function called through `[DllImport("__Internal")]` receives a
    string and a float array, returns a string that C# reads intact, and calls
    a `[MonoPInvokeCallback]` method with a number. A `SendMessage` from the
    page reaches a named GameObject. A JavaScript exception in the jslib
    function raises a C# exception that names the function and carries the
    JavaScript message, and the player keeps running. (PLT-001, RUN-007)
43. A browser game saves with `PlayerPrefs.Save` and writes a file under
    `Application.persistentDataPath`. After the file reports synchronized, the
    page is reloaded, and both values load in the new session. A write beyond
    the storage quota fails in the calling code. In private browsing, storage
    is reported as session-only at startup. (PLT-001, SAV-001)
44. On Safari for iOS, a game that starts music before any touch reports
    audio as not permitted. The first touch enables audio, the music
    continues from its current time, and a context that the browser suspends
    later resumes after the next touch. (PLT-001, AUD-001)
45. When the WebGL 2 context is lost and restored during play, game logic keeps
    running, rendering resumes on the next frame with every texture and mesh
    intact, and render textures report `IsCreated` false until used again.
    When the browser does not restore the context, the page receives an
    unrecoverable graphics error naming the cause, and the page stays
    responsive. The same holds for a lost WebGPU device. (PLT-001, REN-006)
46. A project template in `Assets/WebGLTemplates` uses `PRODUCT_NAME`,
    `PRODUCT_VERSION`, `DATA_FILENAME`, a `#if DECOMPRESSION_FALLBACK` block,
    and a custom variable. The built page contains the expanded values, the
    custom variable appears in Player Settings with its stored value, and a
    template with an unbalanced `#if` is reported by file and line.
    (PLT-001, BLD-001)
47. A Web build's size report breaks the transfer into code, data, streaming
    assets, and Addressables content, before and after compression, per
    assembly, engine module, and asset, with the largest contributors first.
    Changing Code Optimization from Runtime Speed to Disk Size with LTO shows
    the measured code-size change. A project budget the build exceeds is
    reported, and fails the build when the project setting asks for that.
    (PLT-001, BLD-001, BLD-003)
48. A command-line invocation with no display runs a project's play-mode
    tests and a structured scenario in a Web build in a Chromium-based
    browser and in Firefox, both controlled by the editor. Results, console
    output, and startup reports for each browser land in the `-testResults`
    file and the machine-readable output. A requested browser that is not
    installed exits with the missing-toolchain status. (BLD-002, TST-002,
    TST-003)
49. A multi-format Web build with ASTC, DXT, and ETC2 data downloads only
    ASTC data on a phone that offers ASTC and only DXT data on a desktop that
    offers DXT. A texture in a format the device cannot sample is decompressed
    at load and reported with its extra memory. (PLT-001, REN-002)
50. A browser game that allocates beyond its Maximum Memory Size receives
    `OutOfMemoryException`, and when it does not catch it, the page shows an
    out-of-memory report with the requested size, the heap size, the
    configured maximum, and the high-water mark, and stays responsive.
    (PLT-001, DIA-003)
51. In a Web build embedded in an iframe, a fullscreen request and a cursor
    lock request made from game code apply at the next click on the canvas. In
    an iframe whose policy denies fullscreen, the player reports the denied
    permission. Escape leaves fullscreen and releases the cursor, and
    `Screen.fullScreen` and `Cursor.lockState` report it. (PLT-001)
52. On the reference projects under WebGL 2, the startup report states the
    shader warm-up time, no gameplay frame stalls longer than 4 ms for shader
    or pipeline compilation or longer than 2 ms for a texture upload, and the
    draw-call counts equal those of the Android OpenGL ES 3 player.
    (PLT-001, PRF-002)
53. A 30-minute replayed session of the 2D platformer in a browser shows
    main-thread garbage-collection pauses of 2 ms or less at the 99th
    percentile, the same heap size after the last loop as after the first,
    a heap that stays under 256 MB, and a reported heap high-water mark.
    (PLT-001, PRF-002, TST-002)
54. The Roll-a-Ball-style sample built for the Web target transfers no more
    than 15 MB compressed, shows its first interactive frame within 3 s on a
    cold cache over a 50 Mbps connection and within 1 s on a warm cache on the
    reference desktop, runs at 60 fps at 1080p on the reference
    integrated-GPU laptop in Chrome, Edge, Firefox, and Safari, and runs at
    60 fps on the reference mid-range phone in Chrome on Android and Safari on
    iOS. A measurement beyond any of these budgets is published as a
    regression. (PLT-001, PRF-002)
55. A URP project with a Visual Effect Graph effect, GPU culling, and a
    project `ComputeShader` renders on WebGL 2 in a browser without WebGPU.
    Each feature runs with the same results as on WebGPU, and any quality
    difference is reported by feature and asset. (PLT-001, REN-004, REN-006)
56. A Web build split by scene shows its first interactive frame before its
    later scenes have downloaded, and loading a later scene with
    `LoadSceneAsync` reports progress that includes its download. (PLT-001,
    RUN-005, RUN-006)
57. An agent requests the web build profile through the editor's MCP
    interface and receives size per assembly, engine module, and asset, the
    largest contributors, and recommended stripping, compression, and texture
    format settings. The browser player built afterward contains no MCP
    interface. (PLT-001, AUT-002)
58. A browser game running at `targetFrameRate` -1 presents on every refresh
    of a 120 Hz display. After its tab is hidden for one minute and shown
    again, `Time.deltaTime` on the first frame is no larger than
    `Time.maximumDeltaTime`, no burst of catch-up frames occurs, and
    `OnApplicationFocus` and `OnApplicationPause` were delivered.
    (PLT-001, REN-007)
59. A browser game's `UnityWebRequest` to a server that sends no CORS headers
    returns `ConnectionError`, and diagnostics name the URL and the missing
    header. A `ClientWebSocket` connects to a WebSocket server, and a raw
    socket fails with the platform-not-supported exception. Addressables
    content loads from a CDN on a cross-origin isolated page, and a bundle
    without a cross-origin resource policy is reported by URL. (PLT-001,
    RUN-009, BLD-005)
60. The editor's local development server serves a Web build with every
    header in its manifest, and a phone on the local network opens the
    build over HTTPS with browser threads available. A browser below the
    declared minimum version receives an error naming the browser, its
    version, and the minimum. (PLT-001, BLD-004)
61. A development browser player connects to the editor's profiler and
    debugger with its pairing credential. A breakpoint in a C# method stops
    the player and the debugger steps through it. An unhandled exception
    produces a crash report with the build identifier, browser, graphics API,
    and a stack mapped to C# source lines. (PLT-001, BLD-004, DIA-002,
    DIA-003)
62. A Windows x86-64 build whose Graphics APIs list is Direct3D 12 then
    Direct3D 11 runs on Direct3D 12, falls back to Direct3D 11 on a device
    where Direct3D 12 fails, and names the reason. Its `PlayerPrefs` are in the
    registry and its `persistentDataPath` is under `AppData\LocalLow`, as in
    Unity. A standalone ARM64 build runs natively and is reported as a unitas
    extension. (PLT-001, BLD-001)
63. A UWP build produces a Visual Studio solution whose manifest declares the
    capabilities set in Publishing Settings. Suspending the app delivers
    `OnApplicationPause(true)` within Windows's deadline, and a use of an
    undeclared capability returns the platform's denial. (PLT-001, BLD-001)
64. A universal macOS build runs natively on Apple Silicon and on Intel. With
    the hardened runtime and a distribution identity, it reports its
    entitlements and notarization state. With Mac App Store Validation on,
    the sandboxed app writes saves inside its container and a network access
    that the entitlements do not allow is denied. (PLT-001, BLD-001)
65. A macOS player in Fullscreen Window mode on a display with a camera
    housing reports a `Screen.safeArea` that excludes it, and Command-Q calls
    `Application.wantsToQuit` before `OnApplicationQuit`. (PLT-001, REN-007)
66. A Linux x86-64 build runs under a native Wayland session and under X11, and
    its startup report names the display server. Its Vulkan device fails on a
    test machine and it falls back to OpenGL Core with the reason named. An
    AArch64 build runs natively and is reported as a unitas extension.
    (PLT-001, BLD-001)
67. A Linux build deployed to a Steam Deck over the local network runs at the
    device's resolution with its built-in controls and touch screen, survives
    suspend and resume with `OnApplicationPause`, and stores its saves under
    `~/.config/unity3d/<company>/<product>`. (PLT-001, BLD-004)
68. An Android app bundle with ARM64 and ARMv7 reports whether it meets Google
    Play's target API level, 64-bit, and 16 KB page-size requirements, and a
    build from a release that does not meet the target API level reports
    store submission blocked while a local APK still builds. The same project
    exports a Gradle project. (PLT-001, BLD-001)
69. An Android player on a device where Vulkan fails falls back to OpenGL ES 3
    and names the device and cause. A trim-memory notification raises
    `Application.lowMemory`. On Android TV, the remote's D-pad navigates the
    game's UI, and on a ChromeOS device the x86-64 player receives mouse and
    keyboard input as desktop input. (PLT-001, INP-002)
70. An iOS build produces an Xcode project that builds for a device and for
    the simulator. It reports whether it meets the App Store's Xcode, SDK,
    privacy manifest, usage description, and launch screen requirements, and
    a missing microphone usage description is reported before submission.
    (PLT-001, BLD-001, PRD-002)
71. An iOS player with Behavior In Background set to Suspend delivers
    `OnApplicationPause(true)` when backgrounded and resumes with its state. A
    memory warning raises `Application.lowMemory`. A tvOS player reads the
    Siri Remote as a gamepad and honors `UnityEngine.tvOS.Remote` settings.
    (PLT-001, INP-002)
72. On a host without a console platform holder's SDK, each console target is
    Externally unavailable with the missing SDK or entitlement named, and no
    console capability is implied. (PLT-001, BLD-001)
73. On a host with a console platform holder's SDK and the user's
    entitlement, a console build deploys to a development kit, delivers
    suspend and resume as lifecycle messages, is never presented as
    certified, and states that distribution requires the commercial
    license. Its console-specific settings do not appear in diagnostics that
    leave the machine. (PLT-001, BLD-004, LIC-001)
