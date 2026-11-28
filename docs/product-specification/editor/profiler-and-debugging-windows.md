# Profiler and Debugging Windows

## Scope

This document details the visual editor's analysis and debugging windows: the
Profiler, the Memory Profiler, the Physics Debugger, Import Activity, the IMGUI
Debugger, the UI Toolkit Debugger entry point, the Test Runner, the Device
Simulator, the Progress window, and the Undo History window. It details the
workspace and command behavior of EDT-006, the play-mode behavior of EDT-004,
and the undo behavior of EDT-005, all owned by
[Editor and authoring](../editor-and-authoring.md). This document owns no
catalog ID.

What a profiler capture contains and records is owned by PRF-001; logs,
diagnostics, and frame capture by DIA-001; project tests by TST-003, all in
[Diagnostics, profiling, and playtesting](../diagnostics-profiling-playtesting.md).
Development-player connections and their pairing are owned by BLD-004 in
[Builds, platforms, XR, and networking](../build-platforms-xr-networking.md).
Import state is owned by PRJ-004 in [Projects and assets](../project-and-assets.md).
This document describes the windows through which a person and an agent reach
that behavior.

## Profiler

The Profiler opens from `Window/Analysis/Profiler` (Ctrl/Cmd+7).

### Regions

* **Toolbar.** From left to right: the **Profiler Modules** drop-down, the
  target drop-down, the **Record** toggle, **Back**, **Forward**, and
  **Current** frame buttons, the frame indicator (`Frame: n / m`), **Clear**,
  the **Clear on Play** toggle, the **Deep Profile** toggle, the **Call
  Stacks** toggle with its sample-kind drop-down, **Load**, **Save**, and a
  settings menu (color-blind palette, frame count shown, preferences).
* **Chart area.** The upper region stacks one chart per active module, each
  with a title, a legend whose entries toggle their series, and a vertical
  line marking the selected frame. Clicking or dragging in a chart selects a
  frame. Each module's chart can be reordered by dragging its title and
  closed from its context menu.
* **Details pane.** The lower region shows the selected module's detail for
  the selected frame, with a splitter between it and the chart area.

### Modules

The Profiler Modules drop-down lists the built-in modules with a checkbox each
and a **Configure** item: CPU Usage, GPU Usage, Rendering, Memory, Audio,
Video, Physics, Physics (2D), UI, UI Details, Global Illumination, Virtual
Texturing, File Access, and Asset Loading, plus modules that packages and the
project declare through the profiler module API (RUN-010). Configure opens a
module editor where a person creates a custom module from named counters,
reorders modules, and restores the defaults. The module set is a per-user
preference.

* **CPU Usage** details offer three views from a drop-down: **Timeline**
  (threads as lanes, samples as nested bars on a time axis, zoom with the
  scroll wheel, pan with drag, a click selecting a sample and showing its
  name, duration, and metadata), **Hierarchy** (a tree of samples merged by
  call path with Total, Self, Calls, GC Alloc, Time ms, and Self ms columns,
  sortable by any column), and **Raw Hierarchy** (samples unmerged). A thread
  drop-down chooses the thread for the hierarchy views, a search field filters
  sample names, and a **Related Data** or **Calls** side panel shows the
  objects and callers of the selected sample. Selecting a sample in one view
  keeps it selected when switching views.
* **Memory** details offer a simple view of totals by category and a
  detailed view that takes a snapshot of the target and lists objects with
  their sizes and references.
* **Rendering**, **Audio**, **Physics**, **UI**, and the other modules show
  their counters for the selected frame as the owning specification's data
  defines.

### Target drop-down

The target drop-down lists **Play Mode**, **Edit Mode**, the local players
running on this host, connected development players by device name, and an
**Enter IP** item for a player reachable over the network. Each entry states
whether it can be profiled; a player not built as a development build, or
whose pairing credential the editor does not hold, is listed with the reason
and cannot be selected. Edit Mode profiles the editor's own frame and its
workers; Play Mode profiles the editor's play session.

### Behavior

* **Record** captures frames continuously while on. **Clear on Play** clears
  the capture when play mode is entered. With recording off, the chart keeps
  the frames already captured.
