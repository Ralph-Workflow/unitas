# Physics, Navigation, and Terrain Product Specification

## Scope

This specification defines player-visible motion, collision, spatial queries,
navigation, character movement, and terrain authoring behavior.

## Three-dimensional physics

As a developer, I can use static, kinematic, and dynamic bodies with box,
sphere, capsule, and mesh colliders. Bodies expose mass, gravity participation,
linear and angular velocity, drag, constraints, interpolation, collision
detection choice, sleeping, and supported force operations.

Colliders expose shape, center, orientation where applicable, trigger state,
enabled state, material, contact behavior, layer participation, and owning body.
Changing object activation, collider enablement, hierarchy, scale, or body role
updates its physical participation at a defined simulation boundary.

Physics materials produce observable friction and restitution behavior. Invalid
or unsupported combinations identify the affected collider or body and do not
silently simulate as an unrelated shape.

## Simulation timing and ownership

Physics advances on fixed simulation steps and respects the project's fixed
timestep and time scale. A slow rendered frame can contain multiple fixed steps.
Pausing, frame stepping, and resuming preserve understandable simulation state.

Dynamic bodies own their simulated position. Kinematic and static objects
respond to transform changes according to their declared roles. The editor
communicates when a script, animation, parenting change, or physics simulation
is the effective source of motion.

## Collision and trigger events

Eligible scripts receive enter, stay, and exit events for collisions and
triggers. Collision information identifies the other collider, body, object,
relative motion, and supported contacts. Events reflect layers, ignored pairs,
activation, destruction, shape changes, teleportation, and scene unloading.

No event is reported for a pair excluded by its effective collision rules.
Destroying or disabling an object during a callback leaves the remaining event
sequence defined and inspectable.

## Spatial queries

As a programmer, I can perform raycasts, line casts, shape casts, and overlap
queries. Queries accept origin or shape, direction where relevant, distance,
layer mask, and trigger policy. Results identify object, collider, body, point,
normal, distance, and relevant sub-shape information.

Queries define behavior for zero-length direction, starts inside a collider,
ties, disabled or inactive content, backfaces where relevant, non-uniform scale,
and invalid inputs. Multi-result APIs expose qualifying results within their
declared capacity; capacity-limited forms report the returned count. Result and
event ordering is guaranteed only when the compatible API contract defines it.

## Joints and constraints

Supported fixed, hinge, spring-like, and configurable joints relate bodies with
visible anchors, axes, limits, drives, break thresholds, and connected bodies.
Constraints retain their declared meaning when either body sleeps, becomes
kinematic, is disabled, or is destroyed. Broken constraints are observable by
scripts and diagnostics.

## Character movement

As a gameplay developer, I can move a collision-aware character through steps,
slopes, walls, ceilings, platforms, and ground contacts without treating it as
a freely tumbling rigid body.

Character state exposes grounded status and collision results. Movement has
defined behavior for excessive slopes, narrow steps, moving platforms,
teleportation, crouch-sized shape changes, overlapping starts, and multiple
motions in one frame.

## Two-dimensional physics

For 2D games, bodies, colliders, triggers, joints, layer rules, casts, overlaps,
contacts, sleeping, gravity, and fixed-step events operate in the two-dimensional
world. 2D and 3D physics remain distinct; content does not accidentally interact
across systems.

Supported shapes include common box, circle, capsule, polygon, edge, and
composite use cases within the declared compatibility scope. Sprite scale,
rotation, hierarchy, and animation affect collider presentation consistently.

## Navigation and pathfinding

As a developer, I can designate walkable surfaces, non-walkable regions, area
types, traversal costs, obstacles, and explicit traversal links. I can define
agent radius, height, step, slope, speed, acceleration, stopping distance, and
turning behavior.

At runtime an agent can request a destination and expose:

* whether a path is pending, complete, partial, invalid, or stale;
* path corners, remaining distance, desired velocity, and actual velocity;
* whether it is on navigable space, on a link, stopped, or at its destination;
* the reason a request cannot produce a usable path when known.

Agents avoid one another and supported dynamic obstacles within declared
limits. A destination outside navigable space, a disconnected region, a narrow
passage, runtime obstacle change, or removed surface yields explicit path state.

Navigation visualization shows navigable regions, areas, links, obstacles,
agent footprints, and current paths. The same scene geometry produces matching
navigation meaning in editor play and built players.

## Terrain authoring

As a world designer, I can create and edit terrain height, holes, surface
layers, trees, detail objects, and supported vegetation. I can inspect bounds,
world size, height and surface resolution, material, collider state, vegetation
density, and runtime cost.

Terrain editing supports selections and undo. Neighboring terrain sections meet
without unintended gaps where their declared data aligns. Surface textures,
normals, holes, vegetation, lighting, shadows, rendering, and collision agree on
the same terrain shape.

Large terrain content communicates loading, memory, rendering, and physics
limitations. Missing layers or vegetation assets remain identifiable and do not
silently substitute unrelated content.

## Diagnostics

The editor can display bodies, collider shapes, centers of mass, constraints,
contacts, sleeping state, trigger pairs, query results, navigation regions,
paths, terrain bounds, and costly content. A diagnostic links to the responsible
object, component, layer rule, terrain region, or setting.

## Acceptance stories

1. A dynamic object collides, sleeps, wakes, and emits the callbacks declared by
   the target's compatible behavior across editor play and a supported player;
   this does not promise bit-identical simulation across platforms.
2. Layer and trigger rules affect simulation events and spatial queries exactly
   as declared.
3. A character traverses supported steps, slopes, and moving platforms while
   reporting grounded and collision state accurately.
4. A navigation agent reports complete, partial, and invalid paths distinctly
   and explains unreachable destinations.
5. Terrain rendering and collision agree with its authored height, holes, and
   world placement.

