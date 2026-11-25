# Serialization, Scenes, and Prefabs

## Scope

This specification defines how project data retains meaning across save, load,
Unity interoperability, duplication, instantiation, prefab inheritance, script
changes, and missing or unknown content. It owns PRJ-003 and SER-001
through SER-005 in the [Feature catalog](feature-catalog.md): asset identity
and references, serialized values, scenes and additive scene sets, prefabs and
variants, missing and unknown content, and serialization forms and generated
caches.

These are desired end-state requirements, not claims about currently shipped
capability. Unity 2022.3 LTS serialization is the baseline. Projects saved by
earlier Unity releases, including Unity 2020.3 LTS and Unity 2021.3 LTS, are
migrated to that baseline, as defined in
[Serialized version migration](#serialized-version-migration).
Package-defined serialization is covered only for the package versions listed
in the supported package matrix (CMP-002 in
[Compatibility contract](compatibility-contract.md)).

Every inspection, edit, save, prefab operation, and identity query in this
document is available to authorized agents through automation as defined in
[Automation and MCP](automation-and-mcp.md). Semantic three-way merging of
scenes, prefabs, and assets is owned by EDT-007 in
[Editor and authoring](editor-and-authoring.md); this document defines the
identities and values that merge preserves.

### Compatibility tiers

PRJ-003, SER-001, SER-002, SER-003, SER-004, and SER-005 are Exact: values,
identities, ordering, references, overrides, and saved forms round-trip
exactly as Unity 2022.3 LTS reads and writes them, and any divergence is a
defect. Tiers are defined in the
[Compatibility contract](compatibility-contract.md).

## Asset identity and references

Unity metadata GUIDs in `.meta` files identify source assets, and local file
identifiers distinguish objects and sub-assets within them. A serialized
reference names a local file identifier, a GUID, and a type. A reference
resolves only when both the asset and the intended local object are
unambiguous.

### Required behavior

* A missing metadata file is reported and does not cause arbitrary reassignment
  of an identity already referenced elsewhere.
* Duplicate GUIDs are reported as conflicts and never resolved by
  nondeterministic file order. Duplicate local identifiers within one asset are
  reported and never resolved arbitrarily.
* A missing local identifier remains a missing sub-asset reference even if the
  source asset exists.
* New assets receive a new GUID and a `.meta` file in Unity-readable form. New
  objects receive local identifiers that do not collide with existing ones in
  the same file. Objects inside prefab instances use the identifiers Unity
  derives for them, so Unity and unitas resolve the same references.
* Moving or renaming an asset together with its `.meta` file preserves its
  GUID and every reference to it. Moving an asset without its `.meta` file is
  reported as a lost identity with the references that depended on it.
* Reimporting a model or other multi-object asset keeps the local identifiers
  of unchanged named or identified sub-assets.
* Unrecognized `.meta` fields and importer settings are preserved across
  saves.
* References to Unity built-in resources resolve to content with the same
  identity and observable role. This covers every object in Unity 2022.3 LTS's
  built-in resource sets (`unity default resources` and
  `unity_builtin_extra`), including default materials, built-in meshes,
  default UI sprites and skins, built-in fonts, built-in shaders, the default
  skybox, and default textures. A reference whose identifier names no object
  in those sets remains a missing reference with its original identity, and
  is reported.

## Serialization contract

Serialization preserves the values and relationships that constitute supported
project content. A successful load followed by a save does not change the
observable meaning of unchanged content.

Supported value families include booleans, characters, signed and unsigned
numbers, floating-point values, strings, enums, flags enums, vectors and
integer vectors, quaternions, matrices, colors and 32-bit colors, rectangles
and integer rectangles, bounds and integer bounds, `Hash128`, layer masks,
animation curves, gradients, `GUIStyle` and `RectOffset` values, arrays,
lists, nested `[Serializable]` values, object references,
`LazyLoadReference<T>`, `ExposedReference<T>`, serialized events (RUN-008 in
[Scripting runtime](scripting-runtime.md)), and managed references declared
with `[SerializeReference]`.

### Required behavior

* Field identity is retained across supported script reloads and edits.
* Array and list ordering is stable.
* Repeated references remain repeated references rather than duplicate objects.
* Internal references within a duplicated hierarchy target the corresponding
  duplicated objects, as in Unity.
* External references remain external.
* Intentional null, missing target, incompatible target, and unknown or
  unreadable value remain distinguishable.
* Floating-point special values (infinity, negative infinity, NaN, negative
  zero) and full single- and double-precision values round-trip unchanged.
* Saving cannot silently replace unreadable or unknown data with defaults.
* Unknown data that can be safely retained survives unrelated edits.

## Script field behavior

Public fields and fields marked `[SerializeField]` of supported types are
serialized and appear in the inspector. Fields marked `[NonSerialized]` are not
serialized, and fields marked `[HideInInspector]` are serialized but not
shown. The attributes are listed by RUN-003 in
[Scripting runtime](scripting-runtime.md); their serialized effect is defined
here. Supported serializable nested types retain their field structure.

When script fields change, compatible values remain associated with their
intended fields. Removed or incompatible fields are reported before data is
discarded. A renamed field retains its data when `[FormerlySerializedAs]` names
the previous field name. A compilation failure does not erase serialized script
data.

Two fields with the same display name but different declaring contexts remain
distinct. Type changes that cannot preserve meaning expose the old value and
incompatibility rather than presenting a successful conversion.

Managed references keep their type identity (assembly, namespace, and class
name) and reference identity. A managed reference whose type no longer exists
is reported as a missing type with its serialized data preserved, and its data
is restored when the type returns.

### Serializable types

The serializer applies Unity's type rules:

* Properties, static fields, `const` fields, and `readonly` fields are not
  serialized.
* Dictionaries, multidimensional arrays, jagged arrays, nested collections
  (such as a list of lists), and delegates are not serialized.
* Fields whose declared type is an interface or abstract class are not
  serialized unless they are marked `[SerializeReference]`.
* Closed generic types marked `[Serializable]` are serialized as fields.
* A field of a custom `[Serializable]` class that is not marked
  `[SerializeReference]` is stored inline: it has no shared identity and no
  polymorphism, and a null value loads as a default-constructed instance.
* Inline nesting deeper than Unity's serialization depth limit of 10 levels
  is truncated at that depth, with Unity's warning naming the type.

A field that these rules exclude is absent from saved data and from the
inspector, exactly as in Unity; unitas does not serialize it as an extension.

### Serialization callbacks

`ISerializationCallbackReceiver.OnBeforeSerialize` and `OnAfterDeserialize`
run on every occasion Unity runs them, including saving, loading,
`Object.Instantiate`, inspector refresh, domain reload, and serialization
during play. An exception thrown by a callback is reported with the type and
object and does not corrupt the stored data. Calls to Unity APIs from these
callbacks follow Unity's restrictions for code that can run on the
serialization thread, and a disallowed call produces Unity's error.

### Hide flags

`HideFlags` values `HideInHierarchy`, `HideInInspector`, `DontSaveInEditor`,
`DontSaveInBuild`, `DontUnloadUnusedAsset`, `NotEditable`, `DontSave`, and
`HideAndDontSave` have their Unity effects on hierarchy visibility, inspector
visibility, saving, build inclusion, unloading by `UnloadUnusedAssets`, and
editability. Automation can still list hidden objects, and each one is marked
with its flags.

## Object references

References identify assets, sub-assets, scene objects, components, and
ScriptableObjects within their valid scope.

### Required behavior

* Asset moves and renames do not break identity-based references.
* Scene-local references resolve to the intended scene object and component.
* A reference to a removed component remains missing even if another component
  of the same type exists.
* A reference picker rejects incompatible targets and explains the expected
  type.
* Circular reference graphs load and save without infinite traversal.

## Missing, unknown, and cross-scene content

* A missing script remains an identifiable missing component record with its
  serialized data preserved where safely possible. It does not disappear,
  become a different script, or make its GameObject appear fully compatible.
* A missing prefab source leaves its instances as identifiable broken prefab
  instances with their overrides preserved.
* Serialized classes and fields that unitas does not recognize are preserved
  with their location and identity, and the containing asset reports the
  unknown content.
* Persistent cross-scene references exist only where supported Unity data or a
  declared package feature can express them. Ordinary scene serialization never
  invents persistent cross-scene references. Assigning a reference to an
  object in another scene in the editor produces the Unity-compatible
  diagnostic and the reference is not saved; at runtime, references between
  loaded scenes behave as ordinary object references.
* Cyclic references, including references between components on objects that
  reference each other, resolve to the same targets after reload.

## Scene serialization

A scene retains root ordering, hierarchy, sibling ordering, names, activation,
tags, layers, static flags, component ordering, component values, references,
and its scene-level settings references (lighting settings, baked lighting
data, occlusion data, and navigation data).

Opening and saving one scene does not alter unopened scenes. If part of a
scene is unknown, missing, or unreadable, its sibling content remains
available and those records retain location and identity. A scene is not reported
fully compatible simply because its hierarchy can be displayed.

### Additive scene sets

As a level designer, I can open several scenes together, choose the active
scene, load and unload scenes in the set, and save each scene. Additively open
scenes remain separate save units: saving one scene writes only that scene.
New objects are created in the active scene. Moving an object between scenes
changes its scene ownership explicitly and reports references that cannot
remain valid. A scene set can be reopened as the same set. Loading scenes at
runtime is owned by RUN-005 in [Scripting runtime](scripting-runtime.md).

## Prefab assets and instances

### User stories

* As a developer, I can create a prefab from a GameObject hierarchy.
* As a level designer, I can place multiple instances that remain linked to the
  source.
* As a developer, I can override one instance without changing other instances.
* As a prefab author, I can apply selected instance changes to the source or
  revert them.
* As a developer, I can nest prefabs and inspect where every value originates.
* As a developer, I can create a prefab variant of a base prefab and override
  values in the variant.

### Source and instance relationship

An instance identifies its source prefab and corresponding source objects and
components. Unchanged source values flow to instances. Instance overrides take
precedence only for their declared property or structural change.

The editor and automation distinguish:

* Property overrides.
* Added components.
* Removed source components.
* Added child objects.
* Removed source child objects.
* Nested prefab boundaries.
* Missing or disconnected source relationships.

An override display identifies the source value, instance value, and ownership
level. Reverting removes only selected overrides. Applying changes identifies
which source prefab and which other instances can be affected. Overrides that
Unity does not record, such as reordering source children inside an instance,
are rejected with the reason rather than saved in a form Unity cannot read.

### Nested prefabs and variants

Nested prefab instances retain their own source boundary inside an outer prefab.
An override at one nesting level does not become an override at every level.
The editor exposes the complete inheritance path for a selected value.

Prefab variants inherit from a base prefab while retaining their own declared
overrides. A variant can itself be the base of another variant. Updating the
base propagates through variants and instances unless a nearer level overrides
the affected value. Applying an instance override offers each level of the
inheritance path as a target.

Cycles in prefab inheritance or nesting are rejected and diagnosed. A missing
base remains visible as a broken relationship rather than silently becoming an
independent prefab.

### Structural edits

Adding or removing a component or child from a source updates instances while
retaining compatible instance overrides. If a source element with overrides is
removed, the editor identifies the orphaned overrides and their resolution
options.

Reordering source content preserves instance correspondence. Two similar objects
are never matched solely by display name when their identities differ.

### Unpacking and disconnection

Unpacking converts the selected instance boundary into ordinary scene objects
while preserving current observable values and child relationships. Unpacking
one level keeps nested prefab boundaries; unpacking completely removes them.
Unpacking is distinct from losing a source through an error.

## Instantiation and duplication

Runtime instantiation and cloning are owned by RUN-001 in
[Scripting runtime](scripting-runtime.md). Editor duplication of objects and
prefab instances preserves internal references as defined in the
[Serialization contract](#serialization-contract): references inside the
duplicated hierarchy target the duplicates, and references outside it keep
their targets.

## ScriptableObject assets

ScriptableObjects provide asset identity and serialized data independent of a
scene GameObject. Multiple references to one asset observe the same asset
identity. Duplicating an asset produces a distinct identity and copied supported
values.

Runtime-created ScriptableObjects are distinct from project assets unless
explicitly saved through an editor action. Destroying a runtime instance does
not delete a project asset.

## Serialization forms and generated caches

### Asset serialization modes

The project's asset serialization mode (Force Text, Force Binary, or Mixed),
set in editor settings (PRJ-007 in [Projects and assets](project-and-assets.md)),
decides the form of newly written assets:

* Assets in Unity's text form and in Unity's binary form are both readable
  regardless of the project's mode.
* Assets that Unity always writes in binary form, such as baked lighting data,
  navigation data, and some terrain data, are readable and are saved in
  binary form.
* An edited asset is saved in the form it already has, unless the user
  changes the project's mode.
* A new asset is written in the form the project's mode gives it, as Unity
  applies that mode to that asset type.
* Changing the project's mode is an explicit action that lists the assets that
  would be rewritten before it rewrites them in the new form.

Text-form saves keep Unity's document structure, object ordering, field
ordering, and number formatting, so saving an edited asset changes only the
lines for the edited values and their dependent records, and saving an
unchanged asset writes no change.

### Serialized version migration

This document owns the migration to the Unity 2022.3 LTS baseline of projects
saved by every earlier Unity release that Unity 2022.3 LTS upgrades,
including Unity 2020.3 LTS, Unity 2021.3 LTS, earlier LTS releases, and
non-LTS releases:

* Objects recorded with an older `serializedVersion`, and settings and
  `.meta` importer data in older forms, are read and upgraded to the forms
  Unity 2022.3 LTS uses, with the same values, identities, references, and
  overrides that Unity 2022.3 LTS produces when it upgrades the same project.
* An asset is written in the upgraded form only when it is saved because of an
  edit or by the explicit upgrade action (PRJ-001 in
  [Projects and assets](project-and-assets.md)); opening never rewrites
  unchanged assets.
* Saved assets always use the forms Unity 2022.3 LTS writes, as Unity 2022.3
  LTS does after an upgrade. In a project whose
  `ProjectVersion.txt` still declares an earlier release, the first save that
  writes an upgraded form first lists the assets and states that they then
  need Unity 2022.3 LTS to open in Unity.
* Behavior that differs between the earlier release and Unity 2022.3 LTS is
  not matched; migrated content behaves as it does in Unity 2022.3 LTS.
* A serialized form newer than Unity 2022.3 LTS, written by a later Unity
  release, is read wherever its fields map to the Unity 2022.3 LTS form of
  the same object, and the object is editable and playable with those values.
  Fields that the 2022.3 LTS form has no place for are preserved, and the
  asset reports them by name. An edit that keeps them saves them unchanged;
  a save that cannot retain them is blocked until the user explicitly
  chooses to discard the identified data.

### Generated caches and unitas-owned state

* unitas never requires Unity's `Library/` folder or any other
  Unity-generated folder (`Temp/`, `Logs/`, `obj/`, `UserSettings/`). Opening
  a project with or without them produces the same identities, import results,
  and behavior.
* unitas never writes into Unity-generated folders. After unitas changes
  assets, Unity detects and reimports them as it would any external edit.
* unitas-owned state is machine, session, and per-user data: import results,
  caches, editor view state, session records, and per-user configuration
  (such as trust records and personal preferences). It is stored where Unity
  does not import it as project content and where a repository using the
  standard Unity ignore template does not show it as an untracked change.
* Deleting unitas-owned caches and reopening the project produces the same
  identities, import results, and behavior.
* unitas-specific project content is shared data intended for source
  control: the recorded unitas version (PRD-001 in
  [Builds, platforms, XR, and networking](build-platforms-xr-networking.md)),
  per-project configuration (such as review policy, secret markings, and
  crash-report destinations), and playtest scenarios. It is stored in files
  that Unity does not import or misinterpret. The recorded unitas version is
  written only when unitas saves another change to the project, and is listed
  with that save; opening never writes it.
* Generated IDE project files (`.sln` and `.csproj`) are written at the
  project root as defined by DIA-002 in
  [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md).
  They, and artifacts written to a location that a person or invocation names
  (build outputs, test results, logs, symbols, exports), are the only files
  unitas writes outside project content and unitas-owned state. Either unitas
  or Unity may regenerate the IDE project files.

## Save integrity and interoperability

A save is either reported successful with all committed content represented, or
reported failed with the last valid saved content identifiable. Partial writes
are never presented as success.

For supported shared formats, content saved by unitas retains its meaning when
reopened in supported Unity versions. Content that cannot remain interoperable
requires an explicit user-visible boundary before commitment.

## Acceptance outcomes

1. Saving and reopening unchanged supported content preserves values,
   references, order, and identities, and writes no file change.
   (SER-001, SER-005)
2. Moving an asset with its `.meta` file preserves its GUID and every
   reference; duplicate GUIDs and duplicate local identifiers cannot produce
   arbitrary resolution; a missing sub-asset stays missing. (PRJ-003)
3. A model reimport keeps references to unchanged sub-assets, and a reference
   to a built-in resource keeps its original identity. (PRJ-003)
4. A field renamed with `[FormerlySerializedAs]` keeps its value; a removed
   `[SerializeReference]` type is reported missing and its data returns when
   the type is restored. (SER-001)
5. Unknown, unreadable, or missing values cannot silently become defaults, and
   a failed save leaves the prior valid content identifiable. (SER-001)
6. In an additive scene set, saving one scene writes only that scene, new
   objects go to the active scene, and the set reopens with the same scenes.
   (SER-002)
7. A source prefab change reaches eligible instances and variants without
   erasing unrelated overrides, and applying or reverting an override affects
   only the selected ownership level. (SER-003)
8. Nested prefab and variant values expose an unambiguous inheritance path, and
   a missing base remains a broken relationship. (SER-003)
9. Duplicating a hierarchy in the editor redirects its internal references to
   the duplicates and keeps references to outside objects. (SER-001, SER-004)
10. A missing script, a missing prefab source, and an unrecognized serialized
    class each remain identifiable with their data preserved across an
    unrelated edit and save. (SER-004)
11. A circular reference graph reloads with every reference resolving to the
    same targets, and assigning a reference to an object in another scene in
    the editor produces the diagnostic and saves no reference. (SER-004)
12. In a Mixed-mode project, an edited binary asset stays binary, an edited text
    asset stays text, and baked lighting and navigation data are readable.
    (SER-005)
13. Opening, editing, and saving a project without `Library/` writes nothing
    into Unity-generated folders, and a repository using the standard Unity
    ignore template shows only the edited assets and the recorded unitas
    version file as changed. (SER-005)
14. A component with a property, a `readonly` field, a dictionary, and an
    interface-typed field saves none of them; a null field of a custom
    `[Serializable]` class reloads as a default-constructed instance; a
    closed generic `[Serializable]` field round-trips. (SER-001)
15. A component whose `OnAfterDeserialize` throws reports the type and object,
    and the stored data is unchanged after reload; `OnBeforeSerialize` runs
    on save and on `Object.Instantiate`. (SER-001)
16. An object with `HideFlags.DontSaveInEditor` is absent from the saved scene,
    an object with `HideInHierarchy` is not shown in the hierarchy, and
    automation lists both with their flags. (SER-001)
17. After unitas opens a project and runs a build to a named output folder,
    the only files written outside project content and unitas-owned state are
    the generated `.sln` and `.csproj` files at the project root and the build
    output in the named folder; opening alone writes no recorded unitas
    version. (SER-005)
18. A Unity 2021.3 LTS project opens with every asset's values, identities,
    references, and overrides equal to those Unity 2022.3 LTS produces when it
    upgrades the same project; an asset with an older `serializedVersion` is
    rewritten only after an edit, in the Unity 2022.3 LTS form, and the first
    such save first lists the assets that will then need Unity 2022.3 LTS.
    (SER-005)
19. Per-user configuration and caches never appear as changes in a repository
    using the standard Unity ignore template, while a change to per-project
    review policy appears as a change to a tracked unitas project file.
    (SER-005)
20. A reference to the built-in default skybox material, the built-in
    `Arial` font, and the default UI sprite each resolves to content with the
    same identity and role, and a reference to an identifier outside Unity's
    built-in resource sets stays missing with its identity. (PRJ-003)
21. A Unity 2019.4 LTS project opens with every asset's values, identities,
    references, and overrides equal to those Unity 2022.3 LTS produces when it
    upgrades the same project. (SER-005)
22. An asset saved by a Unity release later than 2022.3 LTS opens with its
    mappable fields editable, an edit saves its unmappable fields unchanged,
    and the asset report names those fields. (SER-005)
23. A component with `Vector3Int`, `RectInt`, `Hash128`, and
    `LazyLoadReference<T>` fields round-trips every value unchanged. (SER-001)
