# Project Window

## Scope

This document details the Project window of the visual editor: browsing,
searching, creating, moving, importing, and opening the project's assets. It
refines EDT-006 (workspace views, commands, drag and drop, and multi-editing)
and the selection and navigation parts of EDT-001, which are owned by
[Editor and authoring](../editor-and-authoring.md); this document owns no
catalog ID. The asset behavior it presents (identity, labels, dependencies,
import state) is owned by PRJ-002, PRJ-005, and PRJ-004 in
[Projects and assets](../project-and-assets.md), and asset identity by PRJ-003
in [Serialization and prefabs](../serialization-and-prefabs.md).

## Layouts

The window has two layouts, chosen from the window menu (vertical three dots
on the tab): One Column Layout and Two Column Layout. The choice persists per
window with the workspace arrangement. Two-column is the default in the
Default workspace layout; one-column is used by the 2 by 3 and Tall layouts
(see [Workspace and layout](workspace-and-layout.md)).

### Two-column layout

* **Left pane: folder tree.** Top to bottom: Favorites, Assets, Packages.
  Favorites holds the built-in searches All Materials, All Models, All
  Prefabs, All Modified, All Conflicts, and All Excluded, followed by saved
  searches. Assets is the project's Assets folder tree. Packages lists every
  resolved package by display name, each expanding to its folder tree.
* **Right pane: contents.** A toolbar row with the breadcrumb path bar, the
  contents grid or list, and a bottom status row with the selected asset's
  path and the zoom slider.
* The divider between the panes is draggable; its position persists.
* Selecting a folder in the tree shows its contents on the right; selecting
  a favorite runs its search.

### One-column layout

A single tree shows Favorites, Assets, and Packages, with files listed inside
their folders. Expanding a model, a sprite sheet, or another asset with
sub-assets reveals them as children. There is no breadcrumb or zoom slider;
the search bar sits above the tree.

## Toolbar

Left to right:

* **Create (+) button.** Opens the same menu as Assets > Create (see
  [Menus and toolbar](menus-and-toolbar.md)); new assets are created in the
  folder currently shown, or in the selected folder in one-column layout.
* **Search field.** Ctrl/Cmd+F focuses it while the window has focus.
* **Search by Type dropdown.** Lists asset types (AnimationClip, AudioClip,
  AudioMixer, ComputeShader, Font, GUISkin, Material, Mesh, Model,
  PhysicMaterial, Prefab, Scene, Script, Shader, Sprite, Texture,
  VideoClip, and the types packages register). Choosing a type adds or
  removes `t:<Type>` in the search field.
* **Search by Label dropdown.** Lists the project's labels with a filter
  field; choosing a label adds or removes `l:<Label>`.
* **Save Search (star).** Saves the current search under a name as a
  favorite.
* **Hidden packages counter.** Shows the number of packages hidden from the
  Packages root and toggles their visibility.

## Breadcrumbs and contents

* The breadcrumb bar shows the path from Assets (or the package root) to the
  shown folder; clicking a segment shows that folder; the arrow after a
  segment opens a list of its subfolders.
* The contents area lists folders first, then assets, sorted by name.
* The **zoom slider** at the bottom right scales icons continuously. At its
  leftmost position the contents switch to a list with small icons; moving it
  right switches to a grid of previews. The value persists.
* Assets with sub-assets show an expand arrow on their tile; expanding shows
  sub-assets inline after the parent.
* Previews render asynchronously; a placeholder icon shows until the preview
  is ready.
* An **empty folder** states that it is empty. A search with no
  results names the filters that excluded content (PRJ-002).

## Search

The search field accepts terms combined with AND, in any order:

* plain words — match asset names, case-insensitively, partially;
* `t:<Type>` — asset type, including script-defined ScriptableObject types and
  base types (`t:Texture` matches every texture type);
* `l:<Label>` — asset label;
* `ref:<path>` or `ref:<instance id>` — assets that reference the named asset;
* `glob:"<pattern>"` — path glob, such as `glob:"Assets/Art/**/*.png"`;
* quoted phrases for names containing spaces.

Search scope is chosen from the scope bar that appears under the toolbar while
a search is active: **All**, **In Packages**, **In Assets**, and **'<Folder>'**
(the folder shown when the search began). Results show each asset's folder
path on hover and in the status row. Clearing the search (Esc in the search
field or the clear button) returns to the folder that was shown.

