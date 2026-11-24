# Physics, Navigation, and Terrain Product Specification

## Scope

This specification defines player-visible motion, collision, spatial queries,
character movement, vehicles, articulated bodies, cloth, simulation control,
navigation, terrain, and large-world behavior. It owns PHY-001 through
PHY-004, NAV-001, and ENV-001 in the [Feature catalog](feature-catalog.md).

The physics, 2D physics, time, layer, and navigation-area settings that these
features read are represented and edited as project settings, owned by PRJ-007
in [Projects and assets](project-and-assets.md). This document defines how
those settings change simulation behavior.

Every capability in this document is available to authorized agents through
automation, including inspecting bodies, running queries, stepping simulation,
and building navigation data, as defined in
[Automation and MCP](automation-and-mcp.md).

## Three-dimensional physics

As a developer, I can use static, kinematic, and dynamic bodies with box,
sphere, capsule, and mesh colliders. Bodies expose mass, center of mass,
inertia tensor, gravity participation, linear and angular velocity, drag,
constraints, interpolation, collision detection mode (discrete, continuous,
continuous dynamic, continuous speculative), sleeping, and the `Rigidbody`
force and torque operations with each `ForceMode`.

Colliders expose shape, center, orientation where applicable, trigger state,
enabled state, material, contact offset, layer participation, and owning body.
Changing object activation, collider enablement, hierarchy, scale, or body role
updates its physical participation at a defined simulation boundary.

Physics materials produce observable friction and restitution behavior,
including their friction and bounce combine modes. Invalid or unsupported
combinations identify the affected collider or body and do not silently
simulate as an unrelated shape.

### Mesh colliders

A `MeshCollider` is either convex or non-convex:

* A convex mesh collider can belong to a dynamic body and can be a trigger. Its
  generated hull respects the Unity-compatible 255-triangle limit; a source
  mesh that exceeds the limit produces the reduced hull Unity produces and a
  diagnostic naming the mesh.
* A non-convex mesh collider collides only as static or kinematic geometry.
  Attaching it to a dynamic body, or marking it as a trigger, produces the
  Unity-compatible diagnostic naming the object, and the collider does not
  participate as a dynamic or trigger shape.
* Mesh cooking options, a missing or unreadable mesh, degenerate triangles,
  and meshes that are not readable at runtime produce diagnostics that name
  the collider and mesh rather than an empty or substituted shape.

### Physics project settings

The project's physics settings change simulation and queries as follows:

* The layer collision matrix decides which layer pairs can collide, trigger,
  and be returned by queries that use the default mask.
* Gravity, default contact offset, default solver iterations, default solver
  velocity iterations, bounce threshold, sleep threshold, and default maximum
  angular speed apply to every body and collider that does not override them.
* The default physics material applies to colliders that have no material.
* Auto sync transforms decides whether transform changes reach the physics
  world before each query or only at the next simulation step; with it off,
  `Physics.SyncTransforms` applies pending changes on request.
* Queries hit triggers, queries hit backfaces, reuse collision callbacks, and
  the contact pairs mode apply as their Unity settings declare.

A setting that is recognized but not honored is reported with its name and
consequence; it is never silently ignored.

## Simulation timing and ownership

Physics advances on fixed simulation steps and respects the project's fixed
timestep, maximum allowed timestep, and time scale. A slow rendered frame can
contain multiple fixed steps, up to the maximum allowed timestep. Pausing, frame
stepping, and resuming preserve simulation state: positions, velocities,
contacts, and sleeping state after resume are the state that existed at pause.

Dynamic bodies own their simulated position. Kinematic and static objects
respond to transform changes according to their declared roles. Interpolation
and extrapolation change only presented transforms between steps, not
simulated positions. The editor and automation identify whether a script,
animation, parenting change, or physics simulation is the effective source of
motion for a selected object.

## Collision and trigger events

