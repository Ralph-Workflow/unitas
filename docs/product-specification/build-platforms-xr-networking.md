# Builds, Platforms, XR, and Networking Product Specification

## Scope

This specification defines the user-visible build product, headless and
continuous-integration operation, scripting backends and code stripping, player
behavior, platform integration, XR behavior, the supported networked-game
experience, product licensing as it applies to shipped players, and product
installation, updates, and target toolchains.

Every capability in this specification is available to authorized agents
through structured automation and MCP as well as to people, under the rules
owned by [Automation and MCP](automation-and-mcp.md). Package-provided behavior
(XR, networking, content delivery, input) is in scope only for the package
families and version lines listed in the supported package matrix (CMP-002) of
the [Compatibility contract](compatibility-contract.md).

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
* execute a named static editor method in the project, equivalent to Unity's
  `-executeMethod`, with editor-time script behavior as defined by RUN-010 in
  [Scripting runtime](scripting-runtime.md).

The Unity command-line arguments that build and test scripts commonly use
(`-batchmode`, `-nographics`, `-quit`, `-projectPath`, `-buildTarget`,
`-executeMethod`, `-logFile`, `-runTests`, `-testPlatform`, `-testResults`) are
accepted with equivalent meaning. An argument that unitas does not recognize is
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
[Projects and assets](project-and-assets.md).

unitas-owned caches can be preserved and restored between continuous-
integration runs. A restored cache whose recorded inputs match the project
state is reused; a stale, incompatible, or corrupt cache is detected, rebuilt,
and reported. A run using a restored cache produces the same results and
artifacts as a run from a clean state.

Multiple invocations on one machine run concurrently on different projects
without interfering. Concurrent invocations on the same project either operate
on isolated state or the later invocation reports that the project is in use,
naming the holder, and waits or exits as the invocation requested; neither run
corrupts project state.

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
limitation is reached at runtime, the player raises the Unity-compatible
exception naming the method that could not be executed.

The Unity managed stripping levels (Disabled, Minimal on baseline releases that
offer it, Low, Medium, and High) remove unused managed code with equivalent
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

The desired target experience covers Windows x86-64 and ARM64, macOS Apple
Silicon and Intel, Linux x86-64 and AArch64, Android ARM64 and x86-64, iOS
ARM64, browser delivery, Linux handhelds, supported XR platforms, and licensed
console environments when the user possesses required access.

Compatibility reporting distinguishes targets and capabilities derived from the
Unity 2020.3–2022.3 baseline from unitas extensions such as WebGPU, visionOS-
class devices, and architecture variants not offered uniformly by those Unity
releases. An extension is not presented as evidence of Unity parity.

Target status uses the compatibility states defined in the
[Compatibility contract](compatibility-contract.md). A target is Supported,
Partially supported with its excluded behavior named, Unsupported, or
Externally unavailable; an Externally unavailable target states whether the
cause is the current host (for example an Apple target on a host without the
required Apple toolchain), a missing toolchain, or a missing user entitlement.
Proprietary platform access is never implied by the existence of a target name.

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
not list is Unknown.

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
line that CMP-002 lists. Netcode for Entities is recognized and reported
Unsupported, as declared in the [index](INDEX.md).

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
a GPLv3 build, the result states that the GPLv3 applies to the included unitas
components, that recipients of the player are entitled to their corresponding
source, and how the build makes that source available or where it is offered;
the player includes the GPLv3 license text. For a commercially licensed build,
the result identifies the commercial license that governs the included unitas
components.

For targets whose distribution terms are known to conflict with GPLv3
distribution, including licensed console environments and app stores whose
terms are known to impose such conflicts, the build result and target status
state that distribution requires the commercial license. A development build for
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

A project records the unitas version last used with it in unitas-owned project
state, without altering any Unity-readable file; that storage location is owned
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
6. Two invocations on the same project at once either run on isolated state or
   the second reports the project in use with its holder; neither corrupts the
   project. (BLD-002)
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
13. A GPLv3 build's result records that the GPLv3 governs the included unitas
    components, the corresponding-source obligation, and the included
    third-party notices; a console build states that distribution requires the
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
