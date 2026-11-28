# Physics, Navigation, and Terrain

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

### Compatibility tiers

PHY-001, PHY-002, and PHY-003 are Semantic: they behave as Unity 2022.3 LTS
does in ordinary use, and a found difference is reported Partially supported
with the difference named. PHY-004, NAV-001, and ENV-001 are Familiar: Unity
content imports with its identity, public types and members keep their Unity
2022.3 LTS names and signatures, and this document defines their behavior.
Every difference from Unity 2022.3 LTS in a Familiar feature is listed in the
API reference (AUT-006 in [Automation and MCP](automation-and-mcp.md)) and in
project compatibility reports. Tiers are defined in the
[Compatibility contract](compatibility-contract.md).

## Architecture

The domain is four subsystems below scripting, layered as in
[Engine architecture](architecture.md): 3D physics, 2D physics, navigation, and terrain. Each
derives its state from the one authoritative world through change tracking, registers its work
into the PlayerLoop, and keeps no second copy of scene state.

* **Solver worlds.** 3D and 2D physics each run on a mature, established solver. Every
  `PhysicsScene` and `PhysicsScene2D`, default or local, is one solver world; 2D and 3D worlds
  share nothing. Bodies, colliders, joints, articulations, wheels, and cloth are solver objects
  derived from their components. Collision meshes, hulls, and heightfields come from cooked
  import artifacts; runtime cooking (`Physics.BakeMesh`, tilemap and composite colliders) uses
  the same cooker.
* **Step.** A step runs where the simulation mode places it (fixed update, Update, or inside
  `Simulate`) as sync in, solve, sync out, dispatch. Sync in applies the component, activation,
  layer, and transform changes recorded since the last step; sync out writes dynamic poses to
  the world and marks them changed. Their cost follows changed and awake content, never the
  count of sleeping or static colliders. Interpolation writes only presented transforms.
* **Parallelism.** Solver work runs as jobs on the engine job system; no solver owns threads.
  Contact modification callbacks run on those jobs during the solve; collision and trigger
  messages, `Physics.ContactEvent`, and joint breaks are gathered during the step and
  dispatched on the main thread after it. By default the solver runs with SIMD and in
  parallel across simulation islands, at the speed the release's budgets require. The
  project's enhanced-determinism setting, off by default as in Unity, selects a
  configuration whose results depend only on inputs and target, never on job scheduling,
  and it costs speed only in projects that enable it.
* **Queries.** Queries read a consistent snapshot: the last step's result plus the changes the
  auto-sync-transforms rule has flushed. Batch queries run as jobs against that snapshot; no
  query observes a half-applied step. Character controllers and wheels use the same queries.
* **Navigation.** A build snapshots its source geometry on the main thread, then voxelizes and
  tiles it as jobs with per-tile progress and cancellation; a finished build replaces the
  previous data atomically at a frame boundary, and a failed or cancelled one leaves it in
  place. Carving and `UpdateNavMesh` rebuild only affected tiles; edit-time builds use the same
  builder. Agent update (path following, steering, avoidance) runs as jobs in navigation's
  PlayerLoop phases; path requests resolve across frames, which is the pending state the API
  reports, and script queries read the same data the agents use.
* **Terrain.** `TerrainData` exists once in the asset system, shared by every terrain and
  collider that uses it. The terrain subsystem derives from it render proxies (height and
  control textures, level-of-detail patches, instanced detail and tree batches, billboards) for
  the render graph and a solver heightfield with tree colliders. An edit marks changed regions,
  and only those are re-derived before the next render and physics sync; `SetHeightsDelayLOD`
  defers the level-of-detail rebuild to `SyncHeightmap`. Terrain loads asynchronously with its
  scene, and drawing cost follows visible patches and instances, not terrain size.
* **Origin shifts** are one world operation between frames: every subsystem offsets its derived
  state (solver origins, navigation data, terrain, render and audio proxies) without
  re-deriving it.
* **Everywhere.** Edit mode, play mode, headless sessions, and every player run these
  subsystems unchanged; editor visualization reads their state as debug-draw data, and
  automation advances simulation only through `Simulate` and play control.

## Three-dimensional physics

As a developer, I can use static, kinematic, and dynamic bodies with box,
sphere, capsule, and mesh colliders. Bodies expose mass, center of mass,
inertia tensor, gravity participation, linear and angular velocity, drag,
constraints, interpolation, collision detection mode (discrete, continuous,
continuous dynamic, continuous speculative), sleeping, and the `Rigidbody`
force and torque operations with each `ForceMode`.

The `Rigidbody` scripting surface behaves as in Unity, including:

