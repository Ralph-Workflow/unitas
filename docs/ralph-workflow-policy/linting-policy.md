<!-- ralph-policy-schema: v7 -->
<!-- ralph-policy-id: linting-policy.md -->
# Linting Policy

## Purpose and scope

This policy governs the selected linters, their exact commands, the
source categories covered, the rule set, the strictness baseline, the
formatting responsibilities, and the rules around inline suppressions and
configuration overrides.

## Default requirements

A maintained first-party language includes interpreted or compiled production
and test source owned by the project. Generated/vendored code, embedded
snippets, and template/IDL/SQL/shell sources are classified explicitly in
`first_party_languages` and `excluded_language_evidence` according to whether
the project maintains and can lint them.

* When a maintained first-party language supports a suitable maintained linter or
  equivalent static quality tool, the project MUST select and run one.
  Missing configuration, legacy findings, or team preference do not make
  linting inapplicable.
* Selection MUST consider active maintenance, compatibility, useful
  diagnostics, configurability, and CI suitability. No product is
  universally preferred by this policy.
* Formatting MUST be enforced by a maintained formatter or by the linter
  when it provides an equivalent deterministic formatting check. Competing
  tools MUST NOT own the same formatting rules.
* Per-language `RALPH-LANG:` declarations are required for every maintained
  first-party code language, each followed by a `RALPH-COMMAND:` line or an
  explicit `RALPH-INAPPLICABLE:` line.
* Inline suppressions are forbidden. The underlying diagnostic is fixed in
  code, or suppressed in the linter's per-file configuration, never inline.
* The authoritative configuration MUST identify file-level disables and broad
  exclusions with their reason; duplicating every entry in this policy is not
  required.
* The lint configuration starts by enabling every rule. It contains no
  per-file ignore of a banned rule, no `extend-ignore` that re-enables a
  banned rule, and no inline suppression marker. Choosing between two enabled
  rules that contradict each other is configuration, not a bypass.
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

## Size limits

`clean-code-policy.md` sets the ceilings that bind every maintained
first-party language, tests included: no first-party source file exceeds 500
lines, no function or method exceeds 50 lines, and no function exceeds a
cyclomatic complexity of 10. These limits are maximums; a project may lower
them, never raise them. Every maintained first-party language, including
every language the project adds later, MUST enforce a function-length limit
and a file-length limit. A language is not admitted to the project until both
limits are wired for it.

* **Function length.** A function, method, or closure body MUST NOT exceed
  50 lines. Where the language's linter counts statements or complexity
  instead of lines, the linter's own strict limit applies in addition. The
  enforced threshold drops from 200 lines to the 50-line ceiling once the
  tree satisfies 200; lowering it is tightening, raising it is weakening.
* **File length.** A tracked first-party source file, meaning every file the
  lint gate covers once the `excluded_paths` fact below is applied, MUST NOT
  exceed 500 lines, counting every physical line, test modules and data
  tables included.
  The limit drops to 1,500 lines once the tree satisfies 3,000 and to the
  500-line ceiling once the tree satisfies 1,500; lowering it is tightening,
  raising it is weakening.
* **Linter first.** Each limit is enforced by the language's linter wherever
  the linter offers a rule for it, configured in the language's root
  configuration file. Where the linter offers no such rule, the repository gate
  in `tools/verify.py` enforces it. A missing linter rule never exempts a
  language from a limit.
* **No exemptions.** There is no per-file, per-function, or per-language
  exemption, allowlist, or baseline, and no inline suppression of a size rule.
  Generated code is governed at its source boundary as `clean-code-policy.md`
  requires. A file or function over a limit is split along its
  responsibilities: a data table moves to its own module, a class that
  implements several provider interfaces splits into one type per interface,
  and a long function splits into named steps.

The repository does not enforce those ceilings yet. The thresholds in force
are the ones in the wiring table below: 200 lines per function and 3,000
lines per file. They stand above the ceilings because the tree is not split
to that size yet, and `size_limit_status` records the measured gap, the
owner, the lane, and the review trigger that brings each threshold down to
its ceiling. The thresholds come down as that work lands, they never come up,
and no exemption, baseline, or allowlist stands in for the missing splits.

Per-language wiring:

