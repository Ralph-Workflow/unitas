# UI Authoring

## Scope

This document details the editor's tools for authoring player-facing user
interfaces: Canvas-based UI in the Scene view and Inspector (creation,
the Rect tool on UI, anchor presets, raw and blueprint editing, Canvas Scaler
preview), and UI Toolkit documents in the UI Builder and UI Toolkit Debugger
windows. It details EDT-001 (hierarchy and selection), EDT-002 (inspector and
serialized property editing), EDT-003 (scene handles and snapping), EDT-004
(Game view preview), EDT-005 (undo), and EDT-006 (workspace views, commands,
shortcuts, and drag and drop), which are owned by
[Editor and authoring](../editor-and-authoring.md). This document owns no
catalog ID.

What the authored interface does at runtime is owned elsewhere: canvases,
RectTransform layout, Canvas Scaler, controls, and the event system by UI-001,
and UI Toolkit runtime (UXML, USS, panel settings) by UI-001, in
[UI, audio, and input](../ui-audio-input.md); text by TXT-001 in the same
specification. Where this document states what a value means at runtime, the
owning specification governs.

The UI Builder and UI Toolkit Debugger are built-in windows. Canvas components
come from the uGUI package (`com.unity.ugui`) within the supported package
matrix (CMP-002); their creation menu items and inspectors are present when
the project includes the package.

Every window here docks, groups as tabs, floats, and maximizes as defined in
[Workspace and layout](workspace-and-layout.md). Every menu item, context menu
item, and toolbar action is a command and an MCP tool titled by its path, as
defined in [Menus and toolbar](menus-and-toolbar.md) and
[Agent-native additions](agent-native-additions.md).

## Canvas UI

### Creating UI objects

* GameObject > UI lists the uGUI elements: Image, Raw Image, Panel, Text -
  TextMeshPro, Button - TextMeshPro, Toggle, Slider, Scrollbar, Dropdown -
  TextMeshPro, Input Field - TextMeshPro, Scroll View, Canvas, Event System,
  and the legacy text-based variants under a Legacy submenu. The same items
  appear in the Hierarchy's create menu and context menu under UI.
* Creating a UI element while the selection is a Canvas or a descendant of
  one creates it as a child of the selection. Otherwise it is created under
  the first Canvas in the active scene, or, when the scene has none, under a
  new Canvas (Screen Space - Overlay, with a Canvas Scaler and a Graphic
  Raycaster) created for it.
* Creating the first Canvas in a scene that has no EventSystem also creates
  an EventSystem with the input module that matches the project's active
  input handling (UI-001). The whole creation is one undo operation and
  selects the new element in rename state.
* New UI objects are placed on the UI layer and centered in their parent.
* Creating a TextMeshPro element in a project whose TextMeshPro essential
  resources are not yet imported offers to import them, as defined by TXT-001.

### Rect tool and RectTransform handles

* The Rect tool (T) is the standard tool for UI ([Scene view](scene-view.md)).
  With a RectTransform selected it draws the rectangle with corner and edge
  handles, the pivot as a circle, and the anchors as four triangles.
* Dragging inside the rectangle moves it. Dragging an edge or corner resizes
  it; Shift preserves the aspect ratio when dragging a corner; Alt resizes
  symmetrically about the pivot. Hovering just outside a corner shows the
  rotate cursor; dragging there rotates about the pivot.
* Dragging the pivot circle moves the pivot without moving the rectangle.
* Dragging an anchor triangle moves that anchor; dragging the center of a
  collapsed anchor set moves all four. While dragging, anchors snap to the
  parent's edges and center, and the Scene view shows the anchor fractions
  and the distances from the anchors to the rectangle edges.
* Edges and centers of the dragged rectangle snap to the edges and centers of
  the parent and siblings, with guide lines drawn while they align.
* In 2D mode the Rect tool's handles lie in the canvas plane; in 3D they lie
  in the plane of the selected rectangle.

