# Automation and MCP

## Primary Interface

Automation through the Model Context Protocol (MCP) is the primary product
interface of unitas. The visual editor described in
[Editor and authoring](editor-and-authoring.md) and the command line described
by BLD-002 in [Builds, platforms, XR, and networking](build-platforms-xr-networking.md)
present and operate the same project model that MCP exposes.

Automation is also the completeness bar. A product capability is complete only
when an authorized agent can use it through MCP with the same behavior,
compatibility state, results, and failure reporting that a person observes. A
capability that a person can use but an agent cannot is incomplete, whatever
its state in the visual editor. Every feature specification relies on this
document for automation rules instead of restating them.

## User Outcome

As an authorized automated agent, I can discover, inspect, operate, and verify
every project feature available to a developer, with or without the visual
editor running. As a developer, I can see what the agent observed and changed,
understand its results, resolve conflicts, and accept, reject, or undo its
changes through change review (EDT-008) in the
[editor](editor-and-authoring.md).

Automation does not receive broader access merely because an operation is
available. Project, runtime, filesystem, build, platform, account, and external
service permissions remain explicit.

## Structured Feature Parity

Every visual editor command is an MCP tool with the same identifier, inputs,
availability, disabled reason, and result (EDT-006). Every view's state—
hierarchy, inspector values, scene and game view selection and camera, console,
compatibility report, build status, play state, and agent activity—is readable
as structured data. Commands that a project contributes through `[MenuItem]`,
`[ContextMenu]`, and custom editor extensions are tools as well. (AUT-001)

Where a person sees an image, an agent can request the same image and the
structured state behind it: a scene or game view capture comes with the
cameras, objects, and settings that produced it.

Agents and people receive the same compatibility states, as defined in the
[Compatibility contract](compatibility-contract.md). A capability that the visual
editor offers and automation does not, or that behaves differently through
automation, leaves that capability incomplete; such a gap is never presented to
an agent as a permission refusal.

## Capability Discovery

An agent can discover available resources, tools, supported feature versions,
required inputs, optional inputs and defaults, result shapes, side effects,
permissions, compatibility limitations, and whether an action is reversible.
(AUT-002)

Discovery reflects the current project state, selected target, editor state,
connected devices, project trust, and granted access. A capability that cannot
currently run is distinguishable from a capability the product does not provide.

Tool and resource schemas are versioned. Discovery reports each schema's version
and any deprecation with the release in which it ends, as defined by CMP-004 in
the [Compatibility contract](compatibility-contract.md).

## Structured Inspection

An agent can inspect projects, assets, dependencies, scenes, objects,
components, properties, prefab relationships, project settings, compilation,
play state, runtime objects, input, rendering, animation, physics, navigation,
audio, UI focus, loaded content, logs, profiling data, builds, playtests, test
results, debugging state, and compatibility reports within its permissions.
(AUT-002)

Results use stable identities and typed values. Pagination, filtering, and
bounded queries preserve a clear statement of whether the result is complete.
An empty result is distinguishable from an unavailable or failed query. Queries
over projects at the scale floor declared by PRF-002 in
[Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md)
remain bounded and paginated rather than returning the whole project at once.

## Structured Editing

An agent can perform the supported equivalents of object, component, asset,
scene, prefab, settings, and build-configuration edits. A request validates all
targets before changing any of them; validation failure leaves the request
unchanged. Operations that permit partial application declare that behavior
before execution and report which changes were accepted, rejected, conflicted,
or not attempted, with affected identities and resulting revisions. (AUT-002)

Related edits can be grouped into one visible undo operation. A change that
would discard unsupported serialized meaning, affect additional objects, or
invalidate references identifies that scope before commitment.

Every edit becomes part of the session's change set (EDT-008). Under the
require-approval review policy, an edit result states that the change is
proposed rather than applied.

## Play and Test Control

An agent can enter and exit play mode, pause, advance frames, start declared
scenarios (TST-002), run project tests (TST-003), inject supported input,
inspect checkpoints, capture requested visual or state evidence, and stop a run.
It can operate declared development players where remote control is enabled.
(AUT-002)

Test actions remain distinguishable from organic player input and from direct
state mutation. A scenario that changes state through inspection privileges
cannot claim the result of an ordinary player path.

## Human-Agent Shared Context

