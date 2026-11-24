# Editor and Authoring Experience

## Scope

This specification defines the observable authoring experience: project
workspace, hierarchy, inspector, scene and game views, selection, manipulation,
play mode, undo, recovery, search, layouts, and source-control-aware editing.

These are desired end-state requirements, not claims about currently shipped
capability. Stock Unity 2022.3 LTS behavior provides the compatibility baseline;
package-dependent features are bounded by explicitly supported package versions.

## Workspace and layouts

### User stories

* As a developer, I can arrange, dock, resize, open, close, save, restore, and
  reset editor windows.
* As a developer using multiple projects, my personal layout does not alter
  shared project content.
* As a keyboard user, I can reach core authoring actions without relying on a
  pointer.

### Required behavior

The workspace exposes the project browser, scene hierarchy, inspector, scene
view, game view, console, build status, and play controls. Selection is shared
across relevant views and identifies the same underlying object or asset.

Layouts and personal preferences are per user. Shared project settings are
clearly distinguished. An invalid or unavailable saved layout can be reset
without affecting the project.

Commands expose names, current availability, shortcuts, and disabled reasons.
Context menus and drag-and-drop never hide their full affected scope.

## Scene hierarchy

As a developer, I can create, name, duplicate, parent, reorder, activate,
deactivate, tag, layer, and delete GameObjects. I can search the hierarchy by
name, type, component, activation, tag, and layer.

The hierarchy displays open scenes as distinct ownership boundaries. It shows
nested parentage, prefab status, hidden or locked state, and unsaved changes
where relevant. Multi-selection supports batch operations while mixed values
remain visible.

Parenting can preserve either the object's world placement or its local values.
Cyclic hierarchy operations are rejected. Moving objects between scenes makes
the effect on references and prefab relationships explicit. Deleting a parent
shows the complete descendant scope.

## Inspector

The inspector presents supported serialized properties using controls
appropriate to numbers, text, booleans, enums, vectors, rotations, colors,
gradients, curves, object references, arrays, lists, nested values, and managed
references.

### Required behavior

* Validation appears beside the affected property.
* Intentional null, missing reference, incompatible reference, and unloaded
  reference are distinct states.
* Multi-object editing exposes shared and mixed values and never overwrites a
  mixed property until the user commits a value.
* Components can be added, reordered, reset, copied, pasted, duplicated, and
  removed when their contract permits it.
* Required or mutually dependent components communicate consequences before a
  removal.
* Runtime values are visually distinct from saved edit-time values.
* Script-defined visibility, ranges, headers, tooltips, and supported custom
  property presentations are honored.
* Edits validate before being represented as successfully saved.

## Scene view

### User stories

* As a level designer, I can navigate two- and three-dimensional scenes and
  frame a selected object.
* As an artist, I can select visible content and manipulate position, rotation,
  and scale using local or global coordinates.
* As a designer, I can snap objects to useful spatial increments and inspect
  component-specific handles.

### Required behavior

The scene view supports perspective, orthographic, axis-aligned, and two-
dimensional views. It displays selection outlines, pivots, centers, grids,
snapping, bounds, cameras, lights, colliders, audio ranges, navigation data, and
component handles when relevant.

Picking remains accurate for nested, rotated, scaled, transparent, inactive,
and overlapping objects. The user can cycle or choose among ambiguous hits.
Locked or non-selectable content remains visible without intercepting selection.

Transform manipulation states the coordinate space, pivot choice, snap state,
and exact resulting values. Cancelling a manipulation restores the prior state.

## Game view

The game view displays the output of a selected game camera or display at a
declared resolution, aspect ratio, scale, and safe area. It communicates whether
it currently owns gameplay input. Resolution simulation never claims to be a
complete substitute for target hardware behavior.

The view supports statistics and diagnostic overlays without altering the
captured game result unless the overlay is explicitly included. Multiple game
displays are selectable when the project uses them.

## Play mode

Entering play mode starts a runtime session from the approved project state. If
unsaved content is relevant, the editor identifies it and the chosen session
source. Play, pause, and frame-step states are unambiguous.

Pausing freezes normal gameplay progression while inspection remains available.
Frame step advances exactly one eligible rendered frame and its associated
runtime work. Exiting play mode restores edit-time state. Runtime changes are
not saved unless the user explicitly applies a supported value, and applied
values identify their destination.

The editor distinguishes failure to enter play mode, a running game that has no
visible camera, a paused game, and a stopped game. Exceptions do not silently
exit play mode.

## Selection, search, and navigation

Selection can originate from the project browser, hierarchy, scene view,
console, diagnostics, profiler, or automation. Relevant views reveal and frame
the same object without changing its identity.

Search results state their scope and active filters. Navigation history allows
returning to recent selections and locations. A diagnostic can navigate to the
responsible asset, scene, object, component, property, script location, or
setting when known.

## Undo and redo

Ordinary authoring actions participate in a visible history. A meaningful user
gesture can form one operation even when it changes multiple values. Undo and
redo restore identities, references, hierarchy, scene ownership, prefab
overrides, and selection where relevant—not only displayed values.

An operation that cannot be undone communicates this before commitment. New
edits after undo make the abandoned redo branch explicit. External file changes
never masquerade as ordinary undoable edits.

## Save, close, and recovery

Unsaved changes are tracked per scene and asset. Closing a scene, project, or
editor identifies the complete unsaved scope and allows the user to save,
discard, or cancel where safe.

Following an interruption, recoverable work is offered with its source,
timestamp, and relationship to saved content. Recovery never silently replaces
newer saved content. A failed save reports whether the previous valid content
remains intact.

## Source-control-aware behavior

The editor distinguishes shared project files, generated or cached files,
personal settings, and transient runtime state. Read-only files and detected
conflicts show explicit status. The user can refresh an externally changed file,
keep unsaved work, or review a conflict without silent loss.

Text-serialized assets remain reviewable, while the editor presents semantic
asset and object context where possible. Source-control status never substitutes
for actual save success.

## Accessibility

Core workflows have keyboard access, accessible names, focus indication,
readable scaling, and state that is not communicated by color alone. Tooltips
and errors remain available without precise pointer movement. Automation can
inspect the same state in textual form.

## Acceptance outcomes

1. A user can author and save a scene using hierarchy, inspector, and scene view
   without losing object identity or references.
2. Undo and redo restore a multi-object hierarchy edit completely.
3. Entering and exiting play mode leaves edit state unchanged unless the user
   explicitly applies a value.
4. Every unsaved or conflicting file is accounted for before close or reload.
5. A console or compatibility finding navigates to the responsible content.
6. Core scene editing is possible through keyboard-accessible commands.
7. Personal layouts and preferences do not dirty shared project content.