### RectTransform Inspector

* Position, size, and offset fields are labeled by what the anchors imply:
  Pos X, Pos Y, Width, Height when anchors coincide on an axis; Left, Right,
  Top, Bottom when they are stretched on that axis. Anchors (Min, Max),
  Pivot, Rotation, and Scale follow.
* **Anchor presets.** Clicking the anchor square opens the presets popup: a
  grid of anchor positions and stretch modes on each axis. Clicking a preset
  sets the anchors. Holding Shift also sets the pivot to the matching
  position; holding Alt also moves the rectangle to the matching position
  (and size, for stretch presets); holding both does both. The popup shows
  the effect of the held modifiers as they change.
* **Blueprint mode** (the B button) shows rectangles of rotated and scaled
  objects in the Scene view as if unrotated and unscaled, so their edges can
  be edited directly.
* **Raw edit mode** (the R button) makes editing anchors and pivot in the
  Inspector change only those values, without adjusting position and size to
  keep the rectangle in place. With raw edit off, editing anchors or pivot
  keeps the rectangle where it is on screen.
* A RectTransform driven by a layout group or a content size fitter shows the
  driven fields disabled, with the name of the driving component (UI-001).

### Canvas and Canvas Scaler preview

* The Scene view draws each Screen Space canvas as a rectangle at its
  computed screen size, positioned in the scene; Screen Space - Overlay
  canvases appear at that scale beside the world, and framing the Canvas (F)
  fits it in the view.
* A Canvas Scaler's computed scale factor follows the Game view's current
  resolution and aspect ([Game view and play mode](game-view-and-play-mode.md)):
  changing the Game view's resolution dropdown re-lays out every screen-space
  canvas in edit mode in both views, without changing any serialized value
  (UI-001).
* The Canvas Scaler Inspector shows the reference resolution, screen match
  mode, and match slider, and, for Match Width Or Height, labels the slider's
  ends Width and Height.
* Canvas Inspector shows render mode, pixel perfect, sort order, target
  display, and, for Screen Space - Camera and World Space, the render camera
  or event camera field, with a warning when it is empty.

### Event system in the editor

* Selecting the EventSystem during play mode shows in its Inspector preview
  the current selected object, the pointer's hovered object, and the
  raycast results, updated live (UI-001).
* Navigation visualization: the Visualize button on a Selectable's Inspector
  draws navigation arrows between selectables in the Scene view, as a
  per-user toggle that persists across selection.

## UI Builder

Opened from Window > UI Toolkit > UI Builder, or by double-clicking a UXML
document in the Project window. It edits one UXML document and the USS style
sheets it uses.

### Regions

* **StyleSheets pane** (top left). The style sheets attached to the document
  and their selectors. The add (+) menu creates a new style sheet or adds an
  existing one; the selector field adds a selector (`.class`, `#name`, a type
  name, or a combination, with pseudo-classes). Selectors are listed in sheet
  order; dragging reorders them within a sheet.
* **Hierarchy pane** (left). The document's element tree: elements with name
  and classes, instanced templates, and the document root. Selection, rename,
  reorder, and reparent work as in the scene [Hierarchy](hierarchy.md).
* **Library pane** (lower left). Standard tab with built-in elements grouped
  as Containers, Controls, Numeric Fields, Value Fields, Choice Fields, Text,
  and Inspectors; Project tab with the project's UXML documents (usable as
  templates) and custom controls that project code declares (UI-001).
* **Viewport** (center). The live canvas showing the document as it renders,
  with a toolbar for theme selection, zoom, fit, and Preview toggle. The
  canvas size is set by dragging its edges or in the canvas settings; Match
  Game View sizes it to the Game view's resolution. A background option
  shows a color, an image, or a camera's view behind the canvas.
* **Code previews** (bottom of the viewport, toggleable). Read-only UXML and
  USS text of the document and the selected sheet, updating as edits are made.
