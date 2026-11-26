<!-- ralph-policy-schema: v7 -->
<!-- ralph-policy-id: clean-code-policy.md -->
# Clean-Code Policy

## Purpose and scope

This policy governs project-specific naming, structure, module-boundary,
and readability rules. It defines the expectations for function, class,
module, and file scope and responsibility; the limits on dead code and
premature extensibility; the error-handling and logging conventions; the
rules for refactoring code that cannot be tested or understood cleanly;
and the treatment of generated, vendored, migration, and compatibility
code.

## Default requirements

* The agent MUST follow project-specific naming conventions verified
  against existing source. Generic style guides do NOT override the
  project's actual convention.
* The agent MUST prefer simple, maintainable solutions and existing
  project patterns over speculative abstractions or duplicated helpers.
  New abstractions require demonstrated boundary, volatility, testing, or
  reuse value; implementation count alone neither requires nor forbids one.
* Dead code, commented-out code, unused compatibility layers, and
  unused imports MUST be removed. The "we might need it later"
  reason does not override the no-dead-code rule.
* Functions, classes, modules, and files MUST remain cohesive, expose
  understandable interfaces, and have bounded reasons to change.
* Error handling MUST surface actionable context. Bare `except: pass` is
  forbidden. Handle or propagate errors according to project convention;
  log once at the owning handling boundary and redact sensitive context.
* Test friction MUST prompt investigation. Refactor only when it reveals a
  real cohesion, dependency, or I/O-seam problem; do not create an artificial
  public seam solely for a test. Test at the narrowest useful observable
  boundary; never expose implementation or add wrappers to evade it
  (`execution-kernel.md`).
* Generated and vendored code MUST be identifiable and MAY be excluded from
  generic checks with a documented reason. First-party migration and
  compatibility code remain subject to applicable quality gates.
* Cross-component dependency direction, state ownership, and external I/O
  boundaries are governed by `architecture-policy.md`.

## Verifiable constructs and banned escape hatches

Code uses only constructs the type checker and linter can fully verify. The
default is ban. The burden of proof is on allowing a construct, never on
banning it: a construct remains allowed only when its common use is safe and
fully verifiable by both tools. Occasional need, careful use, or restriction to
one module does not justify allowance. There are no inline suppressions,
per-file ignores of a banned rule, allowlists, or baselines. The single scoped
allowance is a registered unsafe zone, governed by "Unsafe code zones" below.
The remedy for a finding is restructuring the code. When a banned construct
is genuinely required by a capability the product needs, the agent amends
this policy in the same change: it narrows the ban to where the risk is,
replaces it with enforced guardrails, and updates the enforcing gate and that
gate's tests. It never suppresses the finding, never contorts the product to
avoid the construct, and never stops the work. This rule has evidence tag
`none` and ships as `deterministic` because the project's ban gate enforces
it.

The catalog names the enforcing linter rule IDs. Uppercase hyphenated
identifiers without a linter prefix are project-gate rule IDs; the remediation
agent maps them to the rule IDs enforced by the project's own type checker and
linter configuration.

| Language | Banned constructs | Enforcing rule IDs | Enforcement |
|---|---|---|---|
| Python | `eval`; `exec`; `compile`; `__import__`; `globals`/`locals`/`vars`; `typing.cast`; an `Any` annotation; `assert` outside a test module; a bare or broad `except`; a named `lambda`; a mutable default argument; a star import; `os.environ`; `print` in library code. | `BANNED-CALL`; `BANNED-NAMESPACE`; `TYPING-CAST`; `ANN401`; `RUNTIME-ASSERT`; `E722`; `BLE001`; `E731`; `B006`; `F403`; `OS-ENVIRON`; `PRINT`. | deterministic: make banned-constructs |
| Rust | `unsafe` outside a registered unsafe zone; `transmute`; `static mut`; `Any`/`downcast`; `#[allow(...)]` on a banned lint; `unwrap`/`expect`/`panic!` outside tests; procedural macros generating untyped code. | `unsafe_code`; `unnecessary_transmutes`; `static_mut_refs`; `BANNED-ALLOW`; `clippy::unwrap_used`; `clippy::expect_used`; `clippy::panic`; `clippy::todo`; `clippy::unimplemented`; `clippy::dbg_macro`; `BANNED-UNTYPED-MACRO`; `BANNED-ANY`. | deterministic: make banned-constructs |
| C# | Reflection APIs; `unsafe`, pointers, and `System.Runtime.CompilerServices.Unsafe` outside a registered unsafe zone; raw types; unchecked casts; `dynamic`; `Object` parameter types; `#pragma warning disable`. | `BANNED-REFLECTION`; `BANNED-UNSAFE`; `rawtypes`; `unchecked`; `BANNED-DYNAMIC`; `BANNED-OBJECT-PARAMETER`; `BANNED-SUPPRESSION`. | deterministic: make banned-constructs |

