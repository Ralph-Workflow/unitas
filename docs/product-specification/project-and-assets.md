# Project and Asset Management

## Scope

This specification defines what users experience when opening a Unity project,
deciding whether to trust it, browsing its contents, importing source files,
resolving dependencies and packages, and working with project settings. It
covers project recognition, Unity version handling, project trust, the asset
browser, dependencies, import state and diagnostics, common source-asset
families, package resolution, and project settings. It owns PRJ-001, PRJ-002,
and PRJ-004 through PRJ-008 in the [Feature catalog](feature-catalog.md).

Asset identity (GUIDs, local identifiers, and sub-asset references, PRJ-003)
and serialization forms and generated caches (SER-005) are owned by
[Serialization and prefabs](serialization-and-prefabs.md). This document
relies on those contracts and does not restate them.

These are desired end-state requirements, not claims about currently shipped
capability. Unity 2022.3 LTS behavior is the baseline where a feature is part of
stock Unity. Package behavior is covered only for the package families and
version lines listed in the supported package matrix (CMP-002 in
[Compatibility contract](compatibility-contract.md)).

Every capability in this document, including opening, searching, importing,
package changes, and settings changes, is available to authorized agents
through automation as defined in [Automation and MCP](automation-and-mcp.md).
Granting project trust is the one exception: agents can request it but cannot
grant it.

## Project opening

### User stories

* As a developer, I can open the root directory of a supported Unity project
  without creating a converted copy.
* As a developer returning to a project, I see the same asset identities,
  references, settings, and openable content I saw previously.
* As a developer moving between Unity and unitas, I can see external changes and
  choose how to reconcile them with unsaved editor work.
* As a developer evaluating compatibility, I can identify each assessed item's
  compatibility state and separately see missing, ambiguous, or invalid content
  as diagnostics.
* As an agent, I can open a project headless and receive the same project
  state, diagnostics, and compatibility findings a person sees.

### Required behavior

The project opens as a coherent collection of assets, packages, scenes, scripts,
project settings, and metadata. Recognition does not require moving source files
or replacing the Unity directory structure. Project identity is independent of
the machine-specific absolute path used to open it.

Opening does not require Unity's `Library/` folder or any other
Unity-generated cache, and never writes into Unity-generated folders; the
detailed contract is owned by SER-005 in
[Serialization and prefabs](serialization-and-prefabs.md).

Opening communicates progress and remains inspectable during long operations.
The project can be closed or an operation cancelled without presenting a partial
result as complete. Reopening unchanged content produces the same asset
identities and dependency relationships. A project with at least 100,000
assets opens and remains operable through automation; the timing budgets that
apply are declared per release by PRF-002 in
[Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md).

The editor detects files added, removed, renamed, moved, or changed outside the
editor. An external change never silently overwrites a conflicting unsaved
change. Conflicts identify both versions and the affected asset or setting.
Source-control and merge workflows are owned by EDT-007 in
[Editor and authoring](editor-and-authoring.md).

### Unity versions and upgrades

The project's declared Unity version is read from
`ProjectSettings/ProjectVersion.txt`. A project declaring a release from Unity
2020.3 LTS through Unity 2022.3 LTS opens with that release as its baseline:
version-specific APIs, scripting defines (RUN-007 in
[Scripting runtime](scripting-runtime.md)), settings, and behavior follow the
declared release.

* Older serialized forms within the baseline range are read and migrated as
  defined by SER-005; unchanged assets are not rewritten by opening.
* Opening never rewrites `ProjectVersion.txt`. Changing the project's declared
  Unity version is an explicit action that lists the assets, settings,
  packages, and behaviors the change affects before it is committed.
* A project that declares no version, or a version that cannot be parsed, is
  reported with the missing or invalid value and opens only after the user
  chooses a baseline release.

### Edge cases

* A directory that is not a recognizable project explains which required
  project signals are absent.
* A project created by a Unity version outside the 2020.3 LTS through 2022.3
  LTS range opens for read-only inspection when its content can be classified
  safely; otherwise opening is refused with the version boundary explained. It
  is never silently treated as a fully supported version.
* A partially readable project remains inspectable while unreadable items retain
  specific errors.
* Case-only renames, Unicode names, long paths, read-only files, broken symbolic
  links, and platform-specific filename conflicts have explicit outcomes.
* Two simultaneous editor instances, or an editor and a headless session, do
  not silently overwrite one another.
* A project already open in Unity is opened with a warning that both
  applications can write project files; external writes are handled as
  external changes.

## Project trust

