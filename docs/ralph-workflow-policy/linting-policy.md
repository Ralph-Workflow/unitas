<!-- ralph-policy-schema: v6 -->
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
* The lint configuration starts by enabling every rule and disables only a
  rule that fires on common safe use. It contains no per-file ignore of a
  banned rule, no `extend-ignore` that re-enables a banned rule, and no inline
  suppression marker.

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
| Strictest lint configuration | Lint owner | the lint configuration selects the project rule set and disables no rule | the bypass audit reports that the lint configuration selects a rule set and carries no ignore key | the bypass audit reports that the lint configuration selects a rule set | deterministic: make bypass-audit | when the linter can prove the configuration is the strictest available |
| Lint run coverage | Lint owner | the lint run checks every tracked source file | the lint gate exits 0 over the tracked tree | the lint gate exits 0 | deterministic: make lint | when the lint run reports the files it checked |
| Public / wrapper syntax-proxy | Lint owner | no banned construct reaches the tree through a wrapper | the ban gate reports zero findings over the tracked tree | the ban gate reports zero findings | deterministic: make banned-constructs | when the ban catalog is enforced by the linter itself |

## Project facts to resolve

The `RALPH-FACT:` lines below record verified project facts. Agents rely
on them when enforcing this policy and MUST keep them current as the
project evolves.

RALPH-FACT: linter_per_language: ruff for Python, cargo clippy for Rust, and dotnet format for C#
RALPH-FACT: rule_set_baseline: the B, C4, E, F, I, RUF, SIM, UP, and W rules selected in pyproject.toml
RALPH-FACT: formatter_responsibility: ruff format owns Python formatting and dotnet format owns C# formatting; no Rust formatter is configured yet
RALPH-FACT: excluded_paths: none — the lint gate covers every tracked Python and Rust file
RALPH-FACT: suppression_rationale_policy: no suppression exists; a lint finding is fixed in the code
RALPH-FACT: ci_gate_integration: make lint runs ruff and cargo clippy in the default gate through the Makefile verify target
RALPH-FACT: maintenance_evidence: the lint report the last make verify run printed
RALPH-FACT: first_party_languages: Python for the repository tooling; Rust for the product crates; C# for the UnityEngine bridge
RALPH-FACT: excluded_language_evidence: none — no language is excluded from the lint gate
RALPH-FACT: dead_code_detection_per_language: ruff check reports unused imports and unused locals for Python; cargo clippy reports dead code for Rust; the .NET compiler and analyzer report unused members and unreachable code for C#
RALPH-FACT: retained_compatibility_inventory: none — the repository retains no compatibility layer
RALPH-FACT: live_entry_point_annotation_inventory: none — the repository has no speculative entry point

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

An agent MUST NOT:

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
RALPH-COMMAND: cargo clippy --workspace --all-targets -- -D clippy::correctness -D clippy::suspicious

RALPH-LANG: C#
RALPH-COMMAND: make dotnet-format
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
* Schema version: `<!-- ralph-policy-schema: v6 -->`
