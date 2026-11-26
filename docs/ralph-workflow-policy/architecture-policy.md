<!-- ralph-policy-schema: v7 -->
<!-- ralph-policy-id: architecture-policy.md -->
# Architecture Policy

## Purpose and scope

This policy governs component boundaries, dependency direction, state
ownership, external I/O, and durable contracts. It records this project's
architecture without imposing a named pattern or directory layout.

## Default requirements

* Dependencies MUST follow a documented direction; cycles across declared
  architectural components and forbidden cross-boundary imports are prohibited.
* Domain decisions MUST remain separable from delivery frameworks and external
  I/O where the project has such distinctions.
* Mutable state, transactions, concurrency, caches, and background work MUST
  have explicit owners and lifecycles.
* Public APIs, CLI behavior, protocols, schemas, and persisted formats MUST be
  treated as compatibility boundaries when consumers depend on them.
* New abstractions MUST solve a demonstrated boundary, volatility, testing, or
  reuse need; implementation count alone neither requires nor forbids an
  abstraction.
* Durable contracts (registries, classifiers, name lists) are conditional on
  crossing a documented system boundary where external or decoupled consumers
  depend on them; internal-only implementations need not enter named
  registries or classifiers solely because a starter pattern exists.
* Material, hard-to-reverse architectural decisions MUST be reflected in the
  affected `RALPH-FACT:` lines of this policy.
* Architecture MUST be evaluated against explicit quality-attribute scenarios,
  stakeholder needs, operational context, and known tradeoffs rather than a
  preferred pattern in isolation.
* Relevant static, runtime, deployment, interface, and data-flow views MUST be
  documented when those views affect design or operation.

## Guard inventory

| Guard | Owner | Protected outcome | Detected condition | Claim scope | Enforcement | Retirement trigger |
|---|---|---|---|---|---|---|
| Process and resource ownership | Architecture owner | a long-lived process, file lock, socket, or signal handler is owned by a named module that closes it on every code path | the code review records the owner of every long-lived resource | the code review records the owner of every long-lived resource | advisory: no gate observes runtime resource lifetimes yet; review: when the first long-lived resource is added | when a resource-lifecycle check can observe every long-lived resource |
| Conditional durable contracts | Architecture owner | a durable contract is required only when the system that consumes it exists in the project | every required registry or classifier — including the [[nuget]] entries in tools/dependency-registry.toml, consumed by the dependency gate — has a named consumer, and the corpus names no contract without one | every required registry or classifier has a named consumer | deterministic: make policy-validate | when the consumer system is removed |

## Project facts to resolve

The lines below are the verified project facts that agents rely on and keep
current.

