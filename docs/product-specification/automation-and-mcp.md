# Automation and MCP

## User Outcome

As an authorized automated agent, I can discover, inspect, operate, and verify
the same meaningful project features available to a developer. As a developer,
I can see what the agent observed and changed, understand its results, resolve
conflicts, and undo supported changes through the normal editor experience.

Automation does not receive broader access merely because an operation is
available. Project, runtime, filesystem, build, platform, account, and external
service permissions remain explicit.

## Capability Discovery

An agent can discover available resources, tools, supported feature versions,
required inputs, optional inputs and defaults, result shapes, side effects,
permissions, compatibility limitations, and whether an action is reversible.

Discovery reflects the current project state, selected target, editor state,
connected devices, and granted access. A capability that cannot currently run
is distinguishable from a capability the product does not provide.

## Structured Inspection

An agent can inspect projects, assets, dependencies, scenes, objects,
components, properties, prefab relationships, project settings, compilation,
play state, runtime objects, input, rendering, animation, physics, navigation,
audio, UI focus, loaded content, logs, profiling data, builds, playtests, and
compatibility reports within its permissions.

Results use stable identities and typed values. Pagination, filtering, and
bounded queries preserve a clear statement of whether the result is complete.
An empty result is distinguishable from an unavailable or failed query.

## Structured Editing

An agent can perform the supported equivalents of object, component, asset,
scene, prefab, settings, and build-configuration edits. A request validates all
targets and reports which changes were accepted, rejected, conflicted, or not
attempted.

Related edits can be grouped into one visible undo operation. A change that
would discard unsupported serialized meaning, affect additional objects, or
invalidate references identifies that scope before commitment.

## Play and Test Control

An agent can enter and exit play mode, pause, advance frames, start declared
scenarios, inject supported input, inspect checkpoints, capture requested visual
or state evidence, and stop a run. It can operate declared development players
where remote control is enabled.

Test actions remain distinguishable from organic player input and from direct
state mutation. A scenario that changes state through inspection privileges
cannot claim the result of an ordinary player path.

## Human-Agent Shared Context

With permission, an agent can inspect current selection, open scene set, active
scene, play state, current target, relevant editor focus, and diagnostics. Agent
changes appear in the open editor without a private refresh step.

Every mutating result identifies affected entities and resulting revision or
state. If another actor changed relevant state after inspection, unitas reports
a conflict rather than silently applying an edit to an unintended target.

## Failure and Safety Behavior

Automation failure reports what was requested, what was validated, whether any
state changed, affected identities, recoverability, and current state. Timeout,
cancellation, lost connection, invalid permission, stale identity, failed
precondition, partial success, and product error remain distinct outcomes.

Destructive actions identify their scope. Undoable operations advertise the
undo boundary. Irreversible or externally visible actions require the same
meaningful confirmation boundary as their human-editor equivalents.

Sensitive values are not included in general project inspection, logs, test
evidence, or error details. An agent cannot infer access to a secret, account,
device, service, or proprietary platform merely from project references to it.

## Acceptance Stories

* An agent finds a scene object by stable identity, changes a supported property,
  and a developer immediately sees and can undo that exact edit.
* A stale request reports a conflict and does not overwrite a newer human edit.
* A multi-object edit reports partial validation without claiming all objects
  changed.
* An agent starts a gameplay scenario, supplies declared input, reads runtime
  state, and returns evidence tied to the exact project and play session.
* A denied platform or filesystem operation exposes no sensitive content and is
  distinct from an unsupported product feature.
* Human and automation compatibility reports agree for the same project state.