## Unsafe code zones

Memory-unsafe code is required by the engine: C ABI exports and imports, .NET
runtime hosting, native collections and job memory, GPU and audio buffer
access, SIMD, and platform APIs. It is permitted only inside a registered
unsafe zone. Everywhere else it stays banned. The rules below assume that
agents write and change this code, so each safety claim is checked by a tool,
never accepted on the strength of its explanation.

Avoid unsafe code first:

* Generate FFI bindings from one declaration. Rust exports are declared once,
  and the C# `LibraryImport` declarations and any C headers are generated from
  them (for example with `csbindgen`, `interoptopus`, or `cbindgen`). Nobody
  hand-writes marshalling or maintains a symbol table.
* Use the safe wrappers that maintained crates already provide: `netcorehost`
  for .NET hosting, `bytemuck` or `zerocopy` for byte casts, and `wgpu`, `cpal`,
  and `winit` for GPU, audio, and windowing.
* `transmute`, `static mut`, and linker-level symbol aliasing of mangled names
  stay banned inside zones too. A C symbol is named only with
  `#[unsafe(no_mangle)]` or `#[unsafe(export_name = "...")]` in a zone.

Zone registration:

* A zone is a whole Rust crate or a whole C# project, never a file or a line.
  Its public API is safe: no `unsafe fn`, raw pointer, or `SafeHandle`
  internals cross the zone's public boundary, except the generated C ABI
  surface itself.
* Every zone is recorded in `tools/unsafe-zones.toml` with its path, language,
  purpose, the invariants its unsafe code relies on (aliasing, lifetime,
  alignment, threading, ownership), and the exact count of its unsafe blocks.
  The registry is the only place zones are declared.
* A Rust zone uses edition 2024 and its own `[lints]` table, which equals the
  workspace table except for these differences:
  * `unsafe_code` is not forbidden.
  * `unsafe_op_in_unsafe_fn` is denied.
  * `clippy::undocumented_unsafe_blocks`, `clippy::missing_safety_doc`,
    `clippy::multiple_unsafe_ops_per_block`, and the pointer-cast lints
    (`clippy::ptr_as_ptr`, `clippy::cast_ptr_alignment`,
    `clippy::as_ptr_cast_mut`, and `clippy::borrow_as_ptr`) are denied.
  * `improper_ctypes`, `improper_ctypes_definitions`, and
    `clippy::transmute_ptr_to_ptr` are denied.
* A C# zone sets `AllowUnsafeBlocks` in its own project file only. It applies
  `[assembly: DisableRuntimeMarshalling]`. It uses `LibraryImport` with blittable
  signatures, `SafeHandle` for every owned native resource, and `Span<T>` in place
  of pointers wherever the API allows.

Boundary design:

* Objects cross the boundary as generational handles that native code
  validates on every call, never as pointers to Rust data. A stale handle is an
  error, not a use-after-free.
* Ownership crosses only through explicit create and destroy entry points.
  Buffers and strings cross as pointer and length, with UTF-8 strings.
* Every exported function catches panics with `std::panic::catch_unwind` and
  returns an error code. No panic or exception unwinds across the boundary.
* Native collections and job memory carry safety handles in development
  builds that detect concurrent conflicting access, in the manner of Unity's
  job-safety system.

Type-level safety:

The type checker is the first line of defense. Every safety rule that a type can
express is expressed as a type, so the code that only a tool or a reviewer can
check is as small as possible.

* **Minimal unsafe bodies.** An `unsafe` block performs one primitive operation
  whose preconditions the surrounding types already guarantee. Validation,
  branching, and arithmetic happen in safe code before the block.
* **Raw pointers are wrapped.** A raw pointer appears only in the generated C ABI
  surface and in the private fields of a wrapper type. The wrapper uses
  `NonNull<T>`. It ties the pointer's lifetime to its owner through lifetimes or
  `PhantomData`. Its constructor is the only place that establishes the
  invariant, and its fields are private. A slice or view over native memory
  borrows from its owner and is never `'static`.
