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

The behavioral baseline is the documented, observable behavior of
Unity 2022.3 LTS, as defined in the
[Compatibility contract](compatibility-contract.md). Package behavior is
covered only for the package families and version lines listed in the
supported package matrix (CMP-002 in the same document).

### Compatibility tiers

PRJ-001, PRJ-005, and PRJ-007 are Exact: project recognition, dependency
relationships, and project settings match Unity 2022.3 LTS exactly, and any
divergence is a defect. PRJ-004 and PRJ-006 are Semantic: import and package
resolution behave as Unity 2022.3 LTS does in ordinary use, and a found
difference is reported Partially supported with the difference named. PRJ-002
and PRJ-008 are Native: the asset browser and project trust are defined by
this document. Tiers are defined in the
[Compatibility contract](compatibility-contract.md).

Every capability in this document, including opening, searching, importing,
package changes, and settings changes, is available to authorized agents
through automation as defined in [Automation and MCP](automation-and-mcp.md).
Granting project trust and fetch consent are person-only actions (AUT-007):
agents can request them but cannot grant them.

## Architecture

* **Project model.** The editor process owns the project model: a file
  watcher over `Assets/`, `Packages/`, and `ProjectSettings/`, the asset
  registry mapping each path to its GUID and to the local identifiers of its
  objects, the resolved package set, and the settings store. The model exists
  before trust and contains no project code. Workers and players never scan
  source folders; they ask the model or read cooked data.
* **Identity as the key.** GUID and local identifier are the one key for an
  object in the registry, in the dependency graph, in every cooked artifact,
  in content built for players, and in diagnostics and automation results.
  Compact runtime handles are derived from that key at cook time and map back
  to it; paths are attributes of an identity, never a key.
* **Dependency graph.** One graph records static references read from
  serialized data, dependencies importers declare (including on other
  artifacts, settings, and the target), script and package edges, and
  `Resources`, AssetBundle, and Addressables membership. Change propagation,
  reverse-dependency queries, build inclusion, and cook ordering all read
  this graph; nothing re-walks the project to answer them.
* **Import key and cache.** Every import result is addressed by a content
  hash of its inputs: source bytes, `.meta` importer settings, importer
  identity and version, postprocessor versions, the target and the settings
  the importer declares it reads, and the hashes of its declared
  dependencies. Artifacts are immutable and deterministic in that key and
  live in a per-user cache outside the project tree. A target switch, a
  reopen, a restart after a crash, or a second checkout of the same content
  is a cache hit wherever the key is unchanged.
* **Scheduling.** A changed file or setting invalidates only the keys that
  depend on it. The scheduler imports invalidated assets in dependency order
  and in parallel on import workers, raises the priority of assets a person
  or agent is waiting on, and publishes each result atomically when it
  completes, so a cancelled or failed import never replaces a valid result.
  Opening and importing never block the editor's main thread.
* **Importers.** Built-in importers use mature decoders and run in import
  workers, so a decoder failure fails one asset and the import continues.
  Project-supplied importers and postprocessors run in the worker that hosts
  project code and only in a trusted project. Shader import hands shaders to
  the rendering subsystem's compiler toolchain at this stage.
* **Packages.** Resolution runs in the project model, fetches only with
  consent, and stores fetched package content in a per-user content-addressed
  store; resolved packages appear as read-only roots of the asset registry.
* **Sameness.** The visual editor and headless sessions use the same model,
  cache, and scheduler. Play sessions and builds consume only cooked
  artifacts from that cache, and a build packs the target's artifacts without
  importing differently, so content behaves the same in editor play and in
  the player.

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
identities and dependency relationships. The project scale that opening
supports and its timing budgets are owned by PRF-002 in
[Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md).

The editor detects files added, removed, renamed, moved, or changed outside the
editor. An external change never silently overwrites a conflicting unsaved
change. Conflicts identify both versions and the affected asset or setting.
Source-control and merge workflows are owned by EDT-007 in
[Editor and authoring](editor-and-authoring.md).

### Unity versions and upgrades

