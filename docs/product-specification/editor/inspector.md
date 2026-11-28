# Inspector

## Scope

This document details the Inspector window of the visual editor. It refines
EDT-002 (inspector and serialized property editing), and the parts of EDT-001
(activation, tags, and layers), EDT-005 (one undo operation per gesture), and
EDT-006 (commands, drag and drop, and multi-editing) that surface in the
Inspector. Those features are owned by [Editor and authoring](../editor-and-authoring.md);
this document owns no catalog ID. The serialized values the Inspector edits keep
the guarantees of SER-001 in [Serialization and prefabs](../serialization-and-prefabs.md),
and project and package custom editors draw here as defined in
[Custom editor extensions](../editor-and-authoring.md#custom-editor-extensions).

## Purpose and regions

The Inspector shows and edits the current selection: one or more GameObjects,
one or more assets, or a settings object. It is a single vertical, scrollable
column made of these regions, top to bottom:

1. **Window tab and window menu.** The tab carries the window's title and, at
   its trailing edge, the lock toggle and the window menu (the vertical
   three-dot button). The window menu holds Normal, Debug, and Debug-Internal
   modes, Add Tab, Maximize, and Close Tab.
2. **Header.** For a GameObject, the object header described below. For an
   asset, the asset header: the asset's icon, its name, its type, and, for
   imported assets, an Open button and the import settings toolbar.
3. **Body.** The Transform (or Rect Transform) and then each component in
   serialized order, each with its own header and property list; for an asset,
   its importer settings or its serialized properties.
4. **Add Component button**, below the last component, for GameObjects.
5. **Preview pane**, docked at the bottom for asset types with visual or
   audible content, separated from the body by a draggable divider.

A selection of nothing shows an empty Inspector. A selection of objects of
different kinds (for example a GameObject and an asset) shows the narrowest
shared presentation and states that the remaining selected objects are not
shown.

## GameObject header

The header occupies the top of the Inspector for a GameObject selection.

* **Icon selector.** The leading icon button opens a popup of label icons and
  shape icons, plus Other... to choose any texture. The chosen icon is drawn in
  the Scene view at the object's position (EDT-003). Choosing None clears it.
* **Active toggle.** A checkbox before the name sets the object's own active
  state. Toggling it is one undo operation. An object that is active itself
  but inactive in the hierarchy shows the checkbox checked and the whole
  Inspector body dimmed.
* **Name field.** Editing the name and pressing Return, Enter, or moving focus
  commits it; Esc cancels.
* **Static checkbox and flags dropdown.** The checkbox at the trailing edge sets
  or clears all static flags. The adjacent dropdown lists Nothing, Everything,
  Contribute GI, Occluder Static, Occludee Static, Batching Static, Navigation
  Static, Off Mesh Link Generation, and Reflection Probe Static, each toggled
  individually. When the object has children, changing static flags asks
  whether to apply the change to all child objects as well, offering
  "Yes, change children", "No, this object only", and Cancel. Cancel changes
  nothing.
* **Tag dropdown.** Lists the project's tags (Untagged, Respawn, Finish,
  EditorOnly, MainCamera, Player, GameController, and project-defined tags)
  and, last, Add Tag..., which opens the Tags and Layers settings page with a
  new tag entry focused.
* **Layer dropdown.** Lists the project's named layers by index and, last,
  Add Layer.... When the object has children, changing the layer asks whether
  to set the layer for all child objects as well, with the same three choices
  as static flags.
* **Prefab row.** For a prefab instance, a row below the header shows the
  prefab asset field, Open, Select, and the Overrides dropdown, as defined in
  [Prefab workflow](prefab-workflow.md). For a prefab asset selected in the
  Project window, the header shows Open Prefab in place of the component
  body editing controls of the asset.

## Components

### Component header

Each component, including Transform, has a header row containing, in order:

* a foldout arrow; clicking it, or clicking the header's empty area, expands or
  collapses the component's properties. Alt-click on the foldout expands or
  collapses every component of the object at once;
* the component's icon;
* an enable checkbox, present only when the component type has an enabled
  state (Behaviours, Renderers, Colliders, and similar). Toggling it is one
  undo operation and sets `enabled`;
* the component's display name; for scripts, the class name in title form;
* at the trailing edge: the help button (opens the component's reference
  documentation, or the `[HelpURL]` that a script declares), the presets
  button (opens the preset selector for types that support presets, as defined
  in [Settings and preferences](settings-and-preferences.md)), and the
  component menu button (vertical three dots).

