<!-- ralph-policy-schema: v7 -->
<!-- ralph-policy-id: typechecking-policy.md -->
# Type-Checking Policy

## Purpose and scope

This policy governs every maintained first-party language for which static type checking
applies. It defines the selected type checker, the exact commands, the
source categories included, the strictness level, and the rules around
suppressions, ignores, casts, generated code, and untyped dependencies. It
also defines how first-party code MUST be written and structured so the
checker can prove as much as possible: richer types, total functions,
isolated side effects, and no dynamic constructs that create unchecked holes.

Loading trigger: a change to type-checker configuration, strictness, or a
suppression, or a first-party type shape the checker must prove.

Loading trigger: a change to type-checker configuration, strictness, or a
suppression, or a first-party type shape the checker must prove.

## Default requirements

Every maintained first-party language MUST be statically type-checked (the
tooling and enforcement rules follow), AND first-party code MUST be written to
be maximally checkable. The discipline below applies to each language to the
extent its checker can express and enforce it; where a language cannot enforce
a rule statically, the intent still binds and is met by the nearest available
construct.

### Type-checkability discipline

**Make illegal states unrepresentable.**

* MUST model a closed set of values as an enum, literal-union, or sealed type
  — never a bare string or magic number for a known, finite domain.
* MUST keep dynamic-type escapes (`Any`, widening `unknown`, `interface{}`,
  bare `object`, `Dict[str, Any]`, and equivalents) out of checked
  first-party code, except at a boundary that is IMMEDIATELY narrowed to a
  precise type.
* SHOULD represent alternatives and absence with sum types (tagged unions,
  `Result`/`Option`-style types) instead of nullable-everything, sentinel
  values, or a boolean flag paired with optional fields. Model "or" as a sum
  type and "and" as a product type.
* SHOULD give domain primitives distinct types (newtype, branded type,
  `NewType`, or a wrapper struct) — identifiers, quantities, units — so values
  of different meaning cannot be transposed.

**Prefer total functions.**

* MUST make every match/switch over a closed type exhaustive, compiler-
  enforced where the language supports it; do not add a catch-all default that
  silently swallows new variants.
* SHOULD represent "may fail" or "may be absent" in the return type
  (`Result`/`Option`) rather than exceptions-as-control-flow, `null`, or
  unchecked index / first-element / dictionary access.

**Isolate side effects so the core stays pure and checkable.**

* SHOULD keep a pure functional core and push side effects — I/O, mutation,
  clock, randomness, global state — to a thin shell at the edges. The bulk of
  logic SHOULD be pure functions of their inputs, which the checker and tests
  can verify in isolation.
* SHOULD treat data as immutable by default (frozen/readonly/const, enforced
  where the language supports it) and pass dependencies explicitly rather than
  reaching through globals, singletons, or module-level mutable state.

**Structure code for static analysis, not runtime dynamism.**

* MUST avoid reflection, monkeypatching, dynamic attribute get/set
  (`getattr`/`setattr`/`__getattr__` and equivalents), `eval`/`exec`,
  string-keyed dynamic dispatch, and runtime metaprogramming that hides types
  from the checker, in checked first-party code. Use explicit
  interfaces/protocols/traits, typed dispatch tables, enums, and plain data
  instead.
* MUST give a structural or duck-typed expectation an explicit typed contract
  (protocol, interface, or trait) so the checker verifies the expected shape
  rather than trusting it at runtime.

A genuinely required dynamic or effectful boundary — a serialization library,
plugin loader, FFI surface, or framework hook — is NOT an open exception to
the rules above: contain it behind a typed adapter, record it in
`sanctioned_dynamic_boundaries`, and confine it to the live-entry rules
below. That set of boundaries is ratcheted; it MUST NOT grow
without a documented justification. <!-- ralph-control: sanctioned-dynamic-boundaries -->

### Tooling, suppressions, and enforcement

A maintained first-party language includes interpreted or compiled production
and test source owned by the project. Generated/vendored code, embedded
snippets, and template/IDL/SQL/shell sources are classified explicitly in
`first_party_languages` and `excluded_language_evidence` according to whether
the project maintains and can statically check them.