RALPH-FACT: architecture_style: layered: the Rust core owns asset parsing and runtime state, the C# bridge owns the UnityEngine API emulation, and the repository tooling is a separate Python package
RALPH-FACT: component_map: the Cargo workspace under crates/ holds eleven member crates, unitas-animation, unitas-asset, unitas-audio, unitas-build, unitas-core, unitas-dotnet, unitas-editor-abi, unitas-jobs, unitas-physics, unitas-render and unitas-runtime; the C# bridge component lives under bridge/ and holds nine projects, UnityEngine, Unitas.Bridge, Unitas.Bridge.Tests, Unitas.Debugging, Unitas.PlayerHost, Unitas.PlayerHost.Tests, Unitas.Scripting, Unitas.Testing and Unitas.Testing.NUnit; the C# editor component lives under editor/ and holds Unitas.Editor and Unitas.Editor.Tests; the repository tooling is a separate Python package under tools/ holding verify.py (the gate runner), cla.py (the CLA checker), verify_device.py, verify_perf.py and verify_samples.py (the non-default lane runners), and the gate tests test_verify.py and test_cla.py; the source walks skip the bin, obj, target, build, dist, node_modules, venv and __pycache__ directories
RALPH-FACT: dependency_direction: the tooling imports the standard library only; within the Rust workspace unitas-core and unitas-jobs depend on no other workspace crate, unitas-animation, unitas-asset and unitas-physics depend on unitas-core, unitas-audio depends on unitas-asset and unitas-core, unitas-dotnet depends on unitas-asset, unitas-render depends on unitas-asset and unitas-core, unitas-runtime depends on unitas-animation, unitas-asset, unitas-audio, unitas-core, unitas-jobs, unitas-physics and unitas-render, unitas-build depends on unitas-asset, unitas-audio, unitas-core, unitas-dotnet, unitas-render and unitas-runtime, and unitas-editor-abi depends on unitas-asset, unitas-build and unitas-core; the Rust workspace has no dependency cycle; in C#, Unitas.Editor references Unitas.Bridge, Unitas.Scripting and Unitas.Testing, Unitas.Editor.Tests references Unitas.Editor, and no bridge project references back into editor/
RALPH-FACT: forbidden_dependencies: the tooling must not import a third-party module, and a crate must not depend on a Unity assembly
RALPH-FACT: state_ownership: a gate owns no state between runs beyond two cache files under .ruff_cache/, secret_scan_cache.json and dotnet_format_cache.json, both declared under verification-policy.md gate_cache_mechanism_and_key; every gate reads the working tree and prints its findings
RALPH-FACT: external_io_boundaries: the gate runner reads the working tree and spawns detect-secrets with an argument vector; it writes nothing outside the repository
RALPH-FACT: durable_contracts: the policy corpus is the single source for the gate commands, and the Makefile is the single source for the gate targets
RALPH-FACT: decision_record_location: docs/ralph-workflow-policy/architecture-policy.md
RALPH-FACT: quality_attribute_scenarios: a full gate run is expected to finish inside the verification budget recorded in docs/ralph-workflow-policy/verification-policy.md, and the Makefile budget check is the enforcement point: a run that exceeds the budget fails the default gate instead of passing, and the overrun is attributed to the stage that overran rather than absorbed by raising the budget
RALPH-FACT: runtime_and_deployment_views: the gates run from a clean clone on the development host; no container, service, or deployment target exists yet
RALPH-FACT: major_data_flows: a Make target runs python3 tools/verify.py, which reads the working tree and writes findings to standard output; the lint, typecheck and test targets also run cargo over the workspace
RALPH-FACT: known_risks_and_tradeoffs: the Rust workspace and C# UnityEngine bridge are implemented and gated by the default gate; the bridge is implemented via a managed host loading the native Rust cdylib (libunitas_runtime.dylib); CoreCLR embedding inside the player is implemented in crates/unitas-dotnet, which loads the hostfxr runtime and resolves the managed entry point the native player calls, and the make verify-samples lane boots the packaged player with CoreCLR in process; the C ABI crates name their C symbols with #[unsafe(export_name = ...)] at the definition site, and the only build.rs in the workspace, crates/unitas-editor-abi/build.rs, exports six unmangled editor symbols through -exported_symbol link arguments, so no alias table of mangled names remains and the link-time alias ban reports nothing; lifecycle dispatch documents a deliberate architectural deviation using public virtual methods instead of private reflection; rendering uses a single wgpu render graph in crates/unitas-render (RenderGraph with opaque, transparent, UI/overlay, and capture-ids passes, compiled unconditionally in the default build without feature gating) as the only renderer for both the windowed player and headless offscreen capture (including per-pixel object-id maps and object screen bounds exposed through crates/unitas-runtime/src/presenter.rs and the native C ABI), and the CPU software rasterizer and dead shadow scaffolding have been deleted; the Milestone 4 physics slice ran a custom minimal 3D physics engine and now runs on the Rapier3D backend recorded in physics_backend_rapier, behind Unity-compatible wrappers, with explicit Dynamic, Kinematic, and Static body roles, Sphere, Box, and Capsule collider shapes (including Unity class 136 CapsuleCollider YAML parsing with radius, height, and local X/Y/Z axis direction plus explicit per-entity geometry diagnostics), a kinematic CharacterController mover solved against the query pipeline (including Unity class 143 and built-in MonoBehaviour CharacterController YAML parsing, step-offset and tread-depth checks, slope-limit clamping, wall/ceiling slide, overlap depenetration, moving-platform velocity carry, teleport state reset, crouch-sized shape changes, unitas_runtime_character_move/teleport/controller_config ABI externs, and UnityEngine.CharacterController with ICharacterControllerProvider), the Rapier3D solver (with arithmetic-mean pair restitution, unclamped restitution, and single dynamic friction rather than separate static/dynamic friction or Unity combine-mode switches) across sphere, box, capsule, and mesh-bounds contacts with continuous collision detection via CollisionDetectionMode3D, post-step transform write-back of simulated dynamic body positions into the native SceneGraph and managed Transform via unitas_runtime_body_position and unitas_runtime_body_velocity, managed-to-native velocity and kinematic position push via unitas_runtime_set_body_velocity and unitas_runtime_set_body_position (with kinematic transform changes taking effect before the next fixed step and driving contact response via per-step sweep velocity), PhysicMaterial YAML reference parsing with explicit per-entity diagnostics when a material reference is unresolved or invalid, and a plain Rust physics world API so a later phase can swap the backend without an ABI change; an entity that carries multiple colliders resolves by precedence to sphere, then box, then capsule in this slice; overlapping pairs where either body is a trigger route exclusively to the trigger channel, bypass the collision solver, and never enter the collision channel, and OnTriggerEnter/Stay/Exit receives a null Collider when the other entity is unregistered in C#; the shipped physics query API filters queries by layer mask and QueryTriggerInteraction (including Physics.queriesHitTriggers defaulting to true for UseGlobal, segment-based Physics.Linecast(start, end, ...), double-sided exit-face hits for point casts starting inside solid colliders, distance-zero hits for swept shapes starting overlapped, and ascending EntityId ordering for overlap_sphere) and never routes collision or trigger pairs.
RALPH-FACT: deferred_physics_scope: MeshCollider shape, capsule sweep casts (Physics.CapsuleCast), NonAlloc query forms, 2D physics, rotation and angular dynamics, and sleeping and wake transitions remain deferred past the shipped Milestone 4 capsule-collider, physics-queries, and kinematic character-controller slices; the 3D rigid joints FixedJoint3D, HingeJoint3D, SpringJoint3D, CharacterJoint3D, and ConfigurableJoint3D are no longer deferred, are implemented in crates/unitas-physics/src/joint.rs, and are simulated by crates/unitas-physics/src/world.rs (reviewed on 2026-11-26); owner: the repository maintainers; trigger: a consumer needs one of the deferred surfaces, or the next Milestone 4 follow-up phase names one of them; lane: the default automated lane when implemented; evidence: the make verify run that proves the shipped slice; review condition: review when a consumer needs one of the deferred surfaces and whenever the next Milestone 4 follow-up phase starts
RALPH-FACT: deferred_collider_scale: the collider size of a GameObject under a scaled ancestor is not scaled by the ancestor scale (the physics world places the collider at the world position and writes simulated dynamic world positions back to local transforms across ancestor transforms, while keeping the local collider size; reviewed on 2026-11-24); owner: the repository maintainers; trigger: a consumer needs a collider whose size follows an ancestor's scale, or the next Milestone 4 follow-up phase names it; lane: the default automated lane when implemented; evidence: the make verify run that proves the shipped slice; review condition: review when the trigger fires and whenever the physics world transform handling changes
RALPH-FACT: physics_backend_rapier: the 3D rigid-body backend is Rapier3D 0.36.1 (default-features off; features dim3, enhanced-determinism, f32, std; no rayon, no debug-render), swapped in behind the unchanged unitas-physics World API when the deferred_physics_backend_swap trigger fired (3D continuous collision detection required by the Marble Rush sample); the backend runs f32, matching Unity PhysX precision, and the World API stays f64 with conversion at the boundary; Dynamic/Kinematic/Static map to rapier dynamic/kinematic-position-based/fixed with kinematic sweep via set_next_kinematic; materials combine with CoefficientCombineRule::Average and restitution stays unclamped; the CollisionMatrix is enforced through PhysicsHooks pair filters; CollisionDetectionMode3D { Discrete, Continuous } is additive on BodyDesc and plumbed through unitas-runtime and the C# bridge Rigidbody.collisionDetectionMode; the hand-written solver is deleted; mesh colliders remain bounds-box approximations because MeshColliderDesc carries no vertex data; rapier auto-sleep is disabled so the World sleep API stays authoritative; dynamic bodies keep locked rotations as before; the articulation module stays on the custom solver layered over World (deferred until a consumer needs Rapier multibody articulations); owner: the repository maintainers; trigger: a consumer needs a capability the f32 rapier configuration lacks, such as multibody articulations or a cross-platform determinism guarantee beyond the enhanced-determinism build; lane: the default automated lane; evidence: the make verify run that proves the shipped slice; review condition: review when the trigger fires and whenever the physics world API or backend changes
RALPH-FACT: deferred_prefab_override_scope: prefab instance expansion applies the instance's m_Name and m_LocalPosition, m_LocalRotation and m_LocalScale modifications to any object of the prefab it names, root or child, and applies renderer m_Materials.Array.data[0] object-reference overrides and flat MonoBehaviour serialized-field overrides to the expanded objects (round-tripping exactly on save; shipped when the deferred trigger fired for the Marble Rush PickupGold variant); added and removed components, added and removed children, material slots beyond index 0, structured property paths containing dots or brackets, scene-instance name and transform propagation through a variant source, and the editor surfaces that apply, revert or unpack an override are not shipped; owner: the repository maintainers; trigger: a consumer needs a structural prefab override or an override-edit surface, or the next prefab follow-up phase names one of them; lane: the default automated lane when implemented; evidence: the make verify run that proves the shipped slice; review condition: review when the trigger fires and whenever the prefab expansion path changes
RALPH-FACT: prefab_fixture_provenance: the asset test fixture crates/unitas-asset/tests/fixtures/project/Camera.prefab was authored with the shipped prefab expansion because only its .meta pre-existed, and crates/unitas-asset/tests/fixtures/prefab.prefab instances it through that guid; the pre-existing .meta still describes a folder importer and is left untouched; owner: the repository maintainers; trigger: the fixture layout changes, or a consumer needs the camera prefab as a real prefab asset rather than a guid placeholder; lane: the default automated lane when implemented; evidence: the make verify run that proves the shipped slice; review condition: review when the trigger fires and whenever the asset test fixtures change
RALPH-FACT: frame_loop_time_ownership: the native runtime owns simulation time and clamps its fixed-step catch-up to Time.maximumDeltaTime (default 1/3 s), discarding the excess accumulated time instead of carrying it into the next frame, and it evaluates the animators, particles, navigation, terrain, and audio between Update and LateUpdate; the scaled frame delta handed to Update, LateUpdate, the Animator, particles, and navigation is not clamped, so a long frame still reports its full scaled delta to scripts; the managed Time API owns presentation time and passes the unscaled wall delta to the native tick, which applies its own time scale; owner: the repository maintainers; trigger: a consumer changes the frame-loop clamp shape, the subsystem evaluation order, or the time-ownership contract between the native runtime and the managed Time API; lane: the default automated lane; evidence: the make verify run that proves the shipped slice (crates/unitas-runtime/tests/frame_loop_tests.rs) and the make verify-samples lane; review condition: review when the trigger fires and whenever the frame loop or the Time API changes
RALPH-FACT: conformance_method: make verify