* **Inspector pane** (right). For an element: its attributes, the style
  sheet section listing classes with add and remove, and Inlined Styles
  grouped by category (display, position, flex, align, size, margin and
  padding, text, background, border, transform, transition, cursor). For a
  selector: its styles. For the document root: canvas settings.

### Editing

* Dragging an element from the Library onto the viewport or into the
  Hierarchy inserts it at the drop position, with an insertion indicator
  showing where. Dragging a UXML document from the Project tab inserts it as
  a template instance.
* In the viewport, clicking selects the topmost element; dragging an
  absolutely positioned element moves it; dragging edges resizes elements
  whose size is not fully driven by flex; the selection outline shows
  margin, border, and padding.
* A style property set on the element shows an override marker beside its
  field; right-click on the field offers Unset, and offers Unset All on the
  category. Fields whose value comes from a matching selector show which
  selector supplies it.
* Extract Inlined Styles to New Class (in the Style Sheet section) moves the
  element's inline styles into a new selector in the chosen sheet and adds
  the class to the element.
* Dragging a selector from the StyleSheets pane onto an element in the
  viewport or Hierarchy adds that selector's class to the element.
* Variables (USS custom properties) can be bound to a style field from its
  context menu; the field then shows the variable name and its resolved value.
* Preview mode makes the viewport interactive (hover, focus, and pressed
  states render as at runtime) and disables editing until it is turned off.

### Document state and saving

* The window's title and the document tab show an unsaved marker while the
  document or a sheet it uses has unsaved changes.
* While the UI Builder has focus, Ctrl/Cmd+S saves the open UXML document and
  its edited style sheets; elsewhere the binding keeps its global meaning.
* The File menu of the builder's toolbar offers New, Open, Save, Save As, and
  closing the document; closing, opening another document, or closing the
  window with unsaved changes asks to Save, Discard, or Cancel.
* An external change to the open UXML or USS file while it has no unsaved
  builder edits reloads it; with unsaved edits, the builder offers to reload
  or keep its edits, as defined in EDT-007's external-change handling.
* A UXML document that fails to parse opens with the parse errors listed and
  the document read-only until fixed; unknown elements and attributes are
  shown as unresolved and preserved (UI-001).

## UI Toolkit Debugger

Opened from Window > UI Toolkit > Debugger.

* A panel dropdown lists every live UI Toolkit panel: editor windows, the
  Game view's runtime panels in play mode, and the UI Builder viewport.
* Pick Element lets the pointer hover and click an element in the chosen
  panel to select it.
* The left pane shows the panel's live element tree; the right pane shows the
  selected element's box model (margin, border, padding, content sizes), its
  computed styles with the source of each (inline, selector with sheet and
  line, inherited, default), and its matching selectors in specificity order.
* Editing a computed style in the debugger applies it to the live element
  only; it changes no asset and is not undoable as an asset edit.
* Overlay options highlight layout bounds and repaint regions in the panel.

## Shortcuts

UI Builder (context UI Builder): Ctrl/Cmd+S save document; Delete removes the
selected elements or selectors; Ctrl/Cmd+D duplicates; Ctrl/Cmd+C,
Ctrl/Cmd+X, Ctrl/Cmd+V copy, cut, and paste elements; F2 (Return on macOS)
renames; F frames the selection in the viewport.

Canvas UI uses the Scene view and Inspector bindings ([Shortcuts](shortcuts.md)):
T selects the Rect tool; F frames the selected UI object.

All bindings are listed and rebindable in the Shortcuts Manager.

## Agent presence

* UI objects, UXML elements, selectors, and style sheets that an agent session
  is changing carry the agent badge (session color and label) in the
  Hierarchy, the UI Builder's Hierarchy and StyleSheets panes, and the
  Inspector. Hovering shows the session and operation; clicking opens the
  Change Review window filtered to that item
  ([Agent-native additions](agent-native-additions.md)).