Eligible scripts receive `OnCollisionEnter`, `OnCollisionStay`,
`OnCollisionExit`, `OnTriggerEnter`, `OnTriggerStay`, and `OnTriggerExit`.
Collision information identifies the other collider, body, object, relative
velocity, impulse, and contacts. Events reflect layers, ignored pairs
(`Physics.IgnoreCollision`, `Physics.IgnoreLayerCollision`), activation,
destruction, shape changes, teleportation, and scene unloading.

No event is reported for a pair excluded by its effective collision rules.
Destroying or disabling an object during a callback leaves subsequent event
eligibility and recipients defined and inspectable; callback ordering is
guaranteed only where the compatible API contract defines it.

## Spatial queries

As a programmer, I can perform raycasts, line casts, shape casts, and overlap
queries, including their allocating, non-allocating, and `RaycastCommand`
batch forms. Queries accept origin or shape, direction where relevant,
distance, layer mask, and `QueryTriggerInteraction`. Results identify object,
collider, body, point, normal, distance, and relevant sub-shape information
such as triangle index and texture coordinates for mesh hits.

Queries define behavior for zero-length direction, starts inside a collider,
ties, disabled or inactive content, backfaces, non-uniform scale, and invalid
inputs. Multi-result APIs expose qualifying results within their declared
capacity; capacity-limited forms report the returned count. Query result
ordering is guaranteed only when the compatible API contract defines it.

## Joints and constraints

Fixed, hinge, spring, character, and configurable joints relate bodies with
visible anchors, axes, limits, motors, drives, break force and torque
thresholds, connected bodies, and connected mass scale. Constraints retain
their declared meaning when either body sleeps, becomes kinematic, is disabled,
or is destroyed. A broken joint calls `OnJointBreak` and is observable by
scripts and diagnostics.

## Character movement

As a gameplay developer, I can move a collision-aware character with
`CharacterController` through steps, slopes, walls, ceilings, platforms, and
ground contacts without treating it as a freely tumbling rigid body.

The controller honors slope limit, step offset, skin width, minimum move
distance, radius, height, and center. Character state exposes `isGrounded`,
`collisionFlags`, and velocity, and `OnControllerColliderHit` reports each
contact made during `Move`. Movement has defined behavior for excessive
slopes, narrow steps, moving platforms, teleportation, crouch-sized shape
changes, overlapping starts, `Move` and `SimpleMove` differences, and multiple
motions in one frame.

## Two-dimensional physics

For 2D games, bodies, colliders, triggers, joints, effectors, layer rules,
casts, overlaps, contacts, sleeping, gravity, and fixed-step events operate in
the two-dimensional world. 2D and 3D physics remain distinct; content does not
interact across systems. The project's 2D physics settings, including the 2D
layer collision matrix, gravity, velocity and position iterations, and queries
start in colliders, apply as their Unity settings declare.

### Bodies

`Rigidbody2D` supports the Dynamic, Kinematic, and Static body types. Dynamic
bodies respond to forces, gravity scale, drag, and constraints. Kinematic
bodies move only by velocity or `MovePosition`/`MoveRotation` and contact
other kinematic and static bodies only when full kinematic contacts is
enabled. Static bodies do not move under simulation. The simulated flag
removes a body and its colliders from the simulation without destroying them.
Collision detection mode, sleeping mode, and interpolation behave as their
Unity settings declare.

### Colliders

Box, circle, capsule, polygon, and edge colliders support offset, trigger
state, material, and edge radius where applicable. Sprite-generated polygon
shapes use the sprite's physics shape (see REN-005 in
[Rendering and animation](rendering-and-animation.md)). Sprite scale,
rotation, hierarchy, and animation affect collider shape consistently.

`CompositeCollider2D` merges the colliders marked used by composite into one
shape with outline or polygon geometry, using synchronous or manual
generation; manual generation changes the shape only when the composite is
regenerated. `TilemapCollider2D` generates collision from tiles according to
each tile's collider type (none, sprite, or grid) and updates when tiles
change; with used by composite, its shape merges into the composite.