Opening a project does not execute any code the project supplies until the
user trusts that project. Project code includes runtime and editor scripts,
precompiled managed assemblies, native plugins, Roslyn analyzers, source
generators, scripted importers, asset postprocessors, `[InitializeOnLoad]`
and other editor-load hooks, build callbacks, and code contained in packages.

### Required behavior

* An untrusted project opens in restricted mode. Data assets, scenes, prefabs,
  materials, settings, metadata, package manifests, and script source are
  available for read-only inspection, search, and compatibility reporting.
* In restricted mode, no project script is compiled into loaded code, no
  analyzer or source generator runs, no project-supplied importer or
  postprocessor runs, no native plugin is loaded, and play, tests, scenarios,
  and builds are unavailable. Assets whose import requires project code show
  the import state blocked by trust.
* The trust prompt lists what trusting would run: script assemblies, editor
  hooks, analyzers and source generators, importers and postprocessors, native
  plugins, and packages that contain code.
* Package content is fetched from the registries, git repositories, and URLs
  that a project names only after the user consents to fetching for that
  project. Consent can be given with trust or separately. Without consent,
  unfetched packages are reported unresolved.
* Trust is recorded per user and per project location, outside the project's
  files, so cloning or copying a repository never carries trust. The user can
  revoke trust; revocation returns the project to restricted mode at the next
  open or on request.
* Trusted project code runs with the privileges of the user account running
  unitas. It is not sandboxed. Trusting a project gives its code the same
  access to files, network, and processes that the user has.
* An agent can request trust; the request reaches the user with the project
  location and the list of code that would run. An agent cannot grant trust,
  and an automation request that needs trust fails with a trust-required
  result naming the operation. Headless and command-line runs accept trust
  only from user-controlled invocation configuration (BLD-002 in
  [Builds, platforms, XR, and networking](build-platforms-xr-networking.md)),
  never from an MCP request.

## Asset browser

### User stories

* As a developer, I can browse by folder and search by name, type, label,
  extension, import status, and relationship.
* As a developer, I can create favorites and revisit recent assets.
* As a developer, I can create, rename, move, duplicate, and delete assets while
  preserving valid references.
* As a developer preparing a destructive edit, I can see direct and indirect
  dependents first.

### Required behavior

Every asset exposes a stable identity, display name, type, source path, import
state, labels, dependencies, reverse dependencies, and a preview for asset
types that have visual or audible content. Search results distinguish files,
folders, main assets, and sub-assets. Combined filters return the assets that
satisfy every filter, and an empty result names the filters that excluded
content.

Moving or renaming an asset does not change its identity. Duplication produces a
distinct identity while preserving supported internal structure. Deletion
leaves dependent references visibly missing until repaired; it does not redirect
them to similarly named content. Restoring the same identified content restores
eligible references. The identity rules behind these outcomes are owned by
PRJ-003 in [Serialization and prefabs](serialization-and-prefabs.md).

Multi-selection clearly distinguishes shared and mixed properties. Copy, move,
duplicate, and delete operations state their complete scope, including metadata
and sub-assets. Undo restores both content placement and valid references.

## Dependencies and reverse dependencies

A user or agent can list an asset's dependencies and reverse dependencies,
both direct and transitive, before and after an edit.

### Required behavior

* Dependency cycles are visible and terminate safely.
* Dependency views distinguish direct, transitive, optional, editor-only, and
  target-limited relationships where the project data records that
  distinction.
* Dependencies through scripts, packages, `Resources` folders, AssetBundle
  assignments, and Addressables entries are shown with their kind.
* A reference diagnostic identifies the referring asset, object, property, and
  expected target kind.

## Import state and feedback

Every source asset has one of these observable states: not yet evaluated,
processing, ready, ready with limitations, failed, missing, blocked by a
dependency, or blocked by trust. A user can inspect when its result last
changed and why.

Reimport updates the selected asset and the dependents whose observable result
can change. Unrelated assets retain their current state. A cancelled import does
not replace a previously valid result with an incomplete result. Retrying a
failed import preserves the prior diagnostic history for the current session.

Diagnostics state:

* The asset and sub-object involved.
* The unsupported, invalid, or ambiguous value.
* The observable consequence.
* Whether the asset remains partially usable.
* The target or quality context when the issue is conditional.

Supported import settings, including per-target overrides, survive open, edit,
save, reimport, and reopening without unexplained changes. Settings recognized
but not honored are marked unsupported. Read-only content remains inspectable,
and an attempted edit reports that it was not committed rather than appearing
saved.

Project-supplied scripted importers and asset postprocessors run only in
trusted projects; their behavior is owned by RUN-010 in
[Scripting runtime](scripting-runtime.md).

## Texture and image assets

