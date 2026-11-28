# Lighting and Rendering Tools

## Scope

This document details the visual editor's lighting and rendering windows: the
Lighting window, the Lighting Settings asset's presentation, Light Explorer, the
Occlusion Culling window, the Scene view tie-ins for light probes and
reflection probes, the Frame Debugger, the render pipeline and the Graphics
and Quality settings pages, and the Shader Graph window. It details the
workspace, command, and multi-editing behavior of EDT-006, the Scene view
handles and visualization of EDT-003, and the inspector editing of EDT-002,
all owned by [Editor and authoring](../editor-and-authoring.md). This document
owns no catalog ID.

What a bake computes, what data it writes, staleness, and how baked data
renders are owned by REN-003; render pipelines, shader graphs, and scripted
rendering by REN-006, both in [Rendering and animation](../rendering-and-animation.md).
Rendering diagnostics and structured frame capture are owned there and by
DIA-001 and PRF-001 in
[Diagnostics, profiling, and playtesting](../diagnostics-profiling-playtesting.md).
The storage of the Graphics and Quality settings is owned by PRJ-007 in
[Projects and assets](../project-and-assets.md). This document describes the
windows through which a person and an agent reach that behavior.

## Lighting window

The Lighting window opens from `Window/Rendering/Lighting`. It opens as a
floating window the first time and thereafter where the workspace arrangement
last placed it ([Workspace and layout](workspace-and-layout.md)). It has a tab
strip across its top with four tabs: Scene, Environment, Realtime Lightmaps,
and Baked Lightmaps. A bottom bar is always visible below the tab content.

### Scene tab

The Scene tab presents the lighting configuration of the active scene.

* **Lighting Settings field.** An object field at the top holds the scene's
  Lighting Settings asset. It accepts a dragged Lighting Settings asset from
  the Project window, the object picker lists every Lighting Settings asset
  in the project, and a **New Lighting Settings** button creates an asset
  beside the scene, assigns it, and enters rename on it in the Project window.
  With no asset assigned, the tab shows the scene's default settings read-only
  and states that they are defaults, with the New button enabled.
* **Lighting Settings content.** Below the field, the asset's properties draw
  in foldout sections: Realtime Lighting (Realtime Global Illumination),
  Mixed Lighting (Baked Global Illumination, Lighting Mode with Baked Indirect,
  Shadowmask, and Subtractive), and Lightmapping Settings (lightmapper,
  importance sampling, direct, indirect, and environment sample counts, light
  probe sample multiplier, maximum bounces, filtering, lightmap resolution,
  padding, maximum lightmap size, lightmap compression, ambient occlusion,
  directional mode, albedo boost, indirect intensity, lightmap parameters).
  A property hidden by another's value (for example the filtering details
  while filtering is off) is hidden, not disabled. Edits are edits to the
  asset: they dirty the asset, not the scene, follow the inspector rules of
  [Inspector](inspector.md), and form one undo operation per gesture
  (EDT-005).
* **Shared asset notice.** When the assigned asset is used by more than one
  scene, the section header states how many scenes share it, and the count
  opens a list of those scenes.
* **Workflow section.** The lightmapper and the bake backend choice (CPU or
  GPU) appear here; the GPU choice names the device it will use and states
  when no usable device is present, in which case the bake runs on the CPU.

### Environment tab

The Environment tab presents the active scene's environment: the skybox
material, the sun source light, the realtime shadow color, environment
lighting (source Skybox, Gradient, or Color, intensity multiplier, ambient
mode), environment reflections (source, resolution, compression, intensity
multiplier, bounces), and, when the render pipeline declares them, its own
environment settings. Fog and other scene render settings appear under
Other Settings. Edits dirty the scene, since these values are stored with the
scene.

### Realtime Lightmaps and Baked Lightmaps tabs

Each tab lists the lightmaps of its kind for the active scene as a grid of
thumbnails with each lightmap's index, size, and format. Clicking a
thumbnail selects the lightmap texture; double-clicking opens a preview with
channel selection and zoom. Selecting a lightmap and choosing **Select
Renderers** in its context menu selects every renderer that uses it in the
Hierarchy. The Baked Lightmaps tab also shows the lighting data asset field
and the total memory of baked data. A scene with no baked data shows an empty
state that states there is no baked lighting and offers Generate Lighting.

### Bottom bar

The bottom bar holds:

* the **Auto Generate** toggle, which rebakes stale data in the background as
  REN-003 defines;
