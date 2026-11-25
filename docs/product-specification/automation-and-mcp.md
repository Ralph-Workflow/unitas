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

Throughout this document, an **agent session** is one authorized MCP connection
and its identity; a **play session** is one running instance of the game in
play mode, in the editor, headless, or in a player. An agent session can start
several play sessions.

## User Outcome

As an authorized automated agent, I can discover, inspect, operate, and verify
every project feature available to a developer, with or without the visual
editor running. As a developer, I can see what the agent observed and changed,
understand its results, resolve conflicts, and accept, reject, or undo its
changes through change review (EDT-008) in the
[editor](editor-and-authoring.md).

Automation does not receive broader access merely because an operation is
available. Project, runtime, filesystem, build, platform, account, and external
service permissions are granted explicitly through permission scopes (AUT-007).

## Structured Feature Parity

Every visual editor command is an MCP tool with the same identifier, inputs,
availability, disabled reason, and result (EDT-006). Every view's state—
hierarchy, inspector values, scene and game view selection and camera, console,
compatibility report, build status, play state, and agent activity—is readable
as structured data. Commands that a project contributes through `[MenuItem]`,
`[ContextMenu]`, and custom editor extensions are tools as well. (AUT-001)

Where a person sees an image, an agent can request the same image and the
structured state behind it, as defined in [Visual evidence](#visual-evidence).

Agents and people receive the same compatibility states, as defined in the
[Compatibility contract](compatibility-contract.md). A capability that the visual
editor offers and automation does not, or that behaves differently through
automation, leaves that capability incomplete; such a gap is never presented to
an agent as a permission refusal.

### Client and network independence

Every capability works with any MCP client that conforms to a supported MCP
protocol revision. unitas requires no particular AI model or vendor and makes no
model requests of its own. Every tool and resource, including product knowledge
(AUT-006), works without network access, except an operation that itself needs
an external service (for example fetching a package from a remote registry or
deploying to a networked device). (AUT-001)

## Capability Discovery

An agent can discover available resources, tools, supported feature versions,
required inputs, optional inputs and defaults, result shapes, side effects,
required permission scopes (AUT-007), compatibility limitations, and whether an
action is reversible. (AUT-002)

Discovery reflects the current project state, selected target, editor state,
connected devices, project trust, and granted scopes. A capability that cannot
currently run is distinguishable from a capability the product does not provide.

Tool and resource schemas are versioned. Discovery reports each schema's version
and any deprecation with the release in which it ends, as defined by CMP-004 in
the [Compatibility contract](compatibility-contract.md).

### Discovery at scale

An agent can list tools by category, by origin (built-in, package, or project),
and by text search. Listings return summaries first; full input and result
schemas are read per tool on request. An agent session can subscribe to changes
in the tool set, and is notified when, for example, a recompilation adds or
removes project-contributed commands. (AUT-002)

### Project-contributed tools

Tools contributed by project code or packages are marked as project-contributed
in discovery and name the assembly or package they come from. A
project-contributed tool never replaces a built-in tool or alters a built-in
tool's name, description, or schema. (AUT-002, AUT-007)

## Structured Inspection

An agent can inspect projects, assets, dependencies, scenes, objects,
components, properties, prefab relationships, project settings, compilation,
play state, runtime objects, input, rendering, animation, physics, navigation,
audio, UI focus, loaded content, logs, profiling data, builds, playtests, test
results, debugging state, crash reports, and compatibility reports within its
granted scopes. (AUT-002)

Results use stable identities and typed values. Pagination, filtering, and
bounded queries preserve a clear statement of whether the result is complete.
An empty result is distinguishable from an unavailable or failed query. Queries
over projects at the scale floor declared by PRF-002 in
[Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md)
remain bounded and paginated rather than returning the whole project at once.

### Large results and retention

A request can declare a result size limit. A result that would exceed it returns
a summary and resource links, each with its size and media type, that the agent
reads in ranges. Pagination cursors are bound to the project revision at which
they were issued; reading a cursor after the underlying content changed reports
the change instead of silently mixing revisions. (AUT-002)

Evidence (captures, recordings, profiling data, test and scenario results) is
retained for a period and up to a storage limit that a person configures.
Reading expired evidence returns an expired outcome naming what expired, never
an empty result.

### Untrusted content in results

Every result field that carries text originating from the project, its
packages, a player, or a network response (for example object names, log
messages, script comments, and response bodies) is marked as untrusted content
and names its origin. No project content, log entry, or tool result can change
an agent session's permission scopes, a review policy, project trust, or the
held state of agent writes; only a person can. (AUT-002, AUT-007)

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

Every edit becomes part of the agent session's change set (EDT-008). Under the
require-approval review policy, an edit result states that the change is
proposed rather than applied.

### Request identifiers

Every mutating request accepts a client-supplied request identifier. Repeating a
request with the same identifier within the retention period returns the
original result and applies nothing again. After a timeout or lost connection,
an agent session can query the outcome of a request by its identifier and
receives one of: applied, not applied, or still running. (AUT-002)

## Play and Test Control

An agent can enter and exit play mode, pause, advance frames, start declared
scenarios (TST-002), run project tests (TST-003), inject supported input,
inspect checkpoints, capture requested visual or state evidence, and stop a run.
It can operate development players built with remote control enabled.
(AUT-002)

Test actions remain distinguishable from organic player input and from direct
state mutation. A scenario that changes state through inspection privileges
cannot claim the result of an ordinary player path.

### Time control

A play session can run with a fixed simulated frame delta, honoring
`Time.captureFramerate` as defined by RUN-004 in
[Scripting runtime](scripting-runtime.md); advance a requested number of frames
as fast as the host allows; start from a saved state; and seed
`UnityEngine.Random` at start. The play session's result records each of these
conditions. (AUT-002)

### Visual evidence

A capture request names a camera, a display, or an editor view, and states the
resolution, region, image format, and optionally a frame sequence. A capture
can include an identity map that resolves any pixel to the GameObject, renderer,
or UI element drawn there, and the screen-space bounds of named objects. A
capture also records the cameras, objects, and settings that produced it.
Annotated overlays are produced as separate images and never alter the
unannotated capture. (AUT-002)

## Human-Agent Shared Context

With the read-project scope, an agent can inspect current selection, open scene
set, active scene, play state, current target, relevant editor focus, and
diagnostics. Agent changes appear in the open editor without a private refresh
step, marked with the agent session that made them. (AUT-003)

Every mutating result identifies affected entities and resulting revision or
state. If another actor changed relevant state after inspection, unitas reports
a conflict rather than silently applying an edit to an unintended target.

When the developer holds agent writes (EDT-008), write requests return a held
outcome that names who held them, and nothing changes; inspection continues.

## Headless and Concurrent Agent Sessions

### Headless operation

The MCP server operates on a project without the visual editor running and
without a display. Every tool and resource behaves the same headless as with the
editor open, except that requests which need a person (for example granting
project trust or confirming an action) return an outcome stating that no person
is present. Rendering, captures, play sessions, tests, and builds run headless.
(AUT-004)

When the visual editor opens a project that a headless server is already
serving, both attach to the same project state; neither silently takes over
the other's agent sessions or play sessions.

### Transports and authentication

The MCP server offers the standard input/output transport and the MCP HTTP
transport. It negotiates the MCP protocol revision with each client and reports
the revisions it supports; a client requesting no supported revision is refused
with that list. (AUT-004)

* The HTTP transport accepts local connections only by default: it binds to the
  loopback interface and rejects requests whose origin is not in a configured
  list.
* Network connections are disabled until a person enables them. Once enabled,
  they require TLS and authentication under the MCP authorization
  specification.
* A person can list and revoke the credentials that authorize agent sessions.
  Revoking a credential ends every agent session it authorized.

### Agent session identity

Each agent session has an identity: the agent's declared name, the client that
connected, and the person or credential that authorized it. The identity is
attached to every change (EDT-008), log entry, play session, audit record
(AUT-008), and evidence item the agent session produces. An agent session cannot
present another agent session's identity. (AUT-004)

### Concurrency

Multiple agent sessions and people can work on one project at the same time.
(AUT-004)

* Each agent session can start its own play sessions. A play session started by
  one agent session is isolated from other agent sessions' play sessions and
  from the editor's play mode: runtime state, input, time, and random state are
  not shared, and stopping one does not stop another.
* Writes to shared project state are checked against the revision the agent
  session last observed. A write based on a stale revision returns a conflict
  naming the newer change and its actor, and changes nothing.
* Compilation, import, and other project-wide operations triggered by several
  agent sessions are reported to every agent session whose state they affect.

### Subscriptions and notifications

An agent session can subscribe to changes in resources such as scenes, objects,
assets, compilation state, play state, logs, test runs, change sets, and the
tool set. A notification identifies what changed, the new revision, and the
actor. A subscription that cannot keep up reports that notifications were
dropped and from which revision the agent session must re-read, rather than
silently omitting changes. (AUT-004)

### Long-running operations

Operations that can outlast a request—import, compilation, test runs,
scenarios, builds, profiling captures, and deployments—return an operation
handle immediately and report progress through MCP progress notifications with
the current stage and, when known, completed and total units. (AUT-004)

An agent session can mark an operation as detached so that it survives loss of
the connection. Reconnecting with the same authorization lets the agent session
reattach to the operation, read its result, or cancel it. Operation results are
retained for a period that a person configures.

### Hangs and crashes

A play session, test, or editor-script invocation whose main thread does not
yield within a limit that a person configures is reported as hung, with its
managed stack, and can be interrupted. A crash in project code or in a native
plugin ends only the affected play session or operation and is reported with
crash evidence as defined by DIA-003 in
[Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md).
The MCP server, the visual editor, and other agent sessions continue.
(AUT-004)

### Lifecycle, limits, and cancellation

An agent session states when it starts, is idle, and ends. Ending an agent
session, or losing its connection, stops its play sessions and its running
operations that are not detached, leaves its applied changes in place, and keeps
its change sets available for review. (AUT-004)

Per-agent-session limits on concurrent play sessions, running operations,
memory, and request duration are configurable by a person. A request that would
exceed a limit fails with a distinct outcome naming the limit. Any running
operation can be cancelled; the cancellation result states whether any state
changed and what remains.

## Agent Code Workflow

An agent can author and fix project code through MCP alone. (AUT-005)

* **Create and edit scripts.** An agent creates, edits, renames, and deletes
  C# scripts and assembly definition files. Script edits appear in the agent
  session's change set as text diffs (EDT-008). Renaming a script that is used
  as a component preserves its `.meta` GUID so existing references continue to
  resolve.
* **Compile.** An agent requests compilation, or observes the compilation that
  a change triggers, without a manual refresh step. Compilation behavior is
  owned by RUN-007 in [Scripting runtime](scripting-runtime.md). Under the
  require-approval policy, compilation of proposed code takes place in the
  isolated proposal environment defined by EDT-008 in
  [Editor and authoring](editor-and-authoring.md).
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
  surface lists each type and member with its compatibility state as defined in
  the [Compatibility contract](compatibility-contract.md) (a Partially supported
  member names its exclusions; a Platform-limited member names its targets) and
  the Unity baseline releases in which it exists.
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

## Permission Scopes, Grants, and Confirmations

### Project trust

Automation that executes project code operates only on projects the user has
trusted, as defined by PRJ-008 in [Projects and assets](project-and-assets.md).
In an untrusted project, an agent can perform the read-only inspection that
PRJ-008 allows and can request trust; it cannot grant trust, and no request that
would execute project code runs. A request refused because the project is
untrusted returns a distinct outcome. (AUT-007)

### Scopes

Permissions are expressed as a fixed set of scopes that discovery lists: (AUT-007)

* **Read project:** inspect project content, runtime state, logs, and evidence.
* **Edit content:** change scenes, prefabs, assets, and their serialized values.
* **Edit scripts:** create, change, and delete scripts and assembly definitions.
* **Execute project code:** enter play mode, run tests and scenarios, invoke
  editor methods, and run project-contributed commands.
* **Import external files:** bring files from outside the project folder into
  the project.
* **Change packages and settings:** change package manifests and project
  settings.
* **Build:** create player and content builds.
* **Sign:** apply signing identities to builds.
* **Deploy:** install and run builds on connected devices.
* **External network:** make requests to services outside the local machine.
* **Review:** accept, approve, reject, and revert change sets of other agent
  sessions (EDT-008).

Discovery lists the scopes each tool and resource requires. A request without a
required scope returns a permission outcome naming the missing scope and
changes nothing.

### Grants

A new agent session holds only the read-project scope until a person grants
more. A person can grant scopes to one agent session or bind a grant to an agent
identity and a project, in which case it persists until revoked. For unattended
runs, grants come only from invocation configuration that the user controls,
such as the command-line invocation or a per-user configuration file; an agent
session cannot widen its own scopes or those of another agent session.

Holding both the edit-scripts and execute-project-code scopes amounts to running
arbitrary code with the privileges of the operating-system account running
unitas. The grant prompt states this whenever a grant would combine them.

### Confirmation boundary

Irreversible or externally visible requests—deletion outside the undo history,
deployment to a device, signing for distribution, requests to external network
services, and submission to a store or service—return a confirmation-required
outcome. They proceed only after a person confirms them in the visual editor or
through an MCP client authenticated as that person, or under a standing grant
that a person made for that action. An agent session's own confirmation never
satisfies the boundary. (AUT-007)

### Secrets

A person can mark files, folders, asset fields, and settings as secret, in
per-user or per-project unitas configuration. Agents cannot read secret values;
a result that would contain one shows the value as withheld and names the
secret marking. Secret values never appear in logs, evidence, captures, diffs,
or error details. An agent cannot infer access to a secret, account, device,
service, or proprietary platform merely from project references to it.
(AUT-007)

## Audit Log and Provenance

Every request an agent session makes is recorded in a per-project audit log
held in unitas-owned state outside project content. Each record has the time,
agent session identity, tool or resource, scope used, outcome, and affected
identities. Reads, refusals, permission and trust requests, approvals and
rejections, builds, deployments, and external network requests are all
recorded. (AUT-008)

A person can read and export the audit log through the visual editor, the
command line, and MCP. Agent sessions cannot alter or delete audit records. The
provenance of accepted agent changes—which agent session made each change, under
which request, and who accepted it—remains queryable in the audit log and is
never written into project files or source control.

## Failure and Safety Behavior

Automation failure reports what was requested, what was validated, whether any
state changed, affected identities, recoverability, and current state. Timeout,
cancellation, lost connection, missing permission scope, confirmation required,
untrusted project, no person present, held writes, exceeded limit, hung
operation, crashed play session, expired evidence, stale identity, conflict,
failed precondition, partial success, and product error remain distinct
outcomes.

Destructive actions identify their scope. Undoable operations advertise the
undo boundary.

## Acceptance Stories

* For a sample project, every acceptance outcome of the
  [editor](editor-and-authoring.md) that does not concern visual presentation
  is reproduced by an agent through MCP alone, with the same results. (AUT-001)
* Every command in the editor's command palette is discoverable as an MCP tool
  with the same identifier and availability. (AUT-001, AUT-002)
* With the machine's network disabled, an agent using a generic MCP client opens
  a project, reads product knowledge, edits a scene, and runs tests, and unitas
  makes no model or network requests. (AUT-001, AUT-006)
* Human and automation compatibility reports agree for the same project state.
  (AUT-001)
* An agent finds a scene object by stable identity, changes a supported property,
  and a developer immediately sees and can undo that exact edit. (AUT-002,
  AUT-003)
* A multi-object edit reports partial validation without claiming all objects
  changed. (AUT-002)
* An agent searches tools by category and origin, receives summaries, reads one
  tool's full schema, and is notified when a recompilation adds a project
  `[MenuItem]` tool marked as project-contributed with its assembly. (AUT-002)
* A request repeated with the same request identifier after a lost connection
  returns the original result, and an outcome query by that identifier reports
  it as applied. (AUT-002)
* A query whose result exceeds the declared limit returns a summary and ranged
  resource links; a cursor read after the scene changed reports the change.
  (AUT-002)
* An agent starts a gameplay scenario at a fixed frame delta with a seeded
  `UnityEngine.Random`, supplies declared input, reads runtime state, and
  returns evidence tied to the exact project and play session with those
  conditions recorded. (AUT-002)
* A game view capture with an identity map resolves a clicked pixel to the UI
  element drawn there, and its annotated overlay leaves the unannotated capture
  unchanged. (AUT-002)
* A log message from project code containing instructions to grant permissions
  is returned marked as untrusted content and changes no scope, policy, trust,
  or held state. (AUT-002, AUT-007)
* A stale request reports a conflict and does not overwrite a newer human edit.
  (AUT-003)
* With no visual editor and no display, an agent opens a project, edits a
  scene, enters play mode, captures a game view image, and runs a build.
  (AUT-004)
* Two agent sessions each run a play session on the same project; input sent to
  one play session does not reach the other, and stopping one leaves the other
  running. (AUT-004)
* Two agent sessions edit the same property from the same observed revision; the
  second receives a conflict naming the first agent session's change. (AUT-003,
  AUT-004)