* The agent MUST declare a `RALPH-LANG:` block for EVERY language used for
  maintained first-party executable or compiled source, followed by a `RALPH-COMMAND:` or an
  explicit `RALPH-INAPPLICABLE:` line with a reason.
* When a maintained first-party language supports a suitable maintained type checker,
  compiler check, or equivalent static type-analysis tool, the project
  MUST select and run one. Missing configuration, migration effort,
  existing findings, or team preference do not make checking inapplicable.
* Selection MUST consider active maintenance, security responsiveness,
  compatibility, first-party source coverage, diagnostic quality, and CI
  suitability. No product is universally preferred by this policy.
* Inline suppressions are forbidden. The underlying diagnostic is fixed in
  code, or suppressed in the type checker's configuration, never inline.
* A suppression is a last resort after fixing the type, narrowing the dynamic
  boundary, adding validation, or improving a local adapter/stub has been tried.
  A suppression that remains is a configuration-level entry in the type
  checker's configuration that names the tool limitation or external boundary,
  owner, and removal/review trigger. Suppression count MUST NOT increase,
  and an existing suppression outside the three permitted reasons below is
  removed by fixing the code it covers. <!-- ralph-control: suppression-count-baseline -->
* A lint or type-check rule is disabled, narrowed, or suppressed only for
  one of three reasons: the code is owned by another party and the project
  cannot change it, the code is generated, or the code is the narrow
  boundary that calls a foreign API with no type information (FFI). Every
  other disable, suppression, threshold increase, or strictness reduction is
  forbidden, whatever reason accompanies it.
* Rule strength only increases. An agent never disables, downgrades,
  deletes, or narrows a lint or type-check rule, raises a threshold, or adds
  an exclusion so that a check passes; a failing rule is fixed in the code
  it reports.
* Unused, unreachable, impossible, or obsolete-path diagnostics MUST be
  investigated under the linting policy's zero-tolerance section. Verified
  dead code MUST be removed. A proven live reflection/plugin/FFI/platform or
  external entry point MAY use only the linting policy's evidence-backed,
  narrow live-entry annotation; this is not a dead-code exception.
* Generated and vendored code MAY be excluded with a documented reason.
  First-party migrations, compatibility code, and tests MUST NOT be
  excluded merely because of their category.
* Untyped third-party values MUST be contained by typed adapters, protocols,
  validation, or stubs sufficient to prevent unchecked values escaping into
  checked first-party code. Blanket global silencing is forbidden.
* Unsafe code MUST express its safety invariants as types wherever a type can
  carry them, as `clean-code-policy.md` "Unsafe code zones" requires. This means
  wrapped pointers with lifetimes, derived layout traits, typed generational
  handles, typestate, `unmanaged` constraints, and ref-safety. What remains in an
  `unsafe` block is only what no type can express.

## Guard inventory

| Guard | Owner | Protected outcome | Detected condition | Claim scope | Enforcement | Retirement trigger |
|---|---|---|---|---|---|---|
| Unknown at untrusted boundaries | Typecheck owner | no value crosses a trust boundary as an unchecked dynamic type | the type check reports zero errors with the strict settings enabled | the type check reports zero errors | deterministic: make typecheck | when the type checker can express the boundary |
| Typed tests | Typecheck owner | every test helper carries an explicit annotation | the type check covers the test module and reports zero errors | the type check covers the test module | deterministic: make typecheck | when the tests leave the type check |
| No typecheck bypass | Typecheck owner | no inline type suppression and no disabled error code is present in the tracked tree | the bypass audit reports zero findings in the source and in the tool configuration | the bypass audit reports zero findings | deterministic: make bypass-audit | when the type checker reports its own suppressions |
| `sanctioned-dynamic-boundaries` Sanctioned dynamic boundary | Typecheck owner | each sanctioned dynamic boundary is named and counted | the policy records each sanctioned boundary and the type check reports zero errors | the policy records each sanctioned boundary | deterministic: make typecheck | when no dynamic boundary remains |
| `suppression-count-baseline` Suppression count baseline | Typecheck owner | the number of type suppressions does not grow | the bypass audit reports zero suppression markers | the bypass audit reports zero suppression markers | deterministic: make bypass-audit | when the type checker reports its own suppressions |