The expanded or collapsed state of each component type is remembered per user
and restored for every object that has that component. Collapsing a component
does not change project content.

### Component menu

The component menu button and a right-click on the header open the same
component context menu, with these items in this order, each available only
when its contract permits it:

* Reset — restores the component's default values and runs its `Reset` method;
  on a prefab instance this produces overrides, not a revert;
* Remove Component — disabled with a stated reason when another component
  depends on it through `[RequireComponent]`, naming that component;
* Move Up and Move Down — reorder the component among its siblings; Transform
  never moves;
* Copy Component;
* Paste Component As New — available when a copied component may be added to
  every selected object;
* Paste Component Values — available when the copied component's type matches;
* Find References In Scene — filters the Hierarchy to objects that reference
  this component;
* Edit Script — for script components, opens the script in the configured
  external editor (see
  [Scripts and external code editors](../editor-and-authoring.md#scripts-and-external-code-editors));
* Properties... — opens a floating Inspector-style window showing only this
  component, which stays open when the selection changes;
* prefab items (Modified Component, Apply to Prefab, Revert) when the
  component is overridden, as defined in [Prefab workflow](prefab-workflow.md);
* project items declared with `[ContextMenu]` on the component's script and
  `[MenuItem("CONTEXT/<Type>/...")]` methods, after a separator, in their
  declared order and with their validation state.

Each item is a command whose MCP tool title is its context path, for example
`CONTEXT/Rigidbody/Reset` (EDT-006).

### Reordering by drag

Dragging a component header up or down shows an insertion line between
component headers; dropping moves the component there as one undo operation.
Dropping above the Transform is rejected with the rejection cursor. Dragging a
component header onto another GameObject's Inspector (for example a locked
Inspector) or onto an object in the Hierarchy copies it there.

### Missing scripts

A component whose script cannot be resolved shows the header "Script" with a
warning icon and a statement that its script cannot be loaded, keeps its
serialized data, and offers its Script field for assigning a replacement
script. The data is preserved through save (SER-004 in
[Serialization and prefabs](../serialization-and-prefabs.md)). A component whose
script exists but fails to compile shows the same field and states that
compilation errors must be fixed first, with a link to the Console.

## Add Component

The Add Component button opens a searchable popup anchored below the button;
Ctrl/Cmd+Shift+A opens the same popup for the selected objects from anywhere.

* The search field is focused on open. Typing filters all component types by
  name across categories; Up and Down move the highlight; Return adds the
  highlighted type; Esc closes without change.
* With an empty search, the popup lists categories (Mesh, Effects, Physics,
  Physics 2D, Navigation, Audio, Video, Rendering, Tilemap, Layout, Playables,
  Miscellaneous, Scripts, UI, Event, UI Toolkit, and the categories that
  packages and `[AddComponentMenu]` declare). Right arrow or Return enters a
  category; Left arrow or Backspace returns.
* The last entry for a non-matching search is New script: it prefills the name
  with the search text, then Create and Add writes a new script asset in the
  project's Assets folder, compiles, and adds the component once compilation
  succeeds. Until then the new component shows as pending.
* Types excluded by `[DisallowMultipleComponent]` are listed but disabled with
  the reason. Adding a type with `[RequireComponent]` adds its requirements in
  the same undo operation and names them.

## Property fields

### General rules

* Each property row has a label column and a value column; the divider between
  them is draggable and its width persists per user.
* Hovering a label shows its `[Tooltip]` text.
* A property edit commits on Return, focus change, or the end of a drag, and
  forms one undo operation (EDT-005). `[Delayed]` fields commit only on Return
  or focus change.
* Tab and Shift+Tab move focus between fields; Up and Down move between rows
  when a row (not a field) has focus; Left and Right collapse and expand a
  foldout row.

### Numbers

* Dragging horizontally on the label of a numeric field changes the value
  continuously (a slider drag on the label). Holding Shift while dragging
  changes it faster and holding Alt changes it slower. The cursor over the
  label shows a horizontal-resize shape.
* Numeric fields accept expressions: arithmetic (`+ - * / %`, parentheses),
  functions such as `sqrt`, `sin`, `cos`, `tan`, `floor`, `ceil`, `round`, and
  the multi-object forms `L(a,b)` (linear distribution across the selection),
  `R(a,b)` (random in range per object), and `+=`, `-=`, `*=`, `/=` relative to
  each object's current value.
* `[Range]` draws a slider with a numeric field; `[Min]` clamps on commit.
* Integer fields reject non-integer results by rounding on commit.

### Other controls

* **Vectors and rotations** draw labeled X, Y, Z (and W) sub-fields, each with
  its own label drag. Rotations show Euler angles.
* **Colors** draw a swatch; clicking opens the color picker with a hue
  wheel or square, RGB, HSV, and hexadecimal entry, alpha, the HDR intensity
  slider for `[ColorUsage(hdr: true)]`, swatch libraries, and an eyedropper
  that samples anywhere on screen.
* **Curves** draw a thumbnail; clicking opens the curve editor with key
  editing, tangent modes, and preset shapes.
* **Gradients** draw a bar; clicking opens the gradient editor with color and
  alpha keys, mode (Blend, Fixed, Perceptual Blend), and presets.
* **Enums** draw a popup; **flags enums** and **layer masks** draw a multi-select
  popup with Nothing and Everything.
* **Strings** draw a single-line field; `[Multiline]` and `[TextArea]` draw a
  multi-line field.
* **Booleans** draw a checkbox; clicking the label toggles it.
* **Nested serializable values** draw as a foldout; **managed references**
  (`[SerializeReference]`) draw a foldout with a type popup for choosing or
  clearing the instance type.
* `[Header]`, `[Space]`, `[HideInInspector]`, `[InspectorName]`, and
  `[ContextMenuItem]` are honored as listed in
  [Inspector](../editor-and-authoring.md#inspector).

### Object fields

An object reference field shows the referenced object's icon and name with its
type in parentheses, "None (Type)" when null, and "Missing (Type)" when the
reference does not resolve.

* **Single-click** on a filled field pings the referenced object: the Project
  window or Hierarchy scrolls to it and highlights it briefly without changing
  the selection.
* **Double-click** opens it: assets open as their double-click action defines,
  scene objects become selected.
* **Object picker.** The circle button at the trailing edge opens the object
  picker window with Assets and Scene tabs, a search field that accepts the
  same filters as the Project window, an icon-size slider, and a None entry.
  Single-click assigns and keeps the picker open; double-click or Return
  assigns and closes; Esc restores the original value and closes.
* **Drag and drop.** Dragging a compatible object or asset over the field
  highlights it and shows the copy cursor; dropping assigns it. Dragging a
  GameObject onto a field of a component type assigns that component of the
  object. An incompatible drag shows the rejection cursor, and the status bar
  states the expected type; dropping changes nothing.
* Delete or Backspace with the field focused sets it to None.

### Arrays and lists

Arrays and lists draw as reorderable lists: a foldout header with the element
count field, a drag handle per element, and + and − buttons at the bottom.

* Editing the count resizes the list; new elements copy the last element.
* Dragging a handle reorders the element as one undo operation.
* Dropping assets or objects on the list header appends them as elements.
* `[NonReorderable]` draws the list without handles.
* Right-click on an element offers Duplicate Array Element and Delete Array
  Element, in addition to the property context menu.

### Property context menu

Right-click on any property label opens:

* Copy and Paste (pasting is available when the clipboard value is compatible);
* Copy Property Path (the serialized property path);
* prefab items for overridden properties: Apply to Prefab '<name>' (one entry
  per applicable prefab level) and Revert, as defined in
  [Prefab workflow](prefab-workflow.md);
* Properties... for object reference properties, opening the referenced object
  in a floating Inspector-style window;
* items declared with `[ContextMenuItem]`;
* for animated properties during Animation window recording, Add Key, Delete
  Key, and Remove All Keys (see
  [Animation and Timeline windows](animation-and-timeline-windows.md)).

## Multi-object editing

When several objects are selected, the Inspector shows the components that every
selected object has and lists the others as not shared. A property whose value
differs among the selection shows a dash (—) in place of its value; a mixed
checkbox shows a dash mark. A mixed value is never written until the person
commits a value; committing writes only that property, on every selected
object, as one undo operation. The header states the number of selected
objects. (EDT-006)

## Lock and multiple inspectors

* The lock toggle on the tab freezes the Inspector on its current selection;
  later selection changes do not affect it. Unlocking follows the selection
  again. Locked state persists with the workspace arrangement.
* Window > General > Inspector, or Add Tab > Inspector, opens another
  Inspector. Each Inspector locks independently, so one can stay on one object
  while another follows the selection; dragging between them copies components
  and assigns references.
* A locked Inspector whose object is deleted shows an empty state stating that
  its object no longer exists; undoing the deletion restores it.

## Modes

The window menu sets the mode:

* **Normal** shows each type's default presentation or its custom editor.
* **Debug** shows every serialized field, including private serialized fields,
  with default controls and no custom editors or drawers, and shows the
  Instance ID and local file identifier where they exist.
* **Debug-Internal** additionally shows internal fields of built-in types.

Each Inspector has its own mode, persisted with the arrangement.

## Asset inspectors

* **Imported assets** (textures, models, audio, and other importers) show their
  importer settings with per-target override tabs, followed by Revert and
  Apply buttons. Edits are pending until Apply; Revert discards them. Changing
  the selection or closing the editor with pending import settings asks
  "Unapplied import settings" with Apply, Revert, and Cancel; Cancel keeps
  the selection. Apply reimports the asset (see
  [Project window](project-window.md)).
* **Materials** show the shader dropdown, the material's properties with the
  shader's or custom shader GUI's presentation, render queue, GPU
  instancing, double-sided global illumination, and, for a material variant,
  its parent with locked and overridden properties.
* **Script assets** show the source text read-only with an Open button that
  opens the external editor, and the script's import settings (default
  references, execution order link).
* **Folders** show the folder name and, when the folder holds an assembly
  definition, a link to it.
* **Scene assets** show the scene name and an Open button; a scene's lighting
  settings are edited in the Lighting window (see
  [Lighting and rendering tools](lighting-and-rendering-tools.md)).
* **ScriptableObject assets** show their serialized fields like a component,
  with the script field at the top.
* **Read-only assets** (immutable package content) show every field disabled
  and state why; an attempted edit through MCP returns that it was not
  committed.

## Preview pane

* The pane appears for assets with a preview (meshes, models, textures,
  materials, sprites, audio clips, animation clips, prefabs). Its title bar
  collapses or expands it; dragging the divider resizes it; the size persists
  per user.
* Mesh and material previews orbit by drag and zoom by scroll; the preview
  toolbar offers lighting, background, and shape choices where meaningful.
  Audio previews play, loop, and auto-play.
* The pane's lower edge shows **asset labels**: the label button opens a
  popup listing existing labels with checkboxes and a field for a new label;
  labels are written to the asset's metadata.
* Below the labels, the **AssetBundle** row shows bundle name and variant
  dropdowns with New and Remove Unused Names entries.

## Play mode

During play mode the Inspector edits the live objects in the play session.
These edits are not saved and revert when play mode ends (see
[Game view and play mode](game-view-and-play-mode.md)). To keep values, the
person uses Copy Component during play mode and Paste Component Values on the
same component after exiting; the pasted values form one undo operation on the
edit-time object. Runtime-only state (for example values of non-serialized
fields) appears only in Debug mode. Properties of objects created at runtime
show their values with no prefab or save controls.

## Custom editors

Custom inspectors and property drawers that the project and its packages
declare replace the default presentation of their targets, as defined in
[Custom editor extensions](../editor-and-authoring.md#custom-editor-extensions).
The window menu offers Default Inspector, which switches the current target
between its custom and default presentation. A custom editor that throws shows
the exception in place of its body with a link to the Console entry; the
header, component menu, and other components stay operable.

### Shortcuts

Shortcuts Manager context: **Inspector**.

| Action | Binding |
|---|---|
| Add Component popup | Ctrl/Cmd+Shift+A (global, `Component/Add...`) |
| Open or focus Inspector | Ctrl/Cmd+3 (global) |
| Copy hovered or focused field value | Ctrl/Cmd+C |
| Paste into hovered or focused field | Ctrl/Cmd+V |
| Commit field | Return or Enter |
| Cancel field edit | Esc |
| Next and previous field | Tab, Shift+Tab |
| Clear focused object field | Delete or Backspace |
| Toggle focused checkbox | Space |
| Expand or collapse focused foldout | Right, Left |
| Expand or collapse all components | Alt-click a component foldout |

Ctrl/Cmd+C and Ctrl/Cmd+V act on a field only while a field is hovered or
focused; otherwise the global Copy and Paste of the selection apply. All
bindings are rebindable in the Shortcuts Manager ([Shortcuts](shortcuts.md)).

### Agent presence

* A component that an active agent session is changing shows the agent badge
  at the trailing edge of its header, before the help button, without moving
  the header's other controls.
* A property an agent session changed carries a marker in the right margin of
  its row, in the session color. It coexists with the prefab-override bar in
  the left margin and with bold override text; neither replaces the other.
* Hovering a badge or marker shows the session name, the operation, and the
  time; clicking opens the Change Review window filtered to that object or
  property ([Agent-native additions](agent-native-additions.md)).
* Under the require-approval policy, a previewed proposed value shows in the
  field with the marker and a "proposed" tooltip, and editing the field while
  previewing edits the proposal's base, not the proposal.
* The component and property context menus gain a trailing Review Agent Changes
  group only when the target has agent changes.
* A value changed by an agent while the person is editing the same field does
  not replace the person's uncommitted text; on commit, the conflict rule of
  AUT-003 applies.

### Scale

The Inspector realizes only the rows in view. A component with a list of
100,000 elements draws its header and the visible elements, and the list
paginates its element rows; scrolling, expanding, and editing an element keep
the frame within the declared responsiveness budget. Multi-object editing of a
selection of 100,000 objects computes shared and mixed state incrementally and
commits a value as one command whose result lists every affected object, within
the budgets of the [scale floor](../diagnostics-profiling-playtesting.md#scale-floor).

## Acceptance outcomes

1. Toggling the active checkbox in the header of a selected object deactivates
   it, dims the Inspector body, and one undo restores it. (EDT-001, EDT-005)
2. Changing the layer of an object with children asks whether to change the
   children; "No, this object only" changes only that object and Cancel changes
   nothing. (EDT-001)
3. Choosing Add Tag... in the Tag dropdown opens the Tags and Layers page with a
   new tag entry; the new tag then appears in the dropdown. (EDT-001, EDT-002)
4. The component menu of a Rigidbody lists Reset, Remove Component, Move Up,
   Move Down, Copy Component, Paste Component As New, Paste Component Values,
   Find References In Scene, and Properties... in that order, and invoking
   `CONTEXT/Rigidbody/Reset` through MCP restores its defaults as one undo
   operation. (EDT-002, EDT-006)
5. Remove Component on a component that another component requires is disabled
   and names the requiring component. (EDT-002)
6. Dragging on a float field's label changes the value continuously; the same
   drag distance with Shift held produces a larger change and with Alt held a
   smaller one; the drag forms one undo operation. (EDT-002, EDT-005)
7. Entering `L(0,10)` in the X position field of five selected objects sets
   their X positions to 0, 2.5, 5, 7.5, and 10 in selection order, as one undo
   operation. (EDT-002, EDT-006)
8. With three objects selected whose masses differ, the Mass field shows a
   dash; editing another property leaves the masses unchanged; entering a mass
   sets it on all three. (EDT-006)
9. Dropping a texture on a Mesh Renderer's material slot shows the rejection
   cursor and changes nothing; dropping a material assigns it. (EDT-002,
   EDT-006)
10. Clicking a filled object field pings the referenced asset in the Project
    window without changing the selection, and double-clicking it opens it.
    (EDT-002)
11. In Add Component, typing a name that matches no type and choosing New
    script creates the script, compiles it, and adds the component to the
    selected object. (EDT-002, EDT-006)
12. Locking one Inspector on object A and selecting object B leaves the locked
    Inspector on A while a second Inspector shows B. (EDT-006)
13. Changing a texture's import settings and then selecting another asset asks
    to Apply or Revert; Cancel keeps the texture selected and the pending
    settings intact. (EDT-002)
14. Debug mode shows a script's private serialized field that Normal mode
    hides, without a custom editor. (EDT-002)
15. A value changed in play mode reverts on exit; Copy Component during play
    and Paste Component Values after exit keeps it as one undo operation.
    (EDT-002, EDT-004)
16. A component with a missing script keeps its serialized data through save
    and shows the missing-script state with an assignable Script field.
    (EDT-002)
17. A property changed by an agent session shows the right-margin marker in the
    session color alongside the prefab override bar, and clicking it opens
    Change Review filtered to that property. (EDT-002, EDT-008)
18. Scrolling and editing a serialized list of 100,000 elements stays within
    the declared responsiveness budget. (EDT-002)
