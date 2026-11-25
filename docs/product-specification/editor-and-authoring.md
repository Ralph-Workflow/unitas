# Editor and Authoring Experience

## Scope

This specification defines the visual editor: the views that present project
and runtime state, direct authoring of scenes and content, play mode, undo and
recovery, commands, source-control-aware editing and semantic merge, and the
review, approval, and reversal of changes made by agents.

The visual editor is a focused companion to the agent interface defined in
[Automation and MCP](automation-and-mcp.md). It presents and edits the same
project model that agents use. Its purposes are to make project and runtime
state clear, to support direct authoring of scenes and content, and to let a
developer review, approve, and reverse agent changes. It does not aim for
parity with the breadth, customization, or polish of the Unity Editor's
authoring surfaces; a Unity Editor workflow that this document does not name is
not promised by it.

The visual editor has no built-in code editor. Script source is edited in the
developer's external editor, IDE, or agent, as defined in
[Scripts and external code editors](#scripts-and-external-code-editors) and in
DIA-002 in [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md).

Every capability described here is also available through automation without
the visual editor: each command is an MCP tool (EDT-006), each view's state is
inspectable as structured data, and every edit an agent makes is reviewable
here (EDT-008). Automation rules are owned by
[Automation and MCP](automation-and-mcp.md) and are not restated here.

These are desired end-state requirements, not claims about currently shipped
capability. The project's declared Unity release (2020.3 LTS through 2022.3 LTS)
is the behavioral baseline, with 2022.3 LTS as the principal reference, as
defined in the [Compatibility contract](compatibility-contract.md);
package-dependent features are bounded by the supported package matrix (CMP-002)
in the [Compatibility contract](compatibility-contract.md).

## Workspace

### User stories

* As a developer, I see project content, scene structure, selected object
  state, runtime output, diagnostics, and agent activity at once, without
  assembling a layout first.
* As a developer, I can show, hide, and resize the views I need, and the editor
  restores my arrangement when I reopen the project.
* As a developer whose saved arrangement is invalid or unwanted, I can reset it
  to the default without affecting the project.

### Required behavior

The workspace is a fixed set of purposeful views:

* project browser;
* scene hierarchy;
* inspector;
* scene view;
* game view;
* console and diagnostics;
* compatibility report;
* agent activity and change review (EDT-008);
* build status;
* play controls.

Each view can be shown, hidden, resized, and focused. Free-form docking,
tearing views into floating windows, and arbitrary tabbed arrangements are not
promised. Selection is shared across views and identifies the same underlying
object or asset in each.

The arrangement, view visibility, view sizes, and other personal preferences
are stored per user and per project, outside shared project content, so changing
them never dirties a scene, asset, or project setting. An arrangement that
cannot be restored (for example, saved by a newer unitas version or for a
display that is no longer connected) falls back to the default arrangement and
reports why. Resetting restores the default arrangement and changes nothing
else.

## Scene hierarchy

As a developer, I can create, name, duplicate, parent, reorder, activate,
deactivate, tag, layer, and delete GameObjects. I can search the hierarchy by
name, type, component, activation, tag, and layer. (EDT-001)

The hierarchy displays open scenes as distinct ownership boundaries. It shows
nested parentage, prefab status, hidden or locked state, and unsaved changes
for each scene and object. Multi-selection supports batch operations while
mixed values remain visible.

Parenting can preserve either the object's world placement or its local values.
Cyclic hierarchy operations are rejected. Moving objects between scenes makes
the effect on references and prefab relationships explicit. Deleting a parent
shows the complete descendant scope.

Objects and scenes that an active agent session is currently changing carry a
live indicator naming that agent session (EDT-008).

## Inspector

The inspector presents supported serialized properties using controls for
numbers, text, booleans, enums, flags enums, vectors, rotations, colors,
gradients, curves, layer masks, object references, arrays, lists, nested values,
and managed references (`[SerializeReference]`).

### Required behavior

* Validation appears beside the affected property.
* Intentional null, missing reference, incompatible reference, and unloaded
  reference are distinct states.
* Components can be added, reordered, reset, copied, pasted, duplicated, and
  removed when their contract permits it.
* Required or mutually dependent components (for example through
  `[RequireComponent]` or `[DisallowMultipleComponent]`) communicate
  consequences before an add or removal.
* Runtime values are visually distinct from saved edit-time values.
* Built-in presentation attributes are honored: `[HideInInspector]`,
  `[Header]`, `[Space]`, `[Tooltip]`, `[Range]`, `[Min]`, `[Multiline]`,
  `[TextArea]`, `[ColorUsage]`, `[GradientUsage]`, `[Delayed]`, and
  `[InspectorName]`.
* Edits validate before being represented as successfully saved.

### Custom editor extensions

Custom inspectors (`Editor` subclasses), property drawers (`PropertyDrawer`,
`DecoratorDrawer`), and editor windows (`EditorWindow`) are recognized, but
drawing their user interface is Unsupported, as listed in the
[index](INDEX.md). For a type with a custom inspector or drawer, the inspector
uses the default presentation of the same serialized properties and states that
a custom presentation exists but is not drawn. The commands those extensions
declare, such as `[MenuItem]` and `[ContextMenu]` methods, appear as ordinary
commands (EDT-006); their execution and content effects are owned by RUN-010 in
[Scripting runtime](scripting-runtime.md). An editor window that cannot be drawn
is listed with its declared title and its available commands.

## Scene view

### User stories

* As a level designer, I can navigate two- and three-dimensional scenes and
  frame a selected object.
* As an artist, I can select visible content and manipulate position, rotation,
  and scale using local or global coordinates.
* As a designer, I can snap objects to grid, rotation, and scale increments and
  inspect component-specific handles.
* As a developer reviewing a script's behavior, I can see the gizmos and debug
  lines it draws.

### Required behavior

The scene view supports perspective, orthographic, axis-aligned, and two-
dimensional views. It displays selection outlines, pivots, centers, grids,
snapping, bounds, cameras, lights, colliders, audio ranges, navigation data, and
the built-in handles of built-in components. (EDT-003)

Picking remains accurate for nested, rotated, scaled, transparent, inactive,
and overlapping objects. The user can cycle or choose among ambiguous hits.
Locked or non-selectable content remains visible without intercepting selection.

Transform manipulation states the coordinate space, pivot choice, snap state,
and exact resulting values. Cancelling a manipulation restores the prior state.

### Gizmos and debug drawing

The scene view draws gizmos from `OnDrawGizmos` and `OnDrawGizmosSelected`
using the `Gizmos` drawing calls (lines, rays, wire and solid cubes and spheres,
meshes, frustums, icons) with their color and matrix state. `OnDrawGizmosSelected`
draws for each selected object and for each ancestor of a selected object, as in
Unity. Gizmo drawing can be toggled globally and per component type.

Lines and rays from `Debug.DrawLine` and `Debug.DrawRay` appear in the scene
view, and in the game view while its gizmo display is enabled, with their color,
duration, and `depthTest` behavior. A zero duration draws for one frame.

Custom scene-view drawing performed by custom editors (for example
`Editor.OnSceneGUI` handles) is part of custom editor extension drawing and is
Unsupported. Structured capture of gizmo and debug-line data for agents and
tests is owned by DIA-001 in
[Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md).

## Game view

The game view displays the output of a selected game camera or display at a
declared resolution, aspect ratio, scale, and safe area. It communicates whether
it currently owns gameplay input. Resolution simulation never claims to be a
complete substitute for target hardware behavior.

The view supports statistics and diagnostic overlays without altering the
captured game result unless the overlay is explicitly included. Multiple game
displays are selectable when the project uses them. (EDT-004)

## Play mode

Entering play mode starts a play session from the current project state;
proposed changes held under the require-approval policy (EDT-008) are excluded
unless the developer previews them. If unsaved content is used by the play
session, the editor identifies whether the play session uses saved content or
current unsaved editor content. It does not silently mix the two. Play, pause, and frame-step states are unambiguous.
Domain reload and scene reload follow the project's Enter Play Mode Options as
defined by RUN-003 in [Scripting runtime](scripting-runtime.md).

Pausing freezes normal gameplay progression while inspection remains available.
Frame step advances exactly one eligible rendered frame and its associated
runtime work. Exiting play mode restores edit-time state. Runtime changes are
not saved unless the user explicitly applies a supported value, and applied
values identify their destination.

The editor distinguishes failure to enter play mode, a running game that has no
visible camera, a paused game, and a stopped game. Exceptions do not silently
exit play mode. (EDT-004)

The play session shown in the editor is one play session among those that
agents may run; agent play sessions isolated from it are defined by AUT-004 in
[Automation and MCP](automation-and-mcp.md).

## Scripts and external code editors

The visual editor provides no text-editing surface for script source.

Opening a script, from the project browser, a component's script field, a
console entry, a compiler diagnostic, a stack frame, or a test result, opens the
configured external editor at the script's file and, when known, its line and
column. The external editor is a per-user preference. When none is configured,
the operating system's default handler for the file type is used and the
editor states which application was asked to open it. When the launch fails,
the editor reports the configured command, the file, and the failure, and the
file location remains copyable.

Changes saved by the external editor or by an agent are detected without a
manual refresh and trigger recompilation as defined by RUN-007 in
[Scripting runtime](scripting-runtime.md). The project files that external
editors and language servers use, and script debugging, are owned by DIA-002 in
[Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md).

## Commands, drag and drop, and multi-editing

### Commands

Every editor action is a named command with a stable identifier, a display name,
a description, its required inputs, its current availability, and, when
unavailable, the reason (for example "no selection", "play mode is running",
"asset is read-only", or "agent writes are held"). (EDT-006)

* A command palette lists all commands, searchable by name, description, and
  shortcut, and shows each command's availability and disabled reason.
* Menus and context menus are views of the same commands and never hide the
  full affected scope of an action.
* Shortcuts are per-user preferences. Assigning a shortcut that is already bound
  in the same context reports the conflict and names the other command.
  Defaults can be restored per shortcut or all at once.
* Commands contributed by the project, through `[MenuItem]` methods,
  `[ContextMenu]` methods, and custom editor extensions, appear in the palette
  with their declared menu path and validation state. A `[MenuItem]` validation
  method that returns false makes the command unavailable with that reason.
* Every command, built-in or project-contributed, is also an MCP tool with the
  same identifier, inputs, availability, and result, as defined in
  [Automation and MCP](automation-and-mcp.md).

### Drag and drop

A developer can drag assets into the hierarchy or scene view to instantiate them,
onto object-reference fields to assign them, and onto objects to add components
from scripts; drag hierarchy objects to reparent or reorder them; and drag
objects into the project browser to create prefabs. Before a drop completes, the
target shows the resulting action and its scope. A drop that is not permitted
shows why (for example an incompatible reference type or a cyclic parent) and
changes nothing. Every drag-and-drop outcome is also available as a command.

### Multi-editing

Multi-object editing in the inspector exposes shared and mixed values and never
overwrites a mixed property until the user commits a value. Committing a value
changes only that property on every selected object and forms one undo
operation. Components that are not present on every selected object are listed
as not shared and are not edited. Multi-selection in the hierarchy and project
browser supports batch rename, activation, tag, layer, reparent, duplicate, and
delete, each reporting its full scope before commitment.

## Selection, search, and navigation

Selection can originate from the project browser, hierarchy, scene view,
console, diagnostics, profiler, agent change review, or automation. Each view
that displays the selected object or asset reveals and frames it without
changing its identity. (EDT-001)

Search results state their scope and active filters. Navigation history allows
returning to recent selections and locations. A diagnostic navigates to the
responsible asset, scene, object, component, property, script location, or
setting when known.

## Undo and redo

Ordinary authoring actions participate in a visible history. A single user
gesture, such as a drag or a committed multi-object value, forms one operation
even when it changes multiple values. Undo and redo restore identities,
references, hierarchy, scene ownership, prefab overrides, and selection—not
only displayed values. (EDT-005)

Edits made by agents participate in the same history and are attributed to the
agent session that made them. Undo applies to the most recent operation
regardless of actor; reversing one agent change set out of order is done through
change review (EDT-008).

An operation that cannot be undone communicates this before commitment. New
edits after undo make the abandoned redo branch explicit. External file changes
never masquerade as ordinary undoable edits.

## Save, close, and recovery

Unsaved changes are tracked per scene and asset, including changes made by
agents. Closing a scene, project, or editor identifies the complete unsaved
scope, including which agent sessions produced it, and allows the user to save,
discard, or cancel. Cancel is unavailable only when the operating system is
forcing the editor to exit, in which case recovery data is written. (EDT-005)

Following an interruption, recoverable work is offered with its source,
timestamp, and relationship to saved content. Recovery never silently replaces
newer saved content. A failed save reports whether the previous valid content
remains intact.

## External changes, source control, and semantic merge

### External changes and source-control status

The editor distinguishes shared project files, generated or cached files,
personal settings, and transient runtime state. Read-only files and detected
conflicts show explicit status. The user can reload an externally changed file,
keep unsaved work, or review a conflict without silent loss. (EDT-007)

Text-serialized assets remain reviewable as text, while the editor also presents
semantic object and property context for them. Source-control status never
substitutes for actual save success.

### Semantic diff and three-way merge

unitas compares and merges Unity scenes, prefabs, and other YAML-serialized
assets (for example `.unity`, `.prefab`, `.asset`, `.mat`, `.controller`,
`.anim`, and `.meta` files) at the level of objects and serialized properties
rather than text lines. (EDT-007)

* A semantic diff between two revisions lists added, removed, and changed
  objects, components, and properties, with old and new values, object paths in
  the hierarchy, and prefab override changes.
* A three-way merge takes a base and two changed revisions. Changes to different
  objects or different properties of the same object are combined. File IDs and
  GUIDs are preserved, so references from other content continue to resolve.
* Changes to the same property with different values are surfaced as
  conflicts, each showing the base, both values, and the owning object path. Each
  conflict is resolved by choosing one side or entering a value.
* Structural conflicts are surfaced distinctly: an object deleted on one side
  and changed on the other, conflicting reparenting that would form a cycle, the
  same file ID assigned to different objects, and conflicting array reorderings.
* Before a merge result is written, it is validated: references resolve or
  are reported, and no unknown serialized data is discarded without being
  listed. A merge with unresolved conflicts is never written as merged.
* Assets in binary serialization form, and assets whose types cannot be read,
  are reported as not semantically mergeable rather than merged by text.
  Serialization forms are owned by SER-005 in
  [Serialization and prefabs](serialization-and-prefabs.md).

Semantic diff and merge are available in the visual editor, from the command
line so a source-control client can invoke them as external diff and merge
tools, and to agents through MCP, with the same results.

## Agent change review

### User stories

* As a developer, I can see which agents are connected and what each is doing.
* As a developer, I can review everything an agent session changed as one
  change set, in terms of objects, properties, assets, settings, and script
  text.
* As a developer, I can accept, reject, or revert a whole change set or
  individual items in it.
* As a developer, I can stop agents from writing while I work, without
  disconnecting them.

### Change sets

Every agent session's changes form a reviewable change set. An agent session can
divide its work into multiple named change sets. Each item in a change set
records the agent session, agent identity, time, the request that produced it, and its affected
identities. (EDT-008)

A change set presents:

* property-level semantic diffs for scenes, prefabs, assets, and project
  settings, in the same form as semantic diff (EDT-007);
* text diffs for scripts and other text files;
* asset creation, move, rename, and deletion with their dependency effects;
* compilation, test, and scenario results the agent session recorded against it.

Change sets remain available across editor restarts until they are accepted,
rejected, or reverted, and are readable through MCP as well as in the editor.

### Review policies

Each project and each agent session has one of two review policies, chosen by
the developer:

* **Apply immediately:** agent changes take effect in the project and the open
  editor at once. Accepting marks items as reviewed; rejecting an unreviewed
  item reverts it exactly as Revert does and marks it rejected.
* **Require approval:** agent changes are held as a proposed change set that the
  developer can preview, including in the scene view and inspector, without the
  project files changing. Approving applies them; rejecting discards them.

A session's policy, when set, overrides the project's policy for that session;
a new session inherits the project's policy. An agent session cannot change its
own review policy.

### Isolated proposal environment

Under the require-approval policy, an agent session can compile, run tests
(TST-003) and scenarios (TST-002), and enter play mode against its proposed
change set in an isolated environment. Doing so changes no project files, does
not change the editor's compilation or play mode, and is not visible to other
agent sessions. Results are recorded against the change set and name the
proposed revision they ran on. Approval shows the latest results and states
whether the proposal changed after they were produced.

Agent-authored code is not compiled into the editor process or a headless
server process until it is approved, except in that isolated environment.

### Risk flags

A change set lists the following ahead of its other items, each flagged with its
category:

* changes to tests and test assemblies;
* changes to scenario expectations, approved visual references, and comparison
  tolerances;
* changes to project settings and package manifests;
* changes to build configuration and signing configuration;
* changes to editor-time scripts (scripts in editor assemblies or using
  `UnityEditor`);
* additions or changes of native plugins.

Removing or disabling a test, removing or loosening a scenario expectation, and
widening a tolerance are additionally labeled as weakening verification.

### Side effects of invoked project code

When an agent session invokes project code (for example a `[MenuItem]` command,
an editor method, a test, or a play session), files in the project that the code
creates, changes, or deletes are detected and listed in the change set as
file-level changes attributed to that invocation. Changes the code makes outside
the project folder are reported as untracked effects of that invocation; they
cannot be reverted through change review.

### Who can review

Accepting, approving, rejecting, or reverting a change set requires a person, or
an agent session to which a person granted the review scope for that project
(AUT-007 in [Automation and MCP](automation-and-mcp.md)). An agent session never
accepts or approves a change set it authored; such a request returns a
permission outcome and changes nothing. When no visual editor is running, a
person reviews and approves through the command line or through an MCP client
authenticated as that person.

### Accept, reject, and revert

Accepting, rejecting, and reverting apply to a whole change set or to selected
items. Reverting an applied item restores the content it changed to its prior
state and is itself an undoable operation. When other items depend on a rejected
or reverted item (for example a property set on a component the item created),
the dependent items are listed before the action proceeds. When content changed
by an item has since been changed by a person or another agent session, the revert
reports a conflict for that content instead of overwriting the newer change.
Reverting never rewrites source-control history; it produces ordinary working
changes.

### Live activity and write control

The editor shows each connected agent session, its identity, its current
operation, its play sessions, and the objects and assets it is changing. Changes
from an active agent session appear in the views as they happen, marked with
that agent session.

The developer can hold agent writes for all sessions or for one session. While
writes are held, read and inspection requests continue; write requests return a
distinct held outcome naming who held them, and nothing changes. Permitting
writes again does not replay held requests. Holding and permitting are not
available to agents.

## Accessibility

Accessibility of the visual editor (keyboard operation, accessible names, focus,
scaling, and non-color status) is owned by ACC-001 in
[Accessibility, localization, and persistence](accessibility-localization-persistence.md).
Every command reachable through the command palette (EDT-006) is reachable from
the keyboard.

## Acceptance outcomes

1. A user can author and save a scene using hierarchy, inspector, and scene view
   without losing object identity or references. (EDT-001, EDT-002, EDT-003)
2. Hierarchy search by component, tag, and layer finds every matching object
   across all open scenes, and deleting a parent shows its complete descendant
   scope first. (EDT-001)
3. A component whose script declares `[Range]`, `[Header]`, and a custom
   `PropertyDrawer` shows the range and header, uses default presentation for
   the drawer's field, and states that the custom drawer is not drawn.
   (EDT-002)
4. A script's `OnDrawGizmosSelected` gizmo appears only while its object is
   selected, and a `Debug.DrawLine` with a two-second duration remains visible
   for two seconds of play time. (EDT-003)
5. Entering and exiting play mode leaves edit state unchanged unless the user
   explicitly applies a value, and pause and frame step are reflected in the
   game view and play controls. (EDT-004)
6. Undo and redo restore a multi-object hierarchy edit completely, including one
   made by an agent and attributed to its agent session. (EDT-005)
7. Every unsaved or conflicting file, including agent-produced changes, is
   accounted for before close or reload. (EDT-005, EDT-007)
8. Every editor action is found by name in the command palette, a disabled
   action states its reason, and the same action succeeds as an MCP tool with
   the same identifier. (EDT-006)
9. A project `[MenuItem]` command appears in the command palette, and its
   validation method controls its availability. (EDT-006)
10. Dragging a prefab into the hierarchy instantiates it, and dropping an
    incompatible asset on a reference field states why and changes nothing.
    (EDT-006)
11. Opening a script from a compiler diagnostic opens the configured external
    editor at the reported file and line. (DIA-001, DIA-002)
12. A console or compatibility finding navigates to the responsible content.
    (EDT-001)
13. A three-way merge of a scene changed on two branches combines changes to
    different properties, preserves file IDs, surfaces a same-property conflict
    with base and both values, and produces a scene whose references resolve; an
    agent obtains the same merge result through MCP. (EDT-007)
14. After an agent session adds an object, edits properties, and changes a
    script, the developer sees one change set with property-level and text
    diffs, reverts one property item, and the rest remain applied. (EDT-008)
15. Reverting an agent change whose content a person has since edited reports a
    conflict and does not overwrite the person's edit. (EDT-008)
16. Under the require-approval policy, a proposed agent change is previewable
    and leaves project files unchanged until approved. (EDT-008)
17. While the developer holds agent writes, an agent's write request returns the
    held outcome and changes nothing, and its read requests still succeed.
    (EDT-008)
18. Changing, saving, and resetting the workspace arrangement and shortcuts do
    not dirty shared project content. (EDT-006)
19. An agent session's request to accept or approve its own change set returns a
    permission outcome and changes nothing; a person approves the same change
    set through the command line with no visual editor running. (EDT-008)
20. Under the require-approval policy, an agent session compiles its proposed
    script change, runs the project's tests, and enters play mode against it in
    the isolated environment; project files, the editor's compilation, and
    other agent sessions are unchanged, and approval shows those results with
    the proposed revision they ran on. (EDT-008, AUT-005)
21. A change set that deletes a test and widens a visual comparison tolerance
    lists both ahead of its other items and labels them as weakening
    verification. (EDT-008)
22. A `[MenuItem]` command invoked by an agent session that writes a file in the
    project and one outside it lists the first as a file-level change
    attributed to that invocation and the second as an untracked effect.
    (EDT-008)
23. Entering play mode while an agent's proposed change is held for approval
    runs the play session without that change unless the developer is
    previewing it. (EDT-004, EDT-008)
