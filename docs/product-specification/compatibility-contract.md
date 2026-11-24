# Compatibility Contract

## User Outcome

As the owner of a supported Unity project, I can determine before editing,
playing, or building which project behaviors unitas supports, which have stated
limits, and which cannot yet be represented. Compatibility information refers
to my actual assets, scenes, scripts, settings, packages, and targets.

## Compatibility Baseline

The compatibility baseline covers declared behavior from Unity 2020.3 LTS
through Unity 2022.3 LTS, with Unity 2022.3 LTS as the principal behavioral
reference. A project can contain older serialized forms or settings where those
releases supported migration to the baseline.

Compatibility includes, when the feature is declared supported:

* The meaning and identity of project content.
* Serialized values and object references.
* Public scripting behavior, including timing and failure behavior.
* Editor-visible state that affects saved project content.
* Runtime visuals, input, audio, simulation, application lifecycle, and state.
* Player and build behavior on the selected target.

Matching names, file extensions, public members, or screen layouts alone is not
compatibility.

## Compatibility States

Every assessed behavior has one state:

* **Supported:** all behavior named by the applicable unitas specification is
  available for the stated project and target conditions.
* **Partially supported:** a defined subset behaves as specified and every
  excluded behavior is named.
* **Unsupported:** the content or request is recognized, but its required
  behavior is unavailable.
* **Unknown:** unitas cannot classify the content or behavior reliably.
* **Platform-limited:** the behavior is supported generally but unavailable or
  materially different on the selected target.
* **Externally unavailable:** the behavior requires an entitlement, service,
  device, private SDK, account, or permission that is absent.

The editor and automation interfaces expose the same compatibility state.

## Project Compatibility Report

A report covers project version, packages, settings, scenes, prefabs, assets,
serialized types, scripts and APIs, shaders and materials, input, physics,
animation, UI, audio, platform services, and selected build targets.

Each finding includes:

* Stable identity and location of the affected content.
* Feature and behavior being assessed.
* Compatibility state.
* Observable consequence during editing, play, testing, or building.
* Conditions under which the finding applies.
* Related findings when one limitation causes multiple downstream effects.

A report distinguishes “not present in this project” from “present and
supported.” It never converts absence into a compatibility claim.

## Behavioral Equivalence

Equivalent behavior means a player, developer, test scenario, or supported
script observes the same specified outcome. Exact internal ordering is required
only when Unity exposes that order as a contract or project behavior depends on
it. Exact pixel, sample, floating-point, or timing identity is required only
when the applicable feature specification states a tolerance or exact result.

Permitted environmental variation never excuses a semantic difference such as
the wrong object receiving an event, a reference resolving to a different
asset, a save losing a value, a collision failing to occur, or a control
performing another action.

## Round-Trip Expectations

Supported project edits preserve Unity-readable identities and content where
the corresponding Unity format can express the result. Unknown data that can
be retained without misrepresentation remains intact. If an edit would require
discarding or rewriting unsupported meaning, the user sees the scope and
chooses whether to continue. If saving an unrelated edit cannot retain unknown
data, that save is blocked until the user explicitly chooses to discard the
identified data.

Opening and saving an unchanged asset does not create unexplained semantic
changes. Differences in non-semantic formatting are never presented as proof of
compatibility.

## Failure Behavior

A failed compatibility operation leaves unrelated content unchanged. It
identifies what was attempted, what content was affected, what remains usable,
and whether a retry is safe. Partial success is reported as partial success.

unitas never substitutes a plausible default, fallback shader, empty reference,
different API result, or ignored setting while reporting full support.

## Supported Package Matrix

Unity package behavior is in scope only for the packages and version lines in
the matrix below. The matrix, each package's state, and the owning
specification are available to people and to agents through automation, as
defined in [Automation and MCP](automation-and-mcp.md).

Matrix rules:

* A version line for a baseline editor release means the versions of that
  package line that the Unity Package Manager marks as released or verified for
  that editor release. A version inside the named line that is not released for
  the project's editor release is Unknown.
* A package, or a package version, that the matrix does not list is Unknown
  until it is assessed. Preview, experimental, and prerelease versions are
  Unknown unless the matrix lists them.
* A Supported package provides the behavior its owning specification names;
  any limitation that specification states still applies. A Partially supported
  package names its excluded behavior in the matrix.
* Packages that a listed version requires as dependencies are covered to the
  extent the listed package's behavior depends on them. Project code that uses a
  dependency's own API directly is assessed on its own and is Unknown until
  assessed.
