# Compatibility Contract

## User Outcome

As the owner of a supported Unity project, I can determine before editing,
playing, or building which project behaviors unitas supports, at which
compatibility tier, which have stated limits, and which cannot yet be
represented. Compatibility information refers to my actual assets, scenes,
scripts, settings, packages, and targets.

## Scope

This specification owns the compatibility baseline, the compatibility tiers,
the compatibility states, the project compatibility report, the supported
package matrix, third-party content assessment, product versioning and release
readiness, and the support lifecycle. Every report, matrix entry, and
declaration it defines is available to people and to agents through
automation, as defined in [Automation and MCP](automation-and-mcp.md).

Compatibility tiers of the features this specification owns, as recorded in
the [Feature catalog](feature-catalog.md): CMP-002 Native, CMP-003 Native,
CMP-004 Native, CMP-005 Native.

## Compatibility Baseline

The compatibility baseline is the documented, observable behavior of Unity
2022.3 LTS. Projects saved by Unity 2020.3 LTS, Unity 2021.3 LTS, or any
other earlier release that Unity 2022.3 LTS can upgrade, open by migrating
their serialized forms to the 2022.3 forms, as owned by SER-005 in
[Serialization and prefabs](serialization-and-prefabs.md); behavior that
differs between those releases and Unity 2022.3 LTS is not matched. Unity
releases after 2022.3 LTS are outside the baseline; a feature of a later
release is part of unitas only where a specification adopts it explicitly.

Compatibility includes, to the extent the tier of the owning feature promises
it:

* The meaning and identity of project content.
* Serialized values and object references.
* Public scripting behavior, including timing and failure behavior.
* Editor-visible state that affects saved project content.
* Runtime visuals, input, audio, simulation, application lifecycle, and state.
* Player and build behavior on the selected target.

Matching names, file extensions, public members, or screen layouts alone is not
compatibility.

## Compatibility Tiers

Every catalog feature has exactly one tier, recorded in the
[Feature catalog](feature-catalog.md). A behavior takes the tier of the catalog
feature that owns it, and no section of a specification claims a higher tier
than its feature's. The tier states what is promised; the compatibility state
states how much of that promise holds for specific content and targets.

* **Exact:** content identity and serialization match Unity 2022.3 LTS
  exactly. Values, identities (GUIDs and local identifiers), ordering,
  references, prefab overrides, and round-trip saving are preserved without
  change. Any divergence is a defect; until it is corrected, the affected
  behavior is reported Partially supported, naming the divergence.
* **Semantic:** the documented, observable behavior of Unity 2022.3 LTS in
  ordinary use applies. Ordinary use means using public, documented APIs and
  editor-saved content as the Unity 2022.3 LTS documentation describes them.
  Undocumented behavior, internal ordering that Unity does not document, and
  bug-for-bug reproduction of Unity defects are not promised. A found
  difference is reported, and the affected behavior is Partially supported,
  naming the difference.
* **Familiar:** Unity content imports with its identity, and public types and
  members keep their Unity 2022.3 LTS names and signatures (API shape). The
  unitas specification text defines the behavior; Familiar sections state that
  behavior concretely and do not defer to Unity's behavior. Every known
  difference from Unity 2022.3 LTS is listed in the API reference owned by
  AUT-006 in [Automation and MCP](automation-and-mcp.md) and in the project
  compatibility report for content that relies on it.
* **Native:** behavior is unitas-defined. Unity content (shaders, materials,
  settings, build configuration, and editor extensions) is imported or
  translated. Translated shaders and materials do not promise pixel identity
  with Unity; visual acceptance uses unitas-approved references as defined by
  TST-002 in
  [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md),
  not Unity screenshots. Scripting-visible members of Native features, such as
  `Camera.WorldToScreenPoint`, keep the meaning their Unity 2022.3 LTS
  documentation states.

Evidence for a claim depends on its tier:

* Exact and Semantic claims are established by behavioral compatibility
  benchmarks (CMP-001, owned by
  [Playable samples and benchmarks](sample-games/INDEX.md)) whose expectations are
  observations of Unity 2022.3 LTS.
