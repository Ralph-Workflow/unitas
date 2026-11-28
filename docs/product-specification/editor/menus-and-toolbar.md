# Menus and Toolbar

## Scope

This document details the menu bar, context menus, and main toolbar of EDT-006
(commands and shortcuts) and the play controls of EDT-004, owned by
[Editor and authoring](../editor-and-authoring.md). It owns no catalog ID.
Bindings are listed in full in [Shortcuts](shortcuts.md); the transform tools
and their settings live in the Scene view's overlays, detailed in
[Scene view](scene-view.md).

## Menus are commands

* Every menu item, in the menu bar, in a context menu, or in a window menu, is
  a command (EDT-006). Its MCP tool is titled by its menu path verbatim (for
  example `GameObject/Create Empty`, `Assets/Reimport`,
  `CONTEXT/Rigidbody/Reset`), its tool name is a deterministic encoding of that
  path, and invoking the tool by name or by path has the same effect, inputs,
  availability, and result as choosing the item.
* An unavailable item is drawn disabled, stays in its position, and its
  tooltip and its MCP availability state the reason (for example "no
  selection" or "play mode is running").
* An item with a shortcut shows the binding at its right edge, in the
  platform's notation, and updates when the binding changes in the Shortcuts
  Manager.
* Items that open a dialog or window carry a trailing ellipsis, and items with
  a check state show a check mark.
* Menu items contributed by the project and its packages through `[MenuItem]`
  keep their Unity 2022.3 LTS meaning: they appear at their declared path,
  ordered by priority, with a separator between two items whose priorities
  differ by more than ten, and with their validation method controlling
  availability. A declared top-level menu appears after **Component** and
  before **Window**, in priority order. A declared item never replaces a
  built-in item at the same path; the conflict is logged and the built-in item
  wins.
* Keyboard: `Alt` (Windows and Linux) shows menu mnemonics; arrow keys move
  between menus and items; `Return` chooses; `Escape` closes. On macOS the
  system menu bar behaves as every other application's.

## Menu bar

The menu bar holds, in this order: **File**, **Edit**, **Assets**,
**GameObject**, **Component**, any project- or package-declared top-level
menus, **Window**, and **Help**. On macOS an application menu named unitas
precedes them.

### Application menu (macOS)

About unitas; Settings... (`Cmd+,`, opening Preferences); Check for Updates
(PRD-001 in [Builds, platforms, XR, and networking](../build-platforms-xr-networking.md));
Services, Hide unitas, Hide Others, Show All (system items); Quit unitas
(`Cmd+Q`).

### File

* New Scene (`Ctrl/Cmd+N`), which opens the new-scene dialog with scene
  templates, or creates the default scene directly when templates are turned
  off in Scene Template settings.
* Open Scene (`Ctrl/Cmd+O`); Open Recent Scene, a submenu of recently opened
  scenes for this project.
* Save (`Ctrl/Cmd+S`), saving the open scenes with unsaved changes (or the
  prefab when a Prefab Mode stage is open); Save As... (`Ctrl/Cmd+Shift+S`);
  Save As Scene Template...
* New Project..., Open Project..., Save Project (which writes every unsaved
  asset and setting that is not a scene).
* Build Settings... (`Ctrl/Cmd+Shift+B`); Build And Run (`Ctrl/Cmd+B`).
* Exit (Windows and Linux).

Opening or creating a scene while open scenes have unsaved changes asks to
save them first, naming each scene, with Save, Don't Save, and Cancel.

### Edit

* Undo (`Ctrl/Cmd+Z`) and Redo (`Ctrl+Y` on Windows and Linux,
  `Cmd+Shift+Z` on macOS); each names the operation it applies to (for
  example "Undo Move Cube"). Undo History... opens the Undo History window.
* Select All (`Ctrl/Cmd+A`), Deselect All (`Shift+D`), Select Children
  (`Shift+C`), Select Prefab Root (`Ctrl/Cmd+Shift+R`), Invert Selection
  (`Ctrl/Cmd+I`).
* Cut (`Ctrl/Cmd+X`), Copy (`Ctrl/Cmd+C`), Paste (`Ctrl/Cmd+V`), Paste As Child
  (`Ctrl/Cmd+Shift+V`), Duplicate (`Ctrl/Cmd+D`), Rename (`F2` on Windows and
  Linux, `Return` on macOS), Delete (`Delete` on Windows and Linux,
  `Cmd+Backspace` on macOS).
* Frame Selected in Scene (`F`), Lock View to Selected (`Shift+F`).
* Search All... (`Ctrl/Cmd+K`), opening the Search window.
* Play (`Ctrl/Cmd+P`), Pause (`Ctrl/Cmd+Shift+P`), Step (`Ctrl/Cmd+Alt+P`).
* Selection: Load Selection 1 to 9 (`Ctrl/Cmd+Shift+1` to `9`) and Save
  Selection 1 to 9 (`Ctrl/Cmd+Alt+1` to `9`), per user and per project.
* Project Settings...; Preferences... (Windows and Linux); Shortcuts...,
  opening the Shortcuts Manager.
* Clear All PlayerPrefs, after a confirmation.
* Graphics Tier and Rendering submenus for the editor's preview of shader
  tiers and rendering options (REN-006 in
  [Rendering and animation](../rendering-and-animation.md)).
* Grid and Snap Settings..., opening the Grid and Snap settings popup of the
  Scene view (see [Scene view](scene-view.md)).

### Assets

The Assets menu and the Project window's context menu are the same list
(see [Project window](project-window.md)):

* Create, a submenu of every creatable asset kind: Folder; C# Script; Shader
  and its variants; Scene; Prefab and Prefab Variant (when applicable);
  Material; Render Texture; Lightmap Parameters; Lighting Settings; 2D (Sprite Atlas, Sprites,
  and Tile Palette and tile assets with their packages); Animator Controller; Animation; Animator Override Controller; Avatar
  Mask; Timeline (with the package); Physic Material; Physics Material 2D;
  GUI Skin; Custom Font; Assembly Definition and Assembly Definition Reference;
  UI Toolkit assets (UI Document, StyleSheet, Panel Settings, Theme Style
  Sheet); and every kind a project or package declares through
  `[CreateAssetMenu]` or a `[MenuItem]` under `Assets/Create`.
* Show in Explorer (Windows), Reveal in Finder (macOS), Show in File Manager
  (Linux); Open; Delete; Rename; Copy Path (`Ctrl/Cmd+Alt+C`).
* Open Scene Additive; View in Package Manager.
* Import New Asset...; Import Package, a submenu with Custom Package...;
  Export Package...
* Find References In Scene; Select Dependencies.
* Refresh (`Ctrl/Cmd+R`); Reimport; Reimport All, after a confirmation.
* Extract From Prefab; Update UXML Schema; Open C# Project.
* View in Import Activity Window; Properties... (`Alt+P`), opening a floating
  Inspector locked to the selected asset.

### GameObject

* Create Empty (`Ctrl/Cmd+Shift+N`), Create Empty Child (`Alt+Shift+N`),
  Create Empty Parent (`Ctrl/Cmd+Shift+G`).
* 3D Object: Cube, Sphere, Capsule, Cylinder, Plane, Quad, Text -
  TextMeshPro (with the package), Ragdoll..., Terrain, Tree, Wind Zone.
* 2D Object: Sprites (Square, Circle, Capsule, Isometric Diamond, Hexagon
  Flat-Top and Point-Top, 9-Sliced), Physics, Tilemap (Rectangular,
  Hexagonal, Isometric, Isometric Z As Y), and further items that 2D packages
  declare.
* Effects (Particle System, Particle System Force Field, Trail, Line, and
  Visual Effect with its package), Light (Directional, Point, Spotlight, Area,
  Reflection Probe, Light Probe Group), Audio (Audio Source, Audio Reverb
  Zone), Video (Video Player), UI (Canvas-based controls; see
  [UI authoring](ui-authoring.md)), UI Toolkit (UI Document), Camera.
* Center On Children, Make Parent, Clear Parent.
* Set as first sibling (`Ctrl/Cmd+=`), Set as last sibling (`Ctrl/Cmd+-`).
* Set as Default Parent, Clear Default Parent.
* Move To View (`Ctrl/Cmd+Alt+F`), Align With View (`Ctrl/Cmd+Shift+F`),
  Align View to Selected.
* Toggle Active State (`Alt+Shift+A`).

A created object is placed as [Hierarchy](hierarchy.md) states: as a child of
the selected object when created from that object's context menu or with
Create Empty Child, under the default parent when one is set, and otherwise at
the scene root; positioned at the Scene view's pivot, or at the world origin
when the preference to create objects at origin is on. It is selected, and its
name enters rename mode in the Hierarchy.

### Component

* Add... (`Ctrl/Cmd+Shift+A`), opening the Add Component search on the
  Inspector of the selected objects (see [Inspector](inspector.md)).
* One submenu per component category: Mesh, Effects, Physics, Physics 2D,
  Navigation, Audio, Video, Rendering, Tilemap, Layout, Playables, AR,
  Miscellaneous, Scripts (project script components, grouped by namespace),
  UI, Event, UI Toolkit, and categories that `[AddComponentMenu]` declares.
* Choosing a component adds it to every selected GameObject that permits it,
  as one undo operation; objects that do not permit it are named in the
  Console with the reason.

### Window

* Panels: a numbered list (`1` to `9`, then the rest unnumbered) of every open
  window; choosing one focuses it.
* Next Window (`Ctrl/Cmd+Tab`), Previous Window (`Ctrl/Cmd+Shift+Tab`).
* Layouts: the named layouts, Save Layout..., Delete Layout, Save Layout to
  File..., Load Layout from File..., Reset All Layouts (see
  [Workspace and layout](workspace-and-layout.md)).
* Search: New Window, and the Search window's saved queries.
* Package Manager.
* Asset Management: Version Control (with its package), Import Activity.
* General: Scene (`Ctrl/Cmd+1`), Game (`Ctrl/Cmd+2`), Inspector
  (`Ctrl/Cmd+3`), Hierarchy (`Ctrl/Cmd+4`), Project (`Ctrl/Cmd+5`), Console
  (`Ctrl/Cmd+Shift+C`), Progress, Device Simulator, Test Runner (with its
  package).
* Rendering: Lighting, Light Explorer, Occlusion Culling, and render-pipeline
  windows that a pipeline package declares.
* Animation: Animation (`Ctrl/Cmd+6`), Animator, Avatar Configuration where
  applicable.
* Audio: Audio Mixer (`Ctrl/Cmd+8`).
* Sequencing: Timeline (with its package).
* Analysis: Profiler (`Ctrl/Cmd+7`), Frame Debugger, Physics Debugger, IMGUI
  Debugger, Memory Profiler (with its package).
* 2D: Sprite Editor, Tile Palette (with their packages), Sprite Packer
  settings.
* UI Toolkit: UI Builder, Debugger, Samples.
* AI: Navigation.
* Agents: Agent Activity, Change Review (see
  [Agent-native additions](agent-native-additions.md)).
* Further submenus and items that packages and the project declare (for
  example TextMeshPro and Visual Scripting).

### Help

About unitas (Windows and Linux); Manual; Scripting Reference; Release Notes;
Software Licenses; Report a Problem...; Check for Updates (Windows and Linux);
Reset Packages to defaults, after a confirmation naming the packages it
changes. Manual and Scripting Reference open unitas's product documentation,
which is the same content agents read through AUT-006 in
[Automation and MCP](../automation-and-mcp.md).

## Context menus

* Right-click on any item opens its context menu at the pointer; on macOS,
  Ctrl+click does the same. The `Menu` key and `Shift+F10` (Windows and Linux)
  open it for the focused item.
* The Hierarchy context menu holds Cut, Copy, Paste, Paste As Child, Rename,
  Duplicate, Delete, Select Children, Set as Default Parent, the Prefab submenu
  for prefab instances, and the GameObject menu's create items (see
  [Hierarchy](hierarchy.md)). Scene headers have their own menu.