### Effectors

Colliders marked used by effector apply these effects:

* `PlatformEffector2D`: one-way collision with a surface arc, side friction and
  bounce, side arc, and rotational offset. A body passing through from the
  excluded direction does not collide and generates no collision event for
  that pass.
* `SurfaceEffector2D`: tangential speed and speed variation along the surface.
* `AreaEffector2D`: force with angle, magnitude, variation, target (collider or
  body), and drag.
* `PointEffector2D`: force toward or away from a point with distance scaling
  modes.
* `BuoyancyEffector2D`: a surface level, density, flow, and drag that produce
  floating and sinking.

Effector collider masks and the colliders' layers decide which bodies are
affected.

### Joints and queries

Distance, fixed, friction, hinge, relative, slider, spring, target, and wheel
2D joints behave as their Unity settings declare, including motors, limits, and
break thresholds. `Physics2D` casts and overlaps, including `ContactFilter2D`
filtering by layer, depth, normal angle, and trigger use, return results with
the same identity and capacity rules as 3D queries.

## Vehicles, articulations, cloth, and simulation control

### Wheel colliders

As a vehicle developer, I can use `WheelCollider` with radius, mass,
suspension distance, force application point, suspension spring (spring,
damper, target position), and forward and sideways friction curves (extremum
slip and value, asymptote slip and value, stiffness). Motor torque, brake
torque, and steer angle change wheel behavior on the next simulation step.
Each wheel exposes `isGrounded`, `rpm`, sprung mass, `GetGroundHit`, and
`GetWorldPose`. `ConfigureVehicleSubsteps` changes wheel substepping as Unity
declares. Wheel behavior is equivalent to Unity within the declared
floating-point tolerance and is not promised to be bit-identical.

### Articulation bodies

As a robotics or mechanism developer, I can build `ArticulationBody`
hierarchies with fixed, prismatic, revolute, and spherical joints. Each joint
axis has motion (locked, limited, or free), limits, and a drive with
stiffness, damping, force limit, target, and target velocity. The root can be
immovable. Scripts read and write joint positions, velocities, and forces in
reduced coordinates and apply forces and torques. Members that do not exist in
the project's baseline Unity release are unavailable to its scripts, as for
any version-specific API.

### Cloth

As an artist, I can simulate `Cloth` on a skinned mesh with per-vertex
constraints (maximum distance and surface penetration), stretching and
bending stiffness, damping, external and random acceleration, world velocity
and acceleration scale, friction, and self-collision. Cloth collides only with
the sphere, capsule, and sphere-pair capsule colliders assigned to it, as in
Unity; it does not collide with other scene colliders. Cloth state resets and
teleports as the corresponding Unity members declare.

### Simulation control

The simulation mode decides when physics advances:

* `Physics.simulationMode` (Fixed Update, Update, or Script) and, in baseline
  releases that use it, `Physics.autoSimulation`, behave as the project's
  baseline Unity release declares.
* In script mode, physics advances only when a script calls
  `Physics.Simulate`, by the requested step, and collision and trigger
  callbacks for that step occur during the call.
* `Physics2D.simulationMode` and `Physics2D.Simulate` provide the same control
  for 2D physics.

A scene loaded or created with a local physics mode (`LocalPhysicsMode.Physics3D`
or `LocalPhysicsMode.Physics2D`) has its own `PhysicsScene` or
`PhysicsScene2D`. Objects in a local physics scene do not collide with or
appear in queries of the default physics scene; that scene advances only when
a script calls its `Simulate`, and its queries return only its own content.
Scene loading itself is owned by RUN-005 in
[Scripting runtime](scripting-runtime.md).

## Navigation and pathfinding

### Navigation data

As a developer, I can use navigation data from either source:

* Legacy built-in navigation: scene objects marked navigation static, the
  scene's baked navigation data asset, and `OffMeshLink` components.
