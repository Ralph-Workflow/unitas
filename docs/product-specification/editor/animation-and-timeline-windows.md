# Animation, Animator, and Timeline Windows

## Scope

This document details the editor windows for authoring animation clips,
animator controllers, and sequences: the Animation window, the Animator
window, and the Timeline window. It details EDT-002 (inspector and serialized
property editing), EDT-003 (scene visualization during preview), EDT-004
(play-mode live views), EDT-005 (undo), and EDT-006 (workspace views,
commands, shortcuts, and drag and drop), which are owned by
[Editor and authoring](../editor-and-authoring.md). This document owns no
catalog ID.

The content these windows edit is owned elsewhere: clips, curves, events,
controllers, states, and blend trees by ANI-001; avatars, masks, and playable
graphs by ANI-002; sequences and cinematic tracks by CIN-001, all in
[Rendering and animation](../rendering-and-animation.md). Where this document
states what a value means at runtime, the owning specification governs.

The Animation and Animator windows are built-in windows. The Timeline window
belongs to the Timeline package (`com.unity.timeline`): it is the package's
own editor window, drawn and run as an editor extension
([Custom editor extensions](../editor-and-authoring.md#custom-editor-extensions),
RUN-010 in [Scripting runtime](../scripting-runtime.md)), and appears at its
menu path only when the project includes the package.

Every window here docks, groups as tabs, floats, and maximizes as defined in
[Workspace and layout](workspace-and-layout.md). Every menu item, context menu
item, and toolbar action is a command and an MCP tool titled by its path, as
defined in [Menus and toolbar](menus-and-toolbar.md) and
[Agent-native additions](agent-native-additions.md); graph and keyframe edits
(add key, move keys, create state, make transition, add track) are commands
with the same results for a person and an agent.

## Animation window

Opened from Window > Animation > Animation (Ctrl/Cmd+6). It edits the clips
of the animation root that contains the current selection: the nearest
GameObject at or above the selection that carries an Animator or legacy
Animation component.

### Regions

* **Toolbar.** Preview toggle; Record toggle; go to first keyframe; previous
  keyframe; play; next keyframe; go to last keyframe; current frame field;
  clip dropdown; Samples field when Show Sample Rate is enabled; Add Keyframe
  button; Add Event button; Dopesheet and Curves view switch at the bottom of
  the property list.
* **Property list.** One row per animated property, grouped by object path
  and component, with a foldout for vector, color, and rotation properties.
  Each row shows the property's current value as an editable field and a
  keyframe toggle that adds or removes a key at the current frame. Rows whose
  binding is missing, ambiguous, incompatible, or on an inactive object show
  that state in the row and keep their keys (ANI-001). Add Property opens a
  searchable popup of the animatable components and properties of the root
  and its descendants, each with an add button.
* **Time ruler.** Shows time as frames or as seconds and frames, chosen in the
  window menu. Clicking or dragging in the ruler moves the playhead and
  previews that time.
* **Events lane.** Below the ruler. Event markers sit at their times.
* **Key area.** The Dopesheet or Curves view, sharing the playhead.

### Clip dropdown

Lists every clip referenced by the root's controller or legacy Animation
component, and ends with Create New Clip..., which asks for a save location
and adds the new clip to the controller as a new state on the base layer.
Clips that are sub-assets of imported models are marked read-only; their keys
are visible and selectable but not editable, and the window states that
editing requires duplicating the clip.

### Empty states

* With nothing selected, the window states that a GameObject must be
  selected to animate.
* With a GameObject selected that has no Animator or Animation component, the
  window shows a Create button. Pressing it adds an Animator to the object,
  asks for a save location for a new clip, creates a controller beside it,
  and opens the clip for editing, as one undo operation.
* With an Animator that has no controller, or a controller with no clips, the
  window offers Create New Clip....

### Preview and record

* **Preview** samples the clip at the playhead onto the scene. Previewed
  values appear in the Scene view and Inspector with the preview tint, and
  they never become saved scene values: leaving preview, closing the window,
  changing selection to another root, saving, or entering play mode restores
  the edit-time values (ANI-001).
* **Record** turns on preview and tints the time ruler and the Record button
  with the recording tint (red). While recording, every change made to an
  animatable property of the root or its descendants, from the Inspector, the
  Scene view tools, or an agent's edit, writes a key at the current frame
  instead of changing the scene value. Properties that receive keys show the
  recording tint in the Inspector. Changes to non-animatable properties apply
  to the scene as ordinary edits.
* Entering play mode ends record and preview.

### Dopesheet

* Keys show as diamonds per property row, with a summary row at the top that
  aggregates every key at each time.
* Click selects a key; Shift-click adds; Ctrl/Cmd-click toggles; dragging in
  empty space draws a selection rectangle; clicking a summary key selects
  every key at that time.
* Dragging selected keys moves them in time, snapped to whole frames.
* A multi-key selection shows a box with handles on its left and right edges;
  dragging a handle scales the selection in time about the opposite edge.
* Double-click in a row adds a key at that time with the property's
  evaluated value.
* Context menu on keys: Add Key, Delete Keys, Clamped Auto, Auto, Free Smooth,
  Flat, Broken, Left Tangent, Right Tangent, and Both Tangents (each with
  Free, Linear, and Constant). Context menu on empty space: Add Key, Paste.
* Ctrl/Cmd+C and Ctrl/Cmd+V copy and paste keys; pasted keys land at the
  playhead, keeping their relative times and their property bindings.

### Curves

* Each selected property's curves draw in distinct colors, one color per
  component of a vector or color. Selecting rows in the property list chooses
  which curves show.
* Selecting a key shows its tangent handles. Dragging a key moves it in time
  and value; dragging a tangent handle changes the key's tangent mode to the
  broken or free form the drag implies.
* Context menu on keys offers the Dopesheet's tangent items plus Edit Key...,
  which opens a field popup for an exact time and value.
* The Normalize toggle displays each curve in its own normalized value range
  without changing its keys.
* Pre- and post-wrap shapes of the curve draw outside the clip range.

### Events

* Add Event places an event at the playhead. Right-click in the events lane
  offers Add Animation Event, Delete Animation Event, Copy, and Paste.
* Dragging a marker moves it; dragging several selected markers moves them
  together.
* Selecting a marker shows it in the Inspector: a function dropdown listing
  the public methods of the root's components that the event can call, and
  the parameter fields that method's signature accepts (float, int, string,
  object reference). A function name that no component on the root provides
  is shown as unresolved and kept.