* **Back** and **Forward** move the selected frame by one; **Current** returns
  to the newest frame and follows new frames while recording. Selecting an
  older frame stops following.
* **Deep Profile** instruments every script method call. Turning it on or off
  for Play Mode takes effect on the next entry into play mode, and the toolbar
  states this while pending.
* **Call Stacks** records call stacks for the chosen sample kinds, such as
  managed allocations, and the Hierarchy and Timeline views show them for the
  selected sample.
* **Save** writes the captured frames to a profiler data file; **Load** opens
  one, replacing the current capture or, with Shift held, appending to it. A
  loaded capture names its source project, build, target, and device in the
  toolbar, as the capture record of PRF-001 defines, and two captures from
  different contexts are never shown as equivalent.
* Double-clicking a sample that maps to project script code opens the
  configured external editor at that method when the location is known.
* **Empty and error states.** Before any frame is captured, the chart area
  states that nothing is recorded and how to start. A target that drops its
  connection keeps the captured frames, marks where the connection ended, and
  states the cause.

## Memory Profiler

When the project includes the Memory Profiler package
(`com.unity.memoryprofiler`), `Window/Analysis/Memory Profiler` opens the
package's window, drawn as an editor extension (RUN-010): a snapshot list,
**Capture** against the Profiler's current target, single-snapshot views of
memory by category, type, and object, and comparison of two snapshots.
Snapshots are stored where the package stores them. Without the package, the
menu item is absent, and the Profiler's Memory module states that detailed
snapshot analysis requires the package and links to it in the Package
Manager ([Settings and preferences](settings-and-preferences.md)).

## Physics Debugger

The Physics Debugger opens from `Window/Analysis/Physics Debugger`. It has a
**Filtering** tab and a **Rendering** tab, and while open it adds a Physics
Debug overlay to the Scene view ([Scene view](scene-view.md)).

* **Filtering.** A layer mask and toggles choose which colliders draw: static
  colliders, triggers, rigidbodies, kinematic bodies, sleeping bodies, and
  collider types (box, sphere, capsule, mesh convex, mesh concave, terrain).
  A **Select None** and **Select All** pair resets the toggles.
* **Rendering.** Colors for each body state, transparency, force-overdraw,
  and the option to show contacts and queries.
* **Overlay.** Collision Geometry and Mouse Select toggles; with Mouse Select
  on, clicking a drawn collider selects its GameObject.
* The same collider visualization is available for 2D physics through the
  Physics 2D settings' gizmo options. The data drawn is owned by PHY-001 and
  PHY-002 in [Physics, navigation, and terrain](../physics-navigation-terrain.md).

## Import Activity

Import Activity opens from `Window/Analysis/Import Activity`, and from an
asset's context menu item **View in Import Activity Window** in the
[Project window](project-window.md), which opens it with that asset selected.

* **Asset list.** One row per asset with its last import time, import
  duration, and importer. Columns sort; a search field filters by path.
* **Details pane.** For the selected asset: its GUID, asset size, path,
  importer and version, the reasons for its most recent import (for example
  its file changed, a dependency changed, its importer version changed), and
  its dependencies (source files, other assets, build target, custom
  dependencies), each navigable.
* **Overview.** With nothing selected, the pane lists the assets with the
  longest imports and the assets with the most dependencies.
* **Show previous imports** lists earlier import revisions of the selected
  asset where they are retained.

## IMGUI Debugger

The IMGUI Debugger opens from `Window/Analysis/IMGUI Debugger`. A drop-down
chooses an open window, including an editor extension window. The left pane
lists that window's immediate-mode drawing instructions for its last repaint
(layout groups, controls, clips), and selecting one highlights its rectangle
in the inspected window and shows its rect, style, content, and calling stack
when recorded. A **Pick** mode selects the instruction under the pointer in
the inspected window. Extensions are inspected in the worker that runs them;
the debugger reads their published instructions.

## UI Toolkit Debugger