* A listed package that is embedded in the project or referenced from a local
  path, git repository, or tarball is matched to the listed version only when
  its content matches that released version; otherwise it is assessed as
  third-party content under [Third-Party and Asset-Store Content](#third-party-and-asset-store-content).
* Built-in engine modules (`com.unity.modules.*`) are part of the baseline
  engine rather than matrix entries; their behavior is owned by the feature
  specifications.
* The project compatibility report states, for every package in the project,
  the package identifier, the resolved version, its matrix state, and the
  owning specification.

| Package | 2020.3 LTS | 2021.3 LTS | 2022.3 LTS | State | Owning specification |
|---|---|---|---|---|---|
| Universal Render Pipeline (`com.unity.render-pipelines.universal`) | 10.x | 12.x | 14.x | Supported | [Rendering and animation](rendering-and-animation.md) (REN-006) |
| Shader Graph (`com.unity.shadergraph`) | 10.x | 12.x | 14.x | Partially supported: excludes the nodes that Rendering and animation names as unsupported, and graph targets for HDRP and Visual Effect Graph | [Rendering and animation](rendering-and-animation.md) (REN-006) |
| Post Processing v2 (`com.unity.postprocessing`) | 3.x | 3.x | 3.x | Supported | [Rendering and animation](rendering-and-animation.md) (REN-003) |
| 2D Sprite (`com.unity.2d.sprite`) | Released or verified | Released or verified | Released or verified | Supported | [Rendering and animation](rendering-and-animation.md) (REN-005) |
| 2D Tilemap Editor (`com.unity.2d.tilemap`) | Released or verified | Released or verified | Released or verified | Supported | [Rendering and animation](rendering-and-animation.md) (REN-005) |
| 2D Tilemap Extras (`com.unity.2d.tilemap.extras`) | Released or verified | Released or verified | Released or verified | Supported | [Rendering and animation](rendering-and-animation.md) (REN-005) |
| 2D Animation (`com.unity.2d.animation`) | Released or verified | Released or verified | Released or verified | Supported | [Rendering and animation](rendering-and-animation.md) (REN-005) |
| 2D Pixel Perfect (`com.unity.2d.pixel-perfect`) | Released or verified | Released or verified | Released or verified | Supported | [Rendering and animation](rendering-and-animation.md) (REN-005) |
| 2D PSD Importer (`com.unity.2d.psdimporter`) | Released or verified | Released or verified | Released or verified | Supported | [Projects and assets](project-and-assets.md) (PRJ-004) |
| Cinemachine (`com.unity.cinemachine`) | 2.x | 2.x | 2.x | Supported | [Rendering and animation](rendering-and-animation.md) (CIN-001) |
| Timeline (`com.unity.timeline`) | 1.x | 1.x | 1.x | Supported | [Rendering and animation](rendering-and-animation.md) (CIN-001) |
| uGUI (`com.unity.ugui`) | 1.0.x | 1.0.x | 1.0.x | Supported | [UI, audio, and input](ui-audio-input.md) (UI-001) |
| TextMeshPro (`com.unity.textmeshpro`) | 3.0.x | 3.0.x | 3.0.x | Supported | [UI, audio, and input](ui-audio-input.md) (TXT-001) |
| Input System (`com.unity.inputsystem`) | 1.x | 1.x | 1.x | Supported | [UI, audio, and input](ui-audio-input.md) (INP-002) |
| Localization (`com.unity.localization`) | 1.x | 1.x | 1.x | Supported | [Accessibility, localization, and persistence](accessibility-localization-persistence.md) (LOC-001) |
| Addressables (`com.unity.addressables`) | 1.x | 1.x | 1.x | Supported | [Scripting runtime](scripting-runtime.md) (RUN-006) |
| Burst (`com.unity.burst`) | 1.x | 1.x | 1.x | Supported: results are equivalent within the declared floating-point mode; native acceleration on a target is a reported performance property | [Scripting runtime](scripting-runtime.md) (RUN-011) |
| Mathematics (`com.unity.mathematics`) | 1.x | 1.x | 1.x | Supported | [Scripting runtime](scripting-runtime.md) (RUN-011) |
| Collections (`com.unity.collections`) | Released or verified | Released or verified | Released or verified | Supported | [Scripting runtime](scripting-runtime.md) (RUN-011) |
| Newtonsoft Json (`com.unity.nuget.newtonsoft-json`) | 3.x | 3.x | 3.x | Supported | [Scripting runtime](scripting-runtime.md) (RUN-009) |
| AI Navigation (`com.unity.ai.navigation`) | Not listed | Not listed | 1.x | Supported | [Physics, navigation, and terrain](physics-navigation-terrain.md) (NAV-001) |
| Unity Test Framework (`com.unity.test-framework`) | 1.x | 1.x | 1.x | Supported | [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md) (TST-003) |
| Netcode for GameObjects (`com.unity.netcode.gameobjects`) | 1.x | 1.x | 1.x | Supported | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) (NET-001) |
| XR Plugin Management (`com.unity.xr.management`) | 4.x | 4.x | 4.x | Supported | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) (XR-001) |
| XR Interaction Toolkit (`com.unity.xr.interaction.toolkit`) | 2.x | 2.x | 2.x | Supported | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) (XR-001) |