Supported source families include PNG, JPEG, TGA, PSD, DDS, and KTX2. A file
that uses an encoding or feature of its format that unitas does not support is
reported Partially supported or Unsupported with that feature named. An image asset exposes dimensions, color-space
interpretation, alpha behavior, wrap mode, filtering, mip behavior, normal-map
interpretation, sprite settings, readability, and target-specific quality.

The preview distinguishes source appearance from the in-game result. Changes to
delivery quality do not modify the source image. Invalid dimensions, missing
layers, unsupported pixel encodings, ambiguous alpha, and target limitations
produce specific diagnostics. Normal maps and data textures are not silently
treated as ordinary color images.

## Model and mesh assets

Supported source families include FBX, glTF/GLB, and OBJ, with the same
per-file reporting of unsupported format features. A model exposes only capabilities its source format and file contain.
FBX and glTF/GLB may contain hierarchy, materials, skeletons, skinning,
animation, and blend shapes. OBJ provides static geometry and
associated material data where present; it is never reported as containing
native skinning or animation. All models expose applicable scale, axis
interpretation, normals, tangents, and collision content.

The user can preview geometry, materials, rigging, blend shapes, and clips;
select relevant clips; and assign or remap materials. Reimport preserves stable
references to unchanged named or identified sub-assets as defined by PRJ-003.
Removed nodes or clips leave explicit missing references.

Invalid topology, missing bones, invalid weights, degenerate bounds, ambiguous
units, absent normals, and unsupported animation data are reported separately.
The asset never appears fully ready merely because some geometry was readable.

## Audio assets

Supported source families include WAV, MP3, and OGG, with the same per-file
reporting of unsupported format features. An audio asset exposes duration, channel layout, sample rate, load behavior,
looping suitability, preview controls, and target-specific quality.

The user can tell whether audio is immediately available, progressively
available, or unavailable during play. Unsupported channel layouts, corrupt
frames, truncated data, inaudible clips, and target codec limits are explicit.
Preview failure is distinct from runtime incompatibility.

## Other recognized asset families

The browser identifies scenes, prefabs, materials, shaders, animation clips,
animator controllers, fonts, video, scripts, ScriptableObjects, physics
materials, input configuration, localization content, terrain data, render
textures, and supported package content. Each family links to its owning feature
specification and uses the common identity, dependency, status, and diagnostic
contracts defined here and in
[Serialization and prefabs](serialization-and-prefabs.md).

## Packages and project extensions

### Manifest and lock file

`Packages/manifest.json` declares the project's packages. Each dependency is
resolved from its declared source:

* A version from the Unity registry or a scoped registry. Scoped registries
  route the package names that match their scopes to their registry URL.
* A git URL, with an optional revision and package path.
* A local package by `file:` path outside the project.
* An embedded package stored under `Packages/`.
* A tarball by `file:` path.
* A built-in module (`com.unity.modules.*`). Disabling a module makes its
  APIs unavailable to scripts, and content that needs the module reports the
  missing module.

`Packages/packages-lock.json`, when present and consistent with the manifest,
records the resolved version, source, and dependencies of every package, and
resolution reproduces those versions; git packages resolve to their locked
revision. `testables`, `resolutionStrategy`, and `enableLockFile` behave as
Unity declares.

### Resolution

Resolution produces one version of each package. When packages request
different versions of a dependency, the result follows Unity's resolution
rules: a direct dependency in the manifest takes precedence, and indirect
requests resolve according to the resolution strategy. A conflict that cannot
be resolved identifies the package, the requested versions, and the chain of
packages requesting each version.

* An unreachable registry, git host, or path, a missing version, or an
  invalid manifest is reported with the package and source. Packages already
  resolved and available locally remain usable, and content depending on an
  unresolved package is marked with that cause.
* Offline, resolution uses the lock file and locally available package
  content. Packages that cannot be obtained offline are reported unresolved;
  the project still opens.
