# Automation and MCP

## Scope

An agent works on a unitas project the way a developer does: through the
editor. The **editor** is the unitas IDE, described in
[Editor and authoring](editor-and-authoring.md). It owns the open project's
model and every operation a developer performs on it, and it runs either with a
window (the **visual editor**) or without one (the **headless editor**).

The Model Context Protocol (MCP) interface is the editor's interface for
agents. It exposes every operation a developer can perform in the editor, as
tools and resources over the same project model, through the same
implementation of each operation. The visual editor and the command line
described by BLD-002 in
[Builds, platforms, XR, and networking](build-platforms-xr-networking.md) are
the editor's other front ends. None of them keeps its own copy of the project
state or its own implementation of an operation. A player contains no MCP
interface.

Automation is also the completeness bar. A product capability is complete only
when an authorized agent can use it through MCP with the same behavior,
compatibility state, results, and failure reporting that a person observes. A
capability that a person can use but an agent cannot is incomplete, whatever
its state in the visual editor. Every feature specification relies on this
document for automation rules instead of restating them.

The terms **agent session**, **play session**, and **headless session** are
defined once, in [Sessions and project state](#sessions-and-project-state);
every specification uses them with those meanings.

### Compatibility tiers

Every feature this document owns is in the Native tier, as defined in the
[Compatibility contract](compatibility-contract.md): AUT-001, AUT-002, AUT-003,
AUT-004, AUT-005, AUT-006, AUT-007, and AUT-008 are Native. The agent and MCP
layer is defined entirely by this specification; it has no Unity counterpart to
match. Where it exposes Unity project content or Unity-shaped APIs, those keep
the tier of their owning features.

## Architecture

MCP is a front end of the editor's command layer
([Editor and authoring](editor-and-authoring.md#architecture)). It lives entirely in the
editor instance's own scripts and a support assembly, has no engine counterpart, and is absent
from every player ([Engine architecture](architecture.md)).

* **Server.** One MCP server per editor instance serves every agent session over the standard
  input/output and HTTP transports. A standard input/output client started on a project
  location that an editor already holds reaches that editor through a relay over a local
  channel; it never opens a second copy of the project state.
* **Tools are commands.** Tools are generated from the command registry: each command's
  identifier, input schema, availability, and result shape are the tool's. Project-contributed
  commands, reported by workers, register under their project or package origin; the registry
  rebuilds on recompilation and emits one tool-set change. Resources are the same paginated,
  revision-bound queries that back the visual editor's views.
* **Request path.** A request is decoded off the editor's main thread, authorized against the
  agent session's scopes, project trust, and held-writes state, and then runs as a command
  invocation through the same pipeline as a person's gesture; its result is the command's
  result. Request identifiers map to command records, so a repeated request returns the
  recorded result and an outcome query reads the record.
* **Actors.** An agent session is an actor of the command layer with its own identity, scopes,
  change sets, and undo stream; people and command-line invocations are other actors.
  Conflicts come from the pipeline's revision check, not from MCP-specific logic.
* **Runtime access.** Inspection, input injection, time control, debugging, and captures reach
  a play session's worker or a connected development player over the editor's player
  connection. The player side speaks only that connection and contains no MCP code. Captures
  and identity maps render through the engine's render pipeline offscreen in the worker.
* **Operations and notifications.** Long-running work is an editor operation with a handle;
  progress and subscription notifications are fed from the same operation and change records
  the views observe, through a bounded queue per agent session that reports drops instead of
  slowing the editor.
* **State.** Product knowledge ships with the editor as versioned local resources. The audit
  log, evidence store, credentials, and grants are editor-owned state outside project content;
  nothing in this layer writes provenance into project files.
* **Headless.** A headless editor runs the same instance, server, and pipeline without a
  window; requests that need a person return the no-person-present outcome from the pipeline,
  not from a separate code path.

## User outcome

As an authorized automated agent, I can discover, inspect, operate, and verify
every project feature available to a developer, with or without the visual
editor running. As a developer, I can see what the agent observed and changed,
understand its results, resolve conflicts, and accept, reject, or undo its
changes through change review (EDT-008) in the
[editor](editor-and-authoring.md).

Automation does not receive broader access merely because an operation is
available. Project, runtime, filesystem, build, platform, account, and external
service permissions are granted explicitly through permission scopes (AUT-007).

## Structured feature parity

Every visual editor command is an MCP tool with the same identifier, inputs,
availability, disabled reason, and result (EDT-006). Every view's state—
hierarchy, inspector values, scene and game view selection and camera, console,
compatibility report, build status, play state, and agent activity—is readable
as structured data. Commands that a project contributes through `[MenuItem]`,
`[ContextMenu]`, and custom editor extensions are tools as well. The controls
and values that custom inspectors, property drawers, and editor windows draw
are readable as structured data, and their controls are operable through
tools, as defined in
[Extension controls through MCP](editor-and-authoring.md#extension-controls-through-mcp).
(AUT-001)

Where a person sees an image, an agent can request the same image and the
structured state behind it, as defined in [Visual evidence](#visual-evidence).

Agents and people receive the same compatibility states, as defined in the
[Compatibility contract](compatibility-contract.md). A capability that the visual
editor offers and automation does not, or that behaves differently through
automation, leaves that capability incomplete; such a gap is never presented to
an agent as a permission refusal. The person-only actions enumerated in
[People and person-only actions](#people-and-person-only-actions) are the only
exception: agents can discover and request each of them, and their
unavailability to agents is not a parity gap. (AUT-001)

### Client and network independence

Every capability works with any MCP client that conforms to a supported MCP
protocol revision. unitas requires no particular AI model or vendor and makes no
model requests of its own. Every tool and resource, including product knowledge
(AUT-006), works without network access, except an operation that itself needs
an external service (for example fetching a package from a remote registry or
deploying to a networked device). (AUT-001)

## Capability discovery

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

## Structured inspection

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

## Structured editing

An agent can perform every object, component, asset, scene, prefab,
settings, and build-configuration edit that a person can make. A request validates all
targets before changing any of them; validation failure leaves the request
unchanged. Operations that permit partial application declare that behavior
before execution and report which changes were accepted, rejected, conflicted,
or not attempted, with affected identities and resulting revisions. (AUT-002)

Related edits can be grouped into one visible undo operation. A change that
would discard unknown serialized meaning, affect additional objects, or
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

## Play and test control

An agent can enter and exit play mode, pause, advance frames, start declared
scenarios (TST-002), run project tests (TST-003), inject input from every
device family that INP-001 and INP-002 in
[UI, audio, and input](ui-audio-input.md) define (keyboard, mouse, touch, pen,
gamepad, joystick, and sensors) and from XR devices (XR-001), inspect
checkpoints, capture requested visual or state evidence, and stop a run.
It can operate development players built with remote control enabled.
(AUT-002)

Test actions remain distinguishable from organic player input and from direct
state mutation. A scenario that changes state through inspection privileges
cannot claim the result of an ordinary player path.

### Time control

This section is the single owner of the run conditions of play sessions in the
editor and in headless sessions; other specifications reference it.

A play session can run with a fixed simulated frame delta, honoring
`Time.captureFramerate` as defined by RUN-004 in
[Scripting runtime](scripting-runtime.md); advance a requested number of frames
as fast as the host allows; start from a saved state; and seed
`UnityEngine.Random` at start. The play session's result records each of these
conditions. With no condition requested, frames advance as fast as they
complete, and `Application.targetFrameRate` limits the frame rate in the same
way it does in a unitas player. (AUT-002)

### Visual evidence

A capture request names a camera, a display, an XR render view, or an editor
view, and states the
resolution, region, image format, and optionally a frame sequence. A capture
can include an identity map that resolves any pixel to the GameObject, renderer,
or UI element drawn there, and the screen-space bounds of named objects. A
capture also records the cameras, objects, and settings that produced it.
Annotated overlays are produced as separate images and never alter the
unannotated capture. (AUT-002)

### Audio evidence

An audio capture request names an `AudioListener` or an audio mixer output and a
time range. The result contains the samples at the output sample rate with their
channel layout, the DSP time at which the capture starts, and the voices (audio
sources and clips) that contributed to it. (AUT-002)

### Modal dialogs

A play session, an editor-method invocation, or a command-line invocation can
supply answers in advance for modal editor dialogs (for example those shown by
`EditorUtility.DisplayDialog`), each matched by dialog title and naming the
button to choose. Each dialog shown and the answer used, or the absence of an
answer, are recorded in the invocation's result. A dialog with no supplied
answer in a headless session returns the no-person-present outcome rather than
waiting. (AUT-002)

## Human-agent shared context

With the read-project scope, an agent can inspect current selection, open scene
set, active scene, play state, current target, relevant editor focus, and
diagnostics. Agent changes appear in the open editor without a private refresh
step, marked with the agent session that made them. (AUT-003)

Every mutating result identifies affected entities and resulting revision or
state. If another actor changed relevant state after inspection, unitas reports
a conflict rather than silently applying an edit to an unintended target.

When the developer holds agent writes (EDT-008), write requests return a held
outcome that names who held them, and nothing changes; inspection continues.

## Headless and concurrent agent sessions

### Sessions and project state

This is the only definition of these terms in the specification:

* An **agent session** is one authorized MCP connection together with its
  identity, granted scopes, and change sets.
* A **play session** is one running instance of the game: in play mode in the
  editor, visual or headless, or in a development player that the editor
  launched and connected to. Agents reach a connected player only through the
  editor's MCP interface. An agent session can start several play sessions.
* A **headless session** is a period during which the headless editor has a
  project open, serving agent sessions through MCP or a command-line invocation
  (BLD-002 in
  [Builds, platforms, XR, and networking](build-platforms-xr-networking.md)).

On one machine, one editor process holds the state of a project location. Agent
sessions, command-line invocations, and the person using the visual editor act
on that one state as separate actors. An invocation that requests isolated
state gets an isolated play session or the isolated proposal environment
(EDT-008), each in its own worker, and acts on that instead. An MCP client or command-line invocation that targets a project location
attaches to the editor already holding it, or starts a headless editor. A
process that cannot attach reports that the project is in use and names the
process holding it. Opening the visual editor on a project in a headless session
attaches a window to the running editor: the headless session ends, and its
agent sessions and play sessions continue. (AUT-004)

### Headless operation

The headless editor operates on a project without a display.
Every tool and resource behaves the same in a headless session as with the
editor open, except that requests which need a person (for example confirming an
action or approving a change set) return an outcome stating that no person is
present. Rendering, captures, play sessions, tests, and builds run in headless
sessions. (AUT-004)

### Transports and authentication

The editor's MCP interface offers the standard input/output transport and the MCP HTTP
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
  from the editor's play mode: runtime state, input, time, random state, player
  preferences, and persistent data are not shared, and stopping one does not stop another.
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
Project code runs in workers
([Editor and authoring](editor-and-authoring.md#architecture)), so the editor,
its MCP interface, and the other agent sessions and play sessions continue. (AUT-004)

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

## Agent code workflow

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

## Product knowledge

unitas exposes its own documentation to agents as MCP resources. (AUT-006)

* **API reference.** A reference for the UnityEngine-compatible scripting
  surface lists each type and member with its compatibility state as defined in
  the [Compatibility contract](compatibility-contract.md) (a Partially supported
  member names its exclusions; a Platform-limited member names its targets),
  its tier, and every difference from its documented Unity 2022.3 LTS meaning
  that its tier permits.
* **Behavior documentation.** Documentation of each feature's behavior is
  derived from this product specification and states the same limits.
* **Diagnostic explanations.** Each diagnostic and compatibility finding
  identifier resolves to an explanation of its meaning, its consequence, and
  the actions that resolve it.
* **Guides.** Task guides cover the agent workflows in this document, and state
  the limit of the permission model (AUT-007) for agents that also have shell
  or file access.

The knowledge resources describe the running unitas version exactly; content
for another version is never served as current. People read the same content in
the visual editor and in published documentation. Searching the knowledge
resources returns the same results to agents and people.

## Permission scopes, grants, and confirmations

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
* **External network:** have unitas make requests outside the local machine on
  the agent session's behalf, such as package and git fetches, networked
  deployment, and service submission. Requests made by project code that the
  agent session runs are governed by the execute-project-code scope and project
  trust, not by this scope or the confirmation boundary.
* **Review:** accept, approve, reject, and revert change sets of other agent
  sessions (EDT-008).

Discovery lists the scopes each tool and resource requires. A request without a
required scope returns a permission outcome naming the missing scope and
changes nothing.

### Grants

A new agent session holds only the read-project scope until a person grants
more. A person can grant scopes to one agent session or bind a grant to an agent
identity and a project, in which case it persists until revoked. For unattended
runs, grants come only from standing grants in per-user configuration that a
person set (see [Configuration](#configuration)); an agent session cannot widen
its own scopes or those of another agent session.

Holding both the edit-scripts and execute-project-code scopes amounts to running
arbitrary code with the privileges of the operating-system account running
unitas. The grant prompt states this whenever a grant would combine them. The
prompt for the execute-project-code scope also states that project code the
agent session runs can read files marked secret.

### Confirmation boundary

Irreversible or externally visible requests—deletion outside the undo history,
deployment to a device, signing for distribution, requests to external network
services, and submission to a store or service—return a confirmation-required
outcome. They proceed only after a person confirms them in the visual editor or
through an MCP client using that person's person credential, or under a standing
grant
that a person made for that action. An agent session's own confirmation never
satisfies the boundary. (AUT-007)

### Secrets

A person can mark files, folders, asset fields, and settings as secret, in
per-user or per-project unitas configuration. Agents cannot read secret values;
a result that would contain one shows the value as withheld and names the
secret marking. unitas never includes a secret value in its own results, logs,
evidence, captures, diffs, or error details, and it redacts a marked value
wherever it appears in output produced by project code. Project code that an
agent session runs can read secret files. An agent cannot infer access to a secret, account, device,
service, or proprietary platform merely from project references to it.
(AUT-007)

### People and person-only actions

A person acts through a person credential. unitas issues a person credential
only through an interactive prompt in the visual editor or in an interactive
terminal. A person credential is distinct from every agent-session credential
and cannot be obtained, created, or listed through MCP. Requests that arrive
over the standard input/output transport, and non-interactive command-line
invocations, are never treated as a person's. (AUT-007)

The following actions are person-only:

* granting and revoking project trust and consent to fetch packages (PRJ-008 in
  [Projects and assets](project-and-assets.md)), which are never available
  through MCP;
* granting and revoking scopes, standing grants, and credentials;
* setting review policies, and holding and permitting agent writes (EDT-008);
* accepting and approving change sets (EDT-008), except that an agent session a
  person granted the review scope can accept or approve change sets authored
  under a different credential and grant;
* confirming a request at the confirmation boundary;
* marking files, folders, asset fields, and settings as secret;
* enabling network transports and product telemetry;
* accepting toolchain license terms;
* uploading a build to a store or distribution channel.

Agents can discover and request each person-only action; the request is
recorded (AUT-008) and presented to a person.

The permission model constrains only what agents do through MCP. An agent that
also has shell or file access as the same operating-system account can act
outside it; the grant prompt and product knowledge (AUT-006) state this.

### Configuration

**Per-user configuration** is unitas-owned state (SER-005 in
[Serialization and prefabs](serialization-and-prefabs.md)). It holds trust
records, grants, standing grants, credentials, personal secret markings, the
HTTP transport origin list, evidence and operation retention periods and storage
limits, the hang limit, per-agent-session limits, and commercial license
activation. (AUT-007)

**Per-project configuration** is unitas-specific project content (SER-005). It
holds the project's review policy, project secret markings, and crash-report
destinations (DIA-003 in
[Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md)).

Only a person changes either configuration. A change to per-project
configuration made through MCP, by project code, or outside unitas (for example
by a source-control pull) takes effect only after a person confirms it; until
then the previous configuration applies and the pending change is reported.

Defaults are: review policy require approval; HTTP transport origin list
loopback only; no standing grants; no crash-report destinations. Every other
default is declared per release in product knowledge (AUT-006).

## Audit log and provenance

Every request an agent session makes is recorded in a per-project audit log
held in unitas-owned state (SER-005) outside project content. Each record has the time,
agent session identity, tool or resource, scope used, outcome, and affected
identities. Reads, refusals, permission and trust requests, approvals and
rejections, builds, deployments, and external network requests are all
recorded. (AUT-008)

A person can read and export the audit log through the visual editor, the
command line, and MCP. Agent sessions cannot alter or delete audit records. The
provenance of accepted agent changes—which agent session made each change, under
which request, and who accepted it—remains queryable in the audit log and is
never written into project files or source control.

## Failure and safety behavior

Automation failure reports what was requested, what was validated, whether any
state changed, affected identities, recoverability, and current state. Timeout,
cancellation, lost connection, missing permission scope, confirmation required,
untrusted project, no person present, held writes, exceeded limit, hung
operation, crashed play session, expired evidence, stale identity, conflict,
failed precondition, partial success, and product error remain distinct
outcomes.

Destructive actions identify their scope. Undoable operations advertise the
undo boundary.

## Acceptance outcomes

1. For a sample project, every acceptance outcome of the
   [editor](editor-and-authoring.md) is reproduced by an agent through MCP
   alone, with the same results; an outcome that concerns visual presentation
   is established through the captures and identity maps of
   [Visual evidence](#visual-evidence). (AUT-001, AUT-002)
2. Every command in the editor's command palette is discoverable as an MCP tool
   with the same identifier and availability. (AUT-001, AUT-002)
3. With the machine's network disabled, an agent using a generic MCP client opens
   a project, reads product knowledge, edits a scene, and runs tests, and unitas
   makes no model or network requests. (AUT-001, AUT-006)
4. Human and automation compatibility reports agree for the same project state.
   (AUT-001)
5. An agent finds a scene object by stable identity, changes a property,
   and a developer immediately sees and can undo that exact edit. (AUT-002,
   AUT-003)
6. A multi-object edit reports partial validation without claiming all objects
   changed. (AUT-002)
7. An agent searches tools by category and origin, receives summaries, reads one
   tool's full schema, and is notified when a recompilation adds a project
   `[MenuItem]` tool marked as project-contributed with its assembly. (AUT-002)
8. A request repeated with the same request identifier after a lost connection
   returns the original result, and an outcome query by that identifier reports
   it as applied. (AUT-002)
9. A query whose result exceeds the declared limit returns a summary and ranged
   resource links; a cursor read after the scene changed reports the change.
   (AUT-002)
10. An agent starts a gameplay scenario at a fixed frame delta with a seeded
    `UnityEngine.Random`, supplies declared input, reads runtime state, and
    returns evidence tied to the exact project and play session with those
    conditions recorded. (AUT-002)
11. A game view capture with an identity map resolves a clicked pixel to the UI
    element drawn there, and its annotated overlay leaves the unannotated capture
    unchanged. (AUT-002)
12. A log message from project code containing instructions to grant permissions
    is returned marked as untrusted content and changes no scope, policy, trust,
    or held state. (AUT-002, AUT-007)
13. A stale request reports a conflict and does not overwrite a newer human edit.
    (AUT-003)
14. With no visual editor and no display, an agent opens a project, edits a
    scene, enters play mode, captures a game view image, and runs a build.
    (AUT-004)
15. Two agent sessions each run a play session on the same project; input sent to
    one play session does not reach the other, and stopping one leaves the other
    running. (AUT-004)
16. Two agent sessions edit the same property from the same observed revision; the
    second receives a conflict naming the first agent session's change. (AUT-003,
    AUT-004)
17. An agent session subscribed to a scene receives notifications for another
    agent session's edits, each naming the actor and revision. (AUT-004)
18. A detached build continues after the agent session's connection drops; on
    reconnecting with the same authorization, the agent session reads its progress
    and result. Cancelling a running test run returns a result stating what
    changed and what remains. (AUT-004)
19. A play session whose script loops forever is reported as hung with its managed
    stack and is interrupted; a native-plugin crash in another play session ends
    only that play session with crash evidence, while the editor and other agent
    sessions continue. (AUT-004)
20. The HTTP transport rejects a request from an unlisted origin, and revoking a
    credential ends the agent sessions it authorized. (AUT-004)
21. An agent adds a `MonoBehaviour` with a compile error, receives the error with
    its identifier, file, line, and column, fixes it, attaches the component to a
    prefab, finds that usage by searching for the script, runs the project's
    tests, and enters play mode on the new code without a manual refresh.
    (AUT-005)
22. An agent reads the API reference entry for a Partially supported member and
    receives its named exclusions, matching the running unitas version.
    (AUT-006)
23. An agent resolves a diagnostic identifier to its explanation, and a person
    reads the same explanation in the editor. (AUT-006)
24. A new agent session can read the project but receives a permission outcome
    naming the edit-content scope when it edits a scene; after a person grants
    that scope, the same request succeeds. (AUT-007)
25. Granting edit-scripts and execute-project-code together shows that the agent
    can run arbitrary code with the account's privileges. (AUT-007)
26. An agent's deployment request returns confirmation-required, and the agent
    session's own confirmation does not satisfy it; a person's confirmation does.
    (AUT-007)
27. A settings field marked secret is shown as withheld in inspection, diffs, and
    logs. (AUT-007)
28. In an untrusted project, an agent's request to enter play mode is refused with
    the untrusted-project outcome, and its request for trust does not grant it.
    (AUT-007)
29. A denied platform or filesystem operation exposes no sensitive content and is
    distinct from a product error. (AUT-007)
30. After an agent session's reads, a refused edit, and an accepted change, the
    exported audit log lists each with its time, identity, scope, and outcome; the
    agent session's request to delete an audit record is refused; and the accepted
    change's provenance appears in no project file. (AUT-008)
31. A person credential cannot be obtained or listed through MCP, and a request
    arriving over standard input/output that tries to confirm a deployment is not
    treated as a person's. (AUT-007)
32. An agent session requests project trust through MCP; the request is recorded
    and shown to a person, and trust is granted only through the visual editor or
    an interactive terminal. (AUT-001, AUT-007)
33. A per-project configuration change that sets the review policy to apply
    immediately, arriving by a source-control pull, has no effect until a person
    confirms it. (AUT-007)
34. A new project uses the require-approval review policy, and its HTTP transport
    accepts only loopback origins. (AUT-007)
35. Project code run by an agent session that makes an HTTP request needs the
    execute-project-code scope, not the external-network scope, while a package
    fetch requested by the agent session needs the external-network scope and
    confirmation. (AUT-007)
36. A secret value printed to the log by project code appears redacted in the log
    entry returned to the agent session. (AUT-007)
37. While a headless session holds a project, a command-line invocation on the
    same project location attaches to the shared project state as a separate
    actor, and an invocation that cannot attach reports the holder. (AUT-004)
38. A play session started with no run conditions advances frames as fast as they
    complete, limited by `Application.targetFrameRate`. (AUT-002)
39. An audio capture of the main listener over two seconds returns samples at the
    output rate with channel layout, start DSP time, and the contributing audio
    sources. (AUT-002)
40. An editor-method invocation in a headless session supplied with an answer for
    a dialog title chooses that button and records the dialog and answer; a
    dialog with no answer returns the no-person-present outcome. (AUT-002)
41. An agent reads the fields a project's custom inspector draws for a component,
    changes one through the inspector's control, and the change appears in the
    agent session's change set. (AUT-001, RUN-010)
42. During an XR play session, an agent injects a controller pose and a trigger
    press, and captures the left-eye render view. (AUT-002, XR-001)