* `MovePosition` and `MoveRotation` on kinematic bodies move the body during
  the next simulation step, honor interpolation, and generate contacts with
  the bodies swept through; writing `Transform.position`, `Transform.rotation`,
  `Rigidbody.position`, or `Rigidbody.rotation` teleports the body without
  sweeping.
* Setting `velocity` and `angularVelocity` replaces the current values for the
  next step; `AddForce`, `AddRelativeForce`, `AddForceAtPosition`,
  `AddTorque`, `AddRelativeTorque`, and `AddExplosionForce` accumulate until
  the next step.
* `GetPointVelocity`, `GetRelativePointVelocity`, `ClosestPointOnBounds`,
  `maxDepenetrationVelocity`, `maxAngularVelocity`, and per-body
  `solverIterations` and `solverVelocityIterations`.
* Toggling `isKinematic` keeps the body's position and changes its role at the
  next step; `Sleep`, `WakeUp`, `IsSleeping`, and `sleepThreshold` control
  and report sleeping.
* `ConstantForce` applies its force, relative force, torque, and relative
  torque every step.
* `includeLayers`, `excludeLayers`, and `layerOverridePriority` on bodies and
  colliders add or remove layers from the collision matrix for that body or
  collider, with the higher priority deciding a conflict between the two
  sides of a pair.
* `Physics.ContactEvent` delivers the step's contact pairs to subscribers
  after the step, and `Physics.ContactModifyEvent` and
  `Physics.ContactModifyEventCCD` let scripts change or ignore individual
  contact points before the solver uses them, for colliders that request
  contact modification.
* `Physics.ComputePenetration`, `Physics.ClosestPoint`, `Collider.ClosestPoint`,
  and `Physics.BakeMesh` return separation, closest points, and a cooked
  mesh for the requested shapes.

Colliders expose shape, center, orientation where applicable, trigger state,
enabled state, material, contact offset, layer participation, and owning body.
Changing object activation, collider enablement, hierarchy, scale, or body role
updates its physical participation at the next simulation step, or before the
next query when auto sync transforms is on.

Physics materials produce observable friction and restitution behavior,
including their friction and bounce combine modes. A material value outside
the range Unity 2022.3 LTS accepts, such as negative friction or bounciness
outside 0 to 1, is clamped as Unity clamps it and is reported naming the
material and the affected collider or body.

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
  the contact pairs mode apply as in Unity.
* The solver type (projected or temporal Gauss-Seidel), friction type,
  broadphase type with its world bounds and subdivisions, enhanced
  determinism, adaptive force, improved patch friction, contact generation
  mode, unified heightmaps, default maximum depenetration velocity, and the
  cloth gravity and inter-collision settings apply as in Unity.

Every physics setting that Unity 2022.3 LTS defines is honored. A settings key
that Unity 2022.3 LTS does not define is preserved and reported Unknown with
its name.

## Simulation timing and ownership

Physics advances on fixed simulation steps and respects the project's fixed
timestep, maximum allowed timestep, and time scale. A slow rendered frame can
contain multiple fixed steps, up to the maximum allowed timestep. Pausing, frame
stepping, and resuming preserve simulation state: positions, velocities,
contacts, and sleeping state after resume are the state that existed at pause.

Dynamic bodies own their simulated position. Kinematic bodies follow their
transforms and `MovePosition`/`MoveRotation` requests, and static colliders
move only when their transforms are changed. Interpolation
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

Which messages a pair exchanges follows Unity's collision action matrix for
static, kinematic, and dynamic bodies and trigger colliders. For example, a
collision or trigger message requires a `Rigidbody` on at least one side, two
static colliders exchange no messages, and a kinematic body receives collision
messages only from dynamic bodies. Messages are sent to the collider's
GameObject and to the GameObject of its attached `Rigidbody`, including
disabled MonoBehaviours on those objects, as in Unity.

No event is reported for a pair excluded by its effective collision rules.
Destroying or disabling an object during a callback leaves subsequent event
eligibility and recipients defined and inspectable; callback ordering is
guaranteed only where Unity 2022.3 LTS documents an order (Behavioral
Equivalence in the [Compatibility contract](compatibility-contract.md));
elsewhere the order is unspecified.

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
ordering is guaranteed only where Unity 2022.3 LTS documents an order
(Behavioral Equivalence in the
[Compatibility contract](compatibility-contract.md)); elsewhere the order is
unspecified.

## Joints and constraints