* Familiar and Native claims are established by the acceptance outcomes of the
  owning unitas specification and by unitas-approved references. They are not
  established or refuted by comparison with Unity output, and every difference
  from Unity 2022.3 LTS is listed in the AUT-006 API reference and in project
  compatibility reports.

Behavior outside a feature's tier is reported with a compatibility state and is
never silently approximated.

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

Partially supported, Unsupported, and Platform-limited describe the
product's current measured state. They are never the declared end state for
content or behavior that Unity 2022.3 LTS supports. Every such finding is a
defect against its owning feature, to be closed until the behavior is
Supported. Platform-limited applies only where the target itself lacks the
capability. Externally unavailable applies only where an absent entitlement,
service, device, SDK, account, or permission prevents the behavior.

The editor and automation interfaces expose the same compatibility state.

## Project Compatibility Report

A report covers project version, packages, settings, scenes, prefabs, assets,
serialized types, scripts and APIs, shaders and materials, input, physics,
animation, UI, audio, platform services, and selected build targets.

Each finding includes:

* Stable identity and location of the affected content.
* Feature and behavior being assessed.
* Compatibility tier of the owning feature and compatibility state.
* For Familiar and Native features, each listed difference from Unity 2022.3
  LTS that the content relies on.
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

In sections of Exact-tier and Semantic-tier features, wherever a
specification says "as in Unity", "Unity's rules", "Unity's conditions", or "as
the package defines", the reference is the documented and observable behavior
of Unity 2022.3 LTS, or of the package version in the supported package matrix
that the project resolves. When Unity 2022.3 LTS and that package version
differ in a behavior, Unity 2022.3 LTS controls. These phrases carry no meaning
in sections of Familiar-tier and Native-tier features, which state their
behavior directly.

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
the matrix below. The matrix, each package's tier and state, and the owning
specification are available to people and to agents through automation, as
defined in [Automation and MCP](automation-and-mcp.md).

Matrix rules:

* A version line means the versions of that package line that the Unity
  Package Manager marks as released or verified for Unity 2022.3 LTS. A version
  inside the line that is not released for Unity 2022.3 LTS is Unknown.
* A project saved by an earlier release that Unity 2022.3 LTS can upgrade, and
  that resolves an earlier
  line of a listed package (for example URP 10.x or 12.x) has that package's
  serialized content migrated as SER-005 defines and is assessed against the
  listed line; behavior of the earlier line that differs from the listed line
  is not matched.
* A package, or a package version, that the matrix does not list is Unknown
  until it is assessed. Preview, experimental, and prerelease versions are
  Unknown unless the matrix lists them.
* A package takes the tier of its owning catalog feature.
* A Supported package provides the documented public behavior of the listed
  version, subject to the limitations its owning specification states; for a
  package whose owning feature is in the Familiar or Native tier, the owning
  specification defines the behavior where it says so. Behavior that the owning
  specification neither names nor excludes is Unknown until assessed and is
  reported as such. A Partially supported package names its excluded behavior
  in the matrix.
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
  the package identifier, the resolved version, its tier, its matrix state, and
  the owning specification.

