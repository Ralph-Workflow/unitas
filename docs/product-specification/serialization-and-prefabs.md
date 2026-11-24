# Serialization, Scenes, and Prefabs

## Scope

This specification defines how project data retains meaning across save, load,
Unity interoperability, duplication, instantiation, prefab inheritance, script
changes, and missing or unsupported content.

These are desired end-state requirements, not claims about currently shipped
capability. Stock Unity 2022.3 LTS serialization is the baseline; package-defined
serialization is covered only for explicitly supported package versions.

## Serialization contract

Serialization preserves the values and relationships that constitute supported
project content. A successful load followed by a save does not change the
observable meaning of unchanged content.

Supported value families include booleans, signed and unsigned numbers,
floating-point values, strings, enums, vectors, colors, rectangles, bounds,
curves, gradients, arrays, lists, nested serializable values, object references,
and supported managed references.

### Required behavior

* Field identity is retained across supported script reloads and edits.
* Array and list ordering is stable.
* Repeated references remain repeated references rather than duplicate objects.
* Internal references within a duplicated hierarchy target the corresponding
  duplicated objects where Unity behavior requires it.
* External references remain external.
* Intentional null, missing target, incompatible target, and unsupported value
  remain distinguishable.
* Floating-point special values and precision have explicit compatibility
  behavior.
* Saving cannot silently replace unreadable or unsupported data with defaults.
* Unknown data that can be safely retained survives unrelated edits.

## Script field behavior

Public and explicitly serialized supported fields appear in the inspector.
Hidden or nonserialized fields remain absent according to their declarations.
Supported serializable nested types retain their field structure.

When script fields change, compatible values remain associated with their
intended fields. Removed or incompatible fields are reported before data is
discarded. Renamed fields retain data only when the project supplies a supported
identity relationship. A compilation failure does not erase serialized script
data.

Two fields with the same display name but different declaring contexts remain
distinct. Type changes that cannot preserve meaning expose the old value and
incompatibility rather than presenting a successful conversion.

## Object references

References identify assets, sub-assets, scene objects, components, and
ScriptableObjects within their valid scope.

### Required behavior

* Asset moves and renames do not break identity-based references.
* Scene-local references resolve to the intended scene object and component.
* Persistent cross-scene references exist only where supported Unity data or a
  declared package feature can express them. Ordinary scene serialization never
  invents persistent cross-scene references.
* A reference to a removed component remains missing even if another component
  of the same type exists.
* A reference picker rejects incompatible targets and explains the expected
  type.
* Circular reference graphs load and save without infinite traversal.

## Scene serialization

A scene retains root ordering, hierarchy, sibling ordering, names, activation,
tags, layers, static flags within scope, component ordering, component values,
and references.

Opening and saving one scene does not alter unopened scenes. Additively open
scenes remain separate save units. Moving an object between scenes changes its
scene ownership explicitly and reports references that cannot remain valid.

If part of a scene is unsupported, supported sibling content remains available
and the unsupported records retain location and identity. A scene is not
reported fully compatible simply because its hierarchy can be displayed.

A missing script remains an identifiable missing component record with its
serialized data preserved where safely possible. It does not disappear, become
a different script, or make its GameObject appear fully compatible.

## Prefab assets and instances

### User stories

* As a developer, I can create a prefab from a GameObject hierarchy.
* As a level designer, I can place multiple instances that remain linked to the
  source.
* As a developer, I can override one instance without changing other instances.
* As a prefab author, I can apply selected instance changes to the source or
  revert them.
* As a developer, I can nest prefabs and inspect where every value originates.

### Source and instance relationship

An instance identifies its source prefab and corresponding source objects and
components. Unchanged source values flow to instances. Instance overrides take
precedence only for their declared property or structural change.

The editor distinguishes:

* Property overrides.
* Added components.
* Removed source components.
* Added child objects.
* Removed source child objects.
* Reparenting and sibling-order overrides where supported.
* Nested prefab boundaries.
* Missing or disconnected source relationships.

An override display identifies the source value, instance value, and ownership
level. Reverting removes only selected overrides. Applying changes identifies
which source prefab and which other instances can be affected.

### Nested prefabs and variants

Nested prefab instances retain their own source boundary inside an outer prefab.
An override at one nesting level does not become an override at every level.
The editor exposes the complete inheritance path for a selected value.

Prefab variants, when supported, inherit from a base prefab while retaining
their own declared overrides. Updating the base propagates through variants and
instances unless a nearer level overrides the affected value.

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
while preserving current observable values and child relationships. Nested
prefab boundaries remain or unpack according to the user's chosen scope.
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

## Save integrity and interoperability

A save is either reported successful with all committed content represented, or
reported failed with the last valid saved content identifiable. Partial writes
are never presented as success.

For supported shared formats, content saved by unitas retains its meaning when
reopened in supported Unity versions. Content that cannot remain interoperable
requires an explicit user-visible boundary before commitment.

## Acceptance outcomes

1. Saving and reopening unchanged supported content preserves values,
   references, order, and identities.
2. A source prefab change reaches eligible instances without erasing unrelated
   overrides.
3. Applying and reverting an override affects only the selected ownership level.
4. Nested prefab values expose an unambiguous inheritance path.
5. Runtime cloning redirects internal references to clone counterparts.
6. Unsupported or missing values cannot silently become defaults.
7. A failed save leaves the prior valid content identifiable.