* the **Generate Lighting** button with a drop-down offering **Bake Reflection
  Probes** and **Clear Baked Data**;
* while a bake runs, its stage, fraction complete, elapsed time, and estimated
  time remaining, with a **Cancel** button replacing Generate Lighting;
* the stale state: when the active scene's baked data is stale, the bar names
  it and the number of affected renderers, and the number opens a list that
  selects them;
* the occupied memory of the active scene's lightmaps and probes.

The same running bake appears in the status bar's progress indicator and in
the Progress window ([Profiler and debugging windows](profiler-and-debugging-windows.md)).
Generate Lighting with more than one scene loaded bakes every loaded scene
together, as one bake. A failed or cancelled bake shows its cause in the bar
and in the [Console](console.md), names the affected objects, and leaves the
previous data unchanged, as REN-003 defines.

### Lightmap visualization in the Scene view

The Scene view's draw mode menu lists the global-illumination modes (for
example Baked Lightmap, Directionality, Shadowmask, Albedo, Emission, UV
Charts, Lightmap Indices, Light Overlap, Contributors and Receivers, Texel
Validity, Lightmap Resolution) as defined in [Scene view](scene-view.md).
Selecting a renderer with a lightmap shows its lightmap preview, index,
scale-offset, and chart count in the renderer's Lightmapping section of the
[Inspector](inspector.md).

## Light Explorer

Light Explorer opens from `Window/Rendering/Light Explorer`. It is a table
window with a tab strip of object kinds: Lights, 2D Lights when the project
uses them, Reflection Probes, Light Probes, Static Emissives, and any kinds
that the render pipeline declares.

* **Columns.** Each tab's columns are the frequently edited properties of
  that kind; for Lights: enabled, name, type, mode, color, intensity, indirect
  multiplier, shadow type. Values edit in place with the same controls as the
  Inspector, and a column header click sorts by that column.
* **Rows.** One row per object in the loaded scenes. Clicking a row selects
  the object; Shift and Ctrl/Cmd click extend the selection exactly as in the
  [Hierarchy](hierarchy.md). Editing a cell while several rows are selected
  applies the value to every selected row as one undo operation, and a
  column whose selected values differ shows the mixed-value state.
* **Header bar.** A search field filters rows by name; a **Lock Selection**
  toggle keeps the list to the objects selected when it was turned on.
* **Empty state.** A tab with no objects of its kind states that the loaded
  scenes contain none.

## Occlusion Culling window

The Occlusion Culling window opens from `Window/Rendering/Occlusion Culling`.
It has three tabs: Object, Bake, and Visualization.

* **Object.** With a renderer selected, it shows the Occluder Static and
  Occludee Static flags; with an Occlusion Area selected, its Is View Volume
  setting; a filter row (All, Renderers, Occlusion Areas) switches what the
  Hierarchy search shows, and a **Create New Occlusion Area** button adds one.
* **Bake.** Smallest Occluder, Smallest Hole, and Backface Threshold, a
  **Set default parameters** button, **Clear**, and **Bake**. A running bake
  shows its progress and **Cancel** in the window's bottom bar, the status
  bar, and the Progress window.
* **Visualization.** While this tab is visible, the Scene view shows an
  Occlusion Culling overlay with the camera used for visualization, the
  visualization mode (Edit or Visualize), and toggles for view volumes,
  camera volumes, visibility lines, portals, and geometry culling. Closing
  the tab ends the visualization.

Bake parameters and results are as REN-003 defines. The window states when
the active scene has no occlusion data, when its data is stale, and when it
does not match the scene.

## Probe editing in the Scene view

* **Light Probe Group.** Selecting a Light Probe Group shows its component
  with an **Edit Light Probe Positions** button (an editor tool, listed in the
  Scene view Tools overlay while the component is selected). While editing,
  probes draw as spheres; clicking selects a probe, Shift and Ctrl/Cmd click
  extend the selection, rectangle drag selects, and the move handle moves
  the selected probes. The component offers **Add Probe**, **Select All**,
  **Delete Selected**, and **Duplicate Selected**, and Ctrl/Cmd+D, Delete, and
  Ctrl/Cmd+A act on probes rather than GameObjects while editing. Esc or the
  button ends editing. Tetrahedralization lines between baked probes draw
  while the group is selected.