* **Initialization through types.** Uninitialized memory exists only as
  `MaybeUninit<T>`. `mem::zeroed` is used only on types that derive
  `zerocopy::FromZeros`.
* **Byte reinterpretation through derives.** Bytes are reinterpreted only through
  `zerocopy` (`FromBytes`, `IntoBytes`, `KnownLayout`, `Immutable`) or
  `bytemuck::Pod` derives, which reject padding, invalid bit patterns, and
  mismatched layout at compile time. A pointer cast between distinct types is
  banned.
* **Checked boundary types.**
  * Every type that crosses the C ABI is `#[repr(C)]` or `#[repr(transparent)]`.
    Its size and alignment are fixed by compile-time assertions.
  * The `improper_ctypes` and `improper_ctypes_definitions` lints are denied.
  * Enums and booleans cross as fixed-width integers and are converted with
    `TryFrom` on entry. A value from foreign code is never trusted to be a
    valid Rust enum discriminant or `bool`.
* **Typed handles.** Each object kind has its own handle type, such as
  `Handle<Mesh>`, carrying an index and a generation. The binding generator
  emits a distinct C# `readonly struct` for each kind. No public API in either
  language takes an untyped integer handle.
* **Typestate for ordered protocols.** An API whose calls must happen in order
  (hosting, mapping, recording, submission) encodes its states as types, such as
  `Buffer<Mapped>` and `Buffer<Unmapped>`, so an out-of-order call does not
  compile.
* **Thread-safety promises are rare and proven.** `unsafe impl Send` and
  `unsafe impl Sync` appear only on wrapper types whose registered invariants
  justify them, and each one has a Loom model. A type that holds a raw pointer is
  neither `Send` nor `Sync` by any other route.
* **Sealed unsafe traits.** An `unsafe trait` is sealed and is implemented only
  inside zones.
* **C# boundary rules.**
  * C# zones apply `[assembly: DisableRuntimeMarshalling]`, so a non-blittable
    interop signature is a compile error. Native collections constrain their
    element type with `where T : unmanaged`.
  * Spans over native memory are used through `ref struct` and `scoped`, and
    are never stored in a class field.
  * A pointer exists only inside a `fixed` or local scope.
  * Native-to-managed callbacks are static `[UnmanagedCallersOnly]` methods
    reached through function pointers, never delegates that the garbage
    collector can move or collect.
  * A `GCHandle` is owned by a `SafeHandle` or a `using` scope.
* **Layout agreement is tested.** For every type that crosses the boundary, the
  binding generator emits a test on each side. Rust checks `size_of`,
  `align_of`, and `offset_of!`, and C# checks `Unsafe.SizeOf` and
  `Marshal.OffsetOf`, both against the same expected values.

Documentation inside a zone:

* Each `unsafe` block is preceded by one `// SAFETY:` comment that states why
  the zone's registered invariants hold at that point. Each Rust `unsafe fn`
  carries a `# Safety` doc section, and each C# `unsafe` member carries a
  `// SAFETY:` comment. These are the only comments permitted in source code,
  and only inside zones (see Commenting policy).

Verification:

* The zone gate runs in the default lane. It fails when:
  * any unsafe construct appears outside a registered zone
  * a zone's lint table or project settings differ from the rules above
  * a zone's actual unsafe block count differs from its registered count (in
    either direction, so that every change to the unsafe surface shows in the
    registry diff)
  * a symbol is aliased at link time
  * a safety comment is missing or misplaced
  * a raw pointer type appears in a public signature outside the generated C
    ABI surface
  * a boundary type lacks its `repr` attribute or its size and alignment
    assertions
  * a public API takes an untyped integer handle
* The generated cross-language layout tests run in the default lane.
* `make verify-unsafe` runs in a declared non-default lane and is required for
  every change that touches a zone:
  * every zone's tests under Miri where the code does not call into foreign
    code
  * the zone and its callers built and tested with AddressSanitizer and
    ThreadSanitizer
  * Loom models of the job system's and the audio thread's synchronization
    primitives
  * a time-bounded fuzzing run of every exported entry point with random
    handles, lengths, and call orders
  * Kani proofs, for all inputs up to a stated bound, of the handle tables,
    allocators, ring buffers, and the index and length arithmetic that every
    `unsafe` block depends on
* Every change to a zone is reviewed against the zone's registered invariants
  by an agent other than its author. That review checks that each invariant is
  upheld, that no safe API can reach undefined behavior, and that a Miri,
  sanitizer, or fuzz test exercises the change.