* Fetching from project-named sources requires consent, as defined in
  [Project trust](#project-trust).

### Package content and compatibility

Content of registry, git, tarball, and built-in packages is read-only. An
attempted edit reports that it was not committed and names the package; to
change it, the user embeds the package. Embedded and local packages are
editable like project content.

Each resolved package carries the compatibility state from the supported
package matrix (CMP-002 in [Compatibility contract](compatibility-contract.md));
an unlisted package is Unknown. Third-party and asset-store packages and
plugins are assessed as defined by CMP-003 in the same document.

Adding, removing, upgrading, or downgrading a package updates
`manifest.json` and `packages-lock.json` in Unity-readable form, reports the
resulting dependency changes before they are committed, and is available
through automation.

## Project settings

The settings stored in `ProjectSettings/` are represented, edited, and saved in
Unity-readable form. Each family below links its effect to the owning feature:

* Tags and Layers: tags, layers, and sorting layers (EDT-001; REN-005 in
  [Rendering and animation](rendering-and-animation.md)).
* Time: fixed timestep, maximum allowed timestep, time scale, and maximum
  particle timestep (RUN-004 in [Scripting runtime](scripting-runtime.md)).
* Physics and Physics 2D: gravity, layer collision matrices, contact offset,
  solver iterations, bounce threshold, default material, auto sync
  transforms, and query options (PHY-001, PHY-002 in
  [Physics, navigation, and terrain](physics-navigation-terrain.md)).
* Quality: quality levels, per-target default levels, and their rendering
  settings (REN-003 in [Rendering and animation](rendering-and-animation.md)).
* Graphics: render pipeline asset, always-included shaders, shader stripping,
  and graphics tiers (REN-006 in
  [Rendering and animation](rendering-and-animation.md)).
* Player: company and product names, version, icons, resolution and
  presentation, scripting backend, API compatibility level, scripting define
  symbols, and active input handling (BLD-001 and BLD-003 in
  [Builds, platforms, XR, and networking](build-platforms-xr-networking.md);
  RUN-007; INP-001 and INP-002 in [UI, audio, and input](ui-audio-input.md)).
* Input Manager axes and buttons (INP-001).
* Audio: speaker mode, DSP buffer size, sample rate, and default volume
  (AUD-001 in [UI, audio, and input](ui-audio-input.md)).
* Navigation areas and agent types (NAV-001).
* Editor settings: asset serialization mode (SER-005), version control mode,
  Enter Play Mode Options (RUN-003), and line endings for new scripts.
* Script Execution Order (RUN-003), stored where Unity stores it.
* Preset Manager default presets, applied to newly created components,
  assets, and import settings as Unity applies them.
* Package Manager settings: scoped registries and pre-release package
  visibility.
* Version control settings.
* Build scene list (BLD-001; RUN-005).

### Required behavior

* Per-target overrides (Player settings per build target group, per-target
  quality defaults, per-target scripting defines, and per-target import
  overrides) are visible beside the default value, and the effective value for
  a selected target is shown with its source.
* A change takes effect at the boundary the owning feature declares, such as
  the next recompilation, the next play session, or the next build, and the
  user can see when a change is pending.
* Settings changed outside the editor are detected as external changes.
* Unknown settings keys and settings for unrecognized packages are preserved
  across saves.
* Settings that are recognized but not honored are marked unsupported with
  their consequence. Settings for unsupported families listed in the
  [Index](INDEX.md), such as HDRP or Visual Effect Graph settings, are
  preserved and reported Unsupported.

## Acceptance outcomes

1. Opening the same unchanged project twice, with or without Unity's `Library/`
   folder present, yields identical asset identities and reference targets.
   (PRJ-001)
2. A project declaring Unity 2020.3 LTS opens with 2020.3 behavior, its
   unchanged assets are not rewritten, and `ProjectVersion.txt` is unchanged
   after opening and saving. (PRJ-001)
3. Moving a referenced asset preserves the reference; deleting it exposes a
   missing reference; restoring it can restore the reference. (PRJ-002)
4. Search with combined filters returns only assets matching every filter and
   distinguishes main assets from sub-assets. (PRJ-002)
5. Every asset that is not fully usable has a visible state and consequence.
   (PRJ-004)
6. Reimport never changes unrelated assets without a dependency reason, and
   source previews and runtime interpretations are distinguishable. (PRJ-004)
7. Before deleting an asset, a user or agent sees its direct and transitive
   reverse dependencies, and dependency cycles are displayed without failing.
   (PRJ-005)
8. A project with a scoped registry, a git package, an embedded package, and a
   lock file resolves to the locked versions; offline, the same project opens
   with unavailable packages reported unresolved; editing registry package
   content is reported as not committed. (PRJ-006)
9. A package version conflict reports the requesting chains for each version.
   (PRJ-006)
10. Changing the layer collision matrix, fixed timestep, or a per-target
    scripting define saves Unity-readable settings, shows the effective value
    for the selected target, and changes behavior at the declared boundary.
    (PRJ-007)
11. Opening an untrusted project with an `[InitializeOnLoad]` editor script, a
    source generator, and a git package runs none of them and fetches nothing,
    while scenes and assets remain inspectable; an agent's trust request
    reaches the user and the agent cannot grant it. (PRJ-008)
12. A compatibility report can account for every recognized project asset,
    package, and setting. (PRJ-001, PRJ-006, PRJ-007)
