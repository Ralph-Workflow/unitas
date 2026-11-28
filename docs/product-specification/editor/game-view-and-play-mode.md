# Game View and Play Mode

## Scope

This document details the Game view and the editor's play mode: the play
controls, the Game view's toolbar and overlays, input focus, the play-mode
tint, and what happens to edits on entry and exit. It refines EDT-004 (game
view, play, pause, step, and edit-state restoration), owned by
[Editor and authoring](../editor-and-authoring.md#play-mode); this document
owns no catalog ID. Domain and scene reload on entry follow RUN-003 in
[Scripting runtime](../scripting-runtime.md#domain-reload-and-enter-play-mode-options),
cameras and displays follow REN-001 and REN-007 in
[Rendering and animation](../rendering-and-animation.md), and agent play
sessions isolated from the editor's play mode follow AUT-004 in
[Automation and MCP](../automation-and-mcp.md).

## Play controls

The play controls are three buttons centered in the main toolbar (see
[Menus and toolbar](menus-and-toolbar.md)): Play, Pause, and Step. They are
also Edit > Play, Edit > Pause, and Edit > Step.

| Control | Binding | Behavior |
|---|---|---|
| Play | Ctrl/Cmd+P | Enters play mode; pressed again, exits it. The button shows the pressed (active) state while playing. |
| Pause | Ctrl/Cmd+Shift+P | Toggles pause. Usable before entering play mode, in which case play mode starts paused after its first frame. |
| Step | Ctrl/Cmd+Alt+P | While paused, advances exactly one frame and pauses again. Outside play mode, enters play mode paused and advances one frame. |

* Play, pause, and step states are unambiguous: each button shows its own
  pressed state, and the Game view tab shows the paused state.
* Exiting play mode is available while paused.
* Entering play mode is refused while compiler errors exist; the Game view
  and status bar state that all compiler errors must be fixed first, and the
  Console highlights the first error ([Console](console.md)).
* Entering play mode while scripts are compiling waits for compilation, shows
  progress in the status bar, and then enters.
* The editor distinguishes failure to enter play mode, a running game with no
  camera rendering to the shown display (stated in the Game view), a paused game, and a stopped game (EDT-004).

## Entering play mode

* Play mode runs the scenes as they are currently open in the editor,
  including unsaved changes, without prompting to save. It never mixes an
  open scene's saved file with its unsaved editor state.
* The open scene set, the active scene, and the selection carry into play
  mode. Objects in the Hierarchy remain selectable and inspectable.
* The project's Enter Play Mode Options decide whether the scripting domain
  and the scenes reload (RUN-003); with both reloads disabled, entry skips
  them and the status bar states that domain reload is disabled.
* Proposed agent changes held under the require-approval policy are excluded
  from the play session unless the person is previewing them, in which case
  the Game view tab and the play-mode tint indicate that the session includes
  the previewed proposal (EDT-008).
* Unless Maximized or focus settings say otherwise (see
  [Input focus and cursor](#input-focus-and-cursor)), entry brings the Game view to the
  front of its tab group and gives it input focus.

## Play-mode tint

While in play mode, every editor window's background and toolbar are tinted
with the play-mode tint color, set in Preferences > Colors > Playmode tint
([Settings and preferences](settings-and-preferences.md)). The tint is the
standing indication that edits are temporary. It disappears on exit.

## Edits during play mode

* Every edit in the Hierarchy, Inspector, and Scene view during play mode
  changes the play session's live objects only. On exit, every scene object
  returns to its edit-time state, and the selection returns to the edit-time
  objects it identified. Edits to assets (materials, ScriptableObjects,
  prefab assets, project settings) are not reverted, because they change the
  asset itself; they are saved as ordinary asset edits.
* Saving a scene during play mode is refused with a notice that scenes cannot
  be saved during play mode and that scene changes made in play mode are lost
  when it ends.
* To keep a value, the person copies the component during play mode (Copy
  Component) and, after exiting, uses Paste Component Values on the edit-time
  component, which forms one undo operation ([Inspector](inspector.md)).
* Undo during play mode undoes play-mode edits; play-mode edits are removed
  from the undo history on exit.
* Objects moved to the persistent scene through `DontDestroyOnLoad` appear in
  the Hierarchy under a DontDestroyOnLoad scene header during play mode and
  are destroyed on exit.
* Objects created at runtime appear in the Hierarchy and are inspectable; they
  have no prefab or save controls.

## Pause and step

* Pausing freezes gameplay progression at the end of the current frame. The
  Scene view, Hierarchy, Inspector, and other views stay fully operable; the
  Game view keeps showing the last rendered frame and redraws it when the
  person changes something that affects rendering.
* Step advances one eligible rendered frame with its runtime work (EDT-004).
* **Error Pause** in the Console pauses at the end of the frame in which an
  error or exception is logged. Exceptions never exit play mode
  ([Console](console.md)).
* `Debug.Break()` pauses as Error Pause does.

## Game view

### Regions

1. **Toolbar** across the top.
2. **Display area**, showing the chosen display's output scaled into the view,
   with letterboxing outside the chosen aspect ratio.
3. **Overlays** inside the display area: the Stats panel and gizmos when
   enabled.

### Toolbar

Left to right:

* **View type switch** — Game or Simulator. Simulator replaces the display
  area with the device simulator, showing a chosen device's screen shape,
  safe area, and orientation; it keeps the same play controls.
* **Display dropdown** — Display 1 through Display 8; shows the output of the
  cameras targeting that display. Displays that the project uses are marked.
* **Aspect and resolution dropdown** — Free Aspect (fills the view), the
  standard aspect ratios (5:4, 4:3, 3:2, 16:10, 16:9), the standalone player
  resolution, the active target platform's entries, and custom entries. The +
  button at the bottom of the list adds a custom entry with a label, a type
  (Aspect Ratio or Fixed Resolution), width, and height; custom entries can be
  edited and removed by right-click. The last entry toggles **Low Resolution
  Aspect Ratios**, which renders at a reduced resolution matching low-density
  displays.
* **Scale slider** — zooms the display area; at a scale greater than one the
  view can be panned by scrolling, and at the minimum scale the whole
  resolution fits.
* **Play-mode focus dropdown** — **Play Focused** (the Game view takes focus
  on entry), **Play Maximized** (the Game view maximizes to the whole editor
  window on entry and restores the layout on exit), and **Play Unfocused**
  (entry does not change focus). The choice persists per Game view.
* **Frame Debugger** — opens the Frame Debugger and enables it, pausing at the
  current frame ([Lighting and rendering tools](lighting-and-rendering-tools.md)).
* **Mute Audio** — mutes all audio output of the play session.
* **Stats** — toggles the Stats overlay: frame rate and frame time, CPU main
  and render thread time, batches, saved-by-batching count, triangles,
  vertices, screen resolution, SetPass calls, shadow casters, visible skinned
  meshes, and audio level and clipping. The values are measured from the
  play session's rendered frames.
* **Gizmos** — the toggle draws gizmos in the Game view; the dropdown arrow
  opens the same gizmo menu as the Scene view (per-type toggles, icon size, 3D
  icons) and its choices are independent of the Scene view's
  ([Scene view](scene-view.md)).

The Game view's window menu adds VSync (Game view only) and Warn if No
Cameras Rendering.

### Input focus and cursor

* While the Game view has focus during play mode, keyboard, mouse, gamepad,
  and touch input reach the game (UI-001 and INP-001/INP-002 in
  [UI, audio, and input](../ui-audio-input.md)). Clicking in the Game view
  gives it focus; clicking another view takes focus away, and the game stops
  receiving keyboard input.
* The Game view tab and border state whether it currently owns gameplay input
  (EDT-004).
* When the game locks or hides the cursor, Esc releases the cursor and focus
  so the person can use the editor; clicking the Game view again restores the
  game's cursor state.
* Editor shortcuts that are bound in the Game view's context yield to game
  input while the Game view has focus during play mode; the global play
  controls (Ctrl/Cmd+P, Ctrl/Cmd+Shift+P, Ctrl/Cmd+Alt+P) always reach the
  editor.

### Multiple Game views

More than one Game view can be open (Add Tab > Game). Each has its own
display, aspect, scale, and overlays. Play Maximized maximizes the Game view
that was focused last.

### Edit mode

Outside play mode, the Game view renders the chosen display's cameras from the
edit-time scene, so camera placement is visible before playing. With no camera
targeting the chosen display, it states that no cameras are rendering.

### Shortcuts

Shortcuts Manager contexts: **Main Menu** (play controls) and **Game View**.

| Action | Binding |
|---|---|
| Play | Ctrl/Cmd+P |
| Pause | Ctrl/Cmd+Shift+P |
| Step | Ctrl/Cmd+Alt+P |
| Open or focus Game view | Ctrl/Cmd+2 |
| Maximize hovered view | Shift+Space |
| Release locked cursor | Esc |

Mute Audio, Stats, and Gizmos have no default binding and are bindable in the
Shortcuts Manager ([Shortcuts](shortcuts.md)).

### Agent presence

* The play controls show the editor's play session only. Agent play sessions
  (AUT-004) run isolated from it and appear in the Agent Activity window, not
  in the toolbar, the Game view, or the tint.
* When an agent session controls the editor's own play session through MCP
  (entering, pausing, stepping, or injecting input), the Game view tab shows
  the agent badge for the duration of that control, and the status bar names
  the session; the person's play controls stay operative and take precedence.
* Input injected by an agent session into the Game view is drawn as a
  transient pointer marker in the session color.
* A Game view capture requested through MCP (AUT-002 visual evidence) returns
  the same pixels that the display area shows, without the Stats overlay or
  gizmos unless the request includes them (EDT-004).

### Scale

A scene at the scale floor of 100,000 GameObjects enters play mode within the
declared budget and plays within the declared frame-time budget
([scale floor](../diagnostics-profiling-playtesting.md#scale-floor)). Exiting
play mode restores edit-time state by identity without re-reading the scene
files when no reload is required, and the Hierarchy, which realizes only
visible rows, remains responsive while runtime objects are created and
destroyed every frame.

## Acceptance outcomes

1. Ctrl/Cmd+P enters play mode, the play-mode tint appears on every window,
   and Ctrl/Cmd+P again exits and removes the tint. (EDT-004)
2. Moving an object in the Scene view during play mode and exiting returns the
   object to its edit-time position, while an edit to a material asset made in
   play mode remains. (EDT-004)
3. Pressing Ctrl/Cmd+Shift+P pauses the game; Ctrl/Cmd+Alt+P advances exactly
   one frame, observable through `Time.frameCount` read through MCP. (EDT-004)
4. Entering play mode with an unsaved scene change runs the changed content
   without any save prompt, and the scene's file is unchanged afterwards.
   (EDT-004, EDT-005)
5. Attempting Ctrl/Cmd+S during play mode refuses the save with the notice that
   scenes cannot be saved in play mode. (EDT-004, EDT-005)
6. With Play Maximized chosen, entering play mode maximizes the Game view and
   exiting restores the prior layout. (EDT-004, EDT-006)
7. Choosing 16:9 in the aspect dropdown letterboxes the display area to 16:9,
   and a custom Fixed Resolution entry added with + renders at that
   resolution. (EDT-004)
8. Enabling Stats shows the overlay, and a Game view capture through MCP
   without the overlay option excludes it. (EDT-004, AUT-002)
9. A game that locks the cursor releases it when Esc is pressed, and clicking
   the Game view restores the lock. (EDT-004)
10. With compiler errors present, Play is refused and the Console highlights the
    first error. (EDT-004)
11. An object marked `DontDestroyOnLoad` appears under its own scene header in
    the Hierarchy during play mode and is gone after exit. (EDT-004, EDT-001)
12. With an agent's change held for approval and not previewed, entering play
    mode runs without it; previewing it and entering again runs with it.
    (EDT-004, EDT-008)
13. An agent session entering the editor's play mode through MCP shows its badge
    on the Game view tab, and the person's Ctrl/Cmd+P still exits play mode.
    (EDT-004, AUT-003)