The zone gate (`make zone-gate`, `python3 tools/verify.py zones`) runs in
the default lane and is enforced deterministically; the first zone,
`crates/unitas-editor-abi`, is registered in `tools/unsafe-zones.toml`.
`make verify-unsafe` is declared as a non-default lane and currently wires
the zone's Miri run (when a Miri toolchain is installed) and the zone's
test suite; its remaining declared stages (sanitizer, Loom, fuzz, Kani,
and layout tests) are reported by the lane as not yet wired, never
claimed. Until those stages are wired, every change that touches a zone
still owes the full verification list above.

## Commenting policy

Source code and `Makefile` contain no comments or docstrings, except the
`// SAFETY:` comments and `# Safety` doc sections that "Unsafe code zones"
requires inside a registered zone. Intent,
constraints, and invariants are expressed through names, types, structure,
tests, and assertions. The only `#` line tolerated in source code or scripts
is a shebang on line 1 of an executable script. Tool configuration files
(`pyproject.toml`, `Cargo.toml`, `rustfmt.toml`, `.editorconfig`, and
`Directory.Build.props`) carry only the governing-policy header comment and
the per-rule rationale comments required by `linting-policy.md` and
`typechecking-policy.md`.

Banned without exception in source code:

* Restating the code the comment sits on.
* Change narration ("added for X", "fixed", "now", "previously").
* Commented-out code.
* Section banners.
* Notes addressed to the reviewer instead of to the next maintainer.
* A deferred-work marker (a `todo`, `fixme`, or `hack` prefix).
* Encoding declarations, `# type: ignore`, `# noqa`, `# pragma`,
  `# pylint:` and `# fmt:` suppressions. The underlying warning is fixed
  in code, or suppressed in tool configuration, never inline.

Editing code means deleting any comment found in the touched region.
Docstrings are banned the same way as comments: no module, class,
function, or method docstring, and no string-literal statement used as
documentation. Contracts live in names, type annotations, and tests.

Evidence tags differ by claim. The comment/code-inconsistency link to
defects is tagged moderate (the source is listed under Research basis)
and is the basis for a blocking check. The over-commenting claim — that
agents comment too much, and that removing such comments changes
outcomes — is tagged anecdotal and its effect size is unconfirmed. The
ban itself is a project judgment tagged `none`: no recorded source
establishes that the ban lowers the defect rate, and the ban does not
rest on one. The evidence tag and the enforcement status are separate
properties: the tag names the kind of source behind the rule, the
enforcement status names who decides it. The ban ships as
`deterministic` because the comment audit enforces it; it is one of
two exceptions (along with the banned-construct ban) to the
execution-kernel rule that a rule tagged `anecdotal` or `none` ships
advisory.
No percentage, multiplier, or benchmark figure from either claim
belongs in this policy until it is confirmed against the primary
source.

There is no inline bypass marker and no baseline. A comment in source code
is a finding even when it carries a reason, and the gate holds no list of
exempted source lines.

The comment audit enforces this deterministically: it fails on every comment
token and every docstring in tracked Python, Rust, and C# source files, and on
every comment in shell, YAML, `Makefile`, and Jinja sources, except a line-1
shebang; in tool configuration files (`pyproject.toml`, `Cargo.toml`,
`rustfmt.toml`, `.editorconfig`, and `Directory.Build.props`) it permits the
governing-policy header comment and the per-rule rationale comments required
by `linting-policy.md` and `typechecking-policy.md` while rejecting any other
comment. It reads Python comments with the `tokenize` module and docstrings
through the `ast` module, and it reads the other formats with quote
awareness, so a `#` or `//` inside a string is never misread. A finding carries
`path:line`, the comment text, and the fix hint "delete the comment or
encode it in code".

## Documentation restriction

A change MAY edit documentation only when it changes user-visible
behaviour, a public interface, an install or run command, or a policy
contract, and then only the single owning document and only the affected
statement.

Documentation states current facts and procedures only. It does not
record why a choice was made, which alternatives were weighed, how the
choice evolved, research notes, exclusion lists, migration narration, or
a log of reasoning. A choice lives in the code, the tests, or the gate
that enforces it, and nowhere else.

No new documentation file, plan, design note, ADR, log, or summary is
created unless the task names the file. Documentation that contradicts
this rule is deleted or reduced to current facts.

## Guard inventory