## AI execution instructions

This section is the architecture-specific procedure only. The five
generic rules (truthful evidence, fail-closed verification, bounded
execution, no weakening of checks, explicit unverified-claim
reporting), the precedence over domain policies, the
selective-applicability rule, and the claim-scope rule are owned by
the kernel starter (`execution-kernel.md`) and MUST be loaded from
there — do not re-derive them here. Tests assert at the narrowest
useful observable boundary (see `testing-policy.md`); exposing a
private method or adding a wrapper solely to satisfy a test is a
violation there, not a fitting architectural response here.

Agents MUST inspect the component map before moving responsibilities, preserve
dependency direction, update the affected facts, and remove superseded
architecture rather than leaving parallel paths. Agents MUST identify affected
quality attributes, runtime/data-flow views, and tradeoffs before claiming an
architectural change is fit.

## Verification

RALPH-COMMAND: make deps
RALPH-REVIEW: evaluate quality scenarios, views, boundaries, risks, and tradeoffs; evidence: dated architecture review or explicit blocker; owner: architecture owner

Success means the dependency gate reports no forbidden import and no
unreviewed contract break.

## Exceptions

An exception requires scope, reason, owner, and a removal or review trigger.

## Maintenance triggers

Review this policy when components, dependency direction, state ownership,
external boundaries, or durable contracts change.

## Research basis

* publisher: Martin Fowler
  title: "Software Architecture Guide"
  http: https://martinfowler.com/architecture/
  review date: 2026-07-12

* publisher: Carnegie Mellon Software Engineering Institute
  title: "Software Architecture"
  http: https://www.sei.cmu.edu/software-architecture/
  review date: 2026-07-12

## Portfolio ownership

Shared admission, composition, aggregate-budget, evidence, exception, and
lifecycle rules are owned by `policy-portfolio.toml`; they are not repeated
here. This file owns only its domain-specific observable outcomes, verified
facts, commands, and review triggers. Amendments update the effective portfolio
while its mandatory outcomes remain protected.

## Living document contract

This policy is a living document. Keep its domain-specific facts, commands,
requirements, and review triggers aligned with verified project reality.
Shared amendment admission and retention are governed once by
`policy-portfolio.toml`; generic rule protection and non-weakening are
governed by `execution-kernel.md §Kernel rules`.

## Ralph markers

* Policy id: `<!-- ralph-policy-id: architecture-policy.md -->`
* Schema version: `<!-- ralph-policy-schema: v7 -->`