| Language | Function length | File length |
|---|---|---|
| Rust | `clippy::too_many_lines` denied through `clippy.pedantic`, with `too-many-lines-threshold = 200` in the root `clippy.toml` | no Clippy rule counts lines; the repository gate enforces 3,000 lines |
| Python | ruff `PLR0915` (`max-statements = 50`), `PLR0912` (`max-branches = 12`), and `C901` (`max-complexity = 10`) in `pyproject.toml`; the 200-line threshold by the repository gate | no ruff rule counts lines; the repository gate enforces 3,000 lines |
| C# | Roslyn `CA1502`, `CA1505`, and `CA1506` bound complexity, maintainability, and coupling at their default thresholds; no Roslyn rule counts lines, so the 200-line threshold is enforced by the repository gate | no Roslyn rule counts lines; the repository gate enforces 3,000 lines |
| A language added later | its linter's function-length rule, configured at the ceiling | its linter's file-length rule, or the repository gate |

## Dead code — zero tolerance

Dead code is prohibited. It is better to delete obsolete code and implement it
again if demonstrated need returns than to retain unowned paths indefinitely.

* Unused, unreachable, obsolete, superseded, commented-out, and speculative
  code MUST be removed in the same change that makes it unnecessary.
* “Might be needed later,” compatibility without a verified consumer, and
  incomplete future plans are never exceptions; version control preserves
  history.
* Dead-code findings MUST be fixed at the source, never hidden with ignores,
  exported-name tricks, fake references, coverage exclusions, or disabled rules.
  A proven live path reached through reflection, a framework/plugin registry,
  FFI callback, dynamic dispatch, conditional platform build, or external
  consumer MAY use the narrowest tool-supported live-entry annotation. This is
  not a dead-code exception: it requires observable consumer/entry-point
  evidence, reason, owner, and review trigger; speculative consumers do not qualify.
* Generated or vendored code is governed at its source boundary. A retained
  first-party compatibility path requires a verified consumer, owner, removal
  condition, and expiry/review date.
* Every ecosystem-supported unused/dead-code check MUST run in the declared
  lint or verification gate. When no suitable maintained automated check
  exists, code review MUST still enforce deletion.

## Guard inventory

| Guard | Owner | Protected outcome | Detected condition | Claim scope | Enforcement | Retirement trigger |
|---|---|---|---|---|---|---|
| No lint bypass | Lint owner | no inline lint suppression and no per-file ignore is present in the tracked tree | the bypass audit reports zero findings in the source and in the tool configuration | the bypass audit reports zero findings | deterministic: make bypass-audit | when the linter reports its own suppressions |
| Strictest lint configuration | Lint owner | the lint configuration selects the project rule set and carries only reasoned rule disables | the bypass audit reports that the lint configuration selects the project rule set and carries only reasoned rule disables | the bypass audit reports that the lint configuration selects the project rule set | deterministic: make bypass-audit | when the linter can prove the configuration is the strictest available |
| Lint run coverage | Lint owner | the lint run checks every tracked source file | the lint gate exits 0 over the tracked tree | the lint gate exits 0 | deterministic: make lint | when the lint run reports the files it checked |
| Public / wrapper syntax-proxy | Lint owner | no banned construct reaches the tree through a wrapper | the ban gate reports zero findings over the tracked tree | the ban gate reports zero findings | deterministic: make banned-constructs | when the ban catalog is enforced by the linter itself |
| Rust function length | Lint owner | no Rust function, method, or closure body exceeds the 200-line threshold in force | cargo clippy reports zero clippy::too_many_lines findings at too-many-lines-threshold = 200 | the clippy gate exits 0 | deterministic: make lint | when Clippy no longer offers clippy::too_many_lines |
| Python function size | Lint owner | no Python function exceeds 50 statements, 12 branches, or cyclomatic complexity 10 | ruff check reports zero PLR0915, PLR0912, and C901 findings | the ruff gate exits 0 | deterministic: make lint | when ruff no longer offers PLR0915, PLR0912, or C901 |
| Line-counted function length for Python and C# | Lint owner | no Python or C# function or method body exceeds the 200-line threshold in force | the repository size gate reports zero over-limit functions | the repository size gate reports zero findings | deterministic: make size-gate | when ruff and the Roslyn analyzers enforce a 200-line function limit directly |
| File length | Lint owner | no tracked first-party source file in any maintained language exceeds the 3,000-line threshold in force | the repository size gate reports zero over-limit files | the repository size gate reports zero findings | deterministic: make size-gate | when every configured linter enforces a file-length limit directly |
| Size ceiling conformance | Lint owner | no tracked first-party source file exceeds the 500-line ceiling and no function or method exceeds the 50-line ceiling | the ceiling gap measured on 2026-11-26 and recorded in `size_limit_status` | the ceiling gap measured on 2026-11-26 | advisory: the size gate and clippy enforce the thresholds in force while the split work that brings them to the ceilings is outstanding; review: at every change that touches a file or function above the threshold in force and at every staged tightening | when the thresholds in force reach the ceilings |