| Guard | Owner | Protected outcome | Detected condition | Claim scope | Enforcement | Retirement trigger |
|---|---|---|---|---|---|---|
| Syntax-proxy ban | Clean-code owner | no banned construct reaches the tree through a wrapper or an alias | the ban gate reports zero findings over the tracked tree | the ban gate reports zero findings | deterministic: make banned-constructs | when the ban catalog is enforced by the linter itself |
| Bounded premature abstraction | Clean-code owner | a new abstraction carries a demonstrated boundary, volatility, testing, or reuse value | the change record names the boundary, volatility, testing, or reuse value the abstraction serves | the change record names the value the abstraction serves | advisory: no gate measures an abstraction against its justification; review: at every architecture review | when a gate can compare an abstraction with its justification |
| Comment and docstring absence | Documentation owner | the tracked tree carries no comment token and no docstring | the comment audit reports zero findings over the tracked tree | the comment audit reports zero findings | deterministic: make comment-audit | when the comment gate is retired |
| Python type-system escape-hatch ban | Clean-code owner | the Python ban catalog reports zero findings over the tracked tree | the ban gate reports zero findings in the Python sources | the ban gate reports zero findings in the Python sources | deterministic: make banned-constructs | when the linter enforces the catalog itself |
| Rust type-system escape-hatch ban | Clean-code owner | the Rust ban catalog reports zero findings over the tracked Rust sources | the ban gate reports zero findings in the Rust sources | the ban gate reports zero findings in the Rust sources | deterministic: make banned-constructs | when the linter enforces the catalog itself |
| C# type-system escape-hatch ban | Clean-code owner | the C# ban catalog reports zero findings over the tracked C# sources | the ban gate reports zero findings in the C# sources | the ban gate reports zero findings in the C# sources | deterministic: make banned-constructs | when the linter enforces the catalog itself |
| Unsafe zone containment | Clean-code owner | memory-unsafe code exists only in registered zones, each with enforced lints, a registered block count, and safety comments | the zone gate reports zero findings over the tracked tree, including each zone's enforced lint levels and its registered unsafe block count | the zone gate reports zero findings | deterministic: make zone-gate | when the linter alone enforces zone containment |
| Unsafe zone verification | Clean-code owner | every change to a zone passes Miri, sanitizer, Loom, fuzz, and Kani checks and an independent invariant review | make verify-unsafe exits zero for the change and the review record names the invariants checked | make verify-unsafe exits zero for the change | advisory: sanitizer Loom fuzz Kani and layout-test stages are declared and not yet wired; review: at every change that touches a zone | when no unsafe zone remains |
| Unsafe boundary layout agreement | Clean-code owner | Rust and C# agree on the size, alignment, and field offsets of every type that crosses the C ABI | the generated layout tests pass on both sides | the generated layout tests pass | advisory: the layout tests are generated in the change that registers the first zone; review: in the change that registers the first zone | when no type crosses the C ABI |

## Project facts to resolve

The `RALPH-FACT:` lines below record verified project facts. Agents rely
on them when enforcing this policy and MUST keep them current as the
project evolves.

RALPH-FACT: naming_convention: snake_case for modules, functions, and variables; CapWords for classes; UPPER_CASE for module constants
RALPH-FACT: module_boundary_rule: the gate logic lives in tools/verify.py and the gate tests in tools/test_verify.py; a new module needs a named owner
RALPH-FACT: error_handling_convention: a gate prints one finding per line and exits non-zero; an unexpected condition raises instead of returning a partial result
RALPH-FACT: logging_convention: a gate writes findings to standard output and writes no log file
RALPH-FACT: generated_code_marker: none — the repository generates no source
RALPH-FACT: vendored_code_marker: none — the repository vendors no third-party source
RALPH-FACT: dead_code_audit_command: make lint

## AI execution instructions

This section is the clean-code-specific procedure only. The five
generic rules (truthful evidence, fail-closed verification, bounded
execution, no weakening of checks, explicit unverified-claim
reporting), the precedence over domain policies, the
selective-applicability rule, and the claim-scope rule are owned by
the kernel starter (`execution-kernel.md`) and MUST be loaded from
there — do not re-derive them here. Exposing a private method or
adding a wrapper solely to satisfy a test or a structural proxy rule
is a violation (see `testing-policy.md`, `architecture-policy.md`);
the same mechanism is never justified here as a clean-code fix.

To follow this policy, an agent making any change MUST:

* PREFER existing project utilities and patterns over new abstractions.
* REMOVE dead code in the same change that obsoletes it.
* RECORD the clean-code review evidence appropriate to the change,
  covering the judgment a dead-code audit cannot: naming, cohesion,
  interface clarity, and abstraction justification.