### Playback and timing

* Play previews the clip looping at its sample rate in edit mode. The clip's
  sample rate is edited in the Samples field and changes the frame grid
  without moving keys.
* The playhead and the current frame field stay in step with the Timeline
  window when the Timeline window is recording or previewing an animation
  track for the same root.

### Selection behavior

* Selecting a descendant of the root in the Hierarchy keeps the root's clip
  open and highlights the descendant's properties.
* The window menu's Filter by Selection shows only the properties of the
  selected objects.
* Selecting a state in the Animator window whose motion is a clip makes that
  clip current in the Animation window.
* The window has a lock in its tab menu; a locked window keeps its root when
  selection changes.

### Shortcuts

Context: Animation.

| Action | Binding |
|---|---|
| Play or stop preview playback | Space |
| Previous frame / next frame | `,` / `.` |
| Previous keyframe / next keyframe | Alt+`,` / Alt+`.` |
| First keyframe / last keyframe | Shift+`,` / Shift+`.` |
| Key selected properties | K |
| Key modified properties | Shift+K |
| Toggle Dopesheet and Curves | C |
| Frame selected keys | F |
| Frame all keys | A |
| Delete selected keys | Delete (Cmd+Backspace on macOS) |

These bindings apply while the Animation window has focus and do not shadow
global bindings elsewhere. They are listed and rebindable in the Shortcuts
Manager ([Shortcuts](shortcuts.md)).

## Animator window

Opened from Window > Animation > Animator. It edits the controller of the
selected GameObject's Animator, or the controller asset selected in the
Project window.

### Regions

