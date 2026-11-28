# Scene View

## Scope

This document details the Scene view: navigation, the scene gizmo, transform
tools and handles, snapping, view options, scene visibility and pickability,
selection, overlays, and drag-and-drop placement. It details EDT-003, with the
selection behaviour of EDT-001 and the commands, shortcuts, and drag and drop
of EDT-006, all owned by [Editor and authoring](../editor-and-authoring.md).
This document owns no catalog ID. Gizmo and debug-line drawing semantics and
editor-extension drawing (`Editor.OnSceneGUI`, `Handles`, `[EditorTool]`,
`[Overlay]`) are defined in
[Editor and authoring](../editor-and-authoring.md#gizmos-and-debug-drawing);
this document places them in the view.

## Regions

A Scene view is a dockable window titled Scene. It has these regions:

* **Viewport.** The rendered scene, drawn by the project's render pipeline with
  editor additions (grid, gizmos, handles, selection outline, icons).
* **Overlays.** Floating or docked panels over the viewport: Tools, Tool
  Settings, Grid and Snap, View Options, Orientation (the scene gizmo), Search,
  and the overlays that the project and its packages declare. Overlays are
  described in [Overlays](#overlays).
* **Header bar.** Shown only in Prefab Mode: the breadcrumb, the Context
  visibility control, and Auto Save, as defined in
  [Prefab workflow](prefab-workflow.md#prefab-mode).
* **Camera preview.** A picture-in-picture preview in the bottom-right corner
  showing what the selected Camera renders, shown while exactly one Camera is
  selected and the camera preview preference is on. The preview uses the aspect
  ratio selected in the Game view.
* **Notifications.** Transient messages drawn centered near the top of the
  viewport, such as the current flythrough speed or the name of a toggled mode.
  They fade on their own and never capture input.

More than one Scene view can be open at once (Window > General > Scene, or Add
Tab > Scene on a tab's menu). Each has its own camera, draw mode, 2D state,
overlay arrangement, and view options; selection, tools, and snap settings are
shared. The most recently focused Scene view is the target of menu commands
such as GameObject > Move To View and of framing requests.

## Navigation

Navigation never changes scene content and never enters undo history. The view
camera's pivot, rotation, size, and projection are per Scene view and are
restored with the workspace arrangement.

| Input | Result |
|---|---|
| Right-drag | Look around (first-person rotation about the camera position). |
| Right-drag + W/A/S/D | Fly forward, left, back, right. |
| Right-drag + Q/E | Fly down and up. |
| Shift while flying | Fly faster for as long as Shift is held. |
| Scroll while flying | Change flythrough speed; the new speed shows as a notification. |
| Alt+left-drag | Orbit around the pivot. Unavailable in 2D mode and while the orientation is locked. |
| Alt+right-drag | Zoom toward or away from the pivot. |
| Middle-drag, or Alt+middle-drag | Pan. |
| Hand tool (Q) + left-drag | Pan; Alt+left-drag orbits; Alt+right-drag zooms. |
| Scroll wheel | Zoom toward the pivot; with the zoom-toward-cursor preference on, toward the point under the cursor. |
| Arrow keys | Move the camera across the ground plane while the viewport has focus; Shift moves faster. |
| F | Frame the selection: move the pivot to the selection's bounds and fit them in view. |
| Shift+F | Lock view to selected: frame the selection and keep following it as it moves, until the camera is navigated or the selection changes. |
| Double-click a row in the Hierarchy | Frame that object in the last-focused Scene view. |

While a navigation gesture is active, the cursor shows the gesture (eye for
look, hand for pan, magnifier for zoom, orbit for orbit). Releasing the mouse
button ends the gesture; keys released early stop flying in that direction. The
Camera settings in the View Options overlay hold the field of view, dynamic
clipping, near and far clip planes, occlusion culling toggle, camera easing,
acceleration, base speed and its minimum and maximum, and these values persist
per Scene view.

Framing an object with no renderer or collider frames a small default bounds
around its position. Framing a selection that spans several scenes frames the
union of their bounds. Framing with nothing selected does nothing and shows a
notification.

## Scene gizmo

The Orientation overlay, by default in the top-right corner of the viewport,
shows the scene gizmo: three colored axis cones (X red, Y green, Z blue) with
grey cones for the negative axes, a center cube, and a projection label below.

* Clicking an axis cone aligns the view to look along that axis toward the
  pivot (Right, Left, Top, Bottom, Front, Back), switching to an orthographic
  view; the label then names the view.
* Clicking the center cube or the label toggles between perspective and
  isometric (orthographic) projection. The label reads Persp or Iso, or the
  named axis view.
* Shift+clicking the center cube returns to the default perspective view.
* Clicking the padlock beside the gizmo locks orientation: orbit and axis
  clicks are disabled; pan and zoom still work.
* Right-clicking the gizmo opens a menu of the six axis views, Free, and the
  perspective toggle.
* Dragging on the gizmo orbits the view.

View transitions animate when camera easing is on, and the result is the same
as without animation.

## 2D mode

Pressing 2 with the viewport focused, or the 2D toggle in View Options, toggles
2D mode. In 2D mode the view is orthographic looking down the negative Z axis,
the scene gizmo is hidden, orbit is disabled, right-drag and middle-drag pan,
the grid lies in the XY plane, and the Rect tool is the natural default for
sprites and UI. Leaving 2D mode restores the previous 3D camera. The project's
Default Behaviour Mode (Project Settings > Editor) sets whether new Scene views
open in 2D.

## Tools

### Tools overlay

The Tools overlay is docked to the viewport's left edge by default, laid out
vertically. Its buttons, in order, are View (Q), Move (W), Rotate (E), Scale
(R), Rect (T), and Transform (Y), followed by the component tools available for
the current selection (for example Edit Collider, or the tools a package
declares with `[EditorTool]`) and the custom global tools. The active tool is
highlighted. A tool button with several variants shows a small corner marker and
opens them on long-press or right-click. When a tool cannot act on the current
selection, it stays selectable and its handles are simply not drawn.

### Tool Settings overlay

The Tool Settings overlay, docked at the top-left by default, holds:

* **Pivot / Center (Z).** Pivot places the handle at the active object's pivot;
  Center places it at the center of the selection's combined bounds. Rotation
  and scale use the same point.
* **Global / Local (X).** Global aligns handles to world axes; Local aligns them
  to the active object's rotation. Scale always uses local axes.
* Tool-specific settings that the active tool or an `[EditorTool]` declares.

Changing either toggle changes no content and does not enter undo history.

### Move tool (W)

The Move handle shows three axis arrows, three plane squares, and a center
square.

* Dragging an arrow moves along that axis.
* Dragging a plane square moves within that plane. Plane squares flip to the
  side facing the camera.
* Dragging the center square moves in the camera's view plane (screen space).
* Shift while dragging the center square also moves in screen space regardless
  of view.
* Ctrl/Cmd while dragging snaps the movement to the move increment.
* Ctrl/Cmd+Shift while dragging the center square snaps the selection onto the
  surface of the collider or mesh under the cursor (surface snapping); with
  rotation alignment enabled in Grid and Snap, the object also aligns its up axis
  to the surface normal.
* Holding V enters vertex snapping: the handle jumps to the vertex of the
  selection nearest the cursor; dragging it snaps that vertex onto the nearest
  vertex of any other visible mesh under the cursor. Releasing V leaves the mode.
  Vertex snapping also works with the Rect tool.

### Rotate tool (E)

The Rotate handle shows three axis circles, an outer view-aligned circle, and a
translucent sphere inside.

* Dragging an axis circle rotates about that axis; while dragging, a filled arc
  shows the swept angle and its value.
* Dragging the outer circle rotates about the view direction.
* Dragging inside the sphere rotates freely (trackball).
* Ctrl/Cmd while dragging snaps to the rotation increment.

### Scale tool (R)

The Scale handle shows three axis lines ending in cubes and a center cube.

* Dragging an axis cube scales along that local axis.
* Dragging the center cube scales uniformly on all axes.
* Ctrl/Cmd while dragging snaps to the scale increment.

### Rect tool (T)

The Rect tool draws the selection's rectangle in the plane facing the camera (or
the object's own plane for RectTransform and sprite content) with corner and edge
handles and a pivot circle.

* Dragging inside the rectangle moves it within its plane.
* Dragging an edge or corner resizes; Shift keeps the aspect ratio; Alt scales
  around the center instead of the opposite edge.
* Dragging just outside a corner rotates.
* Dragging the pivot circle moves the pivot without moving content.
* For RectTransforms, resizing changes size and anchored position rather than
  scale; the anchor handles are drawn and dragged as defined by UI-001 in
  [UI, audio, and input](../ui-audio-input.md), detailed in
  [UI authoring](ui-authoring.md).
* With Raw Edit mode (the R button in a RectTransform's Inspector header area,
  or the Tool Settings toggle) on, changing pivot or anchors does not move the
  content's visible rectangle.

### Transform tool (Y)

The Transform tool combines the Move, Rotate, and Scale handles in one: arrows
and plane squares for moving, circles for rotating, and a center cube for
uniform scale. Each part behaves as on its own tool, including modifiers.

### Common handle behaviour

* Hovering a handle part highlights it in the hover color; dragging shows it in
  the selection color.
* A drag on a multi-object selection moves, rotates, or scales every selected
  object together about the shared handle point. Children of selected objects
  move with their parent only once.
* One drag is one undo operation (EDT-005), named after the tool and the
  selection, for example "Move 3 GameObjects".
* While dragging, the Inspector shows the resulting values live and the status
  of snapping is visible in the Grid and Snap overlay.
* Pressing Esc during a drag cancels it and restores the prior values; no undo
  operation is recorded.
* A locked Transform (for example one driven by an Animator in preview, or a
  RectTransform driven by a layout group) shows its handle disabled and the
  driving component named in a tooltip.

## Snapping and grid

### Grid and Snap overlay

The Grid and Snap overlay, docked at the top by default, holds:

* **Grid visibility** toggle with a dropdown for the grid plane (X, Y, Z), grid
  opacity, and a To Handle / To Origin button that moves the grid to the active
  handle or back to the origin.
* **Grid size** per axis, linked or unlinked.
* **Increment snapping** settings: move increment (per axis), rotate increment,
  and scale increment.
* **Grid snapping** toggle. While on, the Move tool moves selected objects in
  absolute grid positions in world space rather than by increments, and the
  handle is world-aligned.
* **Align Selected** buttons for X, Y, Z, and all axes, which push the selection
  onto the grid (Ctrl/Cmd+\ pushes all axes).
* **Surface snapping options**: align rotation to surface, and the object's own
  colliders ignored.

Ctrl/Cmd+[ halves the grid size and Ctrl/Cmd+] doubles it. Grid and snap
settings are stored per user, not in the project, except where the project's
own settings declare snap values for a component.

### Snapping behaviour

* Holding Ctrl/Cmd during any handle drag applies the matching increment; the
  first snapped step is relative to the starting value, so non-grid-aligned
  objects move by whole increments.
* With grid snapping on, positions land on grid lines in world space.
* Snapping applies equally to agent-driven handle operations through MCP, which
  state whether snapping is on in their inputs and results.

## View options

The View Options overlay, docked at the top-right by default, holds:

* **Draw mode** dropdown. Shading modes: Shaded, Wireframe, and Shaded
  Wireframe. Miscellaneous modes: Shadow Cascades, Render Paths, Alpha Channel,
  Overdraw, Mipmaps, Sprite Mask, and Texture Streaming. Global illumination and
  lighting debug modes: Systems, Clustering, Lit Clustering, UV Charts,
  Contributors / Receivers, Albedo, Emissive, Indirect, Directionality, Baked
  Lightmap, Shadowmask, and Light Overlap. Render pipelines add their own debug
  modes, and project code adds modes through
  `SceneView.AddCameraMode`. A mode the active pipeline cannot draw is listed
  disabled with the reason.
* **2D** toggle.
* **Lighting** toggle: on uses scene lights; off lights the view from the camera.
* **Audio** toggle: plays scene audio sources in edit mode.
* **Effects** dropdown: Skybox, Fog, Flares, Clouds, Always Refresh (animated
  materials), Post Processing, Particle Systems, and Visual Effect Graphs, each
  toggleable, with a master toggle.
* **Scene visibility** toggle: shows or ignores the per-object hidden state.
* **Camera settings** dropdown, as described in [Navigation](#navigation).
* **Gizmos** toggle and dropdown. The dropdown holds 3D Icons (with an icon size
  slider), Fade Gizmos, Selection Outline, Selection Wire, Light Probe
  Visualization options, and a list of every component type and script that
  draws gizmos or has an icon, each with an icon toggle and a gizmo toggle,
  grouped into Recently Changed, Scripts, and Built-in Components. Toggling a
  row changes drawing in every Scene view and, for gizmo toggles, in the Game
  view's gizmo display.

## Scene visibility and pickability

The Hierarchy's visibility and pickability columns (see
[Hierarchy](hierarchy.md#visibility-and-pickability-columns)) set two per-user
states for each GameObject. Neither changes the scene file or the object's
active state, and neither enters the scene's undo history; both are stored per
user per project.

* **Hidden** objects are not drawn in the Scene view while the Scene visibility
  toggle is on. They are still drawn in the Game view and in builds.
* **Isolation** (Shift+H, or the isolation button in the Hierarchy header) shows
  only the selection and its children; a bar at the top of the Scene view states
  isolation is on and offers Exit.
* **Non-pickable** objects are drawn normally but cannot be selected by clicking
  or rectangle selection in the Scene view; they can still be selected in the
  Hierarchy and edited in the Inspector.

H toggles the visibility of the selection and L toggles its pickability.

## Selection

### Click selection

* Clicking an object selects it and makes it the active object; clicking empty
  space clears the selection.
* Shift+click adds the clicked object to the selection; Ctrl/Cmd+click toggles
  it.
* Clicking an object inside a prefab instance selects the outermost prefab
  instance root first; clicking again at the same place selects the clicked
  object itself. Objects marked with `[SelectionBase]` on one of their
  components are selected before their children in the same way.
* Clicking again at the same screen position, without moving the mouse, cycles
  through the overlapping objects under the cursor, front to back, and wraps.
* Picking uses rendered geometry, including skinned, scaled, rotated, and
  transparent renderers, sprites, and gizmo icons. Objects that are inactive,
  hidden, or non-pickable are not picked.

### Rectangle selection

Left-dragging from empty space draws a selection rectangle. Releasing selects
every pickable object whose rendered bounds intersect the rectangle, replacing
the selection; Shift adds to it and Ctrl/Cmd removes from it. Prefab instance
roots and `[SelectionBase]` objects are selected in place of their children.

### Selection display

Selected objects draw a selection outline (the active object in a brighter tone
than the rest) and, with Selection Wire on, a wireframe. Children of a selected
object draw a fainter outline. Colors come from Preferences > Colors. The
selection outline is suppressed in Wireframe draw mode, which uses the wire
color instead.

## Overlays

Every overlay can be:

* **Docked** to any edge or corner of the viewport by dragging its handle, laid
  out horizontally on top and bottom edges and vertically on side edges; docked
  overlays in the same edge stack in drag order.
* **Floated** anywhere in the viewport.
* **Collapsed** to a single button from its context menu, and expanded again by
  clicking it. An overlay too large for its docked edge collapses automatically.
* **Hidden or shown** from the overlay menu.

Pressing the backtick key (`) with the viewport focused opens the overlay menu
at the cursor: a list of every overlay with a visibility toggle, a master toggle
for all overlays, and the Overlay Presets menu. Presets save and load the whole
overlay arrangement by name; presets are per user and available in every project.
The arrangement of each Scene view is restored with the workspace arrangement.

## Drag and drop into the Scene view

* Dragging a prefab, model, or sprite from the Project window into the viewport
  creates a preview instance that follows the cursor. Over geometry with a
  collider or renderer, the instance is placed on the surface under the cursor;
  over empty space it is placed at a fixed distance in front of the camera on
  the grid plane in 2D mode. Releasing creates the instance in the active scene
  under the default parent if one is set (see
  [Hierarchy](hierarchy.md#default-parent)); pressing Esc cancels.
* Dropping a material onto a renderer assigns it to the submesh under the
  cursor; the target highlights before release. With Alt held, it assigns to all
  submeshes.
* Dropping a script asset onto an object adds the component; dropping a script
  that is not a valid component states why and changes nothing.
* Dropping an audio clip, video clip, or texture creates the matching object or
  assigns to a matching field per the asset's kind.
* Each drop is one undo operation and is also available as a command with the
  target and position as inputs (EDT-006).

## Component handles and visualizations

Built-in components draw their own handles and gizmos when selected:

* **Colliders:** the collider shape in wire; the Edit Collider tool (in the
  Tools overlay and as a button in the component's Inspector header) shows
  face and size handles.
* **Lights:** range spheres, spot cones with range and angle handles, area light
  rectangle handles, and the light's icon.
* **Audio Sources and Reverb Zones:** min and max distance spheres with radius
  handles.
* **Cameras:** the frustum, with near and far plane edges and field of view.
* **Reflection Probes and Light Probe Groups:** bounds and probe handles with
  their edit tools.
* **Navigation, terrain, particles, and joints:** the tools and visualizations
  defined by their owning specifications ([Physics, navigation, and
  terrain](../physics-navigation-terrain.md) and [Rendering and
  animation](../rendering-and-animation.md)).

Handles and tools declared by project and package extensions appear alongside
these as described in
[Editor and authoring](../editor-and-authoring.md#gizmos-and-debug-drawing).

## Empty and error states

* **No scene loaded:** the viewport shows the grid and the default skybox, and
  the Hierarchy shows the untitled scene.
* **Render failure:** when the render pipeline or a shader fails to draw the
  view, the viewport keeps the last good frame, overlays a message naming the
  failing pipeline asset, shader, or feature, and links to the Console entry.
  Selection and handles keep working.
* **Shaders compiling:** objects whose shaders are compiling draw with the
  placeholder shader until compilation completes.
* **Extension failure:** an `OnSceneGUI`, `duringSceneGui`, tool, or overlay that
  throws is skipped for that frame with its error in the Console; the rest of
  the view keeps drawing, as defined by RUN-010 in
  [Scripting runtime](../scripting-runtime.md).
* **Worker restart:** when the worker that renders the view restarts, the
  viewport states that it is reconnecting and resumes with the same camera.

### Shortcuts

All bindings below are in the Scene View context unless noted, and are
rebindable in the Shortcuts Manager ([Shortcuts](shortcuts.md)).

| Binding | Command | Context |
|---|---|---|
| Q / W / E / R / T / Y | View, Move, Rotate, Scale, Rect, Transform tool | Global (Tools) |
| Z | Toggle Pivot / Center | Global (Tools) |
| X | Toggle Global / Local | Global (Tools) |
| F | Frame Selected | Scene View |
| Shift+F | Lock View to Selected | Scene View |
| 2 | Toggle 2D Mode | Scene View |
| ` (backtick) | Show Overlay Menu | Scene View |
| H | Toggle Visibility of Selection | Scene Visibility |
| Shift+H | Toggle Isolation | Scene Visibility |
| L | Toggle Pickability of Selection | Scene Picking |
| Ctrl/Cmd+[ | Decrease Grid Size | Grid |
| Ctrl/Cmd+] | Increase Grid Size | Grid |
| Ctrl/Cmd+\ | Push to Grid | Grid |
| V (hold) | Vertex Snapping | Scene View |
| W/A/S/D/Q/E (while right-dragging) | Fly forward, left, back, right, down, up | Scene View |
| Shift (while flying) | Fly faster | Scene View |
| Esc | Cancel handle drag or drop | Scene View |
| Ctrl/Cmd+Alt+F | Move To View | Global (GameObject menu) |
| Ctrl/Cmd+Shift+F | Align With View | Global (GameObject menu) |

Modifiers held during gestures (Ctrl/Cmd increment snap, Ctrl/Cmd+Shift surface
snap, Shift add to selection, Ctrl/Cmd toggle selection, Alt with mouse buttons
for orbit, zoom, and pan) are fixed gesture modifiers and are listed in the
Shortcuts Manager as read-only.

### Agent presence

* An object that an active agent session is changing draws an outline in that
  session's assigned color with a small label carrying the session's short
  label, placed above the object's bounds. The label never covers handles; when
  it would, it moves aside. Hovering the label shows the session name and its
  current operation; clicking it opens the Change Review window filtered to the
  object ([Agent-native additions](agent-native-additions.md)).
* Agent edits to positions, rotations, and scales appear in the viewport as
  they are applied; an object an agent moves animates to its new value only if
  the agent's operation itself is incremental.
* Under the require-approval policy, proposed changes are not drawn unless the
  developer turns on Preview Proposals in the View Options overlay's agent
  section (appended after the familiar controls). Proposed objects and
  positions then draw with a dashed outline and translucent fill in the session
  color and cannot be selected for editing; selecting one opens it in the Change
  Review window.
* Agent indicators never move, rename, or rebind a familiar overlay, tool, or
  shortcut. They are hidden with all overlays when overlays are hidden.
* An agent's handle operation through MCP (see
  [Editor and authoring](../editor-and-authoring.md#extension-controls-through-mcp))
  shows the handle in its dragged state for the duration of the operation.

### Scale

* Drawing cost follows what is visible: culling, gizmo collection, and icon
  drawing consider only objects in the view frustum, and objects that neither
  change nor are visible add no per-frame cost. A scene with 100,000 GameObjects
  navigates, selects, and frames within the release's declared budgets (the
  [scale floor](../diagnostics-profiling-playtesting.md#scale-floor)).
* Picking uses a spatial index maintained incrementally from command records,
  so a click costs time proportional to candidates under the cursor, not scene
  size.
* Rectangle selection of tens of thousands of objects completes as one
  selection change; the Inspector then shows the multi-object state as defined
  in [Inspector](inspector.md).
* The Gizmos dropdown lists component types, not objects, and virtualizes its
  list.
* A drag on a large multi-selection updates the moved objects each frame and
  forms one undo operation regardless of count.

## Acceptance outcomes

1. With a cube selected, pressing F frames it; right-dragging with W held moves
   the camera forward, and holding Shift moves faster; scroll during the flight
   changes the speed and shows it; none of this enters undo history.
   (EDT-003, EDT-005)
2. Clicking the scene gizmo's Y cone shows a Top orthographic view labeled Top;
   clicking the center cube toggles Persp and Iso; Shift+clicking it returns to
   the default perspective view. (EDT-003)
3. Pressing W, E, R, T, and Y activates the Move, Rotate, Scale, Rect, and
   Transform tools in that order, and the Tools overlay highlights each; Z and
   X toggle Pivot/Center and Global/Local and the handle moves and reorients
   accordingly. (EDT-003, EDT-006)
4. Dragging the Move tool's XZ plane square changes only X and Z; dragging with
   Ctrl/Cmd held changes position in whole move increments; the drag is one
   undo operation and Esc during a second drag restores the prior values.
   (EDT-003, EDT-005)
5. Holding V and dragging a cube's corner onto another mesh's vertex makes the
   two vertices coincide in world space; Ctrl/Cmd+Shift dragging the center
   square places the object on a collider surface under the cursor. (EDT-003)
6. With grid snapping on and a grid size of one unit, moving an object leaves
   its position on integer coordinates; Ctrl/Cmd+\ pushes an off-grid object
   onto the grid. (EDT-003)
7. Clicking a child of a prefab instance selects the instance root; clicking
   again at the same place selects the child; a further click at the same
   place selects the next overlapping object behind it. (EDT-001, EDT-003)
8. Rectangle-dragging over three objects selects them; Shift-dragging over a
   fourth adds it; Ctrl/Cmd-dragging over one removes it. (EDT-001, EDT-003)
9. An object made non-pickable with L is not selected by clicking it in the
   Scene view but is selected from the Hierarchy; an object hidden with H is not
   drawn in the Scene view, is drawn in the Game view, and the scene file is
   unchanged. (EDT-001, EDT-003)
10. Pressing backtick opens the overlay menu; hiding the Grid and Snap overlay
    and saving an overlay preset, then loading the default preset and the saved
    one, restores each arrangement. (EDT-003, EDT-006)
11. Dragging a prefab from the Project window over a plane places its preview
    on the plane's surface; releasing creates one instance under the default
    parent as one undo operation; Esc during the drag creates nothing. (EDT-003,
    EDT-006)
12. Switching the draw mode to Wireframe and toggling Fog off in Effects changes
    the Scene view and leaves the Game view and project content unchanged.
    (EDT-003)
13. Turning off a script's gizmo in the Gizmos dropdown stops its
    `OnDrawGizmos` drawing in every Scene view while other gizmos continue.
    (EDT-003)
14. An agent moving an object through MCP produces the session-colored outline
    and label on that object while it changes; clicking the label opens the
    Change Review window filtered to that object. (EDT-003, EDT-008)
15. In a scene with 100,000 GameObjects, clicking, rectangle selection,
    framing, and a Move drag of the whole selection complete within the
    release's declared budgets. (EDT-003, PRF-002)