* RUN every `RALPH-COMMAND:` gate declared under Verification before
  claiming the change complies.
* UPDATE this policy (facts, commands, requirements) in the same
  workflow that changes the naming convention, module boundary rules,
  or error-handling and logging conventions.

An agent MUST NOT:

* Introduce speculative abstractions ("we might need it later").
* Add unused compatibility layers.
* Weaken the no-dead-code rule to obtain a passing result.

## Verification

Run every gate below before claiming a change complies with this policy.

RALPH-COMMAND: make lint
RALPH-REVIEW: review naming, cohesion, interface clarity, abstraction justification, comment content, and error/logging conventions against verified project convention; evidence: dated clean-code review or explicit not-performed blocker; owner: code quality owner

RALPH-COMMAND: make comment-audit

RALPH-COMMAND: make zone-gate

The command gates' expected successful results are a clean dead-code audit
and a clean comment audit over the new and changed lines; report any audit
findings. The zone gate must report zero findings; a registered zone's
unsafe block count and lint table are checked against
tools/unsafe-zones.toml and the workspace lint table. The review gate
certifies the judgment the audits cannot.

## Exceptions

A documented exception (e.g. legacy compatibility shim with a removal
date) requires a documented reason, scope, owner, and removal or
review date.

## Maintenance triggers

This policy MUST be reviewed in the same workflow as any of:

* A new naming convention is introduced.
* A new module boundary rule is introduced.
* The error-handling or logging convention changes.
* A new generated/vendored code category is added.
* The commenting rule changes: a new banned comment shape, a change to
  the scanned file set, or a change to the enforcement mode.
* An unsafe zone is registered, retired, or changes its invariants, or a
  banned construct is found to be required by a product capability.

## Research basis

* publisher: Robert C. Martin ("Uncle Bob")
  title: "The Clean Code Blog"
  http: https://blog.cleancoder.com/
  review date: 2026-07-11

* publisher: Martin Fowler
  title: "Refactoring: Improving the Design of Existing Code"
  http: https://martinfowler.com/books/refactoring.html
  review date: 2026-07-11

* publisher: Google Engineering Practices
  title: "Code Health: Reduce Nesting, Reduce Complexity"
  http: https://google.github.io/eng-practices/review/developer/
  review date: 2026-07-11

* publisher: Sandi Metz
  title: "Practical Object-Oriented Design in Ruby"
  http: https://www.poodr.com/
  review date: 2026-07-11

* publisher: Microsoft Press / Steve McConnell
  title: "Code Complete: A Practical Handbook of Software Construction"
  http: https://www.microsoftpressstore.com/articles/article.aspx?p=2222451
  review date: 2026-07-12

* publisher: arXiv (Radmanesh, Imani, Ahmed, Moshirpour)
  title: "Investigating the Impact of Code Comment Inconsistency on Bug Introducing"
  http: https://arxiv.org/abs/2409.10781
  review date: 2026-11-09

* publisher: Google Style Guides
  title: "Documentation Best Practices"
  http: https://google.github.io/styleguide/docguide/best_practices.html
  review date: 2026-11-09

* publisher: The Rust Project
  title: "The Rustonomicon"
  http: https://doc.rust-lang.org/nomicon/
  review date: 2026-11-25

* publisher: The Rust Project
  title: "Miri: an interpreter for Rust's mid-level intermediate representation"
  http: https://github.com/rust-lang/miri
  review date: 2026-11-25

* publisher: The Rust Project
  title: "Clippy lints: undocumented_unsafe_blocks and multiple_unsafe_ops_per_block"
  http: https://rust-lang.github.io/rust-clippy/master/index.html
  review date: 2026-11-25

* publisher: Microsoft
  title: "Native interoperability best practices (.NET)"
  http: https://learn.microsoft.com/en-us/dotnet/standard/native-interop/best-practices
  review date: 2026-11-25

* publisher: Google and contributors
  title: "zerocopy: zero-cost memory manipulation"
  http: https://docs.rs/zerocopy/latest/zerocopy/
  review date: 2026-11-25

* publisher: Model Checking (AWS)
  title: "The Kani Rust Verifier"
  http: https://model-checking.github.io/kani/
  review date: 2026-11-25

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

* Policy id: `<!-- ralph-policy-id: clean-code-policy.md -->`
* Schema version: `<!-- ralph-policy-schema: v7 -->`
