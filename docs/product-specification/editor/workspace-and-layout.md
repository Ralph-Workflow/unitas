# Workspace and Layout

## Scope

This document details the workspace views, docking, layouts, and status bar
of EDT-006, owned by [Editor and authoring](../editor-and-authoring.md). It
owns no catalog ID. Menus and the main toolbar are detailed in
[Menus and toolbar](menus-and-toolbar.md); each window's contents are
detailed in its own document, listed in the [reading map](INDEX.md#reading-map).

The visual editor's main window holds, from top to bottom: the menu bar (inside
the main window on Windows and Linux, the system menu bar on macOS), the main
toolbar, the dock area, and the status bar. Every other window is a dockable
window that lives in the dock area, in a floating container, or in a utility
or popup window of its own.

## Dock area

### Windows, tabs, and containers

* Every dockable window has a tab carrying its icon and title. Windows docked
  in the same place form a tab group; one tab of a group is in front.
* Clicking a tab brings it to the front. Dragging a tab within its group
  reorders it.
* Dragging a tab out of its group shows a drop preview over the target: the
  center of another group adds the window as a tab of that group; an edge of
  another group or of the dock area splits it and docks the window on that
  side; any position outside a dock target makes the window float in a
  container of its own at the release position, including on another display.
* Floating containers accept docked windows exactly as the main window's dock
  area does, so a floating container can hold its own split and tab groups.
* Dividers between docked groups drag to resize both sides. Each window
  declares a minimum size, and a divider stops where a neighbour would fall
  below it.
* Middle-clicking a tab closes it. Closing the last tab of a floating container
  closes the container.
* Dragging an asset, object, or other dragged content over a tab that is not
  in front, and holding it there, brings that tab to the front so the drop can
  complete inside it.

### Tab context menu

Right-clicking a tab, or clicking the window menu button (the vertical
three-dot button at the right end of the window's tab strip), opens the
window's menu. Its first items are window-specific (for example Lock in the
Inspector and Project window, or Normal and Debug in the Inspector). Its
common items, in this order, are:

* **Maximize**, which shows the window alone in its container, and restores it
  when chosen again;
* **Close Tab**;
* **Add Tab**, a submenu of every window kind, which opens a new window of that
  kind as a tab in this group.

### Maximize

* Double-clicking a tab maximizes its window; double-clicking it again
  restores the previous arrangement.
* `Shift+Space` maximizes the window under the pointer, or restores it when it
  is already maximized. The binding is global in the window's context, so it
  works while the pointer is over the Scene view or Game view.
* A maximized window keeps its toolbar and state. Maximize is not saved as part
  of a layout; reopening the project restores the arrangement before
  maximizing.

### Focus and keyboard routing