* The Project window context menu is the Assets menu.
* Component headers open the component's menu, including `[ContextMenu]` items
  and `CONTEXT/<Type>/` `[MenuItem]` items declared for the component's type
  (see [Inspector](inspector.md)). Property labels open the property menu,
  including `[ContextMenuItem]` items.
* A context menu's items act on the whole selection when the clicked item is
  part of it, and on the clicked item alone otherwise, without changing the
  selection in the latter case.

## Main toolbar

The main toolbar spans the main window below the menu bar.

### Left

* **Account dropdown.** Shows the person's unitas account and license state
  (LIC-001 and PRD-001 in
  [Builds, platforms, XR, and networking](../build-platforms-xr-networking.md)),
  with sign in and sign out. unitas never requires an account to open or edit
  a project.
* **Version control button**, present when the project includes the Version
  Control package, opening that package's window.

### Center: play controls

Play, Pause, and Step buttons, grouped, in this order.

* **Play** toggles play mode (`Ctrl/Cmd+P`). While play mode runs it is drawn
  pressed, and the editor applies the play-mode tint (see
  [Game view and play mode](game-view-and-play-mode.md)).
* **Pause** toggles pause (`Ctrl/Cmd+Shift+P`). It can be pressed in edit mode,
  in which case entering play mode pauses after the first frame.