The UI Toolkit Debugger (`Window/UI Toolkit/Debugger`) inspects any panel's
visual tree, styles, and layout. It is defined in [UI authoring](ui-authoring.md).

## Test Runner

When the project includes the Test Framework package
(`com.unity.test-framework`), `Window/General/Test Runner` opens the package's
window, drawn as an editor extension (RUN-010). Discovery, execution, and
results are owned by TST-003.

* **Tabs.** EditMode and PlayMode.
* **Toolbar.** **Run All**, **Run Selected**, **Rerun Failed**, a search field,
  a category drop-down, and outcome filter toggles with counts (passed,
  failed, not run). On the PlayMode tab, a **Run All in Player** drop-down
  lists the build targets to build a test player for.
* **Tree.** Assemblies, namespaces, fixtures, and tests, each with an outcome
  icon; a parent shows the worst outcome of its children. Double-clicking a
  test opens its source in the external editor at the test method. The
  context menu offers **Run**, **Open source code**, and, for a selection of
  several, run of the selection.
* **Result pane.** The selected test's message, duration, and stack trace;
  each stack frame with a file and line is a link that opens the external
  editor there, as in the [Console](console.md).
* **States.** A test assembly that fails to compile shows its tests as not
  runnable with the reason. While a run executes, the tree marks the running
  test and the window offers **Cancel**; a run that stops early marks the
  remainder as not run, never passed.

Without the package, the menu item is absent, and the Package Manager lists
the package for installation.

## Device Simulator

The Game view's toolbar drop-down switches between **Game** and **Simulator**
([Game view and play mode](game-view-and-play-mode.md)). In Simulator:

* the toolbar holds a device drop-down, a scale slider, **Fit to Screen**,
  **Rotate** left and right, a **Safe Area** toggle, and the play-mode focus
  drop-down;
* the device's screen draws inside its bezel at the chosen scale, the safe
  area outlines when shown, and pointer input over the screen arrives as
  touch input;
* a **Control Panel** beside the device shows application settings
  (orientation, system language, internet reachability) and the simulated
  screen's resolution, density, and orientation;
* device definitions come from the built-in set and from device definition
  assets in the project and packages; a malformed definition is listed with
  its error and is not selectable.

Simulation never claims to substitute for target hardware behavior, as the
Game view's requirements state.

## Progress window

The Progress window opens from `Window/General/Progress` and by clicking the
progress bar at the right of the status bar ([Workspace and layout](workspace-and-layout.md)).

* One row per background task (imports, compilation, bakes, builds, package
  resolution, source-control operations, semantic merges, searches being
  indexed, project-declared progress items) with name, description, progress
  bar or indeterminate state, elapsed time, and remaining-time estimate when
  known.
* Tasks with sub-tasks expand to show them. A task that can be cancelled
  shows a **Cancel** button; one that cannot shows none.
* Filter toggles show active, succeeded, failed, and cancelled tasks; **Clear
  inactive** removes finished rows. A failed task's row shows its error and
  links to the matching [Console](console.md) entry.
* The status bar's progress bar shows the most important active task and the
  number of others.

## Undo History window

The Undo History window opens from `Edit/Undo History` and lists the undo
history (EDT-005) of the editor.

* Entries are listed oldest at top, newest at bottom. The entry that is the
  current state is highlighted; entries after it are the redo branch and draw
  dimmed.
* Each entry shows the operation's name, as in the Edit menu's Undo item
  (for example `Undo Move Cube`), and the actor that made it: the person, or
  an agent session with its agent badge.
* Clicking an entry undoes or redoes to that point as successive operations.
  Choosing an entry by an agent session for a person's undo follows the
  person-undo rule of EDT-005, which applies to the most recent operation
  regardless of actor.
* A search field filters entries by name and actor. An abandoned redo branch,
  once replaced by a new edit, is shown as discarded at the point where it
  diverged.
* An empty history states that there is nothing to undo.

### Shortcuts

Window-scoped shortcuts appear under these Shortcuts Manager contexts and are
rebindable there ([Shortcuts](shortcuts.md)):