* **Side panel** with two tabs, Layers and Parameters, collapsible from the
  toolbar's eye toggle.
* **Breadcrumb bar** at the top of the graph, naming the layer and each
  sub-state machine or blend tree entered; clicking an element returns to it.
* **Graph area** showing states, sub-state machines, the Entry, Exit, and Any
  State nodes, and transitions as arrows. The layer's default state is marked
  in its own color, and Entry, Exit, and Any State each have a distinct color.
* **Window menu** with Auto Live Link and a lock toggle that keeps the window
  on the current controller when selection changes.

### Layers tab

* Add (+) appends a layer. Rows reorder by drag; the first layer is the base
  layer.
* Each row shows the layer name, rename by double-click, and markers for
  sync, IK pass, and avatar mask when those are set.
* The gear opens the layer settings: Weight (fixed for the base layer),
  Mask, Blending (Override or Additive), Sync with a source layer and Timing,
  and IK Pass. Context menu: Delete.

### Parameters tab

* Add (+) offers Float, Int, Bool, and Trigger. A new parameter enters rename
  immediately. A search field filters by name.
* Each row shows the parameter's default value in edit mode, and its live
  value during play mode, editable in both. Rows reorder by drag. Context
  menu: Rename, Delete.
* Deleting a parameter used by conditions or blend trees lists those uses
  before the deletion proceeds; the deletion is one undo operation.

### Graph editing

* Right-click on empty space: Create State (Empty, From Selected Clip, From
  New Blend Tree), Create Sub-State Machine, Paste.
* Right-click on a state: Make Transition, Set as Layer Default State, Copy,
  Create new BlendTree in State, Delete. Right-click on Any State or Entry:
  Make Transition.
* Make Transition attaches an arrow to the pointer; clicking a target state,
  sub-state machine, or Exit creates the transition. Escape cancels.
* Dragging a clip from the Project window into the graph creates a state with
  that clip as its motion. Dragging a controller onto the graph is rejected
  with the reason.
* Click selects; Shift-click and Ctrl/Cmd-click add and toggle; dragging in
  empty space draws a selection rectangle; dragging selected nodes moves them.
  Middle-drag or Alt-drag pans; scroll zooms about the pointer.
* Double-click on a sub-state machine or a blend-tree state enters it and adds
  it to the breadcrumb.
* Several transitions between the same two nodes draw as one arrow marked as
  multiple; selecting it lists each in the Inspector.

### State, transition, and blend-tree inspectors

* **State.** Name, tag, Motion, Speed with an optional multiplier parameter,
  Motion Time with an optional parameter, Mirror, Cycle Offset, Foot IK, Write
  Defaults, the ordered list of outgoing transitions with Solo and Mute
  toggles, and Add Behaviour, which searches `StateMachineBehaviour` scripts
  and offers creating a new one.
* **Transition.** Has Exit Time, Exit Time, Fixed Duration, Transition
  Duration, Transition Offset, Interruption Source (None, Current State, Next
  State, Current State Then Next State, Next State Then Current State),
  Ordered Interruption, and the Conditions list with add and remove. The
  transition timeline shows the source and destination bars and the blend
  region; dragging them edits exit time, duration, and offset. The preview
  plays the transition on the selected model.
* **Blend tree.** Blend Type (1D, 2D Simple Directional, 2D Freeform
  Directional, 2D Freeform Cartesian, Direct), its parameters, and the motion
  list with thresholds or positions, Automate Thresholds, Compute Thresholds,
  time scale, and mirror per motion. 2D types show the blend diagram; dragging
  its point previews the blend at that parameter value. Inside the blend tree
  graph, each motion node shows its current weight as a bar.

### Play-mode live view

* With a GameObject that has an Animator selected during play mode, the graph
  shows that instance: the active state and the transition in progress are
  highlighted, each active state shows a normalized-time progress bar, and
  parameters show live values (ANI-001).
* With Auto Live Link enabled, entering and leaving states follows into
  sub-state machines automatically.
* Editing the controller during play mode edits the asset and is not reverted
  when play mode exits, because the controller is an asset, not scene state;
  the window states this while play mode is running.