The project's declared Unity version is read from
`ProjectSettings/ProjectVersion.txt`. Every supported project runs with Unity
2022.3 LTS behavior: APIs, scripting defines (RUN-007 in
[Scripting runtime](scripting-runtime.md)), settings, and behavior are those
of Unity 2022.3 LTS.

* A project declaring Unity 2022.3 LTS opens directly.
* A project declaring any earlier Unity release that Unity 2022.3 LTS can
  upgrade, including Unity 2020.3 LTS, Unity 2021.3 LTS, earlier LTS
  releases, and non-LTS releases, opens by migrating its serialized forms to
  the 2022.3 baseline, as defined by SER-005 in
  [Serialization and prefabs](serialization-and-prefabs.md). The project is
  reported as migrated, with its declared release. Behavior that differs
  between the declared release and Unity 2022.3 LTS is not matched.
* A project declaring a Unity release later than 2022.3 LTS opens for full
  editing with a warning that names the declared release and states that the
  project runs with Unity 2022.3 LTS behavior. Content in a serialized form
  that only the later release writes is preserved and reported per asset, as
  defined by SER-005, and the rest of the project is editable, playable, and
  buildable.
* Opening never rewrites `ProjectVersion.txt` and never rewrites unchanged
  assets. Recording Unity 2022.3 LTS as the project's version is an explicit
  upgrade action that lists the assets, settings, and packages it affects
  before it is committed.
* A project that declares no version, or a version that cannot be parsed, is
  reported with the missing or invalid value and opens only after the user
  confirms that it is to be treated as a Unity 2022.3 LTS project.

### Edge cases

* A directory that is not a recognizable project explains which required
  project signals are absent.
* A project declaring a release so old that Unity 2022.3 LTS has no upgrade
  path for its serialized forms opens for read-only inspection, and the report
  names the declared release and each asset whose form no Unity 2022.3 LTS
  upgrade path reads. Its readable content remains inspectable.
* A partially readable project remains inspectable while unreadable items retain
  specific errors.
* A case-only rename is a rename that keeps the asset's GUID.
* A long or Unicode path opens, or is reported naming the path and the host's
  limit.
* A broken symbolic link is reported missing, naming its target.
* A read-only file is inspectable, and an edit to it reports that it was not
  committed.
* A filename conflict on the host, such as two names that differ only in case
  on a case-insensitive file system, is reported naming both files.
* Several unitas sessions on one project, including editors and headless
  sessions, follow the concurrency rules owned by AUT-004 in
  [Automation and MCP](automation-and-mcp.md).
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
  project. Consent can be given with trust or separately. On the command line,
  the explicit trust argument also gives fetch consent for that invocation,
  and a separate fetch-consent argument gives consent alone. Without consent,
  unfetched packages are reported unresolved.
* Trust and fetch consent are person-only actions (AUT-007 in
  [Automation and MCP](automation-and-mcp.md)) and are never granted through
  MCP.
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
  result naming the operation.
* Headless and command-line runs (BLD-002 in
  [Builds, platforms, XR, and networking](build-platforms-xr-networking.md))
  treat a project as trusted only through the per-user trust record that the
  user created, or through an explicit trust argument on a command-line
  invocation that the user or the user's continuous-integration configuration
  starts. An MCP request can never create either.
* The consequences of granting agent sessions permission to edit scripts and
  to execute project code in a trusted project are owned by AUT-007 in
  [Automation and MCP](automation-and-mcp.md).

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
processing, ready, ready with limitations, failed, missing, externally
unavailable (a required external application is absent), blocked by a
dependency, or blocked by trust. A user can inspect when its result last
changed and why.

Reimport updates the selected asset and the dependents whose observable result
can change. Unrelated assets retain their current state. A cancelled import does
not replace a previously valid result with an incomplete result. Retrying a
failed import preserves the prior diagnostic history for the current session.
Changing the active build target imports again only the assets whose result
depends on the target, and returning to a target reuses its earlier results.

A long import, such as a first open or a reimport of many assets, continues
past individual asset failures, reports each failure, and ends as a partial
success when any asset failed. After an interruption or crash, the next import
reuses completed results whose recorded inputs (source content, metadata,
import settings, importer version, and dependencies) are unchanged, and states
which assets were imported again and why.