* **Step** (`Ctrl/Cmd+Alt+P`) advances one frame. In edit mode it enters play
  mode paused; in play mode it pauses if running and advances one frame.
* While entering or exiting play mode, the buttons show a busy state and
  further presses queue nothing; after a failure to enter play mode they
  return to the edit state, and the Console names the cause.

### Right

* **Undo History** button, opening the Undo History window.
* **Search** button, opening the Search window (`Ctrl/Cmd+K`).
* **Layers dropdown.** Lists every layer with an eye toggle (visibility in the
  Scene view) and a lock toggle (pickability in the Scene view), plus Everything,
  Nothing, and Edit Layers..., which opens Tags and Layers in Project Settings.
  These toggles are per-user view state and never change project content.
* **Layout dropdown.** Shows the current layout's name; lists named layouts and
  the layout commands (see [Workspace and layout](workspace-and-layout.md)).

### Command palette

The command palette is the Search window restricted to commands: in the
Search window (`Ctrl/Cmd+K`), the `m:` filter, or choosing the Menu scope,
lists every command by menu path, display name, and description, with its
binding, availability, and disabled reason (EDT-006). Choosing a result runs
the command. The palette takes no binding of its own.

## Shortcuts

Menu bindings are those shown with each item above, all in the global
context; the complete table is in [Shortcuts](shortcuts.md).