"Released or verified" means the versions the Unity Package Manager marks as
released or verified for that baseline editor release. "Not listed" means no
version of the package is in the matrix for that baseline; a project that uses
one is Unknown for it.

The following families are recognized and reported Unsupported, as declared in
the [index](INDEX.md). Each finding states the observable consequence, and the
rest of the project remains usable.

| Family | Recognized form | State | Owning specification |
|---|---|---|---|
| High Definition Render Pipeline | `com.unity.render-pipelines.high-definition` assets, materials, volumes, and settings | Unsupported | [Rendering and animation](rendering-and-animation.md) (REN-006) |
| Custom scriptable render pipelines | A project render pipeline asset that is neither the Built-in Render Pipeline nor URP | Unsupported | [Rendering and animation](rendering-and-animation.md) (REN-006) |
| Entities and Entities Graphics | `com.unity.entities`, `com.unity.entities.graphics`, and `com.unity.rendering.hybrid` content | Unsupported | [Scripting runtime](scripting-runtime.md) (RUN-011) |
| Netcode for Entities | `com.unity.netcode` content | Unsupported | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) (NET-001) |
| Visual Effect Graph | `com.unity.visualeffectgraph` assets | Unsupported | [Rendering and animation](rendering-and-animation.md) (REN-004) |
| UI Toolkit runtime panels | Runtime panel settings, UXML documents, and `UIDocument` components | Unsupported | [UI, audio, and input](ui-audio-input.md) (UI-001) |
| Visual Scripting | `com.unity.visualscripting` graphs and machines | Unsupported | [Scripting runtime](scripting-runtime.md) (RUN-002) |
| Baking inside unitas | Requests to bake lightmaps or occlusion culling data; Unity-baked lightmaps, light probes, reflection probe captures, and occlusion data remain Supported content | Unsupported | [Rendering and animation](rendering-and-animation.md) (REN-003) |
| Custom editor extension drawing | Custom inspectors, property drawers, and editor windows; their commands and content effects remain Supported | Unsupported | [Scripting runtime](scripting-runtime.md) (RUN-010) |

## Third-Party and Asset-Store Content

Third-party packages, Asset Store content, scoped-registry and git packages,
precompiled assemblies, native plugins, and vendor shaders are assessed like
the project's own content. Their scripts are compiled and their API use is
assessed under RUN-002 and RUN-007 in [Scripting runtime](scripting-runtime.md);
their native plugins are assessed per target; their shaders and materials are
assessed under [Rendering and animation](rendering-and-animation.md); their
editor extensions are assessed under RUN-010; and their serialized content is
assessed like any other asset. Their code runs only in a trusted project, as
owned by PRJ-008 in [Projects and assets](project-and-assets.md).

No vendor package receives a blanket compatibility claim. Each finding concerns
specific content and behavior. A per-package summary counts the package's
findings by state and shows the least-supported state among them, with Unknown
counted; it reads Supported only when every assessed behavior is Supported and
no content of the package is Unknown or unassessed.

Each finding for third-party content identifies:

* the vendor package by name, publisher as the package declares it, version,
  and source (package registry, git repository and revision, tarball, local
  path, or Asset Store import into the project folder);
* the affected content and its location;
* the features affected, by catalog identifier;
* the project scenes, prefabs, and assets that depend on the affected content.

Content imported from the Asset Store into the project folder has no package
manifest; it is identified by its folder and, where the content declares it,
its publisher and version. When the vendor cannot be determined, the content is
reported as project content under its folder.

A native plugin with no binary for the selected target or architecture is
Platform-limited for that target, naming the library and the missing
architecture. Behavior of a precompiled assembly beyond the Unity APIs it
references is Unknown until assessed. Use of non-public `UnityEngine` or
`UnityEditor` members, including through reflection, is outside the
compatibility surface and is reported Unsupported, naming the member, unless an
owning specification names that member.

Updating, adding, or removing a third-party package reassesses the findings it
affects. A vendor's own statement of compatibility with unitas is never treated
as compatibility evidence. unitas does not assess vendor license terms or grant
third-party entitlements.

## Product Versioning, Interface Stability, and Release Readiness

### Versioning

unitas releases use semantic versioning (`MAJOR.MINOR.PATCH`), with preview
releases identified by a prerelease suffix:

* A patch release changes Supported behavior only to correct a deviation from
  the specified behavior.