* An agent's edits to a UXML or USS file open in the UI Builder appear in the
  viewport as they are applied; when the person has unsaved builder edits to
  the same document, the agent's write reports a conflict instead of
  replacing them (AUT-003 in [Automation and MCP](../automation-and-mcp.md)).
* Context menus gain only a trailing Review Agent Changes group when the
  target has agent changes. No familiar control is moved, renamed, or rebound.

## Agent operation

* UI creation menu items are commands with the same parenting, Canvas, and
  EventSystem behavior as for a person, and their results name every object
  created.
* RectTransform edits accept anchors, pivot, offsets, and anchor presets with
  the Shift and Alt variants as explicit inputs, and raw edit mode as an
  input, so an agent obtains the same result as the corresponding gesture.
* UI Builder edits (insert element, set attribute, add or remove class, set or
  unset style, add selector, extract inline styles, save) are commands on the
  document; the document's element tree, selectors, and computed styles are
  readable through MCP.
* The Debugger's element tree and computed styles of any live panel are
  readable through MCP, and viewport and panel images are capturable as
  visual evidence (AUT-002).

## Scale

* The UI Builder's Hierarchy and StyleSheets panes and the Debugger's element
  tree are virtualized; a document with a very large element tree scrolls,
  selects, and edits with the cost of visible rows and changed elements.
* The viewport restyles and re-lays out only elements affected by an edit,
  as UI Toolkit does at runtime (UI-001).
* Canvas re-layout after a Game view resolution change runs once per change,
  not per frame, and the Scene view draws only canvases in view.

## Acceptance outcomes

1. In a scene with no Canvas, GameObject > UI > Button - TextMeshPro creates a
   Canvas with Canvas Scaler and Graphic Raycaster, an EventSystem, and the
   button under the Canvas on the UI layer, and one undo removes all of them.
   (EDT-001, EDT-005, UI-001)
2. With a Canvas child selected, creating an Image creates it as a child of
   the selection. (EDT-001, UI-001)
3. Choosing the stretch-both anchor preset with Alt and Shift held sets the
   anchors to the parent's corners, the pivot to the center, and the offsets
   to zero, and the same preset applied by an agent through MCP gives the same
   values. (EDT-002, UI-001)
4. With raw edit mode off, changing a RectTransform's pivot keeps the
   rectangle's screen position; with raw edit mode on, the rectangle moves.
   (EDT-002, UI-001)
5. Dragging an anchor triangle with the Rect tool snaps it to the parent's
   center, and the Scene view shows the anchor fractions during the drag.
   (EDT-003, UI-001)
6. Changing the Game view resolution from a landscape to a portrait entry
   re-lays out a Scale With Screen Size canvas in edit mode, and no
   serialized RectTransform value changes. (EDT-004, UI-001)
7. A RectTransform driven by a Horizontal Layout Group shows its driven fields
   disabled with the group named. (EDT-002, UI-001)
8. In the UI Builder, dragging a Button from the Library into a container
   inserts it there, the UXML preview shows it, and Ctrl/Cmd+S writes a UXML
   file that reopens with the same tree. (EDT-006, UI-001)
9. Extracting an element's inline styles to a new class moves those styles
   into a selector in the chosen sheet and adds the class to the element, and
   the element's computed styles are unchanged. (EDT-002, UI-001)
10. Closing the UI Builder with unsaved edits asks to Save, Discard, or
    Cancel, and Cancel keeps the window open with its edits. (EDT-005)
11. In play mode, Pick Element in the UI Toolkit Debugger selects a runtime
    element in the Game view and lists the selector and sheet supplying each
    computed style; editing a style there changes no asset. (EDT-004, UI-001)
12. An agent sets a style on an element of a document open in the UI Builder;
    the viewport updates as the edit is applied, and the element carries the
    agent badge in the builder's Hierarchy pane. (EDT-008, UI-001)
