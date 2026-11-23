<!-- ralph-policy-schema: v6 -->
<!-- ralph-policy-id: documentation-policy.md -->
# Documentation Policy

## Purpose and scope

This policy governs every documentation surface the project maintains:
user, operator, contributor, API, architecture, and code documentation.
It defines when a behaviour change requires a documentation update, the
restrictions on docstrings, comments, public APIs, configuration,
commands, examples, migrations, and release notes, and where each kind
of documentation belongs.

## Default requirements

* Documentation MUST explain current behaviour and user decisions. It
  MUST NOT restate obvious code or include fabricated capabilities,
  dependencies, adoption claims, or unsupported technical statements.
* Behaviour changes MUST update affected documentation in the same
  workflow. Stale docs are a defect.
* The authoritative location for each kind of documentation MUST be
  documented in this policy. Stale, duplicated, contradictory, or
  obsolete documentation MUST be removed or reconciled.
* Examples and commands in documentation MUST match actual behaviour
  and MUST be verified where practical.
* Docstrings are banned. No module, class, function, or method
  docstring, and no string-literal statement used as documentation.
  Contracts live in names, type annotations, and tests.
* A change MAY edit documentation only when it changes user-visible
  behaviour, a public interface, an install or run command, or a policy
  contract, and then only the single owning document and only the
  affected statement.
* Documentation states current facts and procedures only. It does not
  record why a choice was made, which alternatives were weighed, how the
  choice evolved, research notes, exclusion lists, migration narration, or
  a log of reasoning. A choice lives in the code, the tests, or the gate
  that enforces it, and nowhere else.
* No new documentation file, plan, design note, ADR, log, or summary is
  created unless the task names the file. Documentation that contradicts
  this rule is deleted or reduced to current facts.
* Externally supported APIs MUST follow the declared documentation convention
  for purpose, parameters, return value, and relevant exceptions. Usage
  examples are required for non-obvious contracts, not trivial accessors.
* Configuration documentation MUST list every option with its
  default, valid range, and effect.
* Release notes MUST enumerate user-visible changes and required
  migrations.

## Guard inventory

| Guard | Owner | Protected outcome | Detected condition | Claim scope | Enforcement | Retirement trigger |
|---|---|---|---|---|---|---|
| Single source of truth | Documentation owner | every policy statement lives in exactly one canonical file | the policy gate reports one canonical policy directory, one managed block, and no cited path that does not resolve | the policy gate reports one canonical policy directory and no cited path that does not resolve | deterministic: make policy-validate | when the canonical directory is removed (the kernel absorbs the contract) |
| Comment and docstring absence | Documentation owner | the tracked tree carries no comment token and no docstring | the comment audit reports zero findings over the tracked tree | the comment audit reports zero findings | deterministic: make comment-audit | when the comment gate is retired |

## Project facts to resolve

The `RALPH-FACT:` lines below record verified project facts. Agents rely
on them when enforcing this policy and MUST keep them current as the
project evolves.

RALPH-FACT: user_docs_path: none — the repository has no user-facing product yet
RALPH-FACT: operator_docs_path: none — the repository has no operator-facing surface yet
RALPH-FACT: contributor_docs_path: AGENTS.md
RALPH-FACT: api_reference_path: none — the repository exposes no public API yet
RALPH-FACT: architecture_docs_path: docs/ralph-workflow-policy/architecture-policy.md
RALPH-FACT: release_notes_path: none — the repository has not released a version
RALPH-FACT: docstring_convention: docstrings are banned; the comment audit reports every docstring it finds
RALPH-FACT: example_verification_command: make policy-validate

## AI execution instructions

This section is the documentation-specific procedure only. The five
generic rules (truthful evidence, fail-closed verification, bounded
execution, no weakening of checks, explicit unverified-claim
reporting), the precedence over domain policies, the
selective-applicability rule, and the claim-scope rule are owned by
the kernel starter (`execution-kernel.md`) and MUST be loaded from
there — do not re-derive them here.

To follow this policy, an agent making any change MUST:

* UPDATE affected documentation in the same change that alters
  behaviour.
* REMOVE duplicated or contradictory documentation; do not silently
  duplicate.
* EXECUTE example commands when safe and feasible. Otherwise perform syntax
  or static validation and record dated review evidence, prerequisites, and
  the reason execution was unsafe or unavailable.
* RECORD the documentation review evidence appropriate to the change,
  covering the judgment execution cannot: accuracy against current behavior,
  absence of fabricated or unsupported claims, and authoritative-location
  reconciliation.
* RUN every `RALPH-COMMAND:` gate declared under Verification before
  claiming the change complies.
* UPDATE this policy (facts, commands, requirements) in the same
  workflow that changes a documentation location, the docstring
  convention, or the example verification command.

An agent MUST NOT:

* Fabricate capabilities, dependencies, adoption claims, or unsupported
  technical statements.
* Leave known drift between code and documentation for a later fix.
* Add duplicated copies of authoritative content.

## Verification

Run every gate below before claiming a change complies with this policy.

RALPH-COMMAND: make policy-validate
RALPH-REVIEW: review documentation for accuracy against current behavior, absence of fabricated or unsupported claims, and reconciliation of authoritative locations; evidence: dated documentation review or explicit not-performed blocker; owner: documentation owner

Command-gate success means the policy gate reports no finding: every cited
path resolves, every fact is resolved exactly once, and every declared
command resolves to a gate target. The review gate certifies accuracy and
non-fabrication.

## Exceptions

A documented exception (e.g. legacy doc kept for backward URL
compatibility) requires a documented reason, scope, owner, and
removal or review date.

## Maintenance triggers

This policy MUST be reviewed in the same workflow as any of:

* A new documentation surface is added.
* The documentation-restriction rule changes.
* The example verification command changes.

## Research basis

* publisher: Google Engineering Practices
  title: "Code Review: Comment Quality"
  http: https://google.github.io/eng-practices/review/developer/
  review date: 2026-07-11

* publisher: Write the Docs
  title: "Documentation Style Guide"
  http: https://www.writethedocs.org/guide/writing/style-guides/
  review date: 2026-07-11

* publisher: The Twelve-Factor App
  title: "IX. Disposability"
  http: https://12factor.net/disposability
  review date: 2026-07-11

* publisher: Daniele Procida
  title: "Diátaxis Documentation Framework"
  http: https://diataxis.fr/
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

* Policy id: `<!-- ralph-policy-id: documentation-policy.md -->`
* Schema version: `<!-- ralph-policy-schema: v6 -->`