* **Reflection Probe.** Selecting a Reflection Probe shows two editor tools,
  **Edit Bounding Volume** (box handles for size and center) and **Edit
  Probe Origin** (move handle for the capture point that keeps the volume in
  place). The component shows its type (Baked, Realtime, Custom), refresh
  and time-slicing modes, importance, intensity, box projection, blend
  distance, and a **Bake** button for that probe. The captured cubemap
  previews as a sphere at the probe's origin in the Scene view and in the
  component's preview.

Probe edits are scene edits that form one undo operation per gesture and mark
baked data stale as REN-003 defines.

## Frame Debugger

The Frame Debugger opens from `Window/Analysis/Frame Debugger`.

### Regions and controls

* **Toolbar.** An **Enable** toggle, a target drop-down (Editor, or a
  connected development player by name), and, while enabled, an event
  slider with previous and next buttons and the current event index of the
  total.
* **Event tree.** On the left, the frame's rendering events as a tree:
  cameras, render passes and renderer features, command buffers, and draws,
  clears, dispatches, blits, and resolves, with their names and, for draw
  batches, the number of draws. Selecting an event replays the frame up to
  and including it. Up and Down move between events; Left and Right
  collapse and expand.
* **Details pane.** On the right, the selected event's details: the render
  target (with a channel selector, levels, and the target's resolution and
  format; multiple render targets are selectable), the shader and pass with
  keywords, the blend, depth, stencil, and cull state, the mesh and the draw's
  vertex and index counts, the reason the event was not batched with its
  predecessor, and the shader properties (textures with thumbnails, floats,
  vectors, matrices, buffers) bound for it. Clicking a texture, mesh, shader,
  or material in the details pings it in the Project window.

### Behavior

* Enabling the Frame Debugger in play mode pauses the play session; in edit
  mode it captures the Game view's frame, or the Scene view's when the Scene
  view has focus. Disabling it resumes the prior state.
* The Game view, or the Scene view, shows the frame rendered up to the
  selected event, so stepping through events builds up the frame.
* Against a development player, the player must have been built as a
  development build; the drop-down states for each listed player whether it
  can be debugged.
* The captured frame is the frame defined by the rendering diagnostics of
  REN-006 and the frame capture of DIA-001; the window presents it. An agent
  reads the same event tree and event details as structured data.
* Empty state: while not enabled, the window states that frame debugging is
  off and how to enable it. A target that refuses the connection states why.

## Render pipeline, Graphics, and Quality pages

These are pages of the Project Settings window
([Settings and preferences](settings-and-preferences.md)); their window-level
behavior is defined there.

* **Graphics.** The Scriptable Render Pipeline Settings field holds the
  default render pipeline asset (empty for the built-in pipeline). Changing it
  states which materials and shaders the new pipeline does not support,
  before the change completes. The page also holds tier settings, built-in
  shader settings, always-included shaders, shader stripping, shader loading
  and preloading, and, for packages that declare them, per-pipeline global
  settings sections.
* **Quality.** A matrix lists quality levels as rows and build target
  groups as columns, with a checkbox per cell for whether the level is
  available on that target and a marker on each target's default level.
  Clicking a level's name selects it and shows its settings below; the
  default marker opens a drop-down to choose the target's default. **Add
  Quality Level** appends a level, and each level has a delete button. Each
  level's settings include its render pipeline asset override, which, when
  set, takes precedence over the Graphics page's default for that level.
* **Render pipeline assets.** A render pipeline asset and its renderer assets
  draw in the Inspector with the pipeline's own editors (RUN-010), and the
  Inspector header of the asset states which quality levels and which
  Graphics default use it.
* **Pending effect.** A change whose effect begins at a later boundary states
  that boundary, as PRJ-007 defines.

## Shader Graph window

