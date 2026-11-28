# Shortcuts

## Scope

This document details the keyboard and mouse bindings, the Shortcuts Manager,
and shortcut profiles of EDT-006, owned by
[Editor and authoring](../editor-and-authoring.md). It owns no catalog ID.
Each window document lists its own window-scoped bindings in its `Shortcuts`
section; this document holds the global bindings, the Scene view's
navigation and tool bindings, the platform mapping, and the rules every
binding follows.

## Notation and platform mapping

* `Ctrl/Cmd` means Ctrl on Windows and Linux and Cmd on macOS.
* `Alt` means Alt on Windows and Linux and Option on macOS.
* `Shift` is the same on every platform.
* On macOS, Control is a distinct modifier that the default bindings do not use
  except where a binding names it, and Ctrl+click is a right-click.
* Bindings that differ by platform beyond that mapping:

| Action | Windows and Linux | macOS |
|---|---|---|
| Redo | `Ctrl+Y` | `Cmd+Shift+Z` |
| Rename | `F2` | `Return` |
| Delete | `Delete` | `Cmd+Backspace` |
| Preferences | Edit > Preferences... (no binding) | `Cmd+,` |
| Quit | File > Exit | `Cmd+Q` |
| Context menu of the focused item | `Menu` or `Shift+F10` | Ctrl+click |

* A binding that the operating system or desktop environment reserves (for
  example a window manager's Alt+drag) is marked reserved in the Shortcuts
  Manager, with the system that reserves it, and the editor's mouse equivalent
  is listed beside it.

## Contexts and dispatch

* Every binding belongs to a context: **Global**, or a window context such as
  **Scene View**, **Hierarchy**, **Project Browser**, **Inspector**,
  **Animation**, **Animator**, **Timeline**, **Tile Palette**, **Sprite
  Editor**, **UI Builder**, and contexts that editor extensions declare.
* A key press goes first to the focused text field, if any; a text field
  consumes printable keys, `Backspace`, `Delete`, arrow keys, and the
  platform's text-editing bindings (cut, copy, paste, select all, undo within
  the field).
* Otherwise it goes to the context of the window that has focus, or, for
  bindings marked "under the pointer" below, the window under the pointer;
  then to the Global context.
* While play mode runs and the Game view has focus, unmodified keys reach the
  game and no editor binding; bindings with `Ctrl/Cmd` (including play, pause,
  and step) still reach the editor. `Escape` releases a locked cursor.
* Bindings are action bindings (one press, one command) or clutch bindings
  (active while held, such as the fly-through keys and vertex snapping).
* Bindings declared by editor extensions through the shortcut attributes keep
  their Unity 2022.3 LTS meaning, appear in the Shortcuts Manager under their
  declared context and identifier, and are rebindable like built-in ones.

## Global bindings

