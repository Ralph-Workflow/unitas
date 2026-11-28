# Console

## Scope

This document details the Console window of the visual editor and the status
bar's console message. It refines EDT-006 (workspace views and commands) and
the play-mode parts of EDT-004 (error pause) owned by
[Editor and authoring](../editor-and-authoring.md); this document owns no
catalog ID. How diagnostics are collected, grouped, and navigated is owned by
DIA-001 in [Diagnostics, profiling, and playtesting](../diagnostics-profiling-playtesting.md#unified-diagnostics),
and the behavior of the `Debug.Log` family and unhandled exceptions by RUN-002
in [Scripting runtime](../scripting-runtime.md). This document defines how the
Console presents and operates on those entries.

## Regions

The Console has three regions, top to bottom:

1. **Toolbar.**
2. **Entry list**, one row per entry (or per collapsed group).
3. **Details pane**, showing the selected entry's full message and stack
   trace, separated from the list by a draggable divider whose position
   persists per user.

## Toolbar

Left to right:

* **Clear** — removes every entry except compiler errors and other entries
  whose condition still holds (see [Persistent entries](#persistent-entries)).
* **Clear options dropdown** (the arrow beside Clear) — toggles **Clear on
  Play** (clear when entering play mode), **Clear on Build** (clear when a
  player build starts), and **Clear on Recompile** (clear when script
  compilation completes). Each toggle persists per user.
* **Collapse** — groups identical entries (same message, same stack trace) into
  one row with a count badge at the trailing edge; individual occurrences keep
  their context and are listed when the group is expanded in the details pane
  (DIA-001).
* **Error Pause** — when on, pauses play mode at the end of the frame in which
  an error or exception is logged (see
  [Game view and play mode](game-view-and-play-mode.md)).
* **Target dropdown** — chooses whose log is shown: Editor (the editor and its
  play mode), or a connected development player or device (BLD-004 in
  [Builds, platforms, XR, and networking](../build-platforms-xr-networking.md)).
  It lists each connected target with its name, platform, and address, and
  offers entering an IP address for a player that is not discovered.
* **Search field** — filters entries by text in the message, case-insensitively,
  as the person types. Ctrl/Cmd+F focuses it while the Console has focus.
* **Counter toggles** — three toggles showing the count of logs, warnings, and
  errors (exceptions and assertions count as errors). Turning one off hides
  that severity from the list; the counts remain visible. Counts above the
  display limit show the limit followed by a plus sign.

## Entry list

Each row shows:

* a severity icon (log, warning, error) and, for exceptions and assertions,
  the error icon;
* the timestamp, when Show Timestamp is on;
* the message, truncated to the number of lines set by Log Entry (from one to
  ten lines), with the first line of the stack trace
  when the line count allows;
* the collapse count when Collapse is on;
* the origin marker for entries from a connected player.

Behavior:

* **Single-click** selects the entry and shows it in the details pane. If the
  entry has a context object (`Debug.Log(message, context)`), the object is
  pinged in the Hierarchy or Project window.
* **Double-click** opens the script at the entry's first user-code stack frame
  (or the compiler diagnostic's file and line) in the configured external
  editor, as defined in
  [Scripts and external code editors](../editor-and-authoring.md#scripts-and-external-code-editors).
  An entry with no script location navigates to its responsible content when
  known (DIA-001).
* Up and Down move the selection; Return opens it as double-click does.
* Ctrl/Cmd+C copies the selected entry's message and stack trace as text.
* New entries append at the bottom. When the list is scrolled to the bottom it
  follows new entries; when the person has scrolled up, it stays in place.
* Rich text tags in messages (`<b>`, `<i>`, `<color>`, `<size>`) render.

## Details pane

The details pane shows the entire message with its stack trace. Every stack
frame that names a project or package source file shows the file path and line
as a link; clicking a link opens that file at that line in the external
editor. Frames in assemblies without source show as plain text. The text is
selectable and copyable. For a collapsed group, the pane lists each
occurrence's time and frame.

## Context menus

Right-click on an entry or the list area:

* **Copy** — copies the entry's message and stack trace.
* **Show Timestamp** — toggles the timestamp column.
* **Log Entry** — submenu with line counts for each row.
* **Stack Trace Logging** — submenu per severity (Error, Assert, Warning, Log,
  Exception), each with None, Script Only, and Full; the choice is the project
  setting that governs which frames are captured, and changes apply to
  entries logged afterwards.
* **Open Player Log** — opens the log file of the last local player run in the
  operating system's default text handler.
* **Open Editor Log** — opens the editor's log file the same way.

The window menu (vertical three dots on the tab) offers the same items plus
Use Monospace Font.

## Persistent entries

Compiler errors and other entries that report an ongoing condition (for
example a script compilation failure, a missing required package, or an
unresolvable assembly reference) stay in the list through Clear while the
condition holds and are removed automatically when it is resolved, such as
when a recompilation succeeds. While compiler errors exist, entering play mode
is refused and the Game view and status bar state that all compiler errors
must be fixed first; the Console scrolls to and highlights the first error.

## Status bar message

The leading area of the editor's status bar shows the most recent Console
entry's icon and first line. Clicking it opens and focuses the Console and
selects that entry. The status bar shows nothing when the Console is cleared.

## Empty and error states

* A Console with no entries shows an empty list; the counters show zero.
* A search or severity filter that hides every entry shows the empty list with
  the counters still reflecting all entries.
* Selecting a target whose connection drops keeps the received entries, marks
  the target as disconnected in the dropdown, and resumes when the player
  reconnects.
* A log file that does not exist (no player was run) disables Open Player Log
  with that reason.

### Shortcuts

Shortcuts Manager context: **Console**.

| Action | Binding |
|---|---|
| Open or focus Console | Ctrl/Cmd+Shift+C (global, `Window/General/Console`) |
| Focus search | Ctrl/Cmd+F |
| Copy selected entry | Ctrl/Cmd+C |
| Open selected entry in external editor | Return |
| Move selection | Up, Down |

Clear, Collapse, and Error Pause have no default binding and are bindable in
the Shortcuts Manager ([Shortcuts](shortcuts.md)).

### Agent presence

* An entry logged during an agent session's invocation (a command, an editor
  method, a test, or an agent play session) carries the agent badge at the
  start of its row, after the severity icon.
* A trailing Agents filter dropdown on the toolbar lists connected and recent agent sessions plus
  Person only, and filters the list to the chosen origin; it sits after the counter toggles and never moves
  existing toolbar controls. With no agent sessions, the dropdown is hidden.
* Entries from isolated agent play sessions and isolated proposal environments
  appear only when that session is chosen in the Agents filter; they never
  trigger Error Pause in the person's play session.
* Clicking an entry's badge opens the Change Review window filtered to the
  invocation that produced the entry.
* Agents read the same entries through MCP with the same filters (AUT-002).

### Scale

The entry list realizes only visible rows. A session of 100,000 entries
scrolls, filters, searches, and collapses within the declared responsiveness
budget, and logging many entries per frame never blocks the frame: entries
are appended in batches and counters update once per frame. Retention beyond
the in-window limit follows DIA-001; the entries remain readable through MCP
with pagination ([scale floor](../diagnostics-profiling-playtesting.md#scale-floor)).

## Acceptance outcomes

1. A `Debug.LogError` in play mode with Error Pause on pauses the play session
   at the end of that frame, and the pause control shows paused. (EDT-004,
   EDT-006)
2. With Clear on Play on, entering play mode empties the list except for
   persistent entries; with it off, entries remain. (EDT-006)
3. With Collapse on, logging the same message ten times shows one row with a
   count of ten. (EDT-006, DIA-001)
4. Turning off the warning counter hides warnings while the count remains, and
   typing in the search field filters the remaining entries by text. (EDT-006)
5. Double-clicking an exception entry opens the configured external editor at
   the first user-code frame's file and line. (EDT-006, DIA-001, DIA-002)
6. Clicking a file link in the details pane's stack trace opens that file at
   that line. (EDT-006, DIA-001)
7. A compiler error stays after Clear, play mode refuses to start while it
   exists, and the entry disappears after a successful recompilation.
   (EDT-004, EDT-006)
8. Clicking the status bar message opens the Console with that entry selected.
   (EDT-006)
9. Selecting a connected development player in the target dropdown shows that
   player's log entries. (EDT-006, BLD-004)
10. Setting Stack Trace Logging for Log to None produces later log entries
    without stack traces. (EDT-006)
11. An entry logged by an agent session's `[MenuItem]` invocation carries its
    session badge, and choosing that session in the Agents filter shows only
    its entries. (EDT-006, EDT-008)
12. Logging 100,000 entries leaves scrolling and searching within the declared
    responsiveness budget. (EDT-006, PRF-002)