## Agent presence

* The menu bar gains one submenu, **Window > Agents**. No other built-in menu,
  menu item, toolbar button, or binding is added, moved, renamed, or rebound
  for agent features.
* A context menu whose target has changes made by an agent session ends with a
  separator and a **Review Agent Changes** item, which opens the Change Review
  window filtered to that target. The item is absent otherwise, so the
  familiar menu is unchanged.
* While the developer holds agent writes, no menu item changes; the agent
  indicator in the status bar shows the held state (see
  [Agent-native additions](agent-native-additions.md)).
* A menu item that an agent session invokes is recorded in the audit log
  (AUT-008) under its menu path.

## Scale

* Menus are built from the command registry, not from scanned assets, so
  opening any menu costs the same regardless of project size.
* A submenu with more items than fit on the display scrolls, and the command
  palette searches every command through the command index.
* `[MenuItem]` validation methods run only for the items of the menu being
  opened, never for the whole registry, and a validation method that throws
  or exceeds its time limit leaves the item disabled with that reason.

## Acceptance outcomes

1. The menu bar lists File, Edit, Assets, GameObject, Component, Window, and
   Help in that order, read through the menu resource over MCP, and on macOS
   the application menu precedes them. (EDT-006)
2. Choosing GameObject > Create Empty by input injection and invoking the
   `GameObject/Create Empty` tool over MCP each create one empty object at
   the same placement, selected, as one undo operation named "Create Empty".
   (EDT-006, EDT-001, EDT-005)
3. A project `[MenuItem("Tools/Bake Data")]` appears in a Tools menu between
   Component and Window; its validation method returning false draws it
   disabled with that reason in the tooltip and in the tool's availability.
   (EDT-006)
4. Two project `[MenuItem]` items under one path with priorities 1 and 20 are
   drawn with a separator between them; with priorities 1 and 5, without one.
   (EDT-006)
5. A project `[MenuItem]` declared at `Edit/Duplicate` does not replace the
   built-in item; the Console logs the conflict and the built-in item runs.
   (EDT-006)
6. With nothing selected, Edit > Duplicate is disabled and its tool reports
   availability "no selection". (EDT-006)
7. Pressing Play, Pause, and Step on the main toolbar by input injection, and
   pressing `Ctrl/Cmd+P`, `Ctrl/Cmd+Shift+P`, and `Ctrl/Cmd+Alt+P`, produce the
   same play states reported by the play-state resource. (EDT-004)
8. Pressing Step in edit mode enters play mode paused after one frame.
   (EDT-004)
9. Hiding a layer through the Layers dropdown hides its objects in the Scene
   view, leaves them visible in the Game view, and dirties no scene or
   setting. (EDT-003, EDT-006)
10. Rebinding Edit > Duplicate in the Shortcuts Manager updates the binding
    shown in the Edit menu. (EDT-006)
11. In the Search window, the `m:` filter with the text "create empty" lists
    GameObject/Create Empty with its binding, and choosing it creates the
    object. (EDT-006)
12. Right-clicking an object an agent session changed shows Review Agent
    Changes as the last item; right-clicking an unchanged object shows the
    same menu without it. (EDT-006, EDT-008)
