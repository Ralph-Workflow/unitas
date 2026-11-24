<!-- ralph-policy-schema: v6 -->
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
| Conditional durable contracts | Architecture owner | a durable contract is required only when the system that consumes it exists in the project | every required registry or classifier has a named consumer, and the corpus names no contract without one | every required registry or classifier has a named consumer | deterministic: make policy-validate | when the consumer system is removed |

## Project facts to resolve

The lines below are the verified project facts that agents rely on and keep
current.

RALPH-FACT: architecture_style: layered: the Rust core owns asset parsing and runtime state, the C# bridge owns the UnityEngine API emulation, and the repository tooling is a separate Python package
RALPH-FACT: component_map: the crates unitas-core, unitas-asset, unitas-audio, unitas-physics, unitas-runtime, unitas-render and unitas-build are implemented in the Cargo workspace under crates/; the C# bridge component lives under bridge/ containing UnityEngine, Unitas.Bridge, and Unitas.Bridge.Tests (with bin and obj build-output directories excluded by the gates from secret scan and ban/bypass source walks); the repository tooling is a separate Python package under tools/
RALPH-FACT: dependency_direction: the tooling imports the standard library only; a crate depends on unitas-core, unitas-asset, unitas-audio, unitas-physics, or unitas-runtime, never the reverse — unitas-asset, unitas-audio, and unitas-physics depend on unitas-core, unitas-runtime depends on unitas-core, unitas-asset, unitas-audio, and unitas-physics, and unitas-build depends on unitas-asset and unitas-runtime
RALPH-FACT: forbidden_dependencies: the tooling must not import a third-party module, and a crate must not depend on a Unity assembly
RALPH-FACT: state_ownership: a gate owns no state between runs; every gate reads the working tree and prints its findings
RALPH-FACT: external_io_boundaries: the gate runner reads the working tree and spawns detect-secrets with an argument vector; it writes nothing outside the repository
RALPH-FACT: durable_contracts: the policy corpus is the single source for the gate commands, and the Makefile is the single source for the gate targets
RALPH-FACT: decision_record_location: docs/ralph-workflow-policy/architecture-policy.md
RALPH-FACT: quality_attribute_scenarios: a full gate run finishes inside the verification budget recorded in docs/ralph-workflow-policy/verification-policy.md
RALPH-FACT: runtime_and_deployment_views: the gates run from a clean clone on the development host; no container, service, or deployment target exists yet
RALPH-FACT: major_data_flows: a Make target runs python3 tools/verify.py, which reads the working tree and writes findings to standard output; the lint, typecheck and test targets also run cargo over the workspace
RALPH-FACT: known_risks_and_tradeoffs: the Rust workspace and C# UnityEngine bridge are implemented and gated by the default gate; the bridge is implemented via a managed host loading the native Rust cdylib (libunitas_runtime.dylib); CoreCLR embedding inside Rust remains the open Milestone 2 item due to the repository ban on unsafe/transmute; lifecycle dispatch documents a deliberate architectural deviation using public virtual methods instead of private reflection; the Milestone 4 physics slice ships a custom minimal 3D physics engine behind Unity-compatible wrappers rather than Rapier3D or PhysX, with explicit Dynamic, Kinematic, and Static body roles, Sphere, Box, and Capsule collider shapes (including Unity class 136 CapsuleCollider YAML parsing with radius, height, and local X/Y/Z axis direction plus explicit per-entity geometry diagnostics), a kinematic CharacterController mover solved against the query pipeline (including Unity class 143 and built-in MonoBehaviour CharacterController YAML parsing, step-offset and tread-depth checks, slope-limit clamping, wall/ceiling slide, overlap depenetration, moving-platform velocity carry, teleport state reset, crouch-sized shape changes, unitas_runtime_character_move/teleport/controller_config ABI externs, and UnityEngine.CharacterController with ICharacterControllerProvider), an iterative positional-correction and normal/tangential impulse solver (with arithmetic-mean pair restitution and single dynamic friction rather than separate static/dynamic friction or Unity combine-mode switches) across sphere-sphere, sphere-box, SAT shallowest-axis box-box, capsule-sphere, capsule-box, and capsule-capsule contacts, post-step transform write-back of simulated dynamic body positions into the native SceneGraph and managed Transform via unitas_runtime_body_position and unitas_runtime_body_velocity, managed-to-native velocity and kinematic position push via unitas_runtime_set_body_velocity and unitas_runtime_set_body_position (with kinematic transform changes taking effect before the next fixed step and driving contact response via per-step sweep velocity), PhysicMaterial YAML reference parsing with explicit per-entity diagnostics when a material reference is unresolved or invalid, and a plain Rust physics world API so a later phase can swap the backend without an ABI change; an entity that carries multiple colliders resolves by precedence to sphere, then box, then capsule in this slice; overlapping pairs where either body is a trigger route exclusively to the trigger channel, bypass the collision solver, and never enter the collision channel, and OnTriggerEnter/Stay/Exit receives a null Collider when the other entity is unregistered in C#; the shipped physics query API filters queries by layer mask and QueryTriggerInteraction (including Physics.queriesHitTriggers defaulting to true for UseGlobal, segment-based Physics.Linecast(start, end, ...), double-sided exit-face hits for point casts starting inside solid colliders, distance-zero hits for swept shapes starting overlapped, and ascending EntityId ordering for overlap_sphere) and never routes collision or trigger pairs.
RALPH-FACT: deferred_physics_scope: MeshCollider shape, capsule sweep casts (Physics.CapsuleCast), NonAlloc query forms, 2D physics, rotation and angular dynamics, sleeping and wake transitions, and joints remain deferred past the shipped Milestone 4 capsule-collider, physics-queries, and kinematic character-controller slices (reviewed on 2026-11-24 at the Milestone 4 kinematic character-controller completion phase); owner: the repository maintainers; trigger: a consumer needs one of the deferred surfaces, or the next Milestone 4 follow-up phase names one of them; lane: the default automated lane when implemented; evidence: the make verify run that proves the shipped slice; review condition: review when a consumer needs one of the deferred surfaces and whenever the next Milestone 4 follow-up phase starts
RALPH-FACT: deferred_collider_scale: the collider size of a GameObject under a scaled ancestor is not scaled by the ancestor scale (the physics world places the collider at the world position and writes simulated dynamic world positions back to local transforms across ancestor transforms, while keeping the local collider size; reviewed on 2026-11-24); owner: the repository maintainers; trigger: a consumer needs a collider whose size follows an ancestor's scale, or the next Milestone 4 follow-up phase names it; lane: the default automated lane when implemented; evidence: the make verify run that proves the shipped slice; review condition: review when the trigger fires and whenever the physics world transform handling changes
RALPH-FACT: deferred_physics_backend_swap: the Rapier3D or PhysX integration named by the product criteria is deferred past the shipped Milestone 4 physics slice, which runs a custom minimal 3D physics engine with an iterative positional-correction and normal/tangential impulse solver (with velocity/position mutators and kinematic sweep velocity) behind Unity-compatible wrappers so the plain Rust World API can still be swapped without an ABI change (reviewed on 2026-11-24 after World::set_velocity, World::set_position, and BodyDesc material fields landed); owner: the repository maintainers; trigger: a consumer needs the Rapier or PhysX feature set, such as continuous collision detection, angular dynamics, or a production-grade constraint solver; lane: the default automated lane when implemented; evidence: the make verify run that proves the shipped slice; review condition: review when the trigger fires and whenever the physics world API changes
RALPH-FACT: deferred_prefab_override_scope: prefab instance expansion applies the instance's m_Name and m_LocalPosition, m_LocalRotation and m_LocalScale modifications to any object of the prefab it names, root or child; added and removed components, added and removed children, property paths other than m_Name and m_Local*, and the editor surfaces that apply, revert or unpack an override are not shipped; owner: the repository maintainers; trigger: a consumer needs a structural prefab override or an override-edit surface, or the next prefab follow-up phase names one of them; lane: the default automated lane when implemented; evidence: the make verify run that proves the shipped slice; review condition: review when the trigger fires and whenever the prefab expansion path changes
RALPH-FACT: prefab_fixture_provenance: the asset test fixture crates/unitas-asset/tests/fixtures/project/Camera.prefab was authored with the shipped prefab expansion because only its .meta pre-existed, and crates/unitas-asset/tests/fixtures/prefab.prefab instances it through that guid; the pre-existing .meta still describes a folder importer and is left untouched; owner: the repository maintainers; trigger: the fixture layout changes, or a consumer needs the camera prefab as a real prefab asset rather than a guid placeholder; lane: the default automated lane when implemented; evidence: the make verify run that proves the shipped slice; review condition: review when the trigger fires and whenever the asset test fixtures change
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
* Schema version: `<!-- ralph-policy-schema: v6 -->`
