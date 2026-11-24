# Serialization, Scenes, and Prefabs

## Scope

This specification defines how project data retains meaning across save, load,
Unity interoperability, duplication, instantiation, prefab inheritance, script
changes, and missing or unsupported content. It owns PRJ-003 and SER-001
through SER-005 in the [Feature catalog](feature-catalog.md): asset identity
and references, serialized values, scenes and additive scene sets, prefabs and
variants, missing and unknown content, and serialization forms and generated
caches.

These are desired end-state requirements, not claims about currently shipped
capability. Stock Unity 2022.3 LTS serialization is the baseline, and a project
declaring an earlier baseline release keeps that release's serialized forms
(see [Serialization forms](#serialization-forms-and-generated-caches)).
Package-defined serialization is covered only for the package versions listed
in the supported package matrix (CMP-002 in
[Compatibility contract](compatibility-contract.md)).

Every inspection, edit, save, prefab operation, and identity query in this
document is available to authorized agents through automation as defined in
[Automation and MCP](automation-and-mcp.md). Semantic three-way merging of
scenes, prefabs, and assets is owned by EDT-007 in
[Editor and authoring](editor-and-authoring.md); this document defines the
identities and values that merge preserves.

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
* References to Unity built-in resources, such as default materials, built-in
  meshes, default UI sprites, and built-in fonts, resolve to content with the
  same identity and observable role; a built-in resource with no equivalent is
  reported Unsupported and remains a reference with its original identity.

## Serialization contract

Serialization preserves the values and relationships that constitute supported
project content. A successful load followed by a save does not change the
observable meaning of unchanged content.

Supported value families include booleans, signed and unsigned numbers,
floating-point values, strings, enums, flags enums, vectors, quaternions,
matrices, colors, rectangles, bounds, layer masks, animation curves, gradients,
arrays, lists, nested `[Serializable]` values, object references, and managed
references declared with `[SerializeReference]`.

### Required behavior

* Field identity is retained across supported script reloads and edits.
* Array and list ordering is stable.
* Repeated references remain repeated references rather than duplicate objects.
* Internal references within a duplicated hierarchy target the corresponding
  duplicated objects where Unity behavior requires it.
* External references remain external.
* Intentional null, missing target, incompatible target, and unsupported value
  remain distinguishable.
* Floating-point special values (infinity, negative infinity, NaN, negative
  zero) and full single- and double-precision values round-trip unchanged.
* Saving cannot silently replace unreadable or unsupported data with defaults.
* Unknown data that can be safely retained survives unrelated edits.

## Script field behavior

Public fields and fields marked `[SerializeField]` of supported types are
serialized and appear in the inspector. Fields marked `[NonSerialized]` are not
serialized, and fields marked `[HideInInspector]` are serialized but not
shown. The attribute contract is owned by RUN-003 in
[Scripting runtime](scripting-runtime.md). Supported serializable nested types
retain their field structure.

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
scene is unsupported, supported sibling content remains available and the
unsupported records retain location and identity. A scene is not reported
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

## Instantiation and cloning

At runtime and in supported editor actions, cloning reproduces the selected
object or prefab hierarchy, components, serialized values, activation, and
internal references. The clone has distinct runtime identity. References from
the clone to objects inside the cloned hierarchy target clone counterparts;
references to external objects retain their intended external targets.

Instantiation applies the requested parent, scene, position, rotation, and
activation behavior consistently. Lifecycle events observe the completed
supported serialized state rather than a partially populated object.

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

Objects recorded with an older `serializedVersion` are read and upgraded to
the form of the project's baseline Unity release. An asset is written in the
upgraded form only when it is saved because of an edit; opening never
rewrites unchanged assets. Saved content uses forms the project's declared
Unity release can read, so a project declaring Unity 2020.3 LTS is never saved
in a form only a later release reads. A serialized form newer than the
project's declared release is reported with the affected asset and is
preserved rather than downgraded silently.

### Generated caches and unitas-owned state

* unitas never requires Unity's `Library/` folder or any other
  Unity-generated folder (`Temp/`, `Logs/`, `obj/`, `UserSettings/`). Opening
  a project with or without them produces the same identities, import results,
  and behavior.
* unitas never writes into Unity-generated folders. After unitas changes
  assets, Unity detects and reimports them as it would any external edit.
* unitas-owned machine and session state, such as import results, caches,
  editor view state, and session records, is stored where Unity does not
  import it as project content and where a repository using the standard Unity
  ignore template does not show it as an untracked change.
* Deleting unitas-owned caches and reopening the project produces the same
  identities, import results, and behavior.
* unitas-specific project content intended for source control, such as the
  recorded unitas version (PRD-001 in
  [Builds, platforms, XR, and networking](build-platforms-xr-networking.md))
  and playtest scenarios, is stored in files that Unity does not import or
  misinterpret.

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
5. Unsupported or missing values cannot silently become defaults, and a failed
   save leaves the prior valid content identifiable. (SER-001)
6. In an additive scene set, saving one scene writes only that scene, new
   objects go to the active scene, and the set reopens with the same scenes.
   (SER-002)
7. A source prefab change reaches eligible instances and variants without
   erasing unrelated overrides, and applying or reverting an override affects
   only the selected ownership level. (SER-003)
8. Nested prefab and variant values expose an unambiguous inheritance path, and
   a missing base remains a broken relationship. (SER-003)
9. Runtime cloning redirects internal references to clone counterparts.
   (SER-003, SER-004)
10. A missing script, a missing prefab source, and an unrecognized serialized
    class each remain identifiable with their data preserved across an
    unrelated edit and save. (SER-004)
11. A circular reference graph and an editor-time cross-scene reference
    attempt behave as declared, and no persistent cross-scene reference is
    invented. (SER-004)
12. In a Mixed-mode project, an edited binary asset stays binary, an edited text
    asset stays text, baked lighting and navigation data are readable, and an
    asset with an older `serializedVersion` is rewritten only after an edit, in
    a form the declared Unity release reads. (SER-005)
13. Opening, editing, and saving a project without `Library/` writes nothing
    into Unity-generated folders, and a repository using the standard Unity
    ignore template shows only the edited assets as changed. (SER-005)