* The AI Navigation package line listed in the supported package matrix
  (CMP-002 in [Compatibility contract](compatibility-contract.md)):
  `NavMeshSurface`, `NavMeshModifier`, `NavMeshModifierVolume`, and
  `NavMeshLink`.

Navigation data that Unity baked is consumed as project content. unitas also
builds navigation data:

* At edit time, a developer or agent builds the data for a scene or a
  `NavMeshSurface` and saves it in a Unity-readable form.
* At runtime, `NavMeshSurface.BuildNavMesh`, `NavMeshSurface.UpdateNavMesh`,
  `NavMeshBuilder.BuildNavMeshData`, `NavMeshBuilder.UpdateNavMeshDataAsync`,
  and `NavMesh.AddNavMeshData`/`RemoveNavMeshData` produce, update, add, and
  remove navigation data.

A build honors agent type, radius, height, step height, maximum slope, voxel
size, tile size, minimum region area, the layers and geometry source (render
meshes or physics colliders) it collects, and modifier overrides. A build
reports progress, can be cancelled, and on failure keeps the previous data.
Saved data that no longer matches its source geometry or settings is reported
stale with the affected surface or scene; it is never silently rebuilt during
play.

### Areas, costs, and links

Navigation areas include the built-in Walkable, Not Walkable, and Jump areas
and the project's named areas. Area costs set in project settings,
`NavMesh.SetAreaCost`, and `NavMeshAgent.SetAreaCost` change path choice; area
masks restrict which areas an agent or query uses.

Off-mesh links (`OffMeshLink`, `NavMeshLink`, and generated drop and jump
links) connect regions with declared endpoints, width, bidirectionality, area,
and cost override. An agent reports `isOnOffMeshLink` and the current link
data; with automatic traversal off, the agent waits on the link until a script
moves it and calls `CompleteOffMeshLink`.

### Agents, obstacles, and queries

As a developer, I can define agent radius, height, base offset, speed, angular
speed, acceleration, stopping distance, auto braking, obstacle avoidance
quality, and priority.

At runtime an agent can request a destination and expose:

* whether a path is pending, complete, partial, invalid, or stale;
* path corners, remaining distance, desired velocity, and actual velocity;
* whether it is on navigable space, on a link, stopped, or at its destination;
* the reason a request cannot produce a usable path when known.

Agents avoid one another and `NavMeshObstacle` obstacles. Carving obstacles,
including carve-only-stationary behavior, change navigable space and make
affected paths replan. A destination outside navigable space, a disconnected
region, a narrow passage, runtime obstacle change, or removed surface yields
explicit path state.

`NavMesh.CalculatePath`, `NavMesh.SamplePosition`, `NavMesh.Raycast`, and
`NavMesh.FindClosestEdge` return results consistent with the agents' paths
over the same data.

Navigation visualization shows navigable regions, areas, links, obstacles,
agent footprints, and current paths. The same navigation data produces
matching navigation meaning in editor play and built players.

## Terrain and environment

### Terrain authoring

As a world designer, I can create and edit terrain height, holes, surface
layers, trees, detail objects, and grass. I can inspect bounds, world size,
height and surface resolution, material, collider state, vegetation density,
and runtime cost.

Terrain editing supports selections and undo. Neighboring terrains connect by
grouping and auto-connect settings and meet without gaps where their declared
data aligns. Surface textures, normals, holes, vegetation, lighting, shadows,
rendering, and collision agree on the same terrain shape.

### Terrain data

`TerrainData` assets hold heightmap resolution and size, `TerrainLayer`
references and alphamaps, hole data, detail prototypes and layers, tree
prototypes and instances, and grass and wind settings. Several `Terrain` and
`TerrainCollider` components can share one `TerrainData`, and an edit is
visible in every user of that asset. `TerrainLayer` assets are shared across
terrains.

