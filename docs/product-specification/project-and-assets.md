# Project and Asset Management

## Scope

This specification defines what users experience when opening a Unity project,
browsing its contents, importing source files, resolving dependencies, and
maintaining asset identity. It covers project recognition, the asset browser,
metadata, diagnostics, reimport, and common source-asset families.

These are desired end-state requirements, not claims about currently shipped
capability. Unity 2022.3 LTS behavior is the baseline where a feature is part of
stock Unity. Package behavior is covered only for explicitly supported package
families and versions.

## Project opening

### User stories

* As a developer, I can open the root directory of a supported Unity project
  without creating a converted copy.
* As a developer returning to a project, I see the same asset identities,
  references, settings, and openable content I saw previously.
* As a developer moving between Unity and unitas, I can see external changes and
  choose how to reconcile them with unsaved editor work.
* As a developer evaluating compatibility, I can identify every item that is
  supported, partially supported, unsupported, missing, ambiguous, or invalid.

### Required behavior

The project opens as a coherent collection of assets, packages, scenes, scripts,
project settings, and metadata. Recognition does not require moving source files
or replacing the Unity directory structure. Project identity is independent of
the machine-specific absolute path used to open it.

Opening communicates progress and remains inspectable during long operations.
The project can be closed or an operation cancelled without presenting a partial
result as complete. Reopening unchanged content produces the same asset
identities and dependency relationships.

The editor detects files added, removed, renamed, moved, or changed outside the
editor. An external change never silently overwrites a conflicting unsaved
change. Conflicts identify both versions and the affected asset or setting.

### Edge cases

* A directory that is not a recognizable project explains which required
  project signals are absent.
* A project created by an unsupported Unity version remains unopened or opens
  with an explicit compatibility boundary; it is never silently treated as a
  fully supported version.
* A partially readable project remains inspectable while unreadable items retain
  specific errors.
* Case-only renames, Unicode names, long paths, read-only files, broken symbolic
  links, and platform-specific filename conflicts have explicit outcomes.
* Two simultaneous editor instances do not silently overwrite one another.

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
state, labels, dependencies, reverse dependencies, and preview when meaningful.
Search results distinguish files, folders, main assets, and sub-assets. Filters
combine predictably and explain when no results match.

Moving or renaming an asset does not change its identity. Duplication produces a
distinct identity while preserving supported internal structure. Deletion
leaves dependent references visibly missing until repaired; it does not redirect
them to similarly named content. Restoring the same identified content restores
eligible references.

Multi-selection clearly distinguishes shared and mixed properties. Copy, move,
duplicate, and delete operations state their complete scope, including metadata
and sub-assets. Undo restores both content placement and valid references.

## Identity, metadata, and dependencies

Unity metadata GUIDs identify source assets, while local identifiers distinguish
objects and sub-assets within them. A reference resolves only when both the asset
and intended local object are unambiguous.

### Required behavior

* A missing metadata file is reported and does not cause arbitrary reassignment
  of an identity already referenced elsewhere.
* Duplicate GUIDs are reported as conflicts and never resolved by nondeterministic
  file order.
* A missing local identifier remains a missing sub-asset reference even if the
  source asset exists.
* Dependency cycles are visible and terminate safely.
* Dependency views distinguish direct, transitive, optional, editor-only, and
  target-limited relationships where that distinction is known.
* A reference diagnostic identifies the referring asset, object, property, and
  expected target kind.

## Import state and feedback

Every source asset has one of these observable states: not yet evaluated,
processing, ready, ready with limitations, failed, missing, or blocked by a
dependency. A user can inspect when its result last changed and why.

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

Supported import settings survive open, edit, save, reimport, and reopening
without unexplained changes. Settings recognized but not honored are marked
unsupported. Read-only content remains inspectable, and an attempted edit
reports that it was not committed rather than appearing saved.

## Texture and image assets

Supported source families include PNG, JPEG, TGA, PSD, DDS, and KTX2 within the
declared compatibility scope. An image asset exposes dimensions, color-space
interpretation, alpha behavior, wrap mode, filtering, mip behavior, normal-map
interpretation, sprite settings, readability, and target-specific quality.

The preview distinguishes source appearance from the in-game result. Changes to
delivery quality do not modify the source image. Invalid dimensions, missing
layers, unsupported pixel encodings, ambiguous alpha, and target limitations
produce specific diagnostics. Normal maps and data textures are not silently
treated as ordinary color images.

## Model and mesh assets

Supported source families include FBX, glTF/GLB, and OBJ within the declared
scope. A model exposes only capabilities its source format and file contain.
FBX and glTF/GLB may contain hierarchy, materials, skeletons, skinning,
animation, and blend shapes within scope. OBJ provides static geometry and
associated material data where present; it is never reported as containing
native skinning or animation. All models expose applicable scale, axis
interpretation, normals, tangents, and collision content.

The user can preview geometry, materials, rigging, blend shapes, and clips;
select relevant clips; and assign or remap materials. Reimport preserves stable
references to unchanged named or identified sub-assets where compatibility
allows. Removed nodes or clips leave explicit missing references.

Invalid topology, missing bones, invalid weights, degenerate bounds, ambiguous
units, absent normals, and unsupported animation data are reported separately.
The asset never appears fully ready merely because some geometry was readable.

## Audio assets

Supported source families include WAV, MP3, and OGG within the declared scope.
An audio asset exposes duration, channel layout, sample rate, load behavior,
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
contracts defined here.

## Acceptance outcomes

1. Opening the same unchanged project twice yields identical asset identities
   and reference targets.
2. Moving a referenced asset preserves the reference; deleting it exposes a
   missing reference; restoring it can restore the reference.
3. Duplicate GUIDs and duplicate local identities cannot produce arbitrary
   resolution.
4. Every asset that is not fully usable has a visible state and consequence.
5. Reimport never changes unrelated assets without a dependency reason.
6. Source previews and runtime interpretations are distinguishable.
7. A compatibility report can account for every recognized project asset.