| Package | Unity 2022.3 LTS version line | Tier | State | Owning specification |
|---|---|---|---|---|
| Universal Render Pipeline (`com.unity.render-pipelines.universal`) | 14.x | Native | Supported, including the Forward, Forward+, and Deferred rendering paths | [Rendering and animation](rendering-and-animation.md) (REN-006) |
| Shader Graph (`com.unity.shadergraph`) | 14.x | Native | Supported, including graph targets for URP, HDRP, the Built-in Render Pipeline, and Visual Effect Graph, and every node of the 14.x line | [Rendering and animation](rendering-and-animation.md) (REN-006) |
| Post Processing v2 (`com.unity.postprocessing`) | 3.x | Native | Supported | [Rendering and animation](rendering-and-animation.md) (REN-003) |
| 2D Sprite (`com.unity.2d.sprite`) | Released or verified | Native | Supported | [Rendering and animation](rendering-and-animation.md) (REN-005) |
| 2D Tilemap Editor (`com.unity.2d.tilemap`) | Released or verified | Native | Supported | [Rendering and animation](rendering-and-animation.md) (REN-005) |
| 2D Tilemap Extras (`com.unity.2d.tilemap.extras`) | Released or verified | Native | Supported | [Rendering and animation](rendering-and-animation.md) (REN-005) |
| 2D Animation (`com.unity.2d.animation`) | Released or verified | Native | Supported | [Rendering and animation](rendering-and-animation.md) (REN-005) |
| 2D Pixel Perfect (`com.unity.2d.pixel-perfect`) | Released or verified | Native | Supported | [Rendering and animation](rendering-and-animation.md) (REN-005) |
| 2D PSD Importer (`com.unity.2d.psdimporter`) | Released or verified | Native | Supported | [Rendering and animation](rendering-and-animation.md) (REN-005) |
| Cinemachine (`com.unity.cinemachine`) | 2.x | Familiar | Supported | [Rendering and animation](rendering-and-animation.md) (CIN-001) |
| Timeline (`com.unity.timeline`) | 1.x | Familiar | Supported | [Rendering and animation](rendering-and-animation.md) (CIN-001) |
| uGUI (`com.unity.ugui`) | 1.0.x | Familiar | Supported | [UI, audio, and input](ui-audio-input.md) (UI-001) |
| TextMeshPro (`com.unity.textmeshpro`) | 3.0.x | Familiar | Supported | [UI, audio, and input](ui-audio-input.md) (TXT-001) |
| Input System (`com.unity.inputsystem`) | 1.x | Familiar | Supported | [UI, audio, and input](ui-audio-input.md) (INP-002) |
| Localization (`com.unity.localization`) | 1.x | Familiar | Supported | [Accessibility, localization, and persistence](accessibility-localization-persistence.md) (LOC-001) |
| AI Navigation (`com.unity.ai.navigation`) | 1.x | Familiar | Supported | [Physics, navigation, and terrain](physics-navigation-terrain.md) (NAV-001) |
| Netcode for GameObjects (`com.unity.netcode.gameobjects`) | 1.x | Familiar | Supported | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) (NET-001) |
| XR Plugin Management (`com.unity.xr.management`) | 4.x | Familiar | Supported | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) (XR-001) |
| XR Interaction Toolkit (`com.unity.xr.interaction.toolkit`) | 2.x | Familiar | Supported | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) (XR-001) |
| Addressables (`com.unity.addressables`) | 1.x | Semantic | Supported | [Scripting runtime](scripting-runtime.md) (RUN-006) |
| Burst (`com.unity.burst`) | 1.x | Semantic | Supported: results are equivalent within the declared floating-point mode; native acceleration on a target is a reported performance property | [Scripting runtime](scripting-runtime.md) (RUN-011) |
| Mathematics (`com.unity.mathematics`) | 1.x | Semantic | Supported | [Scripting runtime](scripting-runtime.md) (RUN-011) |
| Collections (`com.unity.collections`) | Released or verified | Semantic | Supported | [Scripting runtime](scripting-runtime.md) (RUN-011) |
| Newtonsoft Json (`com.unity.nuget.newtonsoft-json`) | 3.x | Semantic | Supported | [Scripting runtime](scripting-runtime.md) (RUN-009) |
| Unity Test Framework (`com.unity.test-framework`) | 1.x | Semantic | Supported | [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md) (TST-003) |
| IDE integration (`com.unity.ide.visualstudio`, `com.unity.ide.rider`, `com.unity.ide.vscode`) | Released or verified | Native | Supported: project-file generation, debugger attachment (DIA-002), and the packages' preferences | [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md) (DIA-002) |
| Version Control (`com.unity.collab-proxy`) | Released or verified | Native | Supported: the package and its settings are preserved, and the editor provides source-control status, history, diff, and commit (EDT-007) | [Editor and authoring](editor-and-authoring.md) (EDT-007) |
| High Definition Render Pipeline (`com.unity.render-pipelines.high-definition`) | 14.x | Native | Supported: assets, materials, volumes, and settings | [Rendering and animation](rendering-and-animation.md) (REN-006) |
| Custom scriptable render pipelines (the Scriptable Render Pipeline API) | 14.x core | Native | Supported: project render pipeline assets that implement their own pipeline | [Rendering and animation](rendering-and-animation.md) (REN-006) |
| Visual Effect Graph (`com.unity.visualeffectgraph`) | 14.x | Native | Supported | [Rendering and animation](rendering-and-animation.md) (REN-004) |
| Entities (`com.unity.entities`) and Entities Graphics (`com.unity.entities.graphics`) | 1.x | Semantic | Supported | [Scripting runtime](scripting-runtime.md) (RUN-011) |
| Netcode for Entities (`com.unity.netcode`) | 1.x | Familiar | Supported | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) (NET-001) |
| Unity Transport (`com.unity.transport`) | 1.x and 2.x | Familiar | Supported, including its network simulator pipeline | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) (NET-001) |
| OpenXR Plugin (`com.unity.xr.openxr`) | 1.x | Familiar | Supported | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) (XR-001) |
| XR Hands (`com.unity.xr.hands`) | 1.x | Familiar | Supported on devices that expose hand tracking | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) (XR-001) |
| UI Toolkit runtime (`UIDocument`, UXML, USS, panel settings) | built into Unity 2022.3 LTS | Familiar | Supported | [UI, audio, and input](ui-audio-input.md) (UI-001) |
| Visual Scripting (`com.unity.visualscripting`) | 1.x | Semantic | Supported | [Scripting runtime](scripting-runtime.md) (RUN-002) |
| AR Foundation (`com.unity.xr.arfoundation`) with Apple ARKit (`com.unity.xr.arkit`) and Google ARCore (`com.unity.xr.arcore`) | 5.x | Familiar | Supported on devices that expose AR tracking | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) (XR-001) |
| Unity Gaming Services SDKs (`com.unity.services.*`, including Authentication, Lobby, Relay, Cloud Save, and Analytics) | Released or verified | Familiar | Supported: the SDK behavior runs, and calls reach the services when the project has a service account and credentials; without them, calls return the services' own unavailable errors | [Builds, platforms, XR, and networking](build-platforms-xr-networking.md) (NET-001, PRD-002) |

