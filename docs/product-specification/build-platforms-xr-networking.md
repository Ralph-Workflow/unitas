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
* unsupported or platform-limited project behavior.

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

A template, template field, or API member that unitas does not honor is
reported in the build result, naming the file or member; it is never silently
ignored.

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
display, no visual editor, and no interactive session. A single invocation can:

* open a project and import or reimport its assets;
* compile project scripts and report diagnostics;
* run project tests, as defined by TST-003 in
  [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md);
* run structured scenarios, as defined by TST-002 in the same specification;
* produce player builds for one or more targets;
* execute a named static editor method in the project, corresponding to Unity's
  `-executeMethod`, with editor-time script behavior as defined by RUN-010 in
  [Scripting runtime](scripting-runtime.md).

The Unity command-line arguments that build and test scripts commonly use
(`-batchmode`, `-nographics`, `-quit`, `-projectPath`, `-buildTarget`,
`-executeMethod`, `-logFile`, `-runTests`, `-testPlatform`, `-testResults`) are
accepted, and each selects the corresponding operation or option defined in
this section. An argument that unitas does not recognize is
reported by name; it is never silently ignored.

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

Ahead-of-time limitations are reported at build time, naming the type and
member involved, when they are statically detectable. These include generic
instantiations that cannot be generated ahead of time, `System.Reflection.Emit`,
the C# `dynamic` keyword, and compilation of expression trees. When such a
limitation is reached at runtime, the player raises an
`ExecutionEngineException` naming the method that could not be executed.

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

Targets are Windows x86-64 and ARM64, macOS Apple Silicon and Intel, Linux
x86-64 and AArch64, Android ARM64 and x86-64, iOS ARM64, browser delivery,
Linux-based handheld devices (for example Steam Deck), XR devices reached
through the XR Plugin Management providers that CMP-002 lists, and licensed
console environments when the user possesses required access.

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

Windows and Linux dedicated server players build from the Dedicated Server
target and run with no rendering or audio output. A project migrated from Unity
2020.3 LTS with the Server Build option enabled opens with the Dedicated Server
target selected. A server build defines `UNITY_SERVER`, applies server-build
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

Editor play and built-player behavior remain equivalent for supported runtime
features except where an editor-only capability is explicitly identified.

## Browser behavior

A browser player communicates initial loading progress, successful startup,
missing browser capability, storage availability, audio activation requirements,
focus changes, resizing, full-screen changes, and unrecoverable startup errors.

Browser security or permission restrictions are visible. Page refresh and
navigation away end the player session; data the game wrote to persistent
storage before that point remains available to the next session. Offline state
is reported to the game and to diagnostics. Storage quota exhaustion makes the
failing write report failure to the game rather than apparent success. A lost
graphics context is either restored with rendering resumed or reported as an
unrecoverable error. A suspended audio context resumes after the next user
gesture, and the game can observe whether audio is currently permitted.

## Mobile behavior

Mobile players handle safe areas, orientation, touch cancellation, application
backgrounding and foregrounding, interruption, permissions, constrained memory,
and supported device changes. A tester can determine whether a pause came from
game logic, operating-system interruption, or focus loss.

## XR and spatial behavior

XR behavior is provided through XR Plugin Management and the XR Interaction
Toolkit within the version lines that CMP-002 lists. An XR provider plug-in is
reported against its own package and version; a provider that the matrix does
not list is Unknown. XR content imports with its identity, and the public types
and members of those packages keep their Unity 2022.3 LTS names and
signatures; the behavior is the behavior this section defines, and every known
difference from Unity is listed in the API reference owned by AUT-006 in
[Automation and MCP](automation-and-mcp.md).

XR exposes tracked head and controller poses, tracking state, declared actions,
device capabilities, render views, recentering, interaction, supported haptics,
and focus state.

Projects distinguish unavailable hardware, initialization failure, denied
permission, lost or limited tracking, unsupported features, and application
interruption. World scale, seated or standing mode, locomotion, comfort choices,
world-space UI, selection, grabbing, and spatial audio have inspectable state.

XR playtests record device, runtime, refresh behavior, frame timing, tracking
events, input capabilities, comfort settings, and failures. A desktop preview is
not represented as proof of real-device correctness.

## Networking behavior

Networked play is provided through Netcode for GameObjects within the version
line that CMP-002 lists, with Unity Transport (`com.unity.transport`) as its
transport dependency. Netcode for Entities is recognized and reported
Unsupported, as declared in the [index](INDEX.md). Netcode content imports with
its identity, and the API shape below keeps its Unity 2022.3 LTS names and
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
duplication, and reordering are visibly marked as test conditions.

Network success is separate from local gameplay success. A locally correct
action is not reported as synchronized until the declared remote observation
occurs. Conflicts, authority rejection, and stale state are explicit.

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
or emulator, an iOS device or simulator, a desktop host, or a local browser.
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

A player that encounters content built for an incompatible player version, or
content built by Unity rather than by unitas, reports each affected bundle by
name with the reason, and does not load that content partially. Runtime loading
behavior is owned by RUN-006 in [Scripting runtime](scripting-runtime.md).

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
itself did not request. The editor, the command line, and the MCP server send
project content only to MCP clients and external tools that a person authorized
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
7. An iOS build reports a statically detectable `System.Reflection.Emit` use by
   type and member, and a development build at High stripping that reflects on a
   stripped member raises an exception naming the member and the preservation
   mechanisms; preserving it with `link.xml` removes the exception. (BLD-003)
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
    a new catalog that the shipped player loads; content built for an
    incompatible player version or by Unity is reported by bundle name and
    reason and is not loaded partially. (BLD-005, RUN-006)
26. A player built from a project that makes no network requests opens no
    network connection, the editor and MCP server send project content only to
    authorized MCP clients and to destinations a person directed, unitas crash
    reporting stays off until a person enables it, and an iOS
    build includes a privacy manifest for the included unitas components.
    (PRD-002)
27. An Android build honors a custom `mainTemplate.gradle` and an `.androidlib`
    folder, runs `IPostGenerateGradleAndroidProject`, and can export the Gradle
    project; an iOS build's `[PostProcessBuild]` method adds a capability
    through `ProjectCapabilityManager`; an unhonored template field is reported
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