Fixed, hinge, spring, character, and configurable joints relate bodies with
visible anchors, axes, limits, motors, drives, break force and torque
thresholds, connected bodies, and connected mass scale. Constraints retain
their anchors, axes, and limits when either body sleeps or becomes kinematic;
disabling or destroying either body or the joint removes the constraint, and
re-enabling it restores the constraint at its current anchors. A broken joint calls `OnJointBreak` and is observable by
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
start in colliders, apply as in Unity. `PhysicsMaterial2D` friction and
bounciness apply as in Unity.

### Bodies

`Rigidbody2D` supports the Dynamic, Kinematic, and Static body types. Dynamic
bodies respond to forces, gravity scale, drag, and constraints. Kinematic
bodies move only by velocity or `MovePosition`/`MoveRotation` and contact
other kinematic and static bodies only when full kinematic contacts is
enabled. Static bodies do not move under simulation. The simulated flag
removes a body and its colliders from the simulation without destroying them.
Collision detection mode, sleeping mode, and interpolation behave as in
Unity.

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
2D joints behave as in Unity, including motors, limits, and break thresholds. `Physics2D` casts and overlaps, including `ContactFilter2D`
filtering by layer, depth, normal angle, and trigger use, return results with
the same identity and capacity rules as 3D queries.

### 2D collision and trigger messages

`OnCollisionEnter2D`, `OnCollisionStay2D`, `OnCollisionExit2D`,
`OnTriggerEnter2D`, `OnTriggerStay2D`, and `OnTriggerExit2D` follow the same
recipient and eligibility rules as their 3D counterparts, using the 2D body
types. `Collision2D` identifies the other collider, body, relative velocity,
and `ContactPoint2D` contacts. `Collider2D.IsTouching`, `IsTouchingLayers`,
`GetContacts`, `Physics2D.IgnoreCollision`, `Physics2D.IgnoreLayerCollision`,
and `Physics2D.callbacksOnDisable` behave as in Unity.

## Vehicles, articulations, cloth, and simulation control

### Wheel colliders

As a vehicle developer, I can use `WheelCollider` with radius, mass,
suspension distance, force application point, suspension spring (spring,
damper, target position), and forward and sideways friction curves (extremum
slip and value, asymptote slip and value, stiffness). Motor torque, brake
torque, and steer angle change wheel behavior on the next simulation step.
Each wheel exposes `isGrounded`, `rpm`, sprung mass, `GetGroundHit`, and
`GetWorldPose`.

* The suspension pushes the attached body with a force set by the spring,
  damper, and the wheel's compression relative to the target position; a
  wheel whose ray finds no ground within the suspension distance reports
  `isGrounded` false and applies no force.
* Friction force along each direction follows the authored friction curve as
  a function of slip, scaled by stiffness and the ground collider's
  material.
* `ConfigureVehicleSubsteps(speedThreshold, stepsBelowThreshold,
  stepsAboveThreshold)` sets how many wheel substeps run in each simulation
  step, depending on whether the vehicle's speed is below or above the
  threshold.
* Wheel results are repeatable for the same inputs on the same target. They
  are not promised to match Unity's numeric results.

### Articulation bodies

As a robotics or mechanism developer, I can build `ArticulationBody`
hierarchies with fixed, prismatic, revolute, and spherical joints. Each joint
axis has motion (locked, limited, or free), limits, and a drive with
stiffness, damping, force limit, target, and target velocity. The root can be
immovable. Scripts read and write joint positions, velocities, and forces in
reduced coordinates and apply forces and torques.

* A locked axis permits no motion; a limited axis keeps the joint position
  between its lower and upper limits; a free axis has no limit.
* A drive applies force toward its target position and target velocity in
  proportion to stiffness and damping, never exceeding its force limit.
* An immovable root does not move under simulation, and its children move
  relative to it.

### Cloth

As an artist, I can simulate `Cloth` on a skinned mesh with per-vertex
constraints (maximum distance and surface penetration), stretching and
bending stiffness, damping, external and random acceleration, world velocity
and acceleration scale, friction, and self-collision.

* A vertex never moves farther from its skinned position than its maximum
  distance; a maximum distance of zero pins the vertex to the skinned mesh.
* Cloth collides only with the sphere colliders, capsule colliders, and
  sphere-pair capsules assigned to its component; it does not collide with
  other scene colliders.
* `ClearTransformMotion` and a teleport of the cloth's transform discard
  accumulated motion so the cloth does not swing from the jump.

### Simulation control

The simulation mode decides when physics advances:

* With `Physics.simulationMode` set to Fixed Update, physics advances once per
  fixed step; with Update, once per frame by that frame's delta time; with
  Script, only when a script calls `Physics.Simulate`. The obsolete
  `Physics.autoSimulation` reads true for Fixed Update and false for Script,
  and setting it selects those modes.
