# Editor UI and UX

## Scope

This folder details the visual editor's windows, layout, menus, gestures,
shortcuts, and workflows. It details EDT-001 through EDT-008, which are owned by
[Editor and authoring](../editor-and-authoring.md), and owns no catalog ID.
Requirements on project content, serialization, runtime behaviour, and
automation stay with their owning specifications; these documents state how the
visual editor presents and edits them, and link to the owner wherever a
behaviour is defined elsewhere.

## The familiarity principle

The visual editor is designed so that a person who already builds games with
component-based scene editors works in it productively from the first hour:

* **Same windows.** The editor's working surface is the Hierarchy, Scene view,
  Game view, Inspector, Project window, and Console, with the Animation,
  Animator, Timeline, Profiler, Frame Debugger, Lighting, Package Manager,
  Project Settings, Preferences, Sprite Editor, Tile Palette, UI Builder, and
  their companions, each with the regions, controls, and states its document
  defines.
* **Same arrangement.** The Default layout and the 2 by 3, 4 Split, Tall, and
  Wide layouts arrange those windows as
  [Workspace and layout](workspace-and-layout.md) states, and every window
  docks, tabs, floats, and maximizes the same way.
* **Same commands in the same places.** The File, Edit, Assets, GameObject,
  Component, Window, and Help menus, the context menus, and the main toolbar
  hold the items [Menus and toolbar](menus-and-toolbar.md) lists, in that
  order.
* **Same hands.** Keyboard bindings, mouse gestures, modifier behaviour,
  drag-and-drop outcomes, and snapping are those [Shortcuts](shortcuts.md) and
  each window document define, mapped between Ctrl and Cmd by platform, and
  rebindable through the Shortcuts Manager with profiles.
* **Same conventions.** Inspector headers and component menus, mixed-value
  display, prefab override display, Prefab Mode, play-mode tint and edit-state
  restoration, Console behaviour, and asset creation and import follow the
  documents in this folder.
* **Additive agent features.** The editor is natively agent-driven: every
  command is an MCP tool titled by its menu path, every window's state and
  controls are readable and operable over MCP, and agent changes are visible
  as they happen, reviewable, and reversible. These features are additions;
  under the non-disturbance rule in
  [Agent-native additions](agent-native-additions.md), they never move,
  rename, or rebind a familiar window, menu item, control, or shortcut.

Where a document in this folder and a general statement in
[Editor and authoring](../editor-and-authoring.md) appear to differ about how a
window looks or responds to input, the document in this folder is the
detailed requirement; where they differ about what an operation does to
project content, undo, change sets, or permissions, the owning specification
governs.

## Conventions used in these documents