### Shortcuts

Context: Animator.

| Action | Binding |
|---|---|
| Frame selected nodes | F |
| Frame all nodes | A |
| Delete selected nodes or transitions | Delete (Cmd+Backspace on macOS) |
| Copy, paste | Ctrl/Cmd+C, Ctrl/Cmd+V |
| Cancel Make Transition | Escape |

## Timeline window

Opened from Window > Sequencing > Timeline when the project includes the
Timeline package. It edits the sequence asset of the selected GameObject's
`PlayableDirector`, or a sequence asset selected in the Project window.

### Regions

* **Toolbar.** Preview toggle, go to start, previous frame, play, next frame,
  go to end, play range toggle, current time field, the sequence name, the
  add-track menu, edit mode (Mix, Ripple, Replace), snapping, the lock, and
  the settings menu (time display in seconds, frames, or timecode; frame rate;
  audio waveforms; snapping options).
* **Track list** with each track's header: name, binding field, mute, lock,
  and, for animation tracks, a record button. Track groups fold.
* **Clip area** with a time ruler, the playhead, clips on their tracks, and a
  markers lane.

### Behavior

* With a GameObject selected that has no `PlayableDirector`, the window
  offers Create, which adds a director and asks for a save location for a new
  sequence asset, as one undo operation.
* Add-track menu lists Track Group, Activation, Animation, Audio, Control,
  Signal, and Playable tracks, plus track types that project code and other
  packages declare.
* Dragging a GameObject from the Hierarchy onto a track's binding field binds
  it. Dragging a GameObject onto empty track space offers the track types that
  can bind to it. Binding an animation track to an object without an Animator
  offers to add one.
* Dragging an animation clip, audio clip, or prefab from the Project window
  onto the clip area creates a clip on a compatible track, creating the track
  when needed.
* Clips move by drag, trim by their edges, blend by overlapping in Mix mode,
  and push later clips in Ripple mode. Selecting a clip shows its timing,
  extrapolation, blend curves, and playable properties in the Inspector.
* Recording on an animation track uses the same record behavior as the
  Animation window: changes to bound properties write keys into the track's
  recorded clip, with the recording tint on the track.
* Scrubbing or playing in preview evaluates the sequence onto the scene
  without persisting preview state (CIN-001); leaving preview restores
  edit-time values.
* Right-click on the markers lane: Add Signal Emitter and Add Marker. Adding a
  signal emitter whose bound object has no Signal Receiver offers to add one.
* Double-clicking a control track clip that references a nested sequence
  enters it, with a breadcrumb back to the parent.
* A binding whose target is missing names the track and the intended target
  (CIN-001).

### Built-in services the package relies on

The Timeline window runs on the editor services that the editor provides to
every extension, and those services behave here as they do elsewhere: playable
graph evaluation and preview (ANI-002), animation recording mode, object
fields with drag and drop from the Hierarchy and Project window, the
Inspector for clip and track properties, undo (EDT-005), Scene view repaint
during scrubbing, audio preview, window docking, and the package's declared
shortcuts, which appear in the Shortcuts Manager under the package's Timeline
context exactly as the package declares them.

### Package not installed

When the project does not include the Timeline package, Window > Sequencing >
Timeline is absent, and sequence assets and `PlayableDirector` components show
the missing-script state defined by SER-004 in
[Serialization and prefabs](../serialization-and-prefabs.md), preserved
unchanged on save. The Package Manager lists the package for adding.

## Agent presence

* Clips, controller states, transitions, parameters, layers, and timeline
  tracks and clips that an agent session is changing carry the agent badge
  (session color and label). Hovering a badge shows the session and its
  current operation; clicking it opens the Change Review window filtered to
  that item ([Agent-native additions](agent-native-additions.md)).
* Keys an agent adds or moves appear in the Dopesheet and Curves as they are
  written. Under the require-approval policy, proposed keys, states, and
  transitions show in their proposal style while previewed and are absent
  otherwise (EDT-008).
* Context menus gain only a trailing Review Agent Changes group when the
  target has agent changes. No familiar control is moved, renamed, or
  rebound.