Diagnostics state:

* The asset and sub-object involved.
* The invalid, corrupt, or ambiguous value.
* The observable consequence.
* Whether the asset remains partially usable.
* The target or quality context when the issue is conditional.

Every import setting that the Unity 2022.3 LTS importers define is honored,
and import settings, including per-target overrides, survive open, edit, save,
reimport, and reopening without unexplained changes. An import setting key that
no Unity 2022.3 LTS importer defines, such as one written by a later Unity
release, is preserved and reported Unknown with the key named. Read-only
content remains inspectable, and an attempted edit reports that it was not
committed rather than appearing saved.

Project-supplied scripted importers and asset postprocessors run only in
trusted projects; their behavior is owned by RUN-010 in
[Scripting runtime](scripting-runtime.md).

## Texture and image assets

PNG, JPEG, TGA, PSD, DDS, KTX2, EXR, HDR, BMP, TIFF, GIF, IFF, and PICT
sources are Supported. A GIF imports its first frame, as Unity imports it, and
the asset shows the frame count of the source. Every encoding and feature of
these formats that Unity 2022.3 LTS reads is read, including 8-bit, 16-bit, and
floating-point channels, indexed and grayscale color, embedded color profiles,
interlaced and progressive encodings, compressed and uncompressed variants,
and multilayer PSD and TIFF files, which import as Unity composites them. A
file whose data is truncated, corrupt, or contradicts its own header is
reported as failed with the file and the invalid feature named.

An image asset exposes dimensions, color-space interpretation, alpha behavior,
wrap mode, filtering, mip behavior, normal-map interpretation, sprite settings,
readability, and target-specific quality. The texture importer honors:

* Texture types: Default, Normal map, Editor GUI and Legacy GUI, Sprite (2D and
  UI), Cursor, Cookie, Lightmap, Directional Lightmap, Shadowmask, and Single
  Channel.
* Texture shapes: 2D, Cube, 2D Array, and 3D, with their layout settings.
* Maximum size, resize algorithm, non-power-of-two handling, sRGB, alpha
  source, alpha is transparency, mipmap generation and filtering, mipmap
  streaming, and Read/Write.
* Per-target format, compression quality, and Crunch compression overrides.
* Sprite Mode Single, Multiple, and Polygon. Multiple-mode slices keep their
  names, rectangles, pivots, borders, and physics shapes, and produce the same
  sprite sub-assets with the same identities as Unity (PRJ-003).

The preview distinguishes source appearance from the in-game result. Changes to
delivery quality do not modify the source image. Invalid dimensions, missing
layers, corrupt pixel data, ambiguous alpha, and target hardware limits, such
as a texture larger than the target GPU's maximum size, produce specific
diagnostics. Normal maps and data textures are not silently
treated as ordinary color images.

## Model and mesh assets

FBX, glTF/GLB, OBJ, COLLADA (`.dae`), DXF, and 3DS sources are Supported and
import directly. Content-creation application formats (`.blend`, `.max`,
`.ma`, `.mb`, `.c4d`, `.lxo`, and `.jas`) are Supported as Unity supports
them: the importer converts the file through the content-creation application
installed on the machine and imports the result with the same sub-assets and
identities Unity produces. Conversion runs only in a trusted project
(PRJ-008), because it runs an external program. The user selects the
application installation, or unitas locates it in its standard locations.
When the application is absent, the asset reports Externally unavailable,
naming the file, the application, and that installing it resolves the state;
an import result produced earlier for unchanged inputs remains usable. A
conversion that the application rejects reports the file and the
application's own error.

A model exposes only capabilities its source format and file contain. FBX and
glTF/GLB may contain hierarchy, materials, skeletons, skinning, animation, and
blend shapes. OBJ provides static geometry and associated material data where
present; it is never reported as containing native skinning or animation. All
models expose applicable scale, axis interpretation, normals, tangents, and
collision content.

The model importer honors:

* Mesh settings: scale factor, convert units, Read/Write, index format, mesh
  compression, mesh optimization, blend shape import and normals, lightmap UV
  generation with its parameters, normals and tangents import or calculation
  modes with smoothing angle, and collider generation.