| Context | Action | Default |
|---|---|---|
| Global | Open Profiler | Ctrl/Cmd+7 |
| Profiler | Previous frame | Left |
| Profiler | Next frame | Right |
| Profiler (CPU Timeline) | Frame selected sample | F |
| Profiler (CPU Timeline) | Show whole frame | A |
| Test Runner | Run selected | Enter |
| Undo History | Move to previous or next entry | Up, Down |

The Physics Debugger, Import Activity, IMGUI Debugger, Progress, and Undo
History windows have no global default shortcut. Undo and redo themselves
keep their global bindings ([Shortcuts](shortcuts.md)).

### Agent presence

* A profiler capture, memory snapshot, or test run started by an agent session
  shows that session's agent badge in the window's toolbar while it runs and
  on its result afterwards; the hover names the agent session and request,
  and a click opens the Agent Activity window at that operation
  ([Agent-native additions](agent-native-additions.md)).
* An agent session's capture of the editor's own Play Mode or Edit Mode is
  visible to the person as a recording state in the Profiler; captures of an
  isolated play session (AUT-004) or isolated proposal environment belong to
  that session's worker and appear under their own target entry, never
  mixed with the person's capture.
* The Progress window lists agent-started tasks with the badge, and a person
  can cancel them.
* The Undo History window marks each agent session's entries with its badge,
  and a click on the badge opens the Change Review window filtered to that
  operation.
* Agent presence adds badges only; it never moves, hides, or rebinds a control
  of these windows.

### Scale

* The Profiler's Hierarchy and Raw Hierarchy views realize only visible rows,
  and the Timeline draws only the visible time range, so captures with many
  threads and samples per frame remain navigable; aggregation runs off the
  main thread with a loading state in the details pane.
* Import Activity, Test Runner, Progress, and Undo History realize only
  visible rows; Import Activity and Test Runner search and sort through
  indexes and stay responsive in projects at the
  [scale floor](../diagnostics-profiling-playtesting.md#scale-floor).
* Loading a large profiler data file shows progress and can be cancelled.

## Acceptance outcomes

1. Ctrl/Cmd+7 opens the Profiler; with Record on and Play Mode targeted,
   entering play mode with Clear on Play on clears the previous capture and
   records new frames; an agent starts and stops the same capture through MCP
   and reads it as structured data. (EDT-004, EDT-006, PRF-001)
2. Selecting a frame in the CPU Usage chart and switching between Timeline,
   Hierarchy, and Raw Hierarchy keeps the selected sample, and sorting the
   Hierarchy by Self ms orders rows by that column. (EDT-006, PRF-001)
3. Turning Deep Profile on during play mode states that it applies on the next
   entry, and after re-entry the capture includes script method samples.
   (EDT-004, PRF-001)
4. Saving a capture and loading it shows the same frames and names the
   capture's source project, target, and device. (EDT-006, PRF-001)
5. The target drop-down lists a release player as not profilable with the
   reason, and a development player built with profiling as selectable.
   (EDT-006, BLD-004)
6. A Profiler Modules configuration that hides the Audio module persists for
   the user across reopening the project and does not dirty project content.
   (EDT-006)
7. In a project with the Test Framework package, Run All on the EditMode tab
   runs every EditMode test, a failed test's stack-trace link opens the
   external editor at the reported file and line, and Rerun Failed runs only
   the failed tests; without the package, the menu item is absent. (EDT-006,
   TST-003)
8. Choosing View in Import Activity Window on an asset opens the window with
   that asset's last import reasons and dependencies. (EDT-006, PRJ-004)
9. Switching the Game view to Simulator, choosing a device, and rotating it
   changes the simulated screen orientation and resolution reported to
   scripts. (EDT-004, EDT-006)
10. A running bake and a running build each appear as rows in the Progress
    window with progress and Cancel, and clicking the status bar's progress
    bar opens the window. (EDT-006)
11. After a person's edit and an agent session's edit, the Undo History window
    lists both with their actors, and clicking the oldest entry undoes both as
    successive operations. (EDT-005, EDT-008)
12. A profiler capture started by an agent session shows that session's badge
    in the Profiler toolbar and in the Progress window. (EDT-008, AUT-003)