## Agent operation

* The Animation window's state (root, clip, playhead, preview and record
  state, visible properties, key selection) and the Animator window's state
  (controller, layer, breadcrumb, selection, live state during play) are
  readable through MCP.
* Keyframe, curve, event, state, transition, parameter, layer, blend-tree,
  track, and clip edits are commands whose results name the affected clip,
  controller, or sequence and the changed items, and each forms one undo
  operation attributed to its actor.
* Recording through MCP is available: an agent turns on record mode and sets
  properties, and the result is the same keys a person's edits produce.
* Timeline window controls are exposed as extension elements
  ([Extension controls through MCP](../editor-and-authoring.md#extension-controls-through-mcp)).

## Scale

* The property list, the Dopesheet rows, and the Parameters and Layers lists
  are virtualized: only visible rows are realized, and a clip with many
  curves or a controller with many parameters scrolls and edits with the cost
  of the visible rows.
* The Dopesheet and Curves draw only keys within the visible time range;
  summary keys aggregate incrementally as keys change.
* The Animator graph culls nodes and arrows outside the view; selection,
  pan, and zoom cost follows visible nodes.
* Timeline draws only visible clips and waveforms, and caches waveforms per
  audio clip.

## Acceptance outcomes

1. Selecting a GameObject without an Animator and pressing Create in the
   Animation window adds an Animator, creates a clip and controller at the
   chosen location, and undoing once removes all three effects. (EDT-005,
   EDT-006, ANI-001)
2. In record mode, moving an object with the Move tool at frame 30 writes
   position keys at frame 30, the Inspector shows the recording tint on those
   fields, and leaving record mode restores the object's saved position.
   (EDT-002, EDT-003, ANI-001)
3. Scrubbing the time ruler in preview changes the object in the Scene view,
   and saving the scene during preview writes the edit-time values, not the
   previewed ones. (EDT-003, ANI-001)
4. Pressing K with a property selected adds a key at the playhead, and
   Alt+`.` moves the playhead to the next keyframe. (EDT-006, ANI-001)
5. Box-selecting keys in the Dopesheet and dragging the right handle scales
   their times about the left edge, as one undo operation. (EDT-005, ANI-001)
6. Choosing Flat from a key's context menu in the Curves view sets both
   tangents flat, and the evaluated curve read through MCP matches. (EDT-006,
   ANI-001)
7. An animation event added at the playhead lists the root's callable
   methods in the Inspector, and a function name no component provides is
   shown as unresolved and kept on save. (EDT-002, ANI-001)
8. A clip inside an imported model is listed as read-only in the clip
   dropdown, and an attempt to move its keys changes nothing and states why.
   (EDT-006, ANI-001)
9. In the Animator window, Create State > Empty, then Make Transition to a
   second state, creates a transition whose settings and conditions are
   editable in the Inspector; an agent performs the same two commands through
   MCP with the same resulting controller. (EDT-006, ANI-001)
10. During play mode, the Animator window shows the selected instance's
    active state highlighted with progress, and setting a Bool parameter in
    the Parameters tab fires the transition that the condition names.
    (EDT-004, ANI-001)
11. Deleting a parameter used by two conditions lists both uses first, and
    undo restores the parameter and both conditions. (EDT-005, ANI-001)
12. Dragging the point in a 2D Freeform Directional blend diagram previews
    the blend, and the motion weight bars match the weights read through MCP.
    (EDT-002, ANI-001)
13. With the Timeline package present, dragging a GameObject from the
    Hierarchy onto an animation track binding binds it, offering to add an
    Animator when it has none; without the package, the menu item is absent
    and a sequence asset is preserved unchanged on save. (EDT-006, CIN-001)
14. Scrubbing a sequence in Timeline preview moves bound objects, and leaving
    preview restores their edit-time values. (EDT-003, CIN-001)
15. An agent's key edits to a clip open in the Animation window appear as
    they are written with the agent badge on the clip, and clicking the badge
    opens Change Review filtered to that clip. (EDT-008, ANI-001)
