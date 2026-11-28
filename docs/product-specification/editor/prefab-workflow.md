# Prefab Workflow

## Scope

This document details how prefabs are created, placed, inspected, overridden,
applied, reverted, edited in Prefab Mode, unpacked, and replaced in the visual
editor. It details the prefab-related behaviour of EDT-001 (Hierarchy display),
EDT-002 (Inspector override display), EDT-003 (Prefab Mode in the Scene view),
EDT-005 (undo and saving), and EDT-006 (commands and drag and drop), all owned by
[Editor and authoring](../editor-and-authoring.md). This document owns no
catalog ID. The persistence semantics of prefab assets, instances, nesting,
variants, overrides, and unpacking are owned by SER-003 in
[Serialization and prefabs](../serialization-and-prefabs.md#prefab-assets-and-instances);
this document defines only how they are presented and operated.

## Creating prefabs

* Dragging one or more GameObjects from the Hierarchy into the Project window
  (onto a folder in the folder tree, into the asset area, or onto a folder
  item) creates one prefab asset per dragged root, named after the object, in
  that folder. The dragged objects become instances of the new prefabs in
  place.
* When a dragged object is already a prefab instance root, a dialog asks
  whether to create a new original prefab or a prefab variant of the existing
  one, with the choices Original Prefab, Prefab Variant, and Cancel. Original
  Prefab creates an independent prefab whose nested prefab instances are kept as
  nested instances; Prefab Variant creates a variant whose base is the dragged
  instance's prefab, carrying the instance's overrides as the variant's own.
* A name collision in the target folder appends the project's numbering scheme
  suffix (see [Hierarchy](hierarchy.md#naming-duplicating-and-deleting)).
* Dropping onto an existing prefab asset asks whether to replace it, with
  Replace, Replace and Keep Overrides (when the dragged object is an instance of
  a different prefab), and Cancel; replacing keeps the prefab's GUID so
  references to it continue to resolve (PRJ-003).
* Assets > Create > Prefab creates an empty prefab asset in rename mode;
  Assets > Create > Prefab Variant on a selected prefab creates a variant of it.
* Creation is one undo operation covering the asset creation and the
  conversion of scene objects into instances.

## Placing instances

Dragging a prefab asset into the Hierarchy or the Scene view creates an
instance, as defined in [Hierarchy](hierarchy.md#parenting-and-reordering-by-drag)
and [Scene view](scene-view.md#drag-and-drop-into-the-scene-view). Duplicating
an instance creates another instance of the same prefab, carrying its overrides.
Dragging a prefab onto an object field assigns a reference to the prefab asset,
not an instance.

## Instance display

* **Hierarchy:** prefab instance roots show the prefab icon (the variant icon
  for variant instances) and a blue name; objects inside an instance show blue
  names; objects added as overrides show a plus marker on their icon; the root
  row ends with the Prefab Mode arrow (›). Nested prefab instance roots inside
  an instance show their own prefab icon.
* **Inspector header:** a prefab instance root's Inspector shows a prefab row
  beneath the GameObject header with a prefab asset field naming the prefab
  (assigning another prefab to it replaces the prefab, see
  [Unpacking and replacing](#unpacking-and-replacing)) and three controls:
  **Open** (opens Prefab Mode in context), **Select** (selects and pings the
  prefab asset in the Project window), and **Overrides** (the Overrides
  dropdown, see [Overrides](#overrides)). Objects inside the instance show the
  same row with Open and Select acting on the nearest prefab, and the Overrides
  dropdown scoped to the outermost instance root.
* **Scene view:** instances are selected root-first, as defined in
  [Scene view](scene-view.md#click-selection).

## Overrides

### Property overrides

* A property whose instance value differs from its source value shows its label
  in bold and a blue bar in the Inspector's left margin beside the property.
* Components with any overridden property show their header label in bold and
  the margin bar beside the header.
* An array or nested value with an overridden element shows bold labels down to
  the overridden element.
* Right-clicking an overridden property shows, at the top of its context menu,
  **Apply to Prefab '<name>'** (one item per level of the inheritance path when
  the property lies inside nested prefabs or variants, each naming its target
  asset, outermost first) and **Revert**. Applying or reverting one property is
  one undo operation, and applying states the assets it writes.
* Overrides that are recorded only for serialization reasons and change no
  observable value (for example a value set equal to its source) are still
  shown and can be reverted.

### Added and removed content

* An added component shows a plus marker on its header icon and its header
  margin bar; its gear menu holds Apply Added Component to Prefab '<name>' (per
  level) and Revert (remove it).
* A removed source component is shown as a greyed placeholder header in its
  original position, labeled as removed, with a minus marker; its context menu
  holds Apply Removed Component and Revert (restore it).
* An added child object shows a plus marker in the Hierarchy; its context menu's
  Prefab submenu holds Apply Added GameObject and Revert.
* A removed source child object is listed in the Overrides dropdown and can be
  restored there or from the parent row's Prefab submenu.

### Overrides dropdown

The Overrides dropdown on the instance root opens a panel that lists every
override of the instance as a tree mirroring the instance hierarchy: changed
objects and components, added components and objects, and removed components
and objects, each with its icon and a marker for its kind.

* Selecting an entry shows a side-by-side comparison of the prefab's value and
  the instance's value for every changed property of that object or component,
  with per-entry Apply and Revert buttons.
* **Revert All** and **Apply All** at the bottom act on every override. Apply
  All has a target dropdown when the instance is nested or a variant, listing
  each level of the inheritance path.
* Multiple entries can be selected with Shift and Ctrl/Cmd and applied or
  reverted together.
* With no overrides, the dropdown states that there are no overrides and its
  buttons are disabled.
* An override that the prefab cannot accept (for example a reference from the
  prefab to an object outside it, which would become a missing reference in the
  prefab) is listed with a warning and the consequence is stated before Apply
  proceeds.
* Each Apply All or Revert All is one undo operation and states, before
  proceeding, the assets it writes and the number of other instances that will
  change.

## Prefab Mode

Prefab Mode edits a prefab asset in its own stage, replacing the scene in the
Hierarchy and the Scene view while it is open.

### Opening

* **Isolation:** double-clicking a prefab asset in the Project window, or Open
  Prefab in the asset's Inspector, opens Prefab Mode in isolation: only the
  prefab's contents are shown, in a preview environment.
* **In context:** clicking the Prefab Mode arrow (›) on an instance row in the
  Hierarchy, or Open in an instance's Inspector prefab row, opens Prefab Mode in
  context: the prefab's contents are shown at the instance's place, with the
  rest of the scene shown according to the Context visibility setting. Alt-
  clicking the arrow opens the same prefab in isolation.
* Opening a nested prefab from inside Prefab Mode opens a deeper stage; the
  breadcrumb records the path.
* While a stage is open, the main scene keeps its own selection, undo history,
  and unsaved state; returning restores them.

### Environment and header bar

* In isolation, the stage uses the project's Prefab editing environments
  (Project Settings > Editor): a regular environment scene for 3D prefabs and a
  UI environment for prefabs whose root has a RectTransform; with none set, a
  default empty environment with neutral lighting is used.
* The Scene view shows a header bar with: the **breadcrumb** (the scene and each
  open prefab stage, each clickable to return to that level), the
  **Context** visibility control (in-context mode only: Normal, Gray, and Hidden,
  setting how the surrounding scene is drawn), and **Auto Save** with a **Save**
  button.
* The Hierarchy shows a header with a back arrow (‹) and the prefab's name and
  icon; its root is the prefab root; the main scene's objects are not listed.
* The background of the Scene view uses the Prefab Mode background color while
  in isolation.

### Editing and saving

* Edits inside the stage change the prefab asset and propagate to every instance
  and variant except where an instance or nearer level overrides the value
  (SER-003). Propagation to open scenes is visible when returning.
* With Auto Save on, each completed edit is saved to the prefab asset; with it
  off, the header shows an unsaved marker and the Save button saves. Auto Save
  is a per-user preference.
* Leaving the stage with unsaved changes (Auto Save off) asks Save, Discard, or
  Cancel.
* Undo inside the stage undoes stage edits; saved prefab edits remain undoable
  until the stage is left.
* Structural edits that a prefab instance cannot hold (reparenting or reordering
  source children inside an instance) are performed here instead; the
  Hierarchy and Inspector direct a person here when they reject such an edit.
* In a variant's stage, values inherited from the base show normally and the
  variant's own overrides show with the bold label and margin bar.

### Leaving

The back arrow in the Hierarchy header, a breadcrumb item, opening a scene, or
entering play mode leaves Prefab Mode (play mode leaves it after the save
question). The previous stage, its selection, and its camera are restored.

## Unpacking and replacing

* **Unpack** (Prefab submenu of the Hierarchy context menu and GameObject >
  Prefab) converts the selected instance's outermost boundary into ordinary
  objects, keeping nested prefab instances as instances. **Unpack Completely**
  removes every nested boundary as well. Both keep current observable values
  and object identities in the scene (SER-003), and each is one undo operation.
* **Replace** (GameObject > Prefab > Replace, or assigning another prefab to
  the prefab asset field in the instance's Inspector prefab row)
  replaces the instance's prefab with another prefab, keeping overrides whose
  objects and properties match in the new prefab and listing those that do not.
* **Select Asset** selects the instance's prefab asset in the Project window.

## Missing prefabs

A prefab instance whose asset is missing keeps its objects, overrides, and
identity. Its Hierarchy name is red and its Inspector prefab row states that the
prefab asset is missing, naming the asset GUID and the last known path, with
**Unpack** (to keep the objects as ordinary objects) and **Replace** (to relink
to another prefab). It is never silently converted into ordinary objects
(SER-003, SER-004). Restoring the missing asset with the same GUID reconnects the
instance without any action.

## Empty and error states

* A prefab that fails to load opens no stage; the Project window and Inspector
  state the reason, and instances of it behave as missing prefabs.
* A prefab inheritance or nesting cycle is reported in the Console and in the
  affected instance's Inspector, and the cycle edge is not followed (SER-003).
* Apply to a read-only prefab (for example one inside an immutable package) is
  disabled with that reason; such prefabs open in Prefab Mode read-only.

### Shortcuts

Prefab workflow commands have no default bindings of their own beyond the
Hierarchy and Edit bindings in [Hierarchy](hierarchy.md#shortcuts); every
command below is listed in the Shortcuts Manager under the Prefab context and
can be bound by the user ([Shortcuts](shortcuts.md)): Open Prefab, Open Prefab in
Context, Exit Prefab Mode, Save Prefab, Toggle Auto Save, Apply All, Revert All,
Unpack, Unpack Completely, and Select Prefab Asset.

### Agent presence

* An instance or prefab stage that an active agent session is changing shows the
  agent badge on its Hierarchy row and in the Inspector prefab row; hovering
  shows the session name and current operation; clicking opens the Change Review
  window filtered to that object
  ([Agent-native additions](agent-native-additions.md)).
* An agent's Apply to a prefab asset appears in the Change Review window as one
  change set item with the prefab's semantic diff and the list of other
  instances it affected; reverting it restores the prefab and those instances.
* An agent editing a prefab in its own stage does not open Prefab Mode in the
  person's editor and does not change the person's stage.
* The Overrides dropdown shows the agent badge beside overrides an agent session
  created; agent indicators never replace the bold label or blue margin bar.

### Scale

* The Overrides dropdown virtualizes its tree and its comparison view; an
  instance with thousands of overrides opens and filters within the release's
  declared budgets.
* Applying to a prefab with many instances in open scenes updates those
  instances incrementally; the command's result reports how many instances
  changed without listing them all, and pagination returns the full list.
* Prefab Mode stages are virtualized in the Hierarchy like scenes, including
  prefabs with 100,000 objects at the
  [scale floor](../diagnostics-profiling-playtesting.md#scale-floor).

## Acceptance outcomes

1. Dragging a GameObject from the Hierarchy into a Project window folder creates
   a prefab asset named after it and turns the object into a blue-named instance;
   one undo removes the asset and restores the plain object. (EDT-006, SER-003)
2. Dragging an existing prefab instance into the Project window shows the
   Original Prefab / Prefab Variant / Cancel dialog; choosing Prefab Variant
   creates a variant whose base is the original prefab. (EDT-006, SER-003)
3. Changing an instance's property shows its label in bold with the blue margin
   bar; right-clicking it offers Apply to Prefab and Revert; Revert restores the
   source value and removes the bold label. (EDT-002, SER-003)
4. Adding a component to an instance shows the plus marker on its header and an
   entry in the Overrides dropdown; selecting the entry shows the comparison;
   Apply All writes it to the prefab and every other instance gains the
   component. (EDT-002, SER-003)
5. Removing a source component from an instance shows a greyed removed header;
   Revert restores it with its prior values. (EDT-002, SER-003)
6. For an instance nested inside another prefab, an overridden property's
   context menu lists one Apply item per inheritance level, each naming its
   target asset, and applying to the inner prefab leaves the outer prefab
   unchanged. (EDT-002, SER-003)
7. Double-clicking a prefab asset opens Prefab Mode in isolation with a
   breadcrumb and back arrow; clicking a row's Prefab Mode arrow opens it in
   context; switching Context to Gray and Hidden changes how the surrounding
   scene is drawn. (EDT-003, EDT-006)
8. With Auto Save off, editing in Prefab Mode shows the unsaved marker; leaving
   asks Save, Discard, or Cancel; Discard leaves the prefab asset unchanged.
   (EDT-005, SER-003)
9. Unpack converts an instance's outer boundary into ordinary objects while
   keeping a nested instance; Unpack Completely removes every boundary; values
   and object identities are unchanged and each is one undo operation.
   (EDT-001, SER-003)
10. Deleting a prefab's asset file leaves its instances in place with red names
    and a missing-prefab Inspector row naming the GUID; restoring the file with
    the same GUID reconnects them. (EDT-001, SER-004)
11. Replace on an instance relinks it to another prefab, keeps overrides whose
    properties exist in the new prefab, and lists those that do not. (EDT-006,
    SER-003)
12. An agent session applying an instance override through MCP produces one
    change set item with the prefab's semantic diff and the affected instances;
    reverting it in the Change Review window restores the prefab and those
    instances. (EDT-008, SER-003)
