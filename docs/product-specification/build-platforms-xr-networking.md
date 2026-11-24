# Builds, Platforms, XR, and Networking Product Specification

## Scope

This specification defines the user-visible build product, player behavior,
platform integration, XR behavior, and supported networked-game experience.

## Build configuration

As a developer, I can select scenes and order, product identity, semantic or
platform version, icons, orientation or window behavior, displays, quality,
development diagnostics, scripting compatibility, content choices, and
target-specific options.

Every option states its current value, default, valid choices, affected targets,
and player-visible effect. Invalid combinations are reported before distribution
with the setting and consequence identified.

Development and release builds are visibly distinct. A release build never
contains development-only access merely because an earlier build enabled it.

## Build results

A successful build produces the artifacts expected by the target, including a
desktop application, Android package, iOS application project or package,
browser bundle, XR application, or licensed-console package as applicable.

Every result records:

* target, architecture, configuration, product and version;
* included scenes and effective player settings;
* success, failure, cancellation, and elapsed time;
* errors and warnings linked to content or settings;
* output location and primary launch artifact;
* total size, useful size categories, and major contributors;
* presence of development diagnostics;
* unsupported or platform-limited project behavior.

Cancellation leaves no artifact presented as a successful player. Repeated
builds from identical project state and choices have no unexplained behavioral
or content differences.

## Target families

The desired target experience covers Windows x86-64 and ARM64, macOS Apple
Silicon and Intel, Linux x86-64 and AArch64, Android ARM64 and x86-64, iOS
ARM64, browser delivery, Linux handhelds, supported XR platforms, and licensed
console environments when the user possesses required access.

Compatibility reporting distinguishes targets and capabilities derived from the
Unity 2020.3–2022.3 baseline from unitas extensions such as WebGPU, visionOS-
class devices, and architecture variants not offered uniformly by those Unity
releases. An extension is not presented as evidence of Unity parity.

Target status distinguishes supported, partially supported, unavailable on the
current host, unavailable due to missing user entitlement, and unsupported.
Proprietary platform access is never implied by the existence of a target name.

## Player lifecycle

The player starts in the configured first scene, exposes startup failures,
handles focus, pause, resume, low-memory notification where available, quit
requests, and shutdown according to platform expectations.

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

Browser security or permission restrictions are visible. Refresh, navigation,
offline state, storage quota, and lost graphics or audio context have defined
player behavior within the supported scope.

## Mobile behavior

Mobile players handle safe areas, orientation, touch cancellation, application
backgrounding and foregrounding, interruption, permissions, constrained memory,
and supported device changes. A tester can determine whether a pause came from
game logic, operating-system interruption, or focus loss.

## XR and spatial behavior

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

When a supported networking package and version is declared for the project, a
game can represent sessions,
connections, player identity, ownership, messages, synchronized state, remote
actions, timeouts, disconnection, reconnection, late joining, and incompatible
versions.

As a player, I receive distinct outcomes for successful connection, refused
connection, unreachable endpoint, full session, authentication failure, version
mismatch, timeout, host departure, and deliberate disconnection.

As a developer, I can associate logs, objects, state changes, traffic, and
errors with the correct participant. Multiple local test instances retain
distinct identity. Simulated latency, jitter, loss, duplication, and reordering
are visibly marked as test conditions.

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

Input, Addressables-like content delivery, localization, networking, and XR
capabilities that originate in separately versioned Unity packages are reported
against the specific compatible package and version range. Project use of one
version is not evidence that every version or provider behaves identically.

## Acceptance stories

1. A configured project produces the expected artifact for each declared target
   or a precise target-specific incompatibility report.
2. A built player launches the configured scene and handles focus, pause,
   permissions, and shutdown correctly on its platform.
3. Mobile and browser lifecycle changes preserve or reject game state in a
   defined, inspectable way.
4. XR play distinguishes real hardware state, tracking loss, input capability,
   and performance from desktop approximation.
5. A multiplayer sample distinguishes connection, synchronization, gameplay,
   and service failures and identifies the affected participant.

