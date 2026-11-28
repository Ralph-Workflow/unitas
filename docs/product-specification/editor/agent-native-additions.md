# Agent-Native Additions

## Scope

This document details how the visual editor presents agent sessions, their
live changes, and change review, as part of EDT-008 (review, approval, and
reversal of agent change sets) and EDT-006 (commands as MCP tools), owned by
[Editor and authoring](../editor-and-authoring.md). It owns no catalog ID.
Change sets, review policies, the isolated proposal environment, risk flags,
and who may review are defined in
[Agent change review](../editor-and-authoring.md#agent-change-review); agent
sessions, scopes, grants, and person-only actions are defined in
[Automation and MCP](../automation-and-mcp.md). This document defines only
their presentation and the rules that keep the familiar editor intact.

## The non-disturbance rule

The editor's agent-native features are additions to the familiar editor and
never alter it:

* No agent feature moves, renames, hides, reorders, or rebinds a familiar
  window, menu, menu item, toolbar control, context-menu item, shortcut,
  gesture, or default layout.
* Agent features add only: the **Window > Agents** submenu with the Agent
  Activity and Change Review windows; the status bar's agent indicator, as its
  last item; badges and outlines in existing windows, drawn where they never
  cover a familiar control; a trailing **Review Agent Changes** item in context
  menus whose target has agent changes; a Preview Proposals toggle at the end
  of the Scene view's View Options overlay, in its own agent section; the
  Agents preferences page; and the unbound Agents context in the Shortcuts
  Manager.
* No agent activity steals keyboard focus, changes the person's selection,
  moves a Scene view camera, changes the arrangement, opens a modal dialog, or
  interrupts a drag, a handle manipulation, a text edit, or a capture in
  progress. Requests that need a person (grants, confirmations, approvals)
  wait in the Agent Activity window and the agent indicator until the person
  attends to them.
* With no agent session connected, the editor is indistinguishable from the
  familiar editor except for the dimmed agent indicator and the Window > Agents
  submenu.

## Session colors and labels

Each agent session is assigned a color from a palette chosen to stay
distinguishable from the editor's selection, prefab-override, error, warning,
and play-mode colors, and a short label from its agent identity (AUT-007).
The color and label identify the session everywhere it is shown. Color is
never the only signal: every badge and outline also carries the label, an
icon, or a tooltip with the session's name, as ACC-001 in
[Accessibility, localization, and persistence](../accessibility-localization-persistence.md)
requires.

## Agent indicator (status bar)

The last item on the status bar:

* **States:** no session connected (dimmed); sessions connected and idle (the
  number of sessions); a session is executing (an activity mark); agent writes
  held (a held mark, naming who held them in its tooltip); person attention
  needed (a count of pending requests: grants, confirmations, approvals).
* **Click** opens and focuses the Agent Activity window.
* **Right-click** opens: Hold All Agent Writes or Permit Agent Writes; Open
  Change Review; Preview Proposed Changes (a check item, present when a
  session is under the require-approval policy); Agents Preferences...

## Agent Activity window

**Window > Agents > Agent Activity**. A dockable, single-instance window.

### Regions

* **Session list** (left): one row per connected agent session, with its color,
  label, agent identity, transport, granted scopes, review policy, and state
  (idle, executing, waiting for a person, writes held, disconnected). Selecting
  a session filters the other regions to it.
* **Current operation** (top right): for the selected session, or for every
  executing session, the operation in progress: its tool title (the menu path
  or tool name), targets, elapsed time, progress, and a link to its
  long-running operation record (AUT-004).
* **Activity feed** (center right): a virtualized, newest-first list of
  command records the sessions produced, each with time, session, tool title,
  targets, outcome (applied, proposed, held, refused with reason, conflict,
  failed), and change-set membership. Clicking a target selects it in the
  person's selection; clicking the change-set link opens Change Review at that
  item.
* **Requests** (bottom right): pending person-only requests from sessions
  (scope grants, confirmations at the confirmation boundary, approvals), each
  with its full statement as AUT-007 requires, and Grant/Deny or
  Confirm/Decline buttons. Acting on a request here is a person action.
* **Play sessions**: the play sessions each agent session runs (AUT-004), with
  their state and a button to view a session's Game view output in a floating
  Game view that is read-only for the person.

### Toolbar

Hold Writes and Permit Writes (for all sessions, or for the selected one);
Review Policy dropdown for the selected session (person-only); Disconnect
Session; Open Change Review; search of the activity feed; filters by outcome.

### Empty state

With no agent session connected, the window states that no agent is
connected and shows how to connect one: the MCP endpoint and transports the
editor offers (AUT-004) and a link to the product knowledge that agents read
(AUT-006).

## Change Review window

**Window > Agents > Change Review**. A dockable window; more than one can be
open, each showing a different change set.

### Regions

* **Change set list** (left): change sets grouped by agent session, each with
  its name, policy (applied or proposed), item count, unreviewed count, risk
  flag count, and the latest compile, test, and scenario results recorded
  against it.
* **Item tree** (center): the selected change set's items, risk-flagged items
  first under a Risk heading with their category and the "weakens
  verification" label where it applies, then grouped by scene (object path,
  component, property), prefab, asset, project setting, package manifest,
  script, and file. Each item shows its state (applied, proposed, accepted,
  rejected, reverted, conflict) and the request that produced it.