"Released or verified" means the versions the Unity Package Manager marks as
released or verified for Unity 2022.3 LTS.

The matrix grows until it lists every package Unity publishes as released or
verified for Unity 2022.3 LTS. No package family and no built-in Unity feature
is excluded from unitas because of engine effort. A package line is reported
Unknown only until it is assessed. Each assessment ends with the line Supported
in the matrix, or with every gap recorded as a defect against its owning
feature.

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
references is Unknown until assessed. Non-public `UnityEngine` and
`UnityEditor` members that packages and projects reach through reflection are
supported where an owning specification names them. Any other non-public
member is reported Unknown, naming the member and the code that uses it. Each
member found in assessed third-party content is then added to its owning
specification and supported.

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
* Deliberately lowering the state of a Supported behavior, removing a package
  line or target, changing Supported behavior other than to correct a
  deviation, or removing an automation tool, resource, command-line argument,
  or field after its deprecation window occurs only in a major release.
* Any release lowers a state when the earlier claim is found inaccurate and
  that release does not correct the deviation; its release notes list the
  change as a corrected claim with the affected catalog identifiers.

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
* the behavioral benchmarks that cover it (CMP-001) pass, or each failing
  behavior is named as an exclusion in the family's Partially supported claim
  for T, and the readiness declaration lists those exclusions;
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

## Support Lifecycle, Host Requirements, and Store Requirements

Each release line declares its support window: the date it was released, the
period during which it receives security patches, the period during which it
receives correction patches, and the date support ends. A release line past the
end of its support window is identified as unsupported in the editor, on the
command line, in build results, and through automation, and continues to run
as installed.