* Rig settings: animation type None, Legacy, Generic, or Humanoid; avatar
  creation from the model or copying from another avatar; root bone; and
  Optimize Game Objects with its exposed transforms.
* Animation settings: clip splitting by frame range, loop time and loop pose,
  cycle offset, root transform rotation, position Y, and position XZ bake
  settings and offsets, mirror, additive reference pose, per-clip curves,
  events, and masks, and animation compression Off, Keyframe Reduction, or
  Optimal with rotation, position, and scale error tolerances.
* Material settings: material creation mode, location (embedded or external),
  extraction of materials and textures, and remapping to existing materials.

The user can preview geometry, materials, rigging, blend shapes, and clips;
select relevant clips; and assign or remap materials. Reimport preserves stable
references to unchanged named or identified sub-assets as defined by PRJ-003.
Removed nodes or clips leave explicit missing references. Avatar and animation
behavior at runtime is owned by ANI-001 and ANI-002 in
[Rendering and animation](rendering-and-animation.md).

Invalid topology, missing bones, invalid weights, degenerate bounds, ambiguous
units, absent normals, and malformed animation data are reported separately.
The asset never appears fully ready merely because some geometry was readable.

## Audio assets

WAV, AIFF, MP3, and OGG sources are Supported. Tracker modules (`.mod`, `.it`,
`.s3m`, `.xm`) are Supported: each imports as an `AudioClip` that plays the
module's patterns, instruments, and samples with its tempo, looping, and
channel effects, and that honors the import settings Unity offers for tracker
modules. A module that uses an effect command its format does not define is
reported with the file, pattern, and command named. An audio asset exposes
duration, channel layout, sample rate, load behavior, looping suitability,
preview controls, and target-specific quality.

The audio importer honors Force To Mono, Normalize, Ambisonic, Load In
Background, Preload Audio Data, load type, and per-target compression format
(PCM, ADPCM, Vorbis, MP3), quality, and sample rate setting (preserve,
optimize, or override). Load-type behavior at runtime is owned by AUD-001 in
[UI, audio, and input](ui-audio-input.md).

The user can tell whether audio is immediately available, progressively
available, or unavailable during play. Every channel layout from mono to 7.1
imports. Corrupt frames, truncated data, and inaudible clips are explicit. A
compression format that the selected target cannot decode in hardware is
decoded in software on that target, and the import result states that the
target decodes the format in software.
Preview failure is distinct from runtime incompatibility.

## Text and other recognized asset families

Files with the extensions `.txt`, `.json`, `.bytes`, `.csv`, `.xml`, `.html`,
`.htm`, `.yaml`, and `.fnt` import as `TextAsset` with `text` and `bytes`
exactly equal to the file content.

The browser identifies scenes, prefabs, materials, shaders, compute shaders,
shader include files (`.hlsl`, `.cginc`), shader variant collections,
animation clips, animator controllers, animator override controllers, avatar
masks, audio mixers, fonts and custom fonts, GUI skins, video, scripts,
ScriptableObjects, physics materials and 2D physics materials, input
configuration and input action assets, localization content, terrain data and
terrain layers, lighting settings, cubemaps, flares, render textures, Sprite
Atlas (v1 and v2) assets, presets, Timeline and signal assets, and supported
package content. Each family links to its owning feature specification and
uses the common identity, dependency, status, and diagnostic contracts defined
here and in [Serialization and prefabs](serialization-and-prefabs.md).

### Unity package files

