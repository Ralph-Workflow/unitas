# 2D Tools

## Scope

This document details the editor's 2D authoring tools: the Sprite Editor
window, the Tile Palette window, the Sprite Atlas inspector and packing, and
the Scene view's 2D mode and sprite workflows. It details EDT-002 (inspector
and serialized property editing), EDT-003 (2D scene navigation, handles,
snapping, and visualization), EDT-005 (undo), and EDT-006 (workspace views,
commands, shortcuts, and drag and drop), which are owned by
[Editor and authoring](../editor-and-authoring.md). This document owns no
catalog ID.

Sprites, sprite atlases, sorting, masks, 2D lights, and tilemaps are owned by
REN-005, and sprite animation clips by ANI-001, in
[Rendering and animation](../rendering-and-animation.md). Sprite import
settings are owned by [Projects and assets](../project-and-assets.md)
(PRJ-004). Where this document states what a value means at runtime, the
owning specification governs.

The Sprite Editor window belongs to the 2D Sprite package
(`com.unity.2d.sprite`) and the Tile Palette window to the 2D Tilemap Editor
package (`com.unity.2d.tilemap`). Each is the package's own editor window,
drawn and run as an editor extension
([Custom editor extensions](../editor-and-authoring.md#custom-editor-extensions),
RUN-010 in [Scripting runtime](../scripting-runtime.md)), and appears at its
menu path only when the project includes its package. The Sprite Atlas
inspector, packing, and preview are built in.

Every window here docks, groups as tabs, floats, and maximizes as defined in
[Workspace and layout](workspace-and-layout.md). Every menu item, context menu
item, and toolbar action is a command and an MCP tool titled by its path, as
defined in [Menus and toolbar](menus-and-toolbar.md) and
[Agent-native additions](agent-native-additions.md).

## 2D mode and sprite workflows in the Scene view

* The 2D toggle in the Scene view's view options, and the `2` key while the
  Scene view has focus, switch the view to 2D mode: an orthographic view
  facing the XY plane with rotation disabled, the scene gizmo hidden, the grid
  on the XY plane, right-drag and middle-drag panning, and scroll zooming
  about the pointer. Switching back restores the previous 3D camera
  ([Scene view](scene-view.md)).
* The project's default behavior mode (2D or 3D), set in the Editor page of
  Project Settings ([Settings and preferences](settings-and-preferences.md)),
  makes new scenes open their Scene view in 2D mode and makes newly imported
  images default to the sprite texture type.
* Dragging one sprite from the Project window into the Scene view or Hierarchy
  creates a GameObject with a Sprite Renderer showing that sprite at the drop
  point. Dragging several sprites at once creates one GameObject and asks for
  a save location for a new animation clip whose frames are those sprites in
  name order, with a controller for the object, as one undo operation.
* The Rect tool (T) moves, resizes, and rotates sprites about their pivot with
  the handles defined in [Scene view](scene-view.md); resizing a sprite whose
  draw mode is Sliced or Tiled changes its size, not its scale.
* Sorting Layer and Order in Layer appear on every renderer that sorts in 2D,
  and the Sorting Layer dropdown ends with Add Sorting Layer..., which opens
  the Tags and Layers settings page.
* Sprite physics shapes and outlines authored in the Sprite Editor are used by
  colliders and meshes that request them; the Scene view draws collider
  gizmos as defined by PHY-002 in
  [Physics, navigation, and terrain](../physics-navigation-terrain.md).

## Sprite Editor window

Opened from Window > 2D > Sprite Editor, or from the Sprite Editor button in
the Inspector of a texture whose texture type is Sprite. It edits the sprite
metadata of one texture; nothing is written until Apply.

### Regions

* **Module dropdown** at the left of the toolbar: Sprite Editor, Custom
  Outline, Custom Physics Shape, Secondary Textures, and, when the 2D
  Animation package is present, Skinning Editor. Modules that project code
  and packages declare appear in the same list.
* **Toolbar.** For the Sprite Editor module: Slice, Trim, Revert, Apply. On
  the right: color and alpha channel toggle, zoom slider, and mip level
  slider.
* **Canvas.** The texture at the chosen zoom with sprite rectangles drawn
  over it; pan by middle-drag or Alt-drag, zoom by scroll about the pointer.
* **Sprite panel** at the lower right for the selected sprite: Name,
  Position (X, Y, W, H), Border (L, T, R, B), Pivot (Center, the eight edge
  and corner positions, Custom), Pivot Unit Mode (Normalized or Pixels), and
  Custom Pivot.

### Sprite Editor module

* For a texture whose sprite mode is Single, the canvas shows one rectangle
  covering the texture; for Multiple, every sprite rectangle.
* Dragging in empty canvas space creates a rectangle. Clicking selects a
  rectangle; dragging inside moves it; dragging an edge or corner resizes it;
  dragging the border handles sets the nine-slice borders; dragging the pivot
  circle moves the pivot, snapping to edges and center while Ctrl/Cmd is
  held. Delete removes the selected rectangle.
* Slice opens a popup with Type (Automatic, Grid By Cell Size, Grid By Cell
  Count, Isometric Grid) and the fields that type uses: minimum size for
  Automatic; pixel size or column and row count, offset, and padding for the
  grids; Keep Empty Rects; pivot; and Method (Delete Existing, Smart, Safe),
  which chooses whether existing rectangles are replaced, matched, or kept.
  Smart and Safe keep the names and identities of existing sprites so that
  references to them survive reslicing (PRJ-003 in
  [Serialization and prefabs](../serialization-and-prefabs.md)).
* Trim shrinks the selected rectangle to its non-transparent pixels.
* Revert discards unapplied changes. Apply writes the sprite metadata to the
  texture's import settings and reimports it, as one undo operation.
* Closing the window, selecting another texture, or entering play mode with
  unapplied changes asks to Apply or Revert, with Cancel where the action can
  be cancelled.

### Outline and physics shape modules

* Custom Outline and Custom Physics Shape show the selected sprite's shapes as
  editable polygons. Generate creates shapes from the sprite's alpha with an
  outline tolerance slider; Snap snaps vertices to pixels.
* Clicking an edge adds a vertex; dragging moves vertices; Delete removes the
  selected vertices; dragging in empty space selects vertices by rectangle.
  Several shapes per sprite are supported; Copy and Paste transfer shapes
  between sprites.

### Secondary Textures module

* Lists named secondary textures (for example a normal map or a mask) with an
  object field for each; dragging a texture from the Project window onto the
  list adds an entry. Names suggest the conventional names that 2D lighting
  shaders read (REN-005).

### Package not installed

When the project does not include the 2D Sprite package, Window > 2D > Sprite
Editor is absent, and the texture Inspector's Sprite Editor button states that
the window is provided by that package and opens the Package Manager at it.
Sprite metadata already in the texture's import settings is used and
preserved unchanged.

## Tile Palette window

Opened from Window > 2D > Tile Palette when the project includes the 2D
Tilemap Editor package.

### Regions

* **Tool strip.** Select, Move, Paint, Box Fill, Pick, Erase, and Flood Fill,
  plus tools that the package and project brushes add.
* **Active Tilemap dropdown.** Lists the Tilemaps in the open scenes and
  follows the Hierarchy selection when a Tilemap or its Grid is selected.
  Painting targets the active Tilemap.
* **Palette dropdown.** Lists palette assets in the project and ends with
  Create New Palette, which asks for a name, grid layout (Rectangle,
  Hexagon, Isometric, Isometric Z As Y), cell size (automatic or manual), and
  a save folder.
* **Palette area.** The palette's tiles on its grid, with an Edit toggle that
  enables editing the palette itself, and Grid and Gizmos toggles. Pan by
  middle-drag or Alt-drag; zoom by scroll.
* **Brush panel.** The brush dropdown (the default brush plus brushes the
  package and project declare) and the selected brush's settings.

### Painting

* Clicking a tile in the palette picks it; dragging a rectangle in the
  palette picks a multi-cell brush. The Scene view shows the brush preview at
  the cell under the pointer on the active Tilemap's grid.
* Paint draws on click and drag, one undo operation per stroke. Box Fill fills
  the dragged rectangle; Flood Fill fills the contiguous region of the same
  tile; Erase removes tiles; Pick picks from the Tilemap in the Scene view;
  Select selects a cell region whose contents show in the Inspector; Move
  moves a selected region.
* The rotate and flip commands rotate the active brush a quarter turn in each
  direction and flip it along X and along Y.
* Painting onto a Tilemap in another scene, or onto a Tilemap in a prefab
  open in Prefab Mode, edits that scene or prefab and marks it unsaved.
* Painting with no active Tilemap states that one is needed and offers to
  create a Tilemap with its Grid.

### Creating tiles

* Dragging sprites, textures sliced into sprites, or folders of them from the
  Project window onto the palette area asks for a folder, creates one tile
  asset per sprite there, and places the tiles in the palette, as one undo
  operation. Dragging existing tile assets places them without creating new
  assets.
* With Edit enabled, the palette's own cells are painted, erased, and moved
  with the same tools, editing the palette asset.

### Built-in services the package relies on

The Tile Palette runs on editor services that behave here as elsewhere: Scene
view input and grid drawing, editor tools and overlays (EDT-003), the
Inspector, undo, asset creation in the Project window, prefab editing, and the
package's declared shortcuts, which appear in the Shortcuts Manager under its
context exactly as the package declares them. Tile rule resolution and
rendering follow REN-005.

### Package not installed

When the project does not include the 2D Tilemap Editor package, Window > 2D >
Tile Palette is absent; Grid, Tilemap, and their placed tiles still render and
are preserved, and tile assets of a missing script type are reported as
REN-005 states.

## Sprite Atlas

### Creation

Assets > Create > 2D > Sprite Atlas creates an atlas asset that enters rename
immediately ([Project window](project-window.md)).

### Inspector

* **Type:** Master or Variant; a Variant names its master atlas and a scale.
* **Include in Build** (REN-005 late binding when off), **Allow Rotation**,
  **Tight Packing**, **Alpha Dilation**, and **Padding**.
* **Texture settings:** Read/Write, Generate Mip Maps, sRGB, and Filter Mode.
* **Platform tabs:** Default plus one tab per build target, each with
  maximum texture size, format, and compression; a target tab overrides the
  default only when its override toggle is on.
* **Objects for Packing:** a reorderable list of sprites, textures, and
  folders. Dragging any of them from the Project window onto the list adds
  them; a folder packs every sprite it contains at pack time.
* **Pack Preview** packs the atlas with the current settings and shows the
  packed pages in the preview area, with a page dropdown when packing
  produces several pages.

### Packing behavior

* The sprite packer mode in the Editor page of Project Settings decides
  whether atlases pack only for builds or also in the editor and play mode.
* A sprite that belongs to several atlases is reported as ambiguous in each
  atlas's Inspector and in the Console (REN-005).
* Packing runs off the editor's main thread and reports progress in the
  status bar; editing continues while it runs.

## Shortcuts

Scene view (context Scene View): `2` toggles 2D mode.

Tile Palette (context Grid Painting, as the package declares): S Select, M
Move, B Paint, U Box Fill, I Pick, D Erase, G Flood Fill; `[` and `]` rotate
the brush; Shift+`[` and Shift+`]` flip it. These bindings apply while the
Scene view or Tile Palette has focus and a painting tool is relevant, and are
rebindable in the Shortcuts Manager ([Shortcuts](shortcuts.md)).

Sprite Editor: Delete removes the selected rectangle or vertices; Escape
deselects.

## Agent presence

* Sprites, sprite rectangles, tiles, palettes, Tilemaps, and atlases that an
  agent session is changing carry the agent badge (session color and label).
  Hovering shows the session and its current operation; clicking opens the
  Change Review window filtered to that item
  ([Agent-native additions](agent-native-additions.md)).
* Cells an agent paints appear on the Tilemap as they are written, with the
  agent's outline on the changed cell region while the operation runs.
* Unapplied Sprite Editor changes belong to the person editing; an agent edit
  to the same texture's sprite metadata reports a conflict instead of
  replacing them (AUT-003 in [Automation and MCP](../automation-and-mcp.md)).
* Context menus gain only a trailing Review Agent Changes group when the
  target has agent changes. No familiar control is moved, renamed, or rebound.

## Agent operation

* Slicing, rectangle edits, pivots, borders, outlines, physics shapes, and
  secondary textures are commands with structured inputs, and their results
  name every created, changed, and removed sprite with its identity.
* Tile painting is available as commands on cell positions and regions
  (paint, box fill, flood fill, erase, move), each with the brush, the target
  Tilemap, and the affected cells in its result.
* Atlas packing preview is a command whose result lists pages, packed
  sprites, and ambiguous memberships, and whose page images are capturable as
  visual evidence (AUT-002).
* Package windows' controls are exposed as extension elements
  ([Extension controls through MCP](../editor-and-authoring.md#extension-controls-through-mcp)).

## Scale

* The Sprite Editor draws only rectangles within the visible canvas region
  and keeps selection and dragging cost independent of the number of sprites
  in the texture.
* Painting cost follows the cells a stroke changes, not the Tilemap's extent.
  Tilemaps render and pick by chunk.
* The palette area realizes only visible cells; palette and brush lists are
  virtualized.
* Atlas packing of large packable sets runs as an asynchronous operation with
  progress and cancellation.

## Acceptance outcomes

1. Pressing `2` in the Scene view switches to 2D mode facing the XY plane
   with rotation disabled, and pressing it again restores the prior 3D view.
   (EDT-003)
2. Dragging one sprite into the Scene view creates a Sprite Renderer object
   at the drop point; dragging four sprites creates one object, a four-frame
   clip, and a controller, and one undo removes all of them. (EDT-005,
   EDT-006, REN-005, ANI-001)
3. Slicing a sprite sheet by Grid By Cell Count and pressing Apply creates the
   stated number of sprites; reslicing with Method Smart keeps the identities
   of matching sprites, so a Sprite Renderer referencing one still resolves.
   (EDT-002, REN-005)
4. Closing the Sprite Editor with unapplied changes asks to Apply or Revert,
   and Revert leaves the texture's import settings unchanged. (EDT-005)
5. Generating a custom physics shape and applying it makes a Polygon Collider
   2D added afterwards use that shape. (EDT-002, REN-005)
6. With the 2D Tilemap Editor package present, picking a tile and dragging a
   Paint stroke across the Scene view paints those cells on the active
   Tilemap as one undo operation, and the cells read through MCP match.
   (EDT-003, EDT-005, REN-005)
7. Dragging a folder of sprites onto a palette creates one tile asset per
   sprite in the chosen folder and places them in the palette. (EDT-006,
   REN-005)
8. Pressing `]` with the Paint tool rotates the brush preview a quarter turn,
   and the painted tile's transform matrix records the rotation. (EDT-006,
   REN-005)
9. Without the 2D Sprite package, the texture Inspector's Sprite Editor
   button states the providing package and opens the Package Manager at it,
   and existing sprite metadata is preserved on save. (EDT-006)
10. Adding a folder to a Sprite Atlas's Objects for Packing and pressing Pack
    Preview shows the packed pages, and a sprite also packed by a second
    atlas is reported as ambiguous. (EDT-002, REN-005)
11. An agent paints a region through MCP while the Tilemap is visible; the
    cells appear as they are written, the Tilemap carries the agent badge,
    and reverting the item in Change Review restores the previous cells.
    (EDT-008, REN-005)
12. Painting with no Tilemap in the open scenes states that one is needed and
    offers to create one; accepting creates a Grid with a Tilemap child.
    (EDT-006, REN-005)