* `Ctrl/Cmd` means Ctrl on Windows and Linux and Cmd on macOS; `Alt` means
  Option on macOS (see [Shortcuts](shortcuts.md#notation-and-platform-mapping)).
* A menu path is written with `>` in prose (File > Save) and with `/` when it
  names the command's MCP tool (`File/Save`).
* Each document opens with `Scope`, details its windows, and ends with
  `Shortcuts` (window-scoped bindings and their Shortcuts Manager context),
  `Agent presence`, `Scale`, and `Acceptance outcomes`.
* Acceptance outcomes are observable and automatable: each is checked
  headless through the editor's MCP interface, or in the visual editor by
  input injection, and cites the catalog IDs it demonstrates.
* Windows provided by packages (for example Timeline, Sprite Editor, Tile
  Palette, Memory Profiler, Shader Graph, and Test Runner) are the packages'
  own editor windows, drawn as editor extensions (RUN-010 in
  [Scripting runtime](../scripting-runtime.md)); their documents state the
  resulting behaviour, the editor services the windows rely on, and the state
  shown when the package is absent.

## Scale

Every list, tree, grid, and graph in these documents is virtualized: it
realizes only visible rows or elements, and per-frame cost follows visible
content and changes, never project or scene size. Each document's `Scale`
section states how its windows hold the
[scale floor](../diagnostics-profiling-playtesting.md#scale-floor) of 100,000
assets and 100,000 GameObjects.

## Reading map

| Document | Covers |
|---|---|
| [Workspace and layout](workspace-and-layout.md) | Dock area, tabs, floating, maximize, factory and named layouts, status bar, window title, empty and error states |
| [Menus and toolbar](menus-and-toolbar.md) | Menu bar and every standard menu, context menus, main toolbar, play controls, Layers and Layout dropdowns, command palette |
| [Shortcuts](shortcuts.md) | Global and Scene view bindings, mouse gestures, platform mapping, dispatch rules, Shortcuts Manager, profiles |
| [Scene view](scene-view.md) | Navigation, scene gizmo, transform tools and handles, snapping and grid, draw modes, visibility and pickability, selection and picking, overlays |
| [Hierarchy](hierarchy.md) | Scene headers, rows, parenting and reordering by drag, rename, duplicate, delete, search, visibility and pickability columns |
| [Inspector](inspector.md) | GameObject header, component headers and menus, Add Component, field controls, object fields, lists, multi-object editing, lock, Debug mode, asset inspectors and previews |
| [Project window](project-window.md) | One- and two-column layouts, Favorites, breadcrumbs, zoom, search filters, asset creation, drag and drop, labels, reimport, reveal in file manager |
| [Console](console.md) | Toolbar, clear options, collapse, error pause, entries, stack traces and source links, player logs |
| [Game view and play mode](game-view-and-play-mode.md) | Game view toolbar, aspect and resolution, scale, stats, gizmos, play, pause, step, play-mode tint, edit-state restoration |
| [Prefab workflow](prefab-workflow.md) | Creating prefabs and variants, instance display, override display, Overrides dropdown, Apply and Revert, Prefab Mode in isolation and in context |
| [Animation and timeline windows](animation-and-timeline-windows.md) | Animation window, Animator window, Timeline window |
| [2D tools](2d-tools.md) | Sprite Editor, Tile Palette, Sprite Atlas, 2D scene mode |
| [UI authoring](ui-authoring.md) | UI Builder, Canvas tools, anchors and the Rect tool on UI |
| [Lighting and rendering tools](lighting-and-rendering-tools.md) | Lighting window, Light Explorer, Occlusion Culling, probes, Frame Debugger, render pipeline settings, Shader Graph |
| [Profiler and debugging windows](profiler-and-debugging-windows.md) | Profiler, Memory Profiler, Physics Debugger, Import Activity, IMGUI and UI Toolkit debuggers, Test Runner, Device Simulator, Progress, Undo History |
| [Settings and preferences](settings-and-preferences.md) | Project Settings, Preferences, Package Manager, Build Settings |
| [Agent-native additions](agent-native-additions.md) | Non-disturbance rule, agent indicator, Agent Activity and Change Review windows, live indicators, proposal preview, commands as tools |

A builder new to the editor reads [Workspace and layout](workspace-and-layout.md),
[Menus and toolbar](menus-and-toolbar.md), and [Shortcuts](shortcuts.md) first,
then the six core windows ([Scene view](scene-view.md),
[Hierarchy](hierarchy.md), [Inspector](inspector.md),
[Project window](project-window.md), [Console](console.md), and
[Game view and play mode](game-view-and-play-mode.md)), then
[Prefab workflow](prefab-workflow.md) and
[Agent-native additions](agent-native-additions.md), and the remaining window
documents as their content kinds are built.

## Acceptance outcomes

1. Every window named in the reading map opens from its Window menu path, and
   its document's acceptance outcomes pass headless through MCP or by input
   injection in the visual editor. (EDT-006)
2. With no agent session connected, a scripted session that creates a scene,
   adds objects, parents them by drag in the Hierarchy, edits their
   components in the Inspector, moves them with the Move tool, saves with
   `Ctrl/Cmd+S`, enters and exits play mode with `Ctrl/Cmd+P`, and undoes each
   step uses only the windows, menu items, and bindings these documents name,
   and every step produces the result these documents state. (EDT-001,
   EDT-002, EDT-003, EDT-004, EDT-005, EDT-006)
3. The same session replayed through MCP, invoking each step's command by its
   menu path or Shortcuts Manager identifier, produces the same scene, undo
   history, and play states. (EDT-006, AUT-001)
4. With agent features active and an agent session connected, the menu bar,
   main toolbar, Default layout, and Default shortcut profile read over MCP
   differ from the same build with no session connected only by the additions
   [Agent-native additions](agent-native-additions.md) lists. (EDT-006, EDT-008)