Each release declares:

* the supported host operating systems, with their versions and architectures,
  for the editor, the command line, and headless operation;
* for each target, the minimum target operating system version and, where the
  target has one, the minimum API level;
* for the web target, the minimum version of each supported browser (Chrome,
  Edge, Firefox, and Safari, on desktop and mobile) for WebGPU and for WebGL 2,
  and the browser features each optional capability needs;
* for each target with a store, the store submission requirements that the
  release meets, such as the Android target API level and the minimum Xcode and
  platform SDK versions used for Apple targets.

A host outside the declared host requirements is reported before a project is
opened, naming the requirement it fails. A target whose store submission
requirement the release does not meet reports store submission blocked, naming
the requirement, while development and local builds for that target remain
available. These declarations are versioned with the release under CMP-004 and
are available to people and agents through automation.

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
* A project using URP 14.x and Input System 1.x on Unity 2022.3 LTS reports both
  as Supported with their tiers and owning specifications, while an unlisted package and a
  preview version of a listed package are each reported Unknown. (CMP-002)
* A project containing HDRP assets and a Visual Scripting graph reports each as
  Supported with its tier and owning specification, renders its HDRP scenes,
  and runs its graph. (CMP-002)
* A locally modified embedded copy of a listed package is not matched to the
  released version and is assessed as third-party content. (CMP-002, CMP-003)
* An Asset Store package containing scripts, a native plugin without an ARM64
  binary, and a custom shader produces findings for each item that name the
  vendor package, version, source, affected features, and dependent scenes; its
  summary never reads Supported while any of its content is Unknown. (CMP-003)
* Reflection on a non-public `UnityEditor` member in a vendor package that no
  owning specification names is reported Unknown, naming the member and the
  calling code. After assessment, the member is specified and supported.
  (CMP-003)
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
* A readiness declaration for a target with a failing benchmark lists the
  failing behavior as an exclusion of that family's Partially supported claim.
  (CMP-004, CMP-001)
* A project saved by Unity 2021.3 LTS with URP 12.x opens with its serialized
  forms migrated, is assessed against Unity 2022.3 LTS and the URP 14.x line,
  and its report does not claim 2021.3-specific behavior. (CMP-002)
* A report finding for a Semantic-tier behavior cites Unity 2022.3 LTS as its
  reference and is backed by a CMP-001 benchmark, while a finding for a
  Familiar-tier behavior cites the owning unitas specification and lists each
  difference from Unity 2022.3 LTS that the content relies on. (CMP-002,
  CMP-001)
* A translated material is accepted against a unitas-approved reference image,
  and its report never presents a pixel difference from a Unity screenshot as
  a compatibility failure. (CMP-002, REN-002, TST-002)
* A Supported package behavior that its owning specification neither names nor
  excludes is reported Unknown, and the matrix shows each package's tier from
  its owning catalog feature. (CMP-002)
* A project that uses Enlighten realtime global illumination, loads Unity-built
  AssetBundles, and includes the Rider IDE and Version Control packages reports
  all four as Supported. Its realtime GI updates, its bundles load, and its
  source-control status appears in the editor. (CMP-002, REN-003, BLD-005,
  DIA-002, EDT-007)
* A release that finds an earlier Supported claim inaccurate and does not fix it
  lowers the state and lists the corrected claim in its release notes with the
  affected catalog identifiers. (CMP-004)
* A Shader Graph asset that uses any node of the 14.x line compiles and renders
  on its URP, HDRP, Built-in, and Visual Effect Graph targets. A malformed graph
  is reported by graph and node. (CMP-002, REN-006)
* A release line reports its security and correction patch periods and end of
  support, and an installation past end of support identifies itself as
  unsupported while continuing to run. (CMP-005)
* An Android build from a release that does not meet the current store target
  API level reports store submission blocked with that requirement, while a
  development build for the device still succeeds; an unsupported host
  operating system version is reported before the project opens. (CMP-005,
  BLD-001)