* An agent session subscribed to a scene receives notifications for another
  agent session's edits, each naming the actor and revision. (AUT-004)
* A detached build continues after the agent session's connection drops; on
  reconnecting with the same authorization, the agent session reads its progress
  and result. Cancelling a running test run returns a result stating what
  changed and what remains. (AUT-004)
* A play session whose script loops forever is reported as hung with its managed
  stack and is interrupted; a native-plugin crash in another play session ends
  only that play session with crash evidence, while the editor and other agent
  sessions continue. (AUT-004)
* The HTTP transport rejects a request from an unlisted origin, and revoking a
  credential ends the agent sessions it authorized. (AUT-004)
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
* A new agent session can read the project but receives a permission outcome
  naming the edit-content scope when it edits a scene; after a person grants
  that scope, the same request succeeds. (AUT-007)
* Granting edit-scripts and execute-project-code together shows that the agent
  can run arbitrary code with the account's privileges. (AUT-007)
* An agent's deployment request returns confirmation-required, and the agent
  session's own confirmation does not satisfy it; a person's confirmation does.
  (AUT-007)
* A settings field marked secret is shown as withheld in inspection, diffs, and
  logs. (AUT-007)
* In an untrusted project, an agent's request to enter play mode is refused with
  the untrusted-project outcome, and its request for trust does not grant it.
  (AUT-007)
* A denied platform or filesystem operation exposes no sensitive content and is
  distinct from an unsupported product feature. (AUT-007)
* After an agent session's reads, a refused edit, and an accepted change, the
  exported audit log lists each with its time, identity, scope, and outcome; the
  agent session's request to delete an audit record is refused; and the accepted
  change's provenance appears in no project file. (AUT-008)