* **Diff pane** (right): for a property item, the semantic diff of EDT-007 with
  the old and new values drawn in the same field controls the Inspector uses
  (colors as swatches, curves as curves, references as object fields); for a
  script or text file, a side-by-side or unified text diff; for asset creation,
  move, rename, and deletion, the dependency effects.
* **Results pane**: compilation, test, and scenario results recorded against
  the change set, naming the revision they ran on and whether the proposal
  changed since.

### Actions

* Accept, Reject, and Revert for the whole change set or for the selected items;
  Approve for proposed items under require-approval. Before acting, the window
  lists dependent items and conflicts, as change review requires.
* **Preview** (require-approval): shows proposed items in the views (see
  [Previewing proposals](#previewing-proposals)).
* **Reveal**: selects the item's target in the person's selection and frames
  it in the Scene view, the Project window, or the Inspector.
* **Open Script**: opens a script item in the external code editor at the first
  changed line.
* Keyboard: arrow keys move through items; `Space` toggles an item's check box
  for multi-item actions; `Return` reveals.

Actions requiring a person (AUT-007) are disabled with that reason when the
window is driven by an agent session without the review scope.

## Live indicators in familiar windows

Each window document's `Agent presence` section details its indicators. The
common forms are:

* **Live badge**: the session's color and label, drawn at the trailing edge of a
  Hierarchy row, Project window item, Inspector component header, Animator
  state, Timeline track, or similar element while the session is changing it.
  Hovering shows the session's name and current operation; clicking opens
  Change Review filtered to that target.
* **Unreviewed badge**: a hollow form of the same badge on content an agent
  changed that is not yet accepted, rejected, or reverted. Preferences >
  Agents can hide unreviewed badges.
* **Inspector property marker**: a mark in the right margin of a property row
  in the session's color, distinct from the prefab-override bar in the left
  margin. Its tooltip names the session, time, and old value.
* **Scene view outline**: objects a session is changing are outlined in its
  color with its label, drawn beneath the person's selection outline so that
  the person's selection always reads first.
* **Console badge**: entries logged during an agent session's invocation carry
  its badge, and the Console's filter can show or hide them per session.
* **Undo History**: each entry names its actor (the person or an agent
  session).

Changes from an applied agent session appear in every view as they happen,
with no refresh, as AUT-003 requires.

## Previewing proposals

Under the require-approval policy, proposed items are invisible in the views
until the person turns on Preview in Change Review or the agent indicator.
While previewing:

* The Hierarchy, Scene view, Inspector, and Project window show the proposal
  overlay: proposed additions, removals, and property values, each drawn with
  the session's outline or marker in its dashed form.
* A banner at the top of the Scene view and Inspector names the change set
  being previewed, with Approve, Reject, and Stop Preview buttons.
* Proposed content is read-only to the person; editing it is unavailable with
  the reason "previewing proposed change". Other content remains editable,
  and the person's edits apply to project content as usual.
* Saving writes only project content, never the proposal overlay.
* Play mode entered while previewing includes the previewed proposal, as
  [Play mode](../editor-and-authoring.md#play-mode) states.

## Agent sessions and selection

* Each agent session has its own selection, which selection-dependent commands
  it invokes (for example `Edit/Duplicate`) act on. A session's selection is
  shown as its Scene view outline when Preferences > Agents > Show Agent
  Selections is on.
* An agent session changes the person's selection or moves a person's Scene
  view camera only when the person has allowed it in Preferences > Agents; it
  is not allowed by default. Agents render their own views and captures
  offscreen (AUT-002) without moving the person's cameras.
* Whatever the person's preference, an agent session reads the person's
  selection with the read-project scope (AUT-003).

## Commands, windows, and elements as tools

* Every menu item, context-menu item, and window-menu item is an MCP tool whose
  title is its menu path verbatim and whose name is a deterministic encoding of
  that path, invocable by either; every command without a menu item (for
  example a Scene view tool switch, a Shortcuts Manager action, or a
  drag-and-drop outcome) is a tool titled by its Shortcuts Manager identifier
  (EDT-006, AUT-001).
* Every built-in window exposes its state as a resource and its controls as an
  element tree in the same form that
  [Extension controls through MCP](../editor-and-authoring.md#extension-controls-through-mcp)
  defines for extension windows, so an agent reads and operates the Hierarchy,
  Inspector, Project window, Console, Scene view overlays, and every other
  window with the same results a person gets.
* The workspace arrangement, the active profile's bindings, and each window's
  own settings are resources, readable with the read-project scope.

## Agents preferences page

Preferences > Agents, detailed in
[Settings and preferences](settings-and-preferences.md), holds the session
color palette, the visibility of live badges, unreviewed badges, and agent
selections, whether Agent Activity opens when an agent first connects, whether
agents may change the person's selection or move the person's Scene view
cameras (both off by default), notifications, transports, standing grants, and
retention. Every setting that widens what an agent may do is person-only
(AUT-007): an agent session reads it but cannot change it.

## Shortcuts

No default bindings. The Agents context in the Shortcuts Manager lists Open
Agent Activity, Open Change Review, Hold All Agent Writes, Permit Agent Writes,
Toggle Preview, Accept Selected Items, Reject Selected Items, and Revert
Selected Items, unbound (see [Shortcuts](shortcuts.md)).

## Scale

* The activity feed and the item tree are virtualized and paginated with
  revision-bound cursors, so a change set of 100,000 items and a session that
  has produced 100,000 command records open and scroll at the cost of their
  visible rows.
* Live badges are computed from the command records of the visible rows only;
  a session changing many objects outside the visible rows costs nothing in
  the Hierarchy's frame.
* Semantic diffs in the diff pane are computed for the selected item on
  request, off the main thread, as asynchronous operations.

## Acceptance outcomes

1. With no agent session connected, the menu bar, main toolbar, context menus,
   Default layout, and Default shortcut profile read over MCP are identical to
   those of the same editor build with agent features turned off, except for
   Window > Agents and the dimmed agent indicator. (EDT-006, EDT-008)
2. While a person drags a Move handle, an agent session's edit to another
   object completes without ending the drag, changing the person's selection,
   or moving keyboard focus. (EDT-008, EDT-003)
3. An agent session that renames a GameObject shows a live badge on its
   Hierarchy row and a right-margin marker on the Inspector's name field while
   the operation runs, then an unreviewed badge until the person accepts the
   item. (EDT-008, EDT-001)
4. Clicking that badge opens Change Review with the rename item selected and
   its old and new names in the diff pane. (EDT-008)
5. An agent session invoking `GameObject/Create Empty` by its menu path and by
   its encoded tool name creates the object in both cases, and the activity
   feed lists both with the tool title `GameObject/Create Empty`. (EDT-006,
   EDT-008)
6. An agent session's request for the edit-scripts scope appears in the
   Requests region and the agent indicator's count, and no modal dialog opens.
   (EDT-008)
7. Under require-approval, a proposed property change is invisible in the
   Inspector until Preview is turned on, then appears with a dashed marker and
   is read-only to the person; Stop Preview removes it and the project file is
   unchanged. (EDT-008)
8. An agent session invoking `Edit/Duplicate` duplicates its own selection, and
   the person's selection is unchanged. (EDT-008, EDT-001)
9. Holding all agent writes from the agent indicator's menu makes the next agent
   write return the held outcome, shows the held state in the indicator, and
   leaves agent reads working. (EDT-008)
10. A change set that edits a project setting and deletes a test lists both
    under Risk at the top of the item tree, the test deletion labelled as
    weakening verification. (EDT-008)
11. An agent session's attempt to turn on "allow agents to change my
    selection" returns a permission outcome and changes nothing. (EDT-008)
12. Every built-in window opened in the Default layout exposes an element
    tree over MCP, and pressing the Console's Clear button through it clears
    the Console as a click does. (EDT-006)
13. A change set of 100,000 property items opens in Change Review and scrolls
    from first to last item while the editor's frame time stays within the
    release's declared budget. (EDT-008, PRF-002)