## Project facts to resolve

The `RALPH-FACT:` lines below record verified project facts. Agents rely
on them when enforcing this policy and MUST keep them current as the
project evolves.

RALPH-FACT: typechecker_per_language: mypy in strict mode for Python, cargo check (rustc with warnings denied) for Rust, the .NET C# compiler (dotnet build with Nullable=enable and TreatWarningsAsErrors=true) for C#, and tsc (TypeScript compiler with --allowJs --checkJs --noEmit --strict) deferred via RALPH-PENDING for JavaScript until tsconfig.json and @types/node are provisioned and wired
RALPH-FACT: strictness_level: strict across all three active type-checked languages (with JavaScript strict checkJs deferred via RALPH-PENDING) — Python runs mypy with strict = true in pyproject.toml over every tracked .py file (tests included); Rust sets [workspace.lints.rust] warnings = "deny" and unsafe_code = "forbid" in the root Cargo.toml, every member crate outside the registered unsafe zones sets lints.workspace = true, and the three zone crates unitas-editor-abi, unitas-runtime and unitas-dotnet carry their own [lints] table that copies the workspace table and relaxes unsafe_code as clean-code-policy.md "Unsafe code zones" requires; C# sets Nullable=enable, TreatWarningsAsErrors=true, AnalysisMode=All, AnalysisLevel=latest-all, and EnforceCodeStyleInBuild=true in the root Directory.Build.props for every project in bridge/Unitas.sln
RALPH-FACT: excluded_paths: Python: none — mypy checks every tracked .py file under tools/; Rust: none — cargo check --workspace --all-targets covers every workspace crate and target; C#: bridge/Unitas.Bridge.Tests/Fixtures/** (test-input C# fixtures excluded via Compile Remove in bridge/Unitas.Bridge.Tests/Unitas.Bridge.Tests.csproj); JavaScript: none excluded by configuration (tools/web_boot_check.mjs is pending tsc --allowJs --checkJs wiring)
RALPH-FACT: stubbed_dependencies: none — the tooling imports the standard library only
RALPH-FACT: suppression_rationale_policy: no type suppression exists; a type error is fixed in the code
RALPH-FACT: suppression_inventory_and_baseline: zero suppressions — the bypass audit reports every suppression marker it finds
RALPH-FACT: ci_gate_integration: the standalone make typecheck target runs mypy, cargo check --workspace --all-targets, and dotnet build bridge/Unitas.sln --configuration Release (via the dotnet-build target); the default gate make verify runs no separate cargo check step and still type-checks every active compiled/checked language — Python through mypy in verify-static-gates, C# through dotnet build in verify-dotnet-gates, and Rust through cargo clippy --workspace --all-targets --target-dir "$(CLIPPY_TARGET_DIR)" -- -D warnings (where CLIPPY_TARGET_DIR := $(CURDIR)/target/clippy) in verify-static-gates, which compiles every workspace crate and target and fails the gate on a Rust type error; JavaScript type checking for tools/web_boot_check.mjs is deferred via RALPH-PENDING until tsconfig.json and @types/node are wired
RALPH-FACT: maintenance_evidence: Python: mypy 2.1.0 (Python typing project https://mypy.readthedocs.io/, compatible with Python 3.11+ and checking all tracked tools/*.py files, selected 2026-11-25, recheck trigger: major Python or mypy upgrade, incompatibility, abandonment, or security signal); Rust: rustc / cargo check 1.93.0 (Rust Project official toolchain https://doc.rust-lang.org/, checking all workspace crates and targets, selected 2026-11-25, recheck trigger: Rust toolchain or edition upgrade, incompatibility, abandonment, or security signal); C#: .NET SDK 8.0.428 csc via dotnet build (Microsoft official .NET 8 SDK, checking all 12 projects in bridge/Unitas.sln across bridge/ and editor/, selected 2026-11-25, recheck trigger: .NET SDK major upgrade, incompatibility, abandonment, or security signal); JavaScript: TypeScript tsc 6.0.2 (Microsoft official TypeScript toolchain https://www.typescriptlang.org/, pending checkJs configuration and @types/node provisioning for tools/web_boot_check.mjs, recorded 2026-11-27, recheck trigger: when tools/web_boot_check.mjs is wired into the verification gate or package.json/tsconfig.json is added)
RALPH-FACT: first_party_languages: Python for the repository verification and CLA tooling (tools/); Rust for the engine, runtime, editor-abi, and build crates (crates/); C# for the UnityEngine bridge, scripting/testing/debugging assemblies, and editor host (bridge/ and editor/); JavaScript for the Node.js WebAssembly runtime boot check script (tools/web_boot_check.mjs)
RALPH-FACT: excluded_language_evidence: none — no language is excluded from the type check
RALPH-FACT: type_modeling_conventions: every function is annotated, and a value crossing a module boundary carries a concrete type
RALPH-FACT: sanctioned_dynamic_boundaries: editor/Unitas.Editor/Registry/ExecuteMethodTools.cs — the `editor.execute_method` tool resolves a named static method in the compiled project script assembly and invokes it, which is string-keyed dynamic dispatch over project code; editor/Unitas.Editor/Play/ScenarioTools.cs — the scenario runner loads the compiled project script assembly, enumerates its types, picks the MonoBehaviour subclass whose type name matches the caller-supplied script name, and constructs it through `Activator.CreateInstance`, which is string-keyed dispatch over project code plus runtime type construction; editor/Unitas.Editor/Registry/SceneTools.cs — the `scene.set_property` command resolves a caller-supplied property path string against the open scene document model, so the property a write lands on is chosen by a runtime string and not by a compile-time accessor; bridge/UnityEngine/GameObject.cs — the `AddComponent(Type)` and `TryAddComponentForAgent(Type)` overloads construct a component from a caller-supplied `Type` through `Activator.CreateInstance`, so the component type is a runtime value. Justification: the first two exist to run and mount project code that only exists at runtime, so no explicit typed contract can express them without a generated dispatch table per project; each dynamic call is confined to one typed adapter, is reachable only through the ExecuteProjectCode scope, and is covered by its own test shard. The scene command exists to edit a Unity tagged-YAML document whose property keys belong to the document, so the path is data the tool must resolve at runtime; the resolution rejects an unknown key or member with a typed error and the commands are covered by the SceneToolsTests shard. The GameObject overloads exist because a caller that discovers a component type at runtime has no compile-time type to name; the typed `AddComponent<T>` and `TryAddComponentForAgent<T>` overloads remain the checked path, the `Type` overloads reject a type that is not a concrete `UnityEngine.Component` before constructing it, and `AddComponent(Type)` is covered by the bridge component tests. Scope: the set covers string-keyed dispatch over project code, runtime construction from a caller-supplied `Type`, and runtime selection of a document property by a caller-supplied path string; it does not cover the JSON argument boundary every MCP command shares, because each command narrows that boundary to a typed value at its edge. <!-- ralph-control: sanctioned-dynamic-boundaries --> Review trigger: any change to the editor tool registry or to the project-code execution path

For each language, `maintenance_evidence` records tool and version range,
official maintenance source, compatibility and first-party coverage evidence,
selection date, and recheck trigger. Recheck on major language/tool changes,
incompatibility, abandonment, or a relevant security signal.

## AI execution instructions

This section is the typechecking-specific procedure only. The five
generic rules (truthful evidence, fail-closed verification, bounded
execution, no weakening of checks, explicit unverified-claim
reporting), the precedence over domain policies, the
selective-applicability rule, and the claim-scope rule are owned by
the kernel starter (`execution-kernel.md`) and MUST be loaded from
there — do not re-derive them here.

To follow this policy, an agent making any change MUST:

* DECLARE one `RALPH-LANG:` block per language with the exact checker
  command. Do not invent languages; do not omit maintained first-party languages.
* MODEL closed value sets, alternatives, and domain primitives with precise
  types (enums, sum types, newtypes) before reaching for strings, flags, or
  broad containers, and keep side effects at the edges so the core stays pure.
* PREFER an established project checker when it remains maintained and
  suitable. Record selection evidence when adopting or replacing one.
* RUN every `RALPH-COMMAND:` gate declared under Verification before
  claiming the change complies.
* UPDATE this policy (facts, commands, requirements) in the same
  workflow that changes the type checker, strictness level, or
  exclusion patterns.

An agent MUST NOT:

* Add `ignore_missing_imports`, `follow_imports = silent`, or similar
  global silencers without a per-dependency reason.
* Use a blanket suppression. Require the narrowest tool-supported identifier
  plus reason; when no identifier exists, use the narrowest scope and
  document that tool limitation.
* Add a suppression merely to make the checker pass, silence first-party
  design errors globally, or expand the established unchecked baseline.
* Suppress verified dead code or annotate a speculative/unverified consumer.
* Introduce reflection, monkeypatching, dynamic attribute access,
  `eval`/`exec`, or string-keyed dynamic dispatch into checked first-party
  code to avoid writing an explicit typed contract.
* Reach for a bare string, boolean flag, or broad container (`Any`,
  `object`, `interface{}`, `Dict[str, Any]`) where an enum, sum type, or
  precise type expresses the value.
* Weaken the strictness level to obtain a passing result.

## Verification

Run every gate below before claiming a change complies with this policy.

RALPH-LANG: Python
RALPH-COMMAND: make typecheck

RALPH-LANG: Rust
RALPH-COMMAND: cargo check --workspace --all-targets

RALPH-LANG: C#
RALPH-COMMAND: make dotnet-build

RALPH-LANG: JavaScript
RALPH-PENDING: tsc --allowJs --checkJs --noEmit --strict tools/web_boot_check.mjs (assumed 2026-11-27); review trigger: once tsconfig.json and @types/node are provisioned and JSDoc type annotations land in tools/web_boot_check.mjs
The expected successful result is a clean type check (exit 0) on the
current code base. Never mask type errors with silencers.

## Exceptions

A maintained first-party language may use `RALPH-INAPPLICABLE:` only for one
of two exceptional cases: documented research proves no suitable maintained
checker exists, or a technically non-checkable first-party surface was
misclassified as a checkable language. Preference, inconvenience, legacy
errors, migration cost, and absent setup are invalid. The exact declaration
MUST start with `exceptional case: no suitable maintained checker exists;` or
`exceptional case: technically non-checkable first-party surface;` and include
`evidence:`, `owner:`, `expiry:`, `warning:`, and `review trigger:`. The visible
warning remains until the exception is removed.

## Maintenance triggers

This policy MUST be reviewed in the same workflow as any of:

* A new language is added to the project.
* The type checker, version, or strictness level changes.
* A new dependency is added that ships without types.
* The suppression policy changes.
* The sanctioned dynamic boundaries or type-modeling conventions change.

## Research basis

* publisher: Python Software Foundation
  title: "typing — Support for type hints (PEP 484)"
  http: https://docs.python.org/3/library/typing.html
  review date: 2026-07-11

* publisher: Microsoft TypeScript
  title: "TypeScript: Handbook - Type Checking .js Files"
  http: https://www.typescriptlang.org/docs/handbook/type-checking-javascript-files.html
  review date: 2026-07-11

* publisher: The Rust Project
  title: "The Rust Reference: Types"
  http: https://doc.rust-lang.org/reference/types.html
  review date: 2026-07-11

* publisher: mypy (Python typing project)
  title: "Using mypy with an existing codebase"
  http: https://mypy.readthedocs.io/en/stable/existing_code.html
  review date: 2026-07-11

* publisher: Alexis King
  title: "Parse, don't validate"
  http: https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/
  review date: 2026-07-12

* publisher: Scott Wlaschin (F# for fun and profit)
  title: "Designing with types: Making illegal states unrepresentable"
  http: https://fsharpforfunandprofit.com/posts/designing-with-types-making-illegal-states-unrepresentable/
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

* Policy id: `<!-- ralph-policy-id: typechecking-policy.md -->`
* Schema version: `<!-- ralph-policy-schema: v7 -->`