With permission, an agent can inspect current selection, open scene set, active
scene, play state, current target, relevant editor focus, and diagnostics. Agent
changes appear in the open editor without a private refresh step, marked with
the session that made them. (AUT-003)

Every mutating result identifies affected entities and resulting revision or
state. If another actor changed relevant state after inspection, unitas reports
a conflict rather than silently applying an edit to an unintended target.

When the developer holds agent writes (EDT-008), write requests return a held
outcome that names who held them, and nothing changes; inspection continues.

## Headless and Concurrent Sessions

### Headless operation

The MCP server operates on a project without the visual editor running and
without a display. Every tool and resource behaves the same headless as with the
editor open, except that requests which need a person (for example granting
project trust) return an outcome stating that no person is present. Rendering,
captures, play sessions, tests, and builds run headless. (AUT-004)

When the visual editor opens a project that a headless server is already
serving, both attach to the same project state; neither silently takes over
the other's sessions.

### Sessions and identity

Each agent connection is a session with an identity: the agent's declared name,
the client that connected, and the person or credential that authorized it.
Session identity is attached to every change (EDT-008), log entry, play
session, and evidence item the session produces. A session cannot present
another session's identity.

The server accepts local connections by default. Network connections are
disabled until a person enables them, and then require authentication.

### Concurrency

Multiple agent sessions and people can work on one project at the same time.

* Each session can start its own play sessions. A play session started by one
  session is isolated from other sessions' play sessions and from the editor's
  play mode: runtime state, input, time, and random state are not shared, and
  stopping one does not stop another.
* Writes to shared project state are checked against the revision the session
  last observed. A write based on a stale revision returns a conflict naming
  the newer change and its actor, and changes nothing.
* Compilation, import, and other project-wide operations triggered by several
  sessions are reported to every session whose state they affect.

### Subscriptions and notifications

A session can subscribe to changes in resources such as scenes, objects,
assets, compilation state, play state, logs, test runs, and change sets. A
notification identifies what changed, the new revision, and the actor. A
subscription that cannot keep up reports that notifications were dropped and
from which revision the session must re-read, rather than silently omitting
changes.

### Lifecycle, limits, and cancellation

A session states when it starts, is idle, and ends. Ending a session, or losing
its connection, stops its play sessions and running operations, leaves its
applied changes in place, and keeps its change sets available for review.

Per-session limits on concurrent play sessions, running operations, memory, and
request duration are configurable by a person. A request that would exceed a
limit fails with a distinct outcome naming the limit. Any running operation can
be cancelled; the cancellation result states whether any state changed and what
remains.

## Agent Code Workflow

An agent can author and fix project code through MCP alone. (AUT-005)

* **Create and edit scripts.** An agent creates, edits, renames, and deletes
  C# scripts and assembly definition files. Script edits appear in the
  session's change set as text diffs (EDT-008). Renaming a script that is used
  as a component preserves its `.meta` GUID so existing references continue to
  resolve.
* **Compile.** An agent requests compilation, or observes the compilation that
  a change triggers, without a manual refresh step. Compilation behavior is
  owned by RUN-007 in [Scripting runtime](scripting-runtime.md).
* **Diagnose.** Compilation results are structured: each diagnostic has its
  identifier (for example `CS0103`), severity, message, assembly, file, line,
  and column. The result states whether the project is now running the new
  code or still running the last valid compilation.
* **Query code.** An agent can query project code by symbol: types, members,
  signatures, base types, interfaces, attributes, and the assembly each belongs
  to. For a `MonoBehaviour` or `ScriptableObject` type, it can list the
  serialized fields as the serializer sees them, including those affected by
  `[SerializeField]`, `[NonSerialized]`, `[SerializeReference]`, and
  `[FormerlySerializedAs]`.
* **Find usages in content.** An agent can find every scene, prefab, and asset
  that uses a script type, with the object path and the serialized values of its
  fields there.
* **Test.** An agent runs project tests and reads their results, as defined by
  TST-003 in [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md).
* **Debug.** An agent sets breakpoints, steps, and inspects call stacks and
  values, as defined by DIA-002 in
  [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md).
* **Loop.** An agent can edit a script, compile, enter play mode, observe
  behavior, and repeat, with each step's result reporting which compilation the
  play session is running.

