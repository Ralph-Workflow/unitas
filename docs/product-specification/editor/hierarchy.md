# Hierarchy

## Scope

This document details the Hierarchy window: how open scenes and their
GameObjects are listed, created, named, selected, parented, reordered,
activated, searched, and deleted. It details EDT-001, with the commands,
shortcuts, drag and drop, and multi-editing of EDT-006 and the undo behaviour of
EDT-005, all owned by [Editor and authoring](../editor-and-authoring.md). This
document owns no catalog ID. Scene and object identity, additive scene sets, and
prefab relationships keep the semantics of SER-002 and SER-003 in
[Serialization and prefabs](../serialization-and-prefabs.md).

## Regions

The Hierarchy is a dockable window titled Hierarchy. From top to bottom:

* **Toolbar.** A create (+) dropdown on the left, a search field with a search
  mode dropdown, and the window menu (⋮) on the right. In Prefab Mode, a back
  arrow and the prefab's breadcrumb appear above the list (see
  [Prefab workflow](prefab-workflow.md#prefab-mode)).
* **Left margin columns.** A visibility column and a pickability column, shown
  on hover and always shown for rows whose state is not the default.
* **Tree.** One header row per open scene, with that scene's root GameObjects
  and their descendants beneath it. In play mode, a DontDestroyOnLoad header
  lists objects moved there at runtime.
* **Row contents.** A foldout arrow when the object has children, an icon (a
  prefab icon for prefab instance roots, the object's assigned icon otherwise),
  the name, and at the right edge a Prefab Mode arrow (›) for prefab instance
  roots, plus badges such as the agent badge (see [Agent presence](#agent-presence)).

## Scene header rows

Each open scene has a header row showing its name (Untitled for an unsaved new
scene) and a scene icon.

* The active scene's name is bold. New objects are created in the active scene.
* A scene with unsaved changes shows an asterisk after its name.
* A loaded scene shows its foldout; an unloaded scene in the scene set shows its
  name greyed with "(not loaded)" and no children.
* A scene asset that is missing on disk shows its name with "(is missing)" in
  the error color.
* The header's menu (the ⋮ button on the row, or right-clicking the row) holds:
  Set Active Scene, Save Scene, Save Scene As, Save All, Unload Scene, Load
  Scene, Remove Scene, Discard Changes, Select Scene Asset, Add New Scene,
  GameObject (the create submenu), and the menu items that project code adds to
  the scene header through `SceneHierarchyHooks`. Unloading or removing a scene
  with unsaved changes asks whether to save first, with Save, Don't Save, and
  Cancel.
* Dragging a scene asset from the Project window into the Hierarchy adds it to
  the scene set as an additive loaded scene, inserted at the drop position
  between headers.
* Dragging scene headers reorders the scene set.

## Rows

* Inactive objects, and objects inactive because an ancestor is inactive, show
  their name greyed.
* Prefab instance roots show the prefab icon and blue name; objects inside an
  instance show blue names as well. Prefab variant instances show the variant
  icon. A prefab instance whose prefab asset is missing shows its name in red
  with the missing-prefab state (see
  [Prefab workflow](prefab-workflow.md#missing-prefabs)).
* Objects added to a prefab instance as overrides show a plus marker on their
  icon.
* An object with a missing script shows a warning marker; hovering it names the
  missing script.
* Clicking a row's foldout expands or collapses it; Alt+clicking it expands or
  collapses the row and all its descendants. Clicking a scene header's foldout
  with Alt expands or collapses the whole scene.
* Expansion state is per user per project and restored on reopen; it is never
  part of the scene file.

## Selection

* Clicking a row selects that object and makes it the active object.
* Ctrl/Cmd+click toggles the row in the selection.
* Shift+click selects the visible range from the active object to the clicked
  row.
* Clicking empty space below the rows clears the selection.
* Up and Down arrows move the selection; Shift+Up and Shift+Down extend it.
* Left collapses the selected row or, when collapsed, moves to its parent;
  Right expands it or, when expanded, moves to its first child. Alt+Left and
  Alt+Right collapse or expand the row and all descendants.
* Home and End select the first and last visible row; Page Up and Page Down move
  by a page.
* Typing letters while the tree has focus moves the selection to the next row
  whose name starts with them.
* Double-clicking a row frames the object in the last-focused Scene view.
* Selection is shared with the Scene view, Inspector, and every other view; a
  selection made elsewhere expands the ancestors of the selected objects and
  scrolls the first one into view.

### Ping

When another view pings an object (for example clicking an object field in the
Inspector, or a Console entry whose context is an object), the Hierarchy
expands to the object, scrolls it into view, and highlights the row with a
brief flash without changing the selection.

## Creating objects

* The create (+) dropdown, right-clicking empty space, and the GameObject menu
  offer the same create items: Create Empty, Create Empty Child, Create Empty
  Parent, 3D Object, 2D Object, Effects, Light, Audio, Video, UI, UI Toolkit,
  Camera, and the items project code and packages add under `GameObject/` with
  `[MenuItem]`. The item lists are defined in
  [Menus and toolbar](menus-and-toolbar.md).
* Created from a right-click on a row, the new object becomes a child of that
  row; created from the toolbar or the menu with objects selected, it becomes a
  sibling of the active object for Create Empty and a child for Create Empty
  Child; with nothing selected, it is created at the scene root of the active
  scene or under the default parent.
* A new object is created at the position of the scene pivot of the
  last-focused Scene view when the preference to create objects at the origin
  is off, and at the world origin when it is on; as a child, it is placed at
  its parent's local origin.
* After creation the object is selected and its name is in rename mode; Enter
  commits and Esc keeps the default name.
* Create Empty Parent (Ctrl/Cmd+Shift+G) creates a new object at the combined
  center of the selection and reparents the selected objects under it,
  preserving their world placement.
* Each creation is one undo operation.

### Default parent

Right-clicking a row and choosing Set as Default Parent makes that object the
parent for objects created or dropped into its scene without an explicit
target; its name is shown with a marker. Clear Default Parent on the same menu
removes it. A default parent is per user per scene and is cleared when the
object is deleted.

## Naming, duplicating, and deleting

* **Rename:** F2 (Windows and Linux) or Return (macOS), a slow second click on
  the selected row's name, or Rename on the context menu. Enter commits, Esc
  cancels, and clicking elsewhere commits. With several rows selected, rename
  applies the typed name to the active object only.
* **Duplicate (Ctrl/Cmd+D):** duplicates every selected object with its
  children, as siblings immediately after the originals, and selects the
  duplicates. Duplicate names follow the project's numbering scheme setting
  (Project Settings > Editor > Numbering Scheme): a suffix of the form
  "Name (1)", "Name.1", or "Name_1", with the configured padding, incrementing
  past the highest existing number among siblings. References inside the
  duplicated hierarchy target the duplicates; references outside keep their
  targets (SER-003).
* **Delete:** Delete (Windows and Linux) or Cmd+Backspace (macOS), or Delete on
  the context menu, deletes the selection and its descendants at once as one
  undo operation. The undo history entry and the command's result list the full
  deleted scope, including descendants. Deleting an object that a prefab
  instance's prefab owns records a removed-GameObject override on the instance
  (SER-003) rather than changing the prefab; the instance root then shows the
  override in its Overrides dropdown
  ([Prefab workflow](prefab-workflow.md#overrides)).
* **Cut, Copy, Paste:** Ctrl/Cmd+X, Ctrl/Cmd+C, and Ctrl/Cmd+V. Paste places
  copies as siblings of the selected object, or at the active scene's root with
  nothing selected. Paste As Child (Ctrl/Cmd+Shift+V) places them under the
  selected object. Cut objects stay in place until pasted; pasting a cut moves
  them, preserving identity.

## Parenting and reordering by drag

* Dragging rows onto another row makes them its children; the target row
  highlights while hovered.
* Dragging rows between two rows reorders them; a horizontal insertion line,
  indented to the depth the drop will produce, shows the exact position.
  Moving the cursor left or right near the bottom of a parent's last child
  changes the insertion depth.
* Dragging onto a scene header moves the rows to that scene's root, at the end;
  dragging between scene headers places them at that position in the scene.
* Dragging onto empty space below the last row unparents the rows to the root of
  their scene, at the end.
* Hovering a collapsed row during a drag expands it after a short pause.
  Hovering near the top or bottom edge scrolls the list.
* Parenting by drag preserves each object's world position, rotation, and scale.
  Holding Alt while dropping preserves local values instead.
* A drop that would make an object its own ancestor is rejected; the cursor
  shows the not-allowed state and the status bar states why. Reparenting an
  object that a prefab instance owns, or reparenting into a prefab instance in a
  way the prefab cannot record, is rejected with the reason (SER-003).
* Moving objects to another scene keeps their identity; references from the
  original scene to them become cross-scene references, which are reported in
  the Console as not saved, as SER-004 defines.
* Dragging rows into the Project window creates prefabs (see
  [Prefab workflow](prefab-workflow.md#creating-prefabs)).
* Dragging a prefab, model, or other instantiable asset from the Project window
  onto a row makes the new instance its child; between rows, a sibling at that
  position; onto a scene header, a root object of that scene.
* Each drop is one undo operation and is available as a command with explicit
  inputs (EDT-006).

The GameObject menu's Set as first sibling (Ctrl/Cmd+=) and Set as last sibling
(Ctrl/Cmd+-) reorder without dragging.

## Context menu

Right-clicking a row opens: Cut, Copy, Paste, Paste As Child, then Rename,
Duplicate, Delete, then Select Children, Select Prefab Root, then Set as Default
Parent or Clear Default Parent, then the create items (as in the GameObject
menu), then Prefab (Open, Open in Context, Select Asset, Unpack, Unpack
Completely, Replace, and Apply or Revert overrides when present), then Toggle
Active State, and the items that packages and project code add to the
Hierarchy context. Items that cannot act on the selection are disabled with
their reason on hover.

## Search

* Typing in the search field filters the tree to a flat list of matching
  objects across all loaded scenes, each with its scene shown in a muted label;
  clearing the field restores the tree with the selection kept and revealed.
* Text matches object names, case-insensitively, by substring.
* `t:` filters by component type, for example `t:Light` or `t:MyScript`; types
  match by name and by base class.
* The search mode dropdown chooses All, Name, or Type matching.
* Search results can be selected, renamed, dragged, and acted on like tree rows.
* Matching objects are also highlighted in the Scene view: non-matching objects
  are drawn greyed while a search is active.
* Searches by tag, layer, and active state use the Search window's hierarchy
  scope (see [Menus and toolbar](menus-and-toolbar.md)), whose results select
  into the Hierarchy.

## Visibility and pickability columns

* The visibility column shows an eye per row: clicking toggles the object and
  its children hidden in the Scene view; Alt+clicking toggles only the object.
  A half-state icon shows when some children differ.
* The pickability column shows a hand per row: clicking toggles the object and
  children non-pickable; Alt+clicking toggles only the object.
* The scene header carries both toggles for the whole scene.
* These states are per user per project, are not saved in the scene, and do not
  enter undo history; Scene view behaviour is defined in
  [Scene view](scene-view.md#scene-visibility-and-pickability).

## Activation, layers, and static flags

The active toggle, tag, layer, and static flags of a GameObject are edited in the
Inspector header ([Inspector](inspector.md)) or with Toggle Active State
(Alt+Shift+A) from the Hierarchy. When a layer or static flag change is made on
an object that has children, a dialog asks whether to apply it to the children
as well, with three choices: "Yes, change children", "No, this object only", and
Cancel. The chosen scope is one undo operation. Batch changes from a
multi-selection apply to every selected object as one undo operation and ask
the children question once.

## Play mode

In play mode the Hierarchy shows the live runtime scene: objects created at
runtime appear as they are created, destroyed ones disappear, and objects moved
with `DontDestroyOnLoad` appear under the DontDestroyOnLoad header. Edits made
in play mode are runtime edits that revert on exit as defined in
[Game view and play mode](game-view-and-play-mode.md). The window carries the
play-mode tint.

## Empty and error states

* With no scene open, the Hierarchy shows a single Untitled scene header.
* With a search that matches nothing, the list states that no objects match and
  names the active filter.
* An object whose data could not be read shows a placeholder row with the error
  marker; the Inspector shows the reason, and the object's identity is kept as
  SER-004 defines.
* While a scene is loading, its header shows a progress indicator and its rows
  appear as they load; the rest of the window stays operable.

### Shortcuts

All bindings are rebindable in the Shortcuts Manager ([Shortcuts](shortcuts.md)).
Edit and GameObject menu bindings act on the Hierarchy when it has focus.

| Binding | Command | Context |
|---|---|---|
| F2 (Windows, Linux) / Return (macOS) | Rename | Global (Edit menu) |
| Ctrl/Cmd+D | Duplicate | Global (Edit menu) |
| Delete (Windows, Linux) / Cmd+Backspace (macOS) | Delete | Global (Edit menu) |
| Ctrl/Cmd+X / C / V | Cut, Copy, Paste | Global (Edit menu) |
| Ctrl/Cmd+Shift+V | Paste As Child | Global (Edit menu) |
| Ctrl/Cmd+A | Select All | Global (Edit menu) |
| Ctrl/Cmd+Shift+N | Create Empty | Global (GameObject menu) |
| Alt+Shift+N | Create Empty Child | Global (GameObject menu) |
| Ctrl/Cmd+Shift+G | Create Empty Parent | Global (GameObject menu) |
| Ctrl/Cmd+= | Set as first sibling | Global (GameObject menu) |
| Ctrl/Cmd+- | Set as last sibling | Global (GameObject menu) |
| Alt+Shift+A | Toggle Active State | Global (GameObject menu) |
| Ctrl/Cmd+F | Focus the search field | Hierarchy |
| Up / Down, Shift+Up / Shift+Down | Move or extend selection | Hierarchy |
| Left / Right, Alt+Left / Alt+Right | Collapse or expand, recursively with Alt | Hierarchy |
| Home / End, Page Up / Page Down | Move selection | Hierarchy |
| H / Shift+H / L | Visibility, isolation, pickability | Scene Visibility, Scene Picking |

### Agent presence

* A row whose object an active agent session is changing shows the agent badge
  at the right edge, before the Prefab Mode arrow: the session's assigned color
  and short label. Hovering shows the session name and its current operation;
  clicking opens the Change Review window filtered to that object
  ([Agent-native additions](agent-native-additions.md)).
* A scene header whose scene contains agent changes not yet reviewed shows the
  badge as well, with the count of changed objects.
* Objects an agent creates appear in place as they are created; objects an
  agent deletes disappear. The selection a person holds is never changed by an
  agent operation; if a selected object is deleted by an agent, it leaves the
  selection and the status bar states which session deleted it.
* Under the require-approval policy, proposed objects appear only while the
  developer previews the proposal, drawn in italics with a dashed session-color
  marker and not selectable for editing.
* The context menu of a row with agent changes gains a trailing Review Agent
  Changes group; no familiar item moves.

### Scale

* The tree is virtualized: only visible rows are realized, and per-frame cost
  follows visible rows and changes, never scene size. A scene with 100,000
  GameObjects opens, scrolls, expands, selects, renames, and searches within the
  release's declared budgets (the
  [scale floor](../diagnostics-profiling-playtesting.md#scale-floor)).
* Expanding a row with tens of thousands of direct children realizes only the
  visible ones; Alt-expanding a deep subtree expands lazily as rows scroll into
  view.
* Search runs against an index maintained incrementally from command records;
  results stream into the flat list, and the list states while results are still
  arriving.
* Shift-range selection across a large range and Select All select by range
  without realizing rows; the Inspector receives one selection change.
* A drag or delete of tens of thousands of objects is one operation and one undo
  entry, and the window stays responsive while it completes.

## Acceptance outcomes

1. Pressing Ctrl/Cmd+Shift+N with nothing selected creates "GameObject" at the
   active scene's root in rename mode; typing a name and Enter commits it; one
   undo removes the object. (EDT-001, EDT-005)
2. Dragging object A onto object B makes A a child of B with its world position
   unchanged; Alt-dropping instead keeps its local values; dragging B onto A's
   child is rejected and changes nothing. (EDT-001, EDT-006)
3. Dragging a row between two siblings shows the insertion line at that depth
   and places the row there; dragging it onto empty space unparents it to the
   scene root. (EDT-001, EDT-006)
4. Ctrl/Cmd+D on "Enemy" with the numbering scheme "Name (1)" creates "Enemy (1)"
   directly after it, and a second duplicate of the original creates
   "Enemy (2)"; references inside the duplicate target the duplicate. (EDT-001,
   SER-003)
5. Deleting a parent with children removes the whole subtree at once with no
   confirmation; the undo history entry lists every deleted object, and one undo
   restores them with their identities. (EDT-001, EDT-005)
6. Alt+clicking a collapsed foldout expands every descendant; Alt+Left
   collapses them again. (EDT-001)
7. Typing `t:Light` in the search field lists every object with a Light across
   all loaded scenes as a flat list; clearing it restores the tree with the
   selection revealed. (EDT-001)
8. Changing the layer of an object with children shows the three-choice dialog;
   "No, this object only" changes only that object as one undo operation.
   (EDT-001, EDT-005)
9. With two scenes open, the active scene is bold, a modified scene shows an
   asterisk, and Set Active Scene from the other header's menu makes it bold and
   the target of new objects. (EDT-001, SER-002)
10. Prefab instance rows show blue names and an instance whose prefab asset is
    missing shows a red name; an object added as an override shows the plus
    marker. (EDT-001, SER-003)
11. Clicking an object field in the Inspector pings its target in the Hierarchy,
    expanding ancestors and scrolling to it, without changing the selection.
    (EDT-001, EDT-002)
12. In play mode, an object marked `DontDestroyOnLoad` appears under the
    DontDestroyOnLoad header, and objects created at runtime appear as they are
    created. (EDT-001, EDT-004)
13. An agent session renaming an object through MCP shows the agent badge on
    that row while the change is applied; clicking the badge opens the Change
    Review window filtered to that object, and the person's selection is
    unchanged. (EDT-001, EDT-008)
14. In a scene with 100,000 GameObjects, scrolling the full list, a name search,
    Select All, and deleting a subtree holding half the scene's objects complete
    within the
    release's declared budgets. (EDT-001, PRF-002)