* A minor release can add features, targets, and package lines, and can raise a
  behavior's state (for example from Unknown or Partially supported to
  Supported).
* Lowering the state of a Supported behavior, removing a package line or target,
  changing Supported behavior other than to correct a deviation, or removing an
  automation tool, resource, command-line argument, or field after its
  deprecation window occurs only in a major release.

Every project compatibility report, build result, test and scenario result, and
benchmark result records the unitas version that produced it. Reports produced
by different unitas versions are distinguishable, and a comparison between them
identifies which differences follow from the version change.

Release notes list every observable behavior change to a Supported feature,
including corrections, every compatibility-state change, every package matrix
change, and every deprecation and removal, each with the affected catalog
identifiers.

### Interface stability

MCP tool and resource schemas, command-line arguments, exit statuses, and
machine-readable output formats are versioned. An agent discovers the schema
version of every tool and resource it uses. A deprecated tool, resource,
argument, or field remains available and is marked deprecated for a
deprecation window that includes at least the next minor release; each use
returns a deprecation notice naming the replacement and the release in which
removal can occur. The detailed automation contract is owned by
[Automation and MCP](automation-and-mcp.md).

### Release readiness

Each release declares, per target, which feature families (by catalog
identifier) are generally available. Release availability is separate from the
compatibility states: a family that is not generally available on a target is
declared preview or not offered there.

A feature family is generally available on target T in a release only when, on
T and with that release:

* the acceptance outcomes in its owning specification pass;
* the sample scenarios that map to it (SMP-001) pass;
* the behavioral benchmarks that cover it (CMP-001) pass, or each failure is
  named as an excluded behavior and the family is declared Partially supported;
* the engine performance budgets owned by PRF-002 in
  [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md)
  are met.

A regression beyond a PRF-002 budget, or a failed acceptance outcome, removes
the generally available declaration for the affected family on that target,
and the release notes state the removal.

A release declares target T production-ready only when:

* every family declared generally available on T meets the criteria above;
* player builds (BLD-001), the player lifecycle on T (PLT-001), and the build's
  licensing report (LIC-001) are generally available on T;
* the Roll-a-Ball-style sample and at least one further playable sample pass all
  of their acceptance scenarios both in editor play mode and in a player on T.

Each readiness declaration links to the scenario, benchmark, and budget
evidence it rests on, recorded with the unitas version, and is available to
people and agents through automation.

## Acceptance Stories

* Before relying on a release build, a project owner can identify every
  recognized, assessed unsupported behavior in the selected project and target;
  unknown or unassessed behavior remains explicit. (CMP-002, CMP-003)
* A missing asset reference remains missing and identifies its original
  identity rather than resolving to a similarly named asset. (PRJ-003, SER-004)
* A partially supported shader reports the omitted pass and visible effect.
  (REN-002, REN-006)
* A console target without private SDK access is externally unavailable rather
  than reported as an engine failure or a supported build. (PLT-001)
* Reassessment after a project change updates only affected findings and makes
  the report's project state clear. (CMP-002, CMP-003)
* A project using URP 14.x and Input System 1.x on Unity 2022.3 reports both as
  Supported with their owning specifications, while an unlisted package and a
  preview version of a listed package are each reported Unknown. (CMP-002)
* A project containing HDRP assets and a Visual Scripting graph reports each as
  Unsupported with its observable consequence, and its URP scenes remain
  editable and playable. (CMP-002)
* A locally modified embedded copy of a listed package is not matched to the
  released version and is assessed as third-party content. (CMP-002, CMP-003)
* An Asset Store package containing scripts, a native plugin without an ARM64
  binary, and a custom shader produces findings for each item that name the
  vendor package, version, source, affected features, and dependent scenes; its
  summary never reads Supported while any of its content is Unknown. (CMP-003)
* Reflection on a non-public `UnityEditor` member in a vendor package is
  reported Unsupported, naming the member. (CMP-003)
* A compatibility report and a build result each record the unitas version, and
  comparing reports from two versions identifies differences caused by the
  version change. (CMP-004)
* An agent calling a deprecated MCP tool receives a deprecation notice naming the
  replacement and removal release, and the tool remains available through the
  next minor release. (CMP-004, AUT-002)
* A release's readiness declaration for a target lists its generally available
  families with links to passing acceptance, sample, benchmark, and budget
  evidence; a family with a budget regression is not declared generally
  available. (CMP-004, SMP-001, CMP-001, PRF-002)
* A target is declared production-ready only when builds, player lifecycle, the
  licensing report, the Roll-a-Ball-style sample, and one further sample pass in
  editor and player on that target. (CMP-004, BLD-001, PLT-001, LIC-001,
  SMP-001)