Scripts read and change terrain through `TerrainData.GetHeights`/`SetHeights`,
`SetHeightsDelayLOD` with `SyncHeightmap`, `GetAlphamaps`/`SetAlphamaps`,
`SetHoles`, `SetDetailLayer`, and `SetTreeInstances`, and query it with
`Terrain.SampleHeight`. Runtime terrain changes affect rendering and collision
at the boundary the Unity API declares. The serialized form of terrain data,
including binary storage, is owned by SER-005 in
[Serialization and prefabs](serialization-and-prefabs.md).

Large terrain content communicates loading, memory, rendering, and physics
limitations. Missing layers or vegetation assets remain identifiable and do not
silently substitute unrelated content.

### Wind zones

`WindZone` components in directional and spherical modes, with main strength,
turbulence, pulse magnitude, pulse frequency, and radius, move trees that
respond to wind. Terrain grass waves according to the terrain's wind settings.
Wind effects on particles are owned by REN-004 in
[Rendering and animation](rendering-and-animation.md).

### Large worlds

Positions, rotations, and simulation use single-precision values as in Unity.
Rendering, physics, animation, and navigation far from the world origin show
precision loss equivalent to Unity within the declared floating-point
tolerance; unitas does not silently rebase the world origin or change
precision. The editor and automation identify objects whose position magnitude
exceeds the Unity floating-point precision warning threshold and the systems
on them whose results are affected. Projects that move content toward the
origin by script observe the same results as in Unity.

## Diagnostics

The editor and automation display bodies, collider shapes, centers of mass,
constraints, contacts, sleeping state, trigger pairs, query results, wheel
contacts and suspension, articulation joint state, cloth constraints,
navigation regions, navigation build state, paths, terrain bounds, and costly
content. A diagnostic links to the responsible object, component, layer rule,
terrain region, or setting.

## Acceptance stories

1. A dynamic object collides, sleeps, wakes, and emits the callbacks declared by
   the target's compatible behavior across editor play and a supported player;
   this does not promise bit-identical simulation across platforms. (PHY-001)
2. Changing the layer collision matrix, auto sync transforms, or the default
   physics material changes simulation events and query results exactly as
   declared, and layer and trigger rules affect events and queries as declared.
   (PHY-001, PHY-002)
3. A non-convex mesh collider on a dynamic body reports the Unity-compatible
   diagnostic naming the object and does not simulate as a dynamic shape. (PHY-001)
4. A character traverses supported steps, slopes, and moving platforms while
   reporting grounded and collision state accurately. (PHY-003)
5. A 2D player character jumps up through a one-way platform built with
   `PlatformEffector2D`, lands on it from above, and receives collision
   events only for the landing; a tilemap with `TilemapCollider2D` merged
   into a `CompositeCollider2D` updates its collision when a tile is removed.
   (PHY-002)
6. A vehicle built from four `WheelCollider` components accelerates, brakes,
   and steers in response to motor torque, brake torque, and steer angle, and
   reports ground hits per wheel. (PHY-004)
7. An `ArticulationBody` arm reaches drive targets on revolute and prismatic
   joints within declared limits, and cloth on a character collides with its
   assigned capsule colliders only. (PHY-004)
8. With the simulation mode set to script, physics advances only when a script
   calls `Physics.Simulate`; a scene loaded with a local physics mode simulates
   independently of the default physics scene. (PHY-004)
9. A navigation agent reports complete, partial, and invalid paths distinctly
   and explains unreachable destinations. (NAV-001)
10. A `NavMeshSurface` is built at edit time by an agent through automation,
    rebuilt at runtime after level geometry changes, and an agent crosses a
    `NavMeshLink` and prefers the lower-cost area. (NAV-001)
11. Terrain rendering and collision agree with its authored height, holes, and
    world placement, including after a runtime `SetHeights` change shared by
    every terrain that uses the same `TerrainData`. (ENV-001)
12. Trees and grass respond to a `WindZone`, and an object placed beyond the
    floating-point precision threshold is identified with its affected
    systems. (ENV-001)