* One window has keyboard focus at a time, marked by a highlighted tab. Clicking
  inside a window focuses it. Window-scoped shortcuts (for example the Scene
  view's tool and navigation keys) act only on the focused window or, where
  [Shortcuts](shortcuts.md) states it, on the window under the pointer.
* `Ctrl/Cmd+Tab` and `Ctrl/Cmd+Shift+Tab` move focus to the next and previous
  docked window.
* `Ctrl/Cmd+1` through `Ctrl/Cmd+8` and `Ctrl/Cmd+Shift+C` focus the window
  they name (see [Shortcuts](shortcuts.md)); when no window of that kind is
  open, one opens in the position the current layout gives that kind, or
  floating when the layout has none.

### Multiple windows of one kind

The Scene view, Game view, Inspector, Project window, Hierarchy, Console, and
Animation window can be opened more than once through **Add Tab** or a window
menu item while one is already open. Each instance keeps its own settings (for
example camera, draw mode, lock state, column layout, and search). A
single-instance window kind (for example Project Settings, Preferences,
Lighting, and Package Manager) brings its existing window to the front instead
of opening a second one.

## Layouts

### Factory layouts

A fresh install offers these layouts in the Layout dropdown (see
[Menus and toolbar](menus-and-toolbar.md)) and in **Window > Layouts**:

* **Default.** The Hierarchy is docked on the left. The Scene view and Game
  view share a tab group in the center, Scene view in front. The Inspector is
  docked on the right at full height. The Project window and Console share a
  tab group below the Hierarchy and center group, Project window in front, in
  two-column mode.
* **2 by 3.** The Scene view on top and the Game view below it form the left
  column. To their right, in separate full-height columns, are the Hierarchy,
  the Project window in one-column mode, and the Inspector.
* **4 Split.** Four Scene views fill a two-by-two grid: three orthographic
  views aligned to the top, front, and right axes, and one perspective view.
  The Hierarchy, Project window in one-column mode, and Inspector are docked
  in columns to the right of the grid.
* **Tall.** The Scene view and Game view share a tall tab group on the left.
  The Hierarchy above the Project window in one-column mode form the middle
  column, and the Inspector is on the right.
* **Wide.** The Scene view and Game view share a wide tab group on the top
  left with the Inspector on the right. The Hierarchy, Project window in
  one-column mode, and Console sit side by side below the Scene view.

Every factory layout contains only the windows named here. The Default layout
opens the first time a project is opened and whenever no valid saved
arrangement exists.

### Current arrangement

* The current arrangement (docking, sizes, tab order, front tabs, floating
  containers and their displays, and each window's own state such as Scene view
  camera, Project window column mode, and Inspector lock) is saved per user
  and per project, outside shared project content, whenever the editor closes
  and at regular intervals while it runs.
* Reopening the project restores it exactly, including floating containers on
  the displays they were on. A container whose display is no longer connected
  opens on the main display, inside its bounds.
* An arrangement that cannot be restored (for example, one saved by a newer
  unitas version, or naming a window whose package or editor extension is no
  longer present) falls back to the Default layout, and the Console logs one
  warning naming the reason. A window whose package is missing but whose
  layout slot is otherwise valid is restored as a placeholder tab stating that
  the window's provider is not present; closing the tab discards it.

### Named layouts

* **Save Layout...** in the Layout dropdown and in **Window > Layouts** asks for
  a name and saves the current arrangement under it. Saving under an existing
  name asks whether to replace it.
* Named layouts are saved per user, shared across every project the user opens,
  and listed in the Layout dropdown above the layout commands, in alphabetical
  order, with factory layouts among them.
* Choosing a named layout replaces the current arrangement. Windows of kinds
  the layout includes keep their own state where the layout does not specify
  it (for example the Scene view keeps its camera).
* **Delete Layout** opens a submenu of saved layouts; choosing one deletes it.
* **Save Layout to File...** and **Load Layout from File...** export and import
  a layout as a file anywhere on disk, so a team can share layouts outside the
  project.
* **Reset All Layouts** restores the factory layouts, removes the user's named
  layouts after a confirmation that names them, and switches to Default. It
  changes nothing else: no project content, setting, preference, or shortcut.

Changing, saving, loading, and resetting layouts never dirties a scene, asset,
or project setting.

## Status bar

The status bar spans the bottom of the main window.

* **Left: last Console message.** The most recent Console entry is shown with
  its severity icon and the first line of its message. Clicking it opens and
  focuses the Console with that entry selected. The status bar shows nothing
  when the Console is cleared.
* **Right, in order:**
  * the progress indicator for background operations (import, compilation,
    lighting generation, shader compilation, source-control operations,
    semantic merge, and builds): a spinner and the current operation's title,
    with a progress bar when the operation reports progress. Clicking it opens
    the Progress window, which lists running and finished operations with their
    progress, elapsed time, and a cancel button for cancellable ones (see
    [Profiler and debugging windows](profiler-and-debugging-windows.md));
  * the lighting-generation indicator, while generation is in progress or Auto
    Generate is on (see
    [Lighting and rendering tools](lighting-and-rendering-tools.md));
  * the code optimization toggle, which switches between debug and release
    code optimization for project scripts, with its state shown by its icon
    and tooltip (DIA-002 in
    [Diagnostics, profiling, and playtesting](../diagnostics-profiling-playtesting.md));
  * the agent indicator defined in [Agent-native additions](agent-native-additions.md),
    always the last item.

During compilation, the status bar shows the compilation spinner, and the
editor remains interactive: windows scroll, select, and inspect while
compilation runs.

## Window title bar

The main window title shows the project name, the active scene's name, the
target platform, the active graphics API where the platform allows a choice,
and an asterisk when any open scene or open asset has unsaved changes. While a
Prefab Mode stage is open, the title names the prefab being edited.

## Empty and error states

* A window whose content depends on selection (Inspector, Animation window,
  Sprite Editor, and similar) shows a neutral empty state with a one-line
  statement of what it needs (for example "Select an object to inspect") and no
  stale content.
* A window provided by a project or package editor extension that has not
  compiled, has thrown, or is untrusted shows the state that RUN-010 defines,
  inside its own tab, while the rest of the workspace keeps working (see
  [Custom editor extensions](../editor-and-authoring.md#custom-editor-extensions)).
* A window that throws an internal error shows the error in place with a button
  to reopen it; other windows are unaffected.

## Shortcuts

Workspace bindings, all in the global context (see [Shortcuts](shortcuts.md)):

| Action | Binding |
|---|---|
| Maximize or restore the window under the pointer | `Shift+Space` |
| Next window | `Ctrl/Cmd+Tab` |
| Previous window | `Ctrl/Cmd+Shift+Tab` |
| Scene view | `Ctrl/Cmd+1` |
| Game view | `Ctrl/Cmd+2` |
| Inspector | `Ctrl/Cmd+3` |
| Hierarchy | `Ctrl/Cmd+4` |
| Project window | `Ctrl/Cmd+5` |
| Animation window | `Ctrl/Cmd+6` |
| Profiler | `Ctrl/Cmd+7` |
| Audio Mixer | `Ctrl/Cmd+8` |
| Console | `Ctrl/Cmd+Shift+C` |

## Agent presence

* Agent windows (Agent Activity and Change Review) are ordinary dockable windows
  under **Window > Agents**. They follow every docking, tab, floating, and
  maximize rule in this document.
* No factory layout includes an agent window, and no agent window ever opens
  in place of, moves, or resizes a window the developer arranged. The first
  time an agent session connects to a project, the Agent Activity window opens
  as the last tab of the group holding the Console (or floating, when no
  Console is docked) without taking focus; once the developer closes it, it
  stays closed until they open it again.
* The status bar's agent indicator is the always-visible entry point; its
  behaviour is defined in [Agent-native additions](agent-native-additions.md).

## Scale

* Layout save and restore cost depends on the number of windows, never on
  project or scene size.
* Restoring an arrangement opens every window before its content is populated;
  windows such as the Hierarchy and Project window show their structure and
  fill rows as the project model provides them, so a project at the
  [scale floor](../diagnostics-profiling-playtesting.md#scale-floor) shows its
  workspace without waiting for full population.
* Any number of windows can be open at once. A window that is not visible (a
  background tab or a minimized container) draws nothing and costs no frame
  time; it refreshes when it comes to the front.

## Acceptance outcomes

1. On first open of a project, the arrangement is the Default layout: Hierarchy
   left, Scene view and Game view tabbed in the center with Scene view in front,
   Inspector right, Project window and Console tabbed at the bottom with the
   Project window in front, read through the workspace-arrangement resource over
   MCP. (EDT-006)
2. Choosing each of 2 by 3, 4 Split, Tall, Wide, and Default from the Layout
   dropdown by input injection produces the window set and positions this
   document states for it, and the four Scene views of 4 Split report top,
   front, right, and perspective cameras. (EDT-006, EDT-003)
3. Dragging the Console tab by input injection to the right edge of the
   Inspector group docks it there; dragging it to an empty area floats it in a
   new container; dragging it onto the Hierarchy's center adds it as a tab
   there; each resulting arrangement matches the arrangement resource. (EDT-006)
4. Double-clicking the Scene view tab maximizes it and double-clicking again
   restores the prior arrangement; with the pointer over the Game view,
   `Shift+Space` maximizes the Game view. (EDT-006)
5. A layout saved as "Review", then modified, then chosen again restores the
   saved arrangement; it is listed in a second project opened by the same user;
   neither project reports a dirty scene, asset, or setting. (EDT-006)
6. **Save Layout to File...** followed by **Load Layout from File...** on
   another machine reproduces the arrangement. (EDT-006)
7. An arrangement naming an editor window from a package that has been
   removed restores with a placeholder tab stating the provider is missing, and
   the rest of the arrangement intact. (EDT-006)
8. An arrangement file written by a newer unitas version falls back to the
   Default layout and logs one warning naming the reason. (EDT-006)
9. **Reset All Layouts** restores factory layouts and Default, removes named
   layouts after confirmation, and leaves project content, preferences, and
   shortcut profiles unchanged. (EDT-006)
10. Clicking the status bar's last-message area opens the Console with that
    entry selected; clicking the progress indicator during an import opens the
    Progress window listing the import. (EDT-006, DIA-001)
11. When an agent session first connects, the Agent Activity window appears as
    the last tab of the Console's group, focus stays where it was, and no
    other window's position or size changes. (EDT-006, EDT-008)
12. With a scene of 100,000 GameObjects open, restoring the arrangement on
    project open shows every window and an operable Hierarchy before the
    Hierarchy has realized more than its visible rows. (EDT-006, PRF-002)