* `Physics.Simulate(step)` advances the default physics scene by the
  requested step, and collision and trigger callbacks for that step occur
  during the call. Calling it outside Script mode reports an error and does
  not advance the simulation.
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
links) connect regions with their authored endpoints, width,
bidirectionality, area, and cost override. An agent reports `isOnOffMeshLink` and the current link
data; with automatic traversal off, the agent waits on the link until a script
moves it and calls `CompleteOffMeshLink`.

### Agents, obstacles, and queries

As a developer, I can define agent radius, height, base offset, speed, angular
speed, acceleration, stopping distance, auto braking, auto repath, obstacle
avoidance quality, and priority.

The `NavMeshAgent` scripting surface includes `SetDestination`,
`destination`, `path` and `SetPath`, `ResetPath`, `isStopped`, `Warp`, `Move`,
`nextPosition`, `updatePosition`, `updateRotation`, `updateUpAxis`,
`autoRepath`, and queries filtered by `NavMeshQueryFilter`:

* With `updatePosition` off, the agent's simulated position advances in
  `nextPosition` while the transform stays where scripts put it; with
  `updateRotation` off, the agent does not rotate its transform; with
  `updateUpAxis` off, the agent does not align its transform's up axis to the
  navigation surface.
* `isStopped` true halts movement along the current path without discarding
  it; `ResetPath` discards it.
* `Warp` moves the agent to a position on navigation data without a path and
  returns false when no navigation data is near that position.
* With `autoRepath` on, an agent whose path becomes invalid requests a new
  path to the same destination.
* An agent that is not placed on navigation data reports `isOnNavMesh` false,
  and calling `SetDestination` or the path-dependent members raises an error
  that names the agent and the member rather than doing nothing silently.

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
agent footprints, and current paths. The same navigation data produces the
same path states, path corners, and query results, within floating-point
variation, in editor play and built players.

## Terrain and environment

### Terrain authoring

As a world designer, I can create and edit terrain height, holes, surface
layers, trees, detail objects, and grass. I can inspect bounds, world size,
height and surface resolution, material, collider state, vegetation density,
resident memory, load time, draw count, and collider count.

Terrain editing supports selections and undo. Neighboring terrains connect by
grouping and auto-connect settings and meet without gaps when their edge
heights match. Surface textures, normals, holes, vegetation, lighting, shadows,
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
`Terrain.SampleHeight`. Runtime terrain changes affect rendering and collision immediately for
`SetHeights`, and at the next `SyncHeightmap` call for
`SetHeightsDelayLOD`. The serialized form of terrain data,
including binary storage, is owned by SER-005 in
[Serialization and prefabs](serialization-and-prefabs.md).

Each terrain reports its resident memory, load time, draw count, and collider
count in the inspector and the profiler (PRF-001). Missing layers or vegetation assets remain identifiable and do not
silently substitute unrelated content.

### Terrain rendering and trees

Terrain rendering honors draw instanced, pixel error, basemap distance,
detail distance and density, tree distance, billboard start, fade length, and
maximum mesh trees. Detail prototypes render in grass texture, vertex lit, and
instanced modes. Trees render with billboards and LOD transitions, and trees
with colliders on their prototype contribute colliders when the terrain
collider enables tree colliders. Terrain materials include the Built-in
Terrain Standard material and the URP Terrain Lit material, and custom terrain
materials follow the shader rules owned by REN-002 in
[Rendering and animation](rendering-and-animation.md).

SpeedTree 7 (`.spm`) and SpeedTree 8 (`.st`) assets are Supported: geometry,
materials, LOD and LOD transitions, billboards, and SpeedTree wind animation.
Wind follows the asset's wind quality setting (none, fastest, fast, better,
best, or palm), responds to the `WindZone` components in the scene, and
animates trunks, branches, leaves, and billboards consistently, in the
Built-in Render Pipeline and URP. A tree whose wind data is missing or
malformed renders without wind and reports the tree and the asset.

### Wind zones

`WindZone` components in directional and spherical modes, with main strength,
turbulence, pulse magnitude, pulse frequency, and radius, move Tree Creator
trees and other tree content whose materials respond to wind. Terrain grass waves according to the terrain's wind settings.
Wind effects on particles are owned by REN-004 in
[Rendering and animation](rendering-and-animation.md).

### Large worlds

The large-world project setting selects how precision behaves far from the
origin. A project opened from Unity starts with the Unity behavior; a person
or agent can change the setting at any time, and the change takes effect at
the next play session.