Importing a `.unitypackage` shows its contents, their destination paths, and
every GUID that collides with an existing asset before anything is written; the
user chooses which items to import. A `.unitypackage` that contains scripts,
precompiled assemblies, native plugins, or editor code adds that code under
the project trust rules of [Project trust](#project-trust).

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
in Unity.

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
* Player color space, Gamma or Linear (REN-002 in
  [Rendering and animation](rendering-and-animation.md)).
* Input Manager axes and buttons (INP-001).
* Audio: speaker mode, DSP buffer size, sample rate, and default volume
  (AUD-001 in [UI, audio, and input](ui-audio-input.md)).
* Navigation areas and agent types (NAV-001).
* Editor settings: asset serialization mode (SER-005),
  Enter Play Mode Options (RUN-003), and line endings for new scripts.
* Preset Manager default presets, applied to newly created components,
  assets, and import settings as Unity applies them.
* Package Manager settings: scoped registries and pre-release package
  visibility.
* Version control settings.
* Build scene list (BLD-001; RUN-005).

Script Execution Order (RUN-003) is stored in each script's `.meta` file, as
Unity stores it, and is edited with the project settings.

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
* Every setting in each family above is honored by its owning feature.
  Settings of a package, such as HDRP or Visual Effect Graph settings, are
  honored by the package's owning feature, take the state its CMP-002 matrix
  entry records, and are preserved across saves in every state.

## Acceptance outcomes

1. Opening the same unchanged project twice, with or without Unity's `Library/`
   folder present, yields identical asset identities and reference targets.
   (PRJ-001)
2. A project declaring Unity 2021.3 LTS opens with Unity 2022.3 LTS behavior
   and is reported as migrated; its unchanged assets and `ProjectVersion.txt`
   are not rewritten by opening, and the upgrade action lists the affected
   assets, settings, and packages before writing. (PRJ-001)
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
    for the selected target, and changes behavior at the boundary its owning
    feature names. (PRJ-007)
11. Opening an untrusted project with an `[InitializeOnLoad]` editor script, a
    source generator, and a git package runs none of them and fetches nothing,
    while scenes and assets remain inspectable; an agent's trust request
    reaches the user and the agent cannot grant it. (PRJ-008)
12. A compatibility report can account for every recognized project asset,
    package, and setting. (PRJ-001, PRJ-006, PRJ-007)
13. A continuous-integration run started with an explicit trust argument
    compiles and runs project code and fetches its git package; a run with
    only the fetch-consent argument fetches the package but runs no project
    code; the same run without either argument or a per-user trust record
    stays restricted and fetches nothing; and an MCP request to trust the
    project is refused. (PRJ-008)
14. A texture imported as Sprite in Multiple mode yields the same named sprite
    sub-assets and identities as in Unity; per-target compression overrides
    survive reimport; an EXR imports, a GIF imports its first frame and shows
    its frame count, and a PICT file and an IFF file import. (PRJ-004)
15. An FBX imported with a Humanoid rig, split clips with loop pose and baked
    root rotation, Optimal compression, and extracted materials produces the
    avatar, clips, and materials Unity produces; in a trusted project with
    Blender installed, a `.blend` file imports with the sub-assets Unity
    produces, and on a machine without Blender it reports Externally
    unavailable, naming the application. (PRJ-004, PRJ-008)
16. An AIFF clip imports with per-target Vorbis compression and Force To Mono
    honored, and an `.xm` module imports and plays its patterns. (PRJ-004)
17. A `.json` and a `.bytes` file import as `TextAsset` with content equal to
    the file, and an avatar mask, an audio mixer, and a Sprite Atlas v2 asset
    are each identified with their family. (PRJ-002, PRJ-004)
18. Importing a `.unitypackage` lists its contents and GUID collisions before
    writing, and its scripts do not run in a project that is not trusted.
    (PRJ-004, PRJ-008)
19. A first-open import with three corrupt textures completes the rest, reports
    the three failures and a partial success, and after a forced interruption
    the next open reuses unchanged results and lists what it imported again.
    (PRJ-004)
20. A project declaring Unity 2019.4 LTS opens as migrated with Unity 2022.3
    LTS behavior, and a project declaring a release later than 2022.3 LTS opens
    for editing with a warning that names the release, while an asset in a
    form only that release writes is preserved and reported. (PRJ-001)
21. A COLLADA model, a DXF model, and a 3DS model each import with their
    hierarchy and geometry, and a 7.1 WAV imports with its channel layout.
    (PRJ-004)
22. An import setting key that no Unity 2022.3 LTS importer defines is
    preserved across reimport and save and reported Unknown with the key
    named. (PRJ-004, PRJ-007)