When the project includes the Shader Graph package, double-clicking a Shader
Graph or Sub Graph asset opens the graph window of unitas's implementation of
the package ([Independent
implementation](../architecture.md#independent-implementation)), with the
workspace behavior of any docked window; its graph semantics and generated
shaders are owned by REN-006. Without the package, such assets show in the
Project window with their import state and an Inspector that names the missing
package. Visual Effect Graph assets follow the same rule with their package.

### Shortcuts

Window-scoped shortcuts appear under these Shortcuts Manager contexts and are
rebindable there ([Shortcuts](shortcuts.md)):

| Context | Action | Default |
|---|---|---|
| Frame Debugger | Previous event | Up |
| Frame Debugger | Next event | Down |
| Frame Debugger | Collapse or expand event | Left, Right |
| Light Probe Group editing | Duplicate selected probes | Ctrl/Cmd+D |
| Light Probe Group editing | Delete selected probes | Delete (Cmd+Backspace on macOS) |
| Light Probe Group editing | Select all probes | Ctrl/Cmd+A |
| Light Probe Group editing | End editing | Esc |

The Lighting, Light Explorer, Occlusion Culling, and Frame Debugger windows
have no global default shortcut.

### Agent presence

* A bake started by an agent session shows that session's agent badge beside
  its progress in the Lighting window's bottom bar, the status bar, and the
  Progress window, and the hover names the agent session and the bake
  request. Cancel acts on it as on any bake; an agent session cancels only
  its own bakes unless it holds the review scope (AUT-007).
* A Lighting Settings asset, Light Explorer row, probe, or quality level that
  an agent session is changing carries the agent badge in its row or section
  header; the hover names the agent session and its current operation, and a
  click opens the Change Review window filtered to that item
  ([Agent-native additions](agent-native-additions.md)).
* Changes to the Graphics and Quality pages and to render pipeline assets
  referenced from them are project settings changes, risk-flagged in the
  change set (EDT-008).
* Under the require-approval policy, a proposed lighting change previews in
  the Lighting window and the Scene view; a bake that an agent session runs
  against its proposal runs in its isolated proposal environment and writes
  no baked data into the project until approval.
* Agent presence adds badges only; it never moves, hides, or rebinds a control
  of these windows.

### Scale

* Light Explorer realizes only visible rows; sorting and filtering use the
  editor's indexes and stay responsive in scenes at the
  [scale floor](../diagnostics-profiling-playtesting.md#scale-floor).
* The lightmap tabs realize only visible thumbnails and load full previews on
  demand.
* The Frame Debugger's event tree realizes only visible rows, so frames with
  very many events stay navigable; replaying to an event never blocks the
  editor's frame, and the details pane shows a loading state until the
  replay completes.
* Bakes run off the main thread; editing continues during a bake.

## Acceptance outcomes

1. Choosing **New Lighting Settings** in the Lighting window's Scene tab
   creates a Lighting Settings asset, assigns it to the active scene, and the
   asset's edits dirty the asset and not the scene; an agent performs the same
   through the `Window/Rendering/Lighting` window's controls over MCP with the
   same result. (EDT-006, EDT-002, REN-003)
2. Pressing Generate Lighting with two scenes loaded bakes both as one bake,
   shows its stage and progress in the Lighting window, status bar, and
   Progress window, and Cancel leaves the previous baked data unchanged.
   (EDT-006, REN-003)
3. Moving a contributing static renderer after a bake shows the stale state in
   the bottom bar with the affected renderer count, and the count selects
   those renderers in the Hierarchy. (EDT-001, EDT-006, REN-003)
4. In Light Explorer, selecting three lights and setting the intensity cell
   sets all three as one undo operation, and a column of differing values
   shows the mixed-value state. (EDT-002, EDT-005, EDT-006)
5. In Light Probe Group editing, Ctrl/Cmd+D duplicates the selected probes,
   not the GameObject, and Esc ends editing; each edit is one undo operation.
   (EDT-003, EDT-005)
6. Dragging a reflection probe's bounding-volume handle changes its size and
   the Inspector shows the new values during the drag. (EDT-002, EDT-003)
7. Enabling the Frame Debugger in play mode pauses the play session, the event
   tree lists the frame's draws, selecting an event shows its render target,
   shader, pass, and state, and the Game view shows the frame up to that event;
   an agent reads the same events as structured data. (EDT-004, EDT-006,
   REN-006, DIA-001)
8. Setting a render pipeline asset on the Graphics page states the materials
   it does not support before the change completes, and a quality level's
   override takes precedence for that level. (EDT-006, REN-006, PRJ-007)
9. In a project without the Shader Graph package, a Shader Graph asset's
   Inspector names the missing package; with the package, double-clicking the
   asset opens the package's window docked in the workspace. (EDT-006,
   REN-006)
10. A bake started by an agent session shows that session's badge beside its
    progress, and a quality-level change by an agent session appears in its
    change set as a risk-flagged project settings item. (EDT-008)
11. In a scene at the scale floor, Light Explorer sorts and filters its Lights
    tab without the editor frame waiting on the operation. (EDT-006, PRF-002)