| Action | Binding |
|---|---|
| New Scene | `Ctrl/Cmd+N` |
| Open Scene | `Ctrl/Cmd+O` |
| Save | `Ctrl/Cmd+S` |
| Save As | `Ctrl/Cmd+Shift+S` |
| Build Settings | `Ctrl/Cmd+Shift+B` |
| Build And Run | `Ctrl/Cmd+B` |
| Undo | `Ctrl/Cmd+Z` |
| Redo | `Ctrl+Y` / `Cmd+Shift+Z` |
| Cut, Copy, Paste | `Ctrl/Cmd+X`, `Ctrl/Cmd+C`, `Ctrl/Cmd+V` |
| Paste As Child | `Ctrl/Cmd+Shift+V` |
| Duplicate | `Ctrl/Cmd+D` |
| Rename | `F2` / `Return` |
| Delete | `Delete` / `Cmd+Backspace` |
| Select All | `Ctrl/Cmd+A` |
| Deselect All | `Shift+D` |
| Select Children | `Shift+C` |
| Select Prefab Root | `Ctrl/Cmd+Shift+R` |
| Invert Selection | `Ctrl/Cmd+I` |
| Save Selection 1 to 9 | `Ctrl/Cmd+Alt+1` to `9` |
| Load Selection 1 to 9 | `Ctrl/Cmd+Shift+1` to `9` |
| Find (focus the focused window's search field) | `Ctrl/Cmd+F` |
| Search window | `Ctrl/Cmd+K` |
| Play | `Ctrl/Cmd+P` |
| Pause | `Ctrl/Cmd+Shift+P` |
| Step | `Ctrl/Cmd+Alt+P` |
| Refresh assets | `Ctrl/Cmd+R` |
| Copy asset path | `Ctrl/Cmd+Alt+C` |
| Properties (floating Inspector for the selection) | `Alt+P` |
| Create Empty | `Ctrl/Cmd+Shift+N` |
| Create Empty Child | `Alt+Shift+N` |
| Create Empty Parent | `Ctrl/Cmd+Shift+G` |
| Set as first sibling | `Ctrl/Cmd+=` |
| Set as last sibling | `Ctrl/Cmd+-` |
| Move To View | `Ctrl/Cmd+Alt+F` |
| Align With View | `Ctrl/Cmd+Shift+F` |
| Toggle Active State | `Alt+Shift+A` |
| Add Component | `Ctrl/Cmd+Shift+A` |
| Scene, Game, Inspector, Hierarchy, Project | `Ctrl/Cmd+1` to `Ctrl/Cmd+5` |
| Animation, Profiler, Audio Mixer | `Ctrl/Cmd+6` to `Ctrl/Cmd+8` |
| Console | `Ctrl/Cmd+Shift+C` |
| Next window, previous window | `Ctrl/Cmd+Tab`, `Ctrl/Cmd+Shift+Tab` |
| Maximize the window under the pointer | `Shift+Space` |

## Scene View bindings

These apply while the Scene view has focus, or, where marked, while the
pointer is over it. Their behaviour is defined in [Scene view](scene-view.md).

| Action | Binding |
|---|---|
| View (hand) tool | `Q` |
| Move tool | `W` |
| Rotate tool | `E` |
| Scale tool | `R` |
| Rect tool | `T` |
| Transform tool | `Y` |
| Toggle pivot position (Pivot / Center) | `Z` |
| Toggle handle rotation (Global / Local) | `X` |
| Frame selected | `F` |
| Lock view to selected (follow) | `Shift+F` |
| Toggle 2D mode | `2` |
| Overlay menu | `` ` `` |
| Toggle visibility of selection | `H` |
| Toggle isolation of selection | `Shift+H` |
| Toggle pickability of selection | `L` |
| Decrease grid size, increase grid size | `Ctrl/Cmd+[`, `Ctrl/Cmd+]` |
| Push selection to grid | `Ctrl/Cmd+\` |
| Fly-through (while right mouse is held; clutch) | `W` `A` `S` `D` forward, left, back, right; `Q` `E` down, up |
| Faster fly-through and navigation (clutch) | `Shift` |
| Incremental snapping during a handle drag (clutch) | `Ctrl/Cmd` |
| Vertex snapping (clutch) | `V` |
| Surface snapping with the move tool's center handle (clutch) | `Ctrl/Cmd+Shift` |
| Move the camera forward, back, left, right | Arrow keys |
| Cancel a handle drag or a drop in progress | `Escape` |

### Scene View mouse bindings

| Gesture | Effect |
|---|---|
| Right-drag | Look around in place (fly-through mode while held) |
| Scroll during right-drag | Change fly-through speed |
| `Alt`+left-drag | Orbit around the pivot (perspective views) |
| `Alt`+right-drag | Zoom toward the pivot |
| Middle-drag, or `Alt`+middle-drag | Pan |
| Left-drag with the View tool | Pan |
| Scroll | Zoom toward the pivot (under the pointer) |
| Left-click | Select; repeated clicks at the same point cycle overlapping objects |
| `Shift`+left-click | Add to selection |
| `Ctrl/Cmd`+left-click | Toggle in selection |
| Left-drag on empty space | Rectangle selection |
| Double-click in the Hierarchy | Frame the object in the Scene view |

## Window-scoped bindings

Each window's bindings, with their context, are defined in that window's
document:

* Hierarchy: [Hierarchy](hierarchy.md#shortcuts)
* Inspector: [Inspector](inspector.md#shortcuts)
* Project window: [Project window](project-window.md#shortcuts)
* Console: [Console](console.md#shortcuts)
* Game view and play mode: [Game view and play mode](game-view-and-play-mode.md#shortcuts)
* Prefab Mode: [Prefab workflow](prefab-workflow.md#shortcuts)
* Animation, Animator, and Timeline: [Animation and timeline windows](animation-and-timeline-windows.md#shortcuts)
* Sprite Editor, Tile Palette, and Sprite Atlas: [2D tools](2d-tools.md#shortcuts)
* UI Builder and Canvas tools: [UI authoring](ui-authoring.md#shortcuts)
* Lighting and Frame Debugger: [Lighting and rendering tools](lighting-and-rendering-tools.md#shortcuts)
* Profiler and debugging windows: [Profiler and debugging windows](profiler-and-debugging-windows.md#shortcuts)
* Settings windows: [Settings and preferences](settings-and-preferences.md#shortcuts)

## Shortcuts Manager

Edit > Shortcuts... (on macOS, also unitas > Shortcuts...) opens the Shortcuts
Manager, a single-instance window.

### Regions

* **Profile bar** at the top: the active profile dropdown with Create New
  Profile..., Rename Profile..., Delete Profile..., Import Profile..., and
  Export Profile...
* **Keyboard map** below it: a keyboard drawn in the operating system's
  current keyboard layout. Keys bound in the Global context and keys bound in
  the selected category's context are colored distinctly, keys reserved by the
  system are marked, and modifier keys on the map toggle to show the bindings
  that use them. Hovering a key lists its bindings; dragging a command from the
  command list onto a key binds it.
* **Category list** on the left: All Unassigned Shortcuts, Binding Conflicts,
  Main Menu (by top-level menu), and one entry per context.
* **Command list** on the right: every command of the selected category, with
  its name and its bindings, searchable by command name or by binding (a
  toggle switches the search field between the two).

### Rebinding

* Double-clicking a binding cell, or choosing Add Shortcut from the command's
  context menu, enters capture mode; the next key press with its modifiers is
  the binding. `Escape` cancels capture; `Backspace` or `Delete` during capture
  removes the binding.
* A command can hold more than one binding, and a binding can be a chord of two
  key presses.
* When the captured binding is already bound in the same context, or in the
  Global context while the command is in a window context, a Binding Conflict
  prompt names the other command and offers **Create Conflict** (keep both),
  **Remove and Set** (unbind the other and bind this), and **Cancel**. A
  conflict that is kept is listed under Binding Conflicts; pressing that key
  opens a chooser listing the conflicting commands.
* Reset to Default on a command's context menu restores that command's
  default bindings. Choosing the Default profile restores every default.

### Profiles

* The **Default** profile holds the bindings in this specification and cannot be
  edited or deleted. Every other profile stores only its differences from
  Default, so new built-in and extension commands appear in it with their
  default bindings.
* Profiles are per user and shared across projects; the active profile is
  per user. Changing profiles, bindings, or the active profile never dirties
  project content.
* Export writes a profile to a file; Import reads one, reporting commands it
  names that do not exist in this project as unresolved and keeping them so
  that they apply in projects where those commands exist.

### MCP

Every binding, context, profile, and conflict is readable over MCP, and
rebinding, profile changes, and import and export are tools with the same
results as in the Shortcuts Manager (EDT-006, AUT-001).

## Agent presence

* No agent feature has a default binding. The Agent Activity and Change Review
  windows, holding and permitting agent writes, and accepting, rejecting, and
  reverting change sets appear in the Shortcuts Manager under an **Agents**
  context and the Main Menu category, unbound, so a person can bind them.
* An agent feature never takes, shadows, or changes a binding in the Default
  profile or in a person's profile. A binding an editor extension declares that
  collides with a Default binding is reported as a conflict and does not
  replace the Default binding until a person resolves it.

## Scale

* Dispatch cost is independent of project size and of the number of commands:
  bindings are indexed by context and key.
* The command list and Binding Conflicts list are virtualized and searchable
  through the command index, so projects whose packages declare thousands of
  commands open the Shortcuts Manager with no delay that depends on command
  count.

## Acceptance outcomes

1. With the Scene view focused and an object selected, injecting `W`, `E`, `R`,
   `T`, `Y`, and `Q` selects the Move, Rotate, Scale, Rect, Transform, and View
   tools in turn, read through the active-tool resource. (EDT-003, EDT-006)
2. On macOS, injecting `Cmd+D` duplicates the selected object and
   `Cmd+Backspace` deletes it; on Windows, `Ctrl+D` and `Delete` do the same;
   on each platform the Redo binding in the table restores the deleted object.
   (EDT-006, EDT-005)
3. With a text field focused in the Inspector, injecting `Ctrl/Cmd+D`, `Delete`,
   and `W` edits the text and does not duplicate, delete, or change tools.
   (EDT-006)
4. While play mode runs with the Game view focused, injecting `W` reaches the
   game's input and leaves the Scene view tool unchanged, and injecting
   `Ctrl/Cmd+P` exits play mode. (EDT-004, EDT-006)
5. In the Shortcuts Manager, binding Edit > Duplicate to `Ctrl/Cmd+Shift+D` in
   a new profile changes the menu's displayed binding, and switching back to
   Default restores `Ctrl/Cmd+D`. (EDT-006)
6. Capturing `F` for a Global command produces the Binding Conflict prompt
   naming Frame Selected; choosing Cancel leaves both bindings unchanged.
   (EDT-006)
7. A profile exported on Windows and imported on macOS maps `Ctrl` bindings to
   `Cmd` and keeps unresolved commands. (EDT-006)
8. A project editor extension declaring a shortcut for its own command appears
   under its context in the Shortcuts Manager and runs when the key is pressed
   in that context; one declaring `Ctrl/Cmd+S` is reported as a conflict and
   `Ctrl/Cmd+S` still saves. (EDT-006)
9. Every row in the Global and Scene View tables above is present in the
   Default profile read over MCP, with the same binding for the platform the
   editor runs on. (EDT-006)
10. No command in the Agents context has a binding in the Default profile.
    (EDT-006, EDT-008)
11. Changing the active profile and rebinding commands leaves every scene,
    asset, and project setting clean. (EDT-006)