## Product Knowledge

unitas exposes its own documentation to agents as MCP resources. (AUT-006)

* **API reference.** A reference for the UnityEngine-compatible scripting
  surface lists each type and member with its compatibility state (Supported,
  Partially supported with named exclusions, Unsupported, Platform-limited with
  the affected targets) and the Unity baseline versions in which it exists.
* **Behavior documentation.** Documentation of each feature's behavior is
  derived from this product specification and states the same limits.
* **Diagnostic explanations.** Each diagnostic and compatibility finding
  identifier resolves to an explanation of its meaning, its consequence, and
  the actions that resolve it.
* **Guides.** Task guides cover the agent workflows in this document.

The knowledge resources describe the running unitas version exactly; content
for another version is never served as current. People read the same content in
the visual editor and in published documentation. Searching the knowledge
resources returns the same results to agents and people.

## Security, Permissions, and Trust

Automation operates only on projects the user has trusted, as defined by PRJ-008
in [Projects and assets](project-and-assets.md). In an untrusted project, an
agent can perform the read-only inspection that PRJ-008 allows and can request
trust; it cannot grant trust, and no request that would execute project code
runs. A request refused because the project is untrusted returns a distinct
outcome.

Permissions are granted per session by a person. A session cannot widen its own
permissions or those of another session.

Sensitive values are not included in general project inspection, logs, test
evidence, or error details. An agent cannot infer access to a secret, account,
device, service, or proprietary platform merely from project references to it.

## Failure and Safety Behavior

Automation failure reports what was requested, what was validated, whether any
state changed, affected identities, recoverability, and current state. Timeout,
cancellation, lost connection, invalid permission, untrusted project, held
writes, exceeded session limit, stale identity, conflict, failed precondition,
partial success, and product error remain distinct outcomes.

Destructive actions identify their scope. Undoable operations advertise the
undo boundary. Irreversible or externally visible actions require the same
meaningful confirmation boundary as their human-editor equivalents.

## Acceptance Stories

* For a sample project, every acceptance outcome of the
  [editor](editor-and-authoring.md) that does not concern visual presentation
  is reproduced by an agent through MCP alone, with the same results. (AUT-001)
* Every command in the editor's command palette is discoverable as an MCP tool
  with the same identifier and availability. (AUT-001, AUT-002)
* An agent finds a scene object by stable identity, changes a supported property,
  and a developer immediately sees and can undo that exact edit. (AUT-002,
  AUT-003)
* A multi-object edit reports partial validation without claiming all objects
  changed. (AUT-002)
* An agent starts a gameplay scenario, supplies declared input, reads runtime
  state, and returns evidence tied to the exact project and play session.
  (AUT-002)
* A stale request reports a conflict and does not overwrite a newer human edit.
  (AUT-003)
* With no visual editor and no display, an agent opens a project, edits a
  scene, enters play mode, captures a game view image, and runs a build.
  (AUT-004)
* Two agent sessions each run a play session on the same project; input sent to
  one does not reach the other, and stopping one leaves the other running.
  (AUT-004)
* Two sessions edit the same property from the same observed revision; the
  second receives a conflict naming the first session's change. (AUT-003,
  AUT-004)
* A session subscribed to a scene receives notifications for another session's
  edits, each naming the actor and revision. (AUT-004)
* Cancelling a running build or test run returns a result stating what changed
  and what remains. (AUT-004)
* An agent adds a `MonoBehaviour` with a compile error, receives the error with
  its identifier, file, line, and column, fixes it, attaches the component to a
  prefab, finds that usage by searching for the script, runs the project's
  tests, and enters play mode on the new code without a manual refresh.
  (AUT-005)
* An agent reads the API reference entry for a Partially supported member and
  receives its named exclusions, matching the running unitas version.
  (AUT-006)
* An agent resolves a diagnostic identifier to its explanation, and a person
  reads the same explanation in the editor. (AUT-006)
* In an untrusted project, an agent's request to enter play mode is refused with
  the untrusted-project outcome, and its request for trust does not grant it.
  (AUT-002, AUT-004)
* A denied platform or filesystem operation exposes no sensitive content and is
  distinct from an unsupported product feature. (AUT-002)
* Human and automation compatibility reports agree for the same project state.
  (AUT-001)