Searching runs against an index maintained incrementally (see
[Architecture](../editor-and-authoring.md#architecture)); results appear
progressively and the window remains responsive while they arrive.

## Selection and keyboard

* Click selects; Ctrl/Cmd-click toggles; Shift-click selects a range; drag in
  empty space draws a selection rectangle in grid view.
* Arrow keys move the selection; in one-column layout, Right expands and Left
  collapses a folder; Alt-click on a folder arrow expands or collapses all its
  descendants.
* Return opens the selected asset on Windows and Linux; on macOS, Cmd+Down
  opens it and Return starts a rename. F2 renames on Windows and Linux.
* Double-click opens: scenes open in the Scene view (prompting to save
  modified scenes), prefabs open in Prefab Mode ([Prefab workflow](prefab-workflow.md)),
  scripts open in the external editor, folders are entered, and other assets
  open with their registered handler or their editor window (Animator
  Controller in the Animator window, Timeline asset in the Timeline window).
* Ctrl/Cmd+D duplicates the selection, Delete (Cmd+Backspace on macOS) deletes
  it after confirmation, Ctrl/Cmd+C and Ctrl/Cmd+V copy and paste assets into
  the shown folder.
* Selecting an asset shows it in the Inspector, and its path appears in the
  status row.

## Creating assets

Creating an asset from the Create menu (folder, C# Script, Scene, Prefab,
Material, Animator Controller, Animation, Render Texture, Physic Material,
ScriptableObject types declared with `[CreateAssetMenu]`, and the other
Create entries) creates it in the shown folder with a default name and enters
rename immediately. Return or a click elsewhere commits the name; Esc commits
the default name. For a C# script, the class name inside the file is derived
from the committed name; renaming the file later does not rewrite the class.
The creation and the rename form one undo operation.

## Drag and drop

* **Within the window:** dragging assets onto a folder (in the tree, the
  breadcrumb, or the contents) moves them, preserving identity (PRJ-003);
  holding Ctrl (Option on macOS) while dropping copies instead. A move into a
  read-only package folder is rejected.
* **Out to the Scene view or Hierarchy:** prefabs, models, sprites, audio
  clips, and other instantiable assets instantiate as defined in
  [Hierarchy](hierarchy.md) and [Scene view](scene-view.md); a scene asset
  dropped on the Hierarchy opens additively.
* **Out to the Inspector:** onto object fields and lists, as defined in
  [Inspector](inspector.md); a script asset dropped on an Inspector adds the
  component.
* **In from the Hierarchy:** dropping a GameObject creates a prefab asset in the
  target folder, as defined in [Prefab workflow](prefab-workflow.md).
* **In from the operating system's file manager:** files and folders dropped
  onto a folder are copied into the project and imported; a name collision
  appends a number to the incoming name.
* A drag over an invalid target shows the rejection cursor and the status row
  states why; dropping changes nothing.

## Context menu

Right-click on an asset, a folder, or empty space opens the Assets menu items
that apply (see [Menus and toolbar](menus-and-toolbar.md)), in this order:

* Create (submenu);
* Show in Explorer on Windows, Reveal in Finder on macOS, Open Containing
  Folder on Linux;
* Open;
* Delete — asks for confirmation that names every selected asset and states
  that the action cannot be undone; the confirmation lists assets that other
  assets still reference (PRJ-005);
* Rename;
* Copy Path (Ctrl/Cmd+Alt+C);
* Open Scene Additive (scene assets);
* View in Package Manager (package content);
* Import New Asset..., Import Package (Custom Package...), Export Package...;
* Find References In Scene;
* Select Dependencies;
* Refresh (Ctrl/Cmd+R);
* Reimport and Reimport All;
* Extract From Prefab (models with materials or animations);
* Open C# Project;
* View in Import Activity Window;
* Properties... (Alt+P);
* project and package items declared with `[MenuItem("Assets/...")]`.

Each item is a command whose MCP tool title is its menu path, for example
`Assets/Reimport` (EDT-006).

## States and indicators

* **Import progress.** While assets import, a progress bar in the status bar
  shows the count and current asset; the Project window shows importing
  assets with a pending overlay and keeps them selectable.
* **Import failure.** An asset that failed to import shows an error overlay;
  selecting it shows the diagnostic in the Inspector and the Console (PRJ-004,
  DIA-001).
* **Source control.** In a Git working tree, each asset carries its status
  overlay as defined in [Git workflows](../editor-and-authoring.md#git-workflows).
* **Read-only package content.** Immutable package folders show a lock
  overlay; their assets open and inspect read-only, and Create, Rename,
  Delete, and drop-into are disabled with the reason.
* **Ping.** A ping from another view (an object field, a Console entry, a
  search result) expands the tree to the asset, scrolls it into view, and
  highlights it briefly.
* **Lock.** The lock toggle on the tab keeps the window on its current folder
  when another view pings or selects an asset elsewhere.

### Shortcuts

Shortcuts Manager context: **Project Browser**.

| Action | Windows and Linux | macOS |
|---|---|---|
| Open or focus Project window | Ctrl+5 (global) | Cmd+5 (global) |
| Focus search | Ctrl+F | Cmd+F |
| Rename | F2 | Return |
| Open selected asset | Return | Cmd+Down |
| Delete (with confirmation) | Delete | Cmd+Backspace |
| Duplicate | Ctrl+D | Cmd+D |
| Copy Path | Ctrl+Alt+C | Cmd+Option+C |
| Refresh | Ctrl+R | Cmd+R |
| Properties... | Alt+P | Option+P |
| Go up one folder (two-column) | Backspace | Cmd+Up |
| Expand all descendants | Alt-click arrow | Option-click arrow |

All bindings are rebindable in the Shortcuts Manager ([Shortcuts](shortcuts.md)).

### Agent presence

* An asset or folder that an active agent session is creating, changing,
  moving, or deleting shows the agent badge at the trailing edge of its row or
  tile, after the source-control overlay; hover shows the session and
  operation, click opens Change Review filtered to that asset.
* Under the require-approval policy, a proposed new asset appears only while
  the person previews the proposal, drawn with the proposed state; a proposed
  deletion shows the asset struck through while previewed.
* Favorites gains no entries; the built-in search syntax gains no agent terms.
  Agent-changed assets are found from the Change Review window, which pings
  them here.
* The context menu gains a trailing Review Agent Changes group only for assets
  with agent changes.

### Scale

The tree and contents realize only visible rows and tiles. A folder holding
100,000 assets opens, scrolls, and selects within the declared responsiveness
budget, and previews are generated only for visible tiles. Search over a
project of 100,000 assets returns paginated results from the incremental
index, with the first page displayed while later pages load, as required by
the [scale floor](../diagnostics-profiling-playtesting.md#scale-floor).
Multi-selection of all assets in such a folder supports batch move, label,
reimport, and delete, each as one command with a result listing every affected
asset.

## Acceptance outcomes

1. Switching the window menu between One Column Layout and Two Column Layout
   changes the layout, and the choice is restored when the project reopens.
   (EDT-006)
2. Searching `t:Material l:Environment` lists exactly the materials labeled
   Environment; changing the scope to In Packages lists only package
   materials. (EDT-006, PRJ-002)
3. Saving a search with the star adds it under Favorites, and selecting the
   favorite reruns it. (EDT-006, PRJ-002)
4. Moving the zoom slider to its leftmost position shows a list; moving it right
   shows a grid of previews. (EDT-006)
5. Clicking a breadcrumb segment shows that folder, and the arrow after it lists
   its subfolders. (EDT-006)
6. Choosing Create > Material creates a material in the shown folder in rename
   state; typing a name and Return names it, and one undo removes it. (EDT-005,
   EDT-006)
7. Dragging a texture from one folder to another moves it, and every material
   that referenced it still references it. (EDT-006, PRJ-003)
8. Dragging a GameObject from the Hierarchy into a folder creates a prefab asset
   there and turns the object into its instance. (EDT-006, SER-003)
9. Delete on an asset that a scene references asks for confirmation that names
   the asset and lists the referring scene; confirming deletes it. (EDT-006,
   PRJ-005)
10. A file dropped from the operating system's file manager onto a folder is
    copied and imported there. (EDT-006, PRJ-004)
11. Reimport on a texture reimports it, and the Console reports a failure in
    a corrupt texture with the asset named. (EDT-006, PRJ-004, DIA-001)
12. Create, Rename, and Delete are disabled with a stated reason on immutable
    package content. (EDT-006)
13. Invoking `Assets/Copy Path` through MCP on a selected asset returns its
    project-relative path, the same as the menu item. (EDT-006, AUT-001)
14. A folder of 100,000 assets opens and scrolls within the declared
    responsiveness budget, and a name search over the project returns its first
    page while later pages load. (EDT-006, PRF-002)
15. An asset an agent session is changing shows the agent badge, and clicking it
    opens Change Review filtered to that asset. (EDT-008)