* **Unity single precision** keeps Unity's behavior. Positions, rotations, and
  simulation use single-precision (32-bit) floating-point values, and unitas
  does not rebase the world origin, so rendering, physics, animation,
  navigation, and audio far from the origin lose precision in proportion to
  the distance, visible as positional jitter and coarser contacts and paths.
  A project that moves its content toward the origin by script keeps working
  as it does in Unity.
* **unitas large world** keeps rendering, physics, animation, navigation,
  particles, trails, and audio far from the origin within the same
  frame-to-frame position and contact error as an identical run at the origin,
  for worlds at least 10,000,000 units across. unitas shifts the world origin
  to follow a focus object, the main camera by default, when the focus moves
  more than a configurable distance from the current origin. A shift moves
  every scene object, physics body, navigation data set, terrain, particle,
  trail, and audio source by the same offset between two frames, keeps
  velocities, contacts, sleeping state, paths, and interpolation unchanged,
  and raises an origin-shift notification with the offset before the next
  frame's scripts run. Scripts read and write `Transform.position` and every
  other position API relative to the current origin, with their Unity
  signatures. Scripts that cache world positions apply the offset from the
  notification. Every world-space value that the editor, automation, logs,
  and scenarios report also carries the cumulative origin offset, so a
  position is identifiable across shifts.

In either mode, the editor and automation identify objects with any
world-position component of magnitude 100,000 units or more from the current
origin (the threshold at which Unity warns about precision) and the systems
on them whose results are affected.

## Diagnostics

The editor and automation display bodies, collider shapes, centers of mass,
constraints, contacts, sleeping state, trigger pairs, query results, wheel
contacts and suspension, articulation joint state, cloth constraints,
navigation regions, navigation build state, paths, terrain bounds, and costly
content. A diagnostic links to the responsible object, component, layer rule,
terrain region, or setting.

## Acceptance outcomes

1. A dynamic box dropped onto a static floor receives `OnCollisionEnter` once,
   `OnCollisionStay` while resting, falls asleep after its velocity stays below
   the sleep threshold, wakes when a force is applied, and receives
   `OnCollisionExit` when lifted off, in editor play and in a supported
   player; the sequence of collision, sleep, and wake events is identical, and
   positions agree within the per-axis position tolerance that product
   knowledge (AUT-006) declares for the release, not bit-identically. (PHY-001)
2. Removing a layer pair from the collision matrix stops collision and trigger
   callbacks between those layers and their hits in default-mask queries; with
   auto sync transforms off, a moved collider is not visible to queries until
   `Physics.SyncTransforms` or the next simulation step; a collider with no
   material uses the default physics material; the same holds for the 2D
   matrix and 2D queries. (PHY-001, PHY-002)
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
   joints without exceeding their authored limits, and cloth on a character collides with its
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
12. Tree Creator trees and terrain grass respond to a `WindZone`, and an object
    at a world-position component of 100,000 units is identified with its
    affected systems. (ENV-001)
13. A kinematic body moved with `MovePosition` pushes a dynamic body and both
    receive collision messages, while a trigger between two colliders without
    a `Rigidbody` produces no trigger messages; a disabled MonoBehaviour on the
    collider's GameObject still receives `OnTriggerEnter`. (PHY-001)
14. A 2D dynamic body hitting a static `BoxCollider2D` delivers
    `OnCollisionEnter2D` with `ContactPoint2D` contacts, `IsTouching` reports
    true while they touch, and `Physics2D.IgnoreCollision` stops further
    messages. (PHY-002)
15. An agent with `updatePosition` off advances `nextPosition` while its
    transform stays put, `Warp` relocates it without a path, and an agent
    placed off navigation data reports `isOnNavMesh` false and `SetDestination`
    raises an error naming the agent. (NAV-001)
16. A terrain using instanced drawing, instanced detail grass, and billboarded
    trees renders with the URP Terrain Lit material, and a SpeedTree 8 tree
    renders with its LODs and sways in its wind quality setting under a
    `WindZone`. (ENV-001)
17. Two colliders on layers that the collision matrix separates collide when
    one collider's `includeLayers` adds the other's layer, and a
    `Physics.ContactModifyEvent` subscriber that ignores a contact point stops
    that point from pushing the bodies apart. (PHY-001)
18. With the temporal Gauss-Seidel solver and enhanced determinism selected in
    the physics settings, the same simulation run twice on one target yields
    identical results. (PHY-001)
19. With the unitas large world setting, a vehicle driven 5,000,000 units from
    the origin has frame-to-frame position and wheel-contact error within the
    tolerance of an identical run at the origin, and a script subscribed to the origin-shift
    notification receives each offset; with the Unity single precision
    setting, the same project has no origin shifts. (ENV-001, PHY-004)