## Project facts to resolve

The `RALPH-FACT:` lines below record verified project facts. Agents rely
on them when enforcing this policy and MUST keep them current as the
project evolves.

RALPH-FACT: linter_per_language: ruff check and ruff format for Python, cargo clippy and rustfmt (cargo fmt) for Rust, and the .NET Roslyn analyzers (dotnet build) and dotnet format for C#
RALPH-FACT: rule_set_baseline: Python selects ALL in pyproject.toml ([tool.ruff.lint] select = ["ALL"]) with only reasoned disables, each choosing between two enabled rules that contradict each other: formatter conflicts, mutually exclusive pydocstyle pairs, clean-code docstring rules, pytest assertions, and gate subprocess execution (COM812, D100-D107, D203, D213, ISC001, S101, S603), no per-file-ignores, and the size thresholds pinned explicitly ([tool.ruff.lint.pylint] max-statements = 50 and max-branches = 12, [tool.ruff.lint.mccabe] max-complexity = 10); Rust declares [workspace.lints] in the root Cargo.toml with rust.warnings = "deny", rust.unsafe_code = "forbid", rust.unnecessary_transmutes = "deny", rust.static_mut_refs = "deny", clippy.all = "deny" (priority = -1), clippy.pedantic = "deny" (priority = -1), and the clean-code ban-catalog lints (unwrap_used, expect_used, panic, todo, unimplemented, dbg_macro) set to "deny", with every non-zone workspace crate setting lints.workspace = true and only reasoned workspace-level pedantic allows, clippy::too_many_lines left denied through clippy.pedantic, and the root clippy.toml setting too-many-lines-threshold = 200; a crate registered as an unsafe zone in tools/unsafe-zones.toml carries its own [lints] table instead, as required by clean-code-policy.md "Unsafe code zones"; C# declares Nullable=enable, TreatWarningsAsErrors=true, AnalysisMode=All, AnalysisLevel=latest-all, and EnforceCodeStyleInBuild=true in the root Directory.Build.props with formatting and reasoned diagnostic severities in the root .editorconfig
RALPH-FACT: formatter_responsibility: ruff format --check . owns Python formatting (pyproject.toml); cargo fmt --all -- --check owns Rust formatting (rustfmt.toml); dotnet format bridge/Unitas.sln --verify-no-changes --no-restore owns C# formatting (.editorconfig), including whitespace, style, and analyzer diagnostics
RALPH-FACT: excluded_paths: Python: none — every tracked .py file under tools/ is linted and formatted; Rust: none — every workspace crate and tracked .rs file is linted and formatted; C#: bridge/Unitas.Bridge.Tests/Fixtures/** (excluded via Compile Remove in bridge/Unitas.Bridge.Tests/Unitas.Bridge.Tests.csproj because those files are test-input C# fixtures compiled dynamically by test cases rather than maintained project source); build output directories target/, bin/, and obj/ are skipped
RALPH-FACT: suppression_rationale_policy: inline suppressions, per-file ignores, allowlists, and baselines are forbidden; every disabled rule or downgraded analyzer severity lives in the language's root configuration file (pyproject.toml, Cargo.toml, or .editorconfig) with an adjacent written reason comment enforced by make bypass-audit
RALPH-FACT: ci_gate_integration: make lint runs ruff check ., ruff format --check ., cargo fmt --all -- --check, cargo clippy --workspace --all-targets --target-dir target/clippy -- -D warnings, and dotnet format bridge/Unitas.sln --verify-no-changes --no-restore; make size-gate runs python3 tools/verify.py size over the tracked first-party source tree; make verify runs all of them in the default gate within VERIFY_BUDGET_SECONDS; .github/workflows/verify.yml runs make verify on ubuntu-24.04 with a 30-minute job timeout for every pull request, every push to main, and every manual dispatch, and pins Python 3.14 with ruff 0.15.16, mypy 2.1.0, pytest 9.0.3, and detect-secrets 1.5.0, Rust 1.93.0 with the rustfmt and clippy components, and the .NET SDK 8.0.428. No remote run of that workflow has been observed; Review trigger: the first recorded run of .github/workflows/verify.yml on the remote repository reports its exit status
RALPH-FACT: maintenance_evidence: Python: ruff 0.15.16 (Astral official release https://docs.astral.sh/ruff/, compatible with Python 3.11+ and covering all tracked tools/*.py files, selected 2026-11-25, recheck trigger: major Python or ruff upgrade, incompatibility, abandonment, or security signal); Rust: clippy 0.1.93 and rustfmt 1.8.0 on rustc 1.93.0 (Rust Project official toolchain https://doc.rust-lang.org/clippy/, covering all workspace crates and targets under edition 2021 and 2024, selected 2026-11-25, recheck trigger: Rust toolchain or edition upgrade, incompatibility, abandonment, or security signal); C#: .NET SDK 8.0.428 Roslyn analyzers and dotnet format (Microsoft official .NET 8 SDK https://learn.microsoft.com/dotnet/fundamentals/code-analysis/overview, covering all 12 projects in bridge/Unitas.sln across bridge/ and editor/, selected 2026-11-25, recheck trigger: .NET SDK major upgrade, incompatibility, abandonment, or security signal)
RALPH-FACT: first_party_languages: Python for the repository verification and CLA tooling (tools/); Rust for the engine, runtime, editor-abi, and build crates (crates/); C# for the UnityEngine bridge, scripting/testing/debugging assemblies, and editor host (bridge/ and editor/)
RALPH-FACT: excluded_language_evidence: none — no language is excluded from the lint gate
RALPH-FACT: dead_code_detection_per_language: ruff check (with select = ["ALL"], including F401, F841, ERA, and PIE) reports unused imports, unused locals, and dead code for Python; rustc and cargo clippy (with rust.warnings = "deny", clippy.all = "deny", and clippy.pedantic = "deny") report dead_code and unused items as errors for Rust; the .NET compiler and Roslyn analyzers (with AnalysisMode=All, AnalysisLevel=latest-all, EnforceCodeStyleInBuild=true, and TreatWarningsAsErrors=true) report unused members, unread private state, and unreachable code as errors for C#
RALPH-FACT: retained_compatibility_inventory: none — the repository retains no compatibility layer
RALPH-FACT: live_entry_point_annotation_inventory: none — the repository has no speculative entry point
RALPH-FACT: size_limit_status: measured on 2026-11-26 with the thresholds in force applied and the repository size gate (`make size-gate`, `python3 tools/verify.py size`) wired into `make verify`. Rust function length: 0 functions exceed 200 lines (`cargo clippy --workspace --all-targets --target-dir target/clippy -- -D warnings` exits 0 across every workspace crate including `unitas-build` and `unitas-editor-abi`). Python and C# function length: 0 Python or C# functions exceed 200 lines (`python3 tools/verify.py size` / `make size-gate` exits 0 in `make verify`, and `ruff check .` exits 0). File length: 0 tracked first-party source files exceed 3,000 lines (`python3 tools/verify.py size` / `make size-gate` exits 0 in `make verify`). Ceiling gap against the ceilings `clean-code-policy.md` sets, measured the same day: 206 tracked first-party source files exceed 500 lines, 566 Rust functions exceed 50 lines (`cargo clippy --workspace --all-targets` with `too-many-lines-threshold = 50`), and 414 Python and C# functions and methods exceed 50 lines (the repository size gate with the 50-line threshold applied); the repository size gate counts 205 of the 206 over-ceiling files, and `crates/unitas-build/tests/build/content_catalog.rs` is outside its directory scope because its directory is named `build`. Staged tightening follow-up work, staged on 2026-11-26: file length 3,000 to 1,500 to 500 lines and function length 200 to 50 lines, in that order, and the size gate must reach `crates/unitas-build/tests/build/` before the file-length ceiling is measured over the whole tracked tree; owner: Lint owner; trigger: when the files and functions above the threshold in force are split below the next threshold; lane: `make size-gate` and `make lint` in `make verify`; evidence: `python3 tools/verify.py size` and `cargo clippy --workspace --all-targets` reporting zero findings above the staged threshold; Review trigger: when any tracked first-party file or function above the threshold in force is touched, or when the next staged threshold is lowered in `tools/verify.py` or `clippy.toml`

For each language, `maintenance_evidence` records tool and version range,
official maintenance source, compatibility and first-party coverage evidence,
selection date, and recheck trigger. Recheck on major language/tool changes,
incompatibility, abandonment, or a relevant security signal.

## AI execution instructions

This section is the linting-specific procedure only. The five generic
rules (truthful evidence, fail-closed verification, bounded
execution, no weakening of checks, explicit unverified-claim
reporting), the precedence over domain policies, the
selective-applicability rule, and the claim-scope rule are owned by
the kernel starter (`execution-kernel.md`) and MUST be loaded from
there — do not re-derive them here. A private method made public or
a wrapper added solely to satisfy a structural proxy lint rule is a
violation (see `clean-code-policy.md`, `testing-policy.md`), not a
justified lint pass.

To follow this policy, an agent making any change MUST:

* DECLARE one `RALPH-LANG:` block per language with the exact linter
  command.
* PREFER existing tooling. Adding a new linter requires a documented
  reason.
* RUN every `RALPH-COMMAND:` gate declared under Verification before
  claiming the change complies.
* UPDATE this policy (facts, commands, requirements) in the same
  workflow that changes the linter, rule set, formatter, or
  suppression policy.
* WIRE both size limits (see Size limits) for a new language in the same
  change that admits it, in its linter configuration where a rule exists and
  in the repository gate where none does.
* SPLIT a function or file that a change pushes over the size ceilings in
  `clean-code-policy.md` in that same change, along its responsibilities.

An agent MUST NOT:

* Raise a size threshold, allow `clippy::too_many_lines`, or exempt a file,
  function, or language from a size limit.

* Add `per-file-ignores`, `extend-per-file-ignores`, or any other
  global rule silencing without a per-file reason.
* Add an inline suppression. Fix the underlying lint finding in code, or
  suppress it in the linter's per-file configuration.
* Lower the lint strictness level to obtain a passing result.
* Retain verified dead code, suppress it, or add a fake use solely to make a
  linter pass. A verified live-entry annotation follows the evidence contract above.

## Verification

Run every gate below before claiming a change complies with this policy.

RALPH-LANG: Python
RALPH-COMMAND: make lint

RALPH-LANG: Rust
RALPH-COMMAND: cargo clippy --workspace --all-targets --target-dir target/clippy -- -D warnings

RALPH-LANG: C#
RALPH-COMMAND: make dotnet-format

RALPH-COMMAND: make size-gate
The expected successful result is a clean lint run (exit 0). Lint
failures MUST be addressed at the source, not by adding suppressions.

## Exceptions

A maintained first-party language may use `RALPH-INAPPLICABLE:` only when
documented research proves no suitable maintained linter/formatter exists or
the surface is technically non-lintable. Preference, inconvenience, legacy
findings, migration cost, and absent setup are invalid. The declaration MUST
start with `exceptional case: no suitable maintained linter exists;` or
`exceptional case: technically non-lintable first-party surface;` and include
`evidence:`, `owner:`, `expiry:`, `warning:`, and `review trigger:`. The visible
warning remains until the exception is removed.

## Maintenance triggers

This policy MUST be reviewed in the same workflow as any of:

* A new language is added to the project.
* The linter, version, or rule set changes.
* A new lint dependency is added.
* The suppression policy changes.
* A size limit, its threshold, or its enforcer changes.
* The dead-code detector, compatibility inventory, or generated-code boundary changes.

## Research basis

* publisher: Astral (ruff)
  title: "ruff documentation: Rules"
  http: https://docs.astral.sh/ruff/rules/
  review date: 2026-07-11

* publisher: ESLint Project
  title: "Configuring ESLint"
  http: https://eslint.org/docs/latest/use/configure/
  review date: 2026-07-11

* publisher: The Rust Project
  title: "Clippy documentation"
  http: https://doc.rust-lang.org/clippy/
  review date: 2026-07-11

* publisher: Google Engineering Practices
  title: "The CL Author's Guide to Getting Through Code Review"
  http: https://google.github.io/eng-practices/review/developer/
  review date: 2026-07-11

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

* Policy id: `<!-- ralph-policy-id: linting-policy.md -->`
* Schema version: `<!-- ralph-policy-schema: v7 -->`
