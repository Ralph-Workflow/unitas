<!-- ralph-policy-schema: v6 -->
<!-- ralph-policy-id: verification-policy.md -->
# Verification Policy

## Purpose and scope

This policy defines the authoritative verification entry point: every gate
that must pass before merge or release, the exact commands, the
prerequisites, the time budgets, the lanes a check may live in, and the
bypass-detection rules.

Two terms are used precisely. A CHECK is any automated verification this
project owns. A GATE is a check in the default lane, run by the
authoritative entry point on every change. Every gate is a check; not every
check is a gate. Cite lanes BY NAME — the testing policy numbers its lanes
differently.

## Default requirements

1. A single authoritative pre-merge entry point MUST exist. Expensive
   platform, device, release, security, and scheduled checks MAY use named
   profiles declared under Gate lanes, when this policy states when each is
   mandatory.
2. Gates MUST include, as applicable: tests, type checking, linting,
   formatting checks, policy enforcement scripts, and any other mandatory
   project gate.
3. Testing is mandatory for behavior-bearing software. Every language's
   maintained type-checking, linting, and formatting gates are mandatory
   when suitable tools exist. Preference, inconvenience, legacy findings,
   or missing setup does not make a supported gate inapplicable.
4. A gate documented here but not runnable is non-compliant. Documented
   impossibility MUST be reported as an active blocker.
5. Bypass detection MUST use native or existing checks when available.
   Custom tooling is required only when repository risk justifies it; never
   create a hollow gate solely to satisfy this policy.
6. Verification MUST pass in full, with NO exemption for a failure the
   current change did not cause. "It was already broken", "unrelated to my
   change", and "someone else introduced it" are NOT acceptable outcomes: a
   red gate is a red gate, and whoever next observes it owns fixing it.
   Preventing regressions outranks completing the task in hand — if the two
   conflict, fix the regression first and finish the task after.
7. Do NOT spend effort establishing WHO caused a failure. Stashing,
   bisecting, or re-running against a clean tree to prove a failure
   pre-existing is almost NEVER useful: the answer does not change what you
   must do next, which is fix it. Provenance is worth investigating only
   when genuinely diagnostic — when the triggering change tells you what the
   bug IS — never to decide whether the failure is yours to own. It always
   is.
8. Every check MUST sit in a declared lane, and every check outside the
   default gate MUST additionally have a declared owner (see Gate lanes). A
   check running only in an undeclared opt-in suite the default gate
   excludes will rot unnoticed: give it a lane, or delete it.
9. Verification MUST complete in time proportional to the codebase, enforced
   by the gate itself (fail on exceed), never by convention. Record the
   enforced limit as `verification_time_budget`: the project's MEASURED gate
   time, never a cap and never a guess.
   - **Cost floor:** the irreducible toolchain cost of answering at all —
     interpreter or runtime startup, environment provisioning, and the
     incremental build and link that a one-line, behaviour-neutral edit to a
     representative implementation file forces — measured
     invocation-to-answer on a warm cache with every check deselected.
     Record it as `verification_cost_floor`, separately for the full gate,
     the fast path, and the default suite, each with its measuring command,
     edited file, machine, and date. Test execution, check work, network,
     sleeps, and anything a check could skip NEVER count toward the floor.
     A floor that cannot be reproduced from its recorded command is zero.
     Re-measure it whenever the toolchain, build configuration, or
     reference machine changes; it shrinks freely and grows only as a
     reviewed change carrying the new measurement. Reducing a large floor —
     build caching, precompiled headers, a persistent build daemon,
     splitting a monolithic link — is owned work under rule 22, not a
     blocker on unrelated changes.
   - **Controllable time** is measured time minus the matching cost floor.
     Every guide and cap in this policy and the testing policy binds
     controllable time only: a project whose toolchain needs minutes just to
     rebuild is held to the same discipline as one that starts in a second.
     A game engine whose one-line edit forces a 4-minute incremental link
     records a 4-minute floor, and its gate is bound to that floor plus at
     most 2 minutes.
   - **Guide:** roughly **1 second of controllable time per 1k LOC**.
   - **HARD CAP: 2 minutes above the recorded cost floor**, whatever the
     size — a gate slower than that
     destroys the edit/verify loop humans and agents depend on, so past
     ~120k LOC the cap, not the rate, binds. This cap is far stricter than
     commonly cited industry guidance for a commit build, because an agent
     BLOCKS on this gate many times per task, paying the cost per iteration
     rather than per commit — a five-iteration task reaches ten minutes at
     this cap.
   - **Nesting:** the suite is ONE STEP inside this budget, not a parallel
     allowance. The testing policy caps the default suite at 60 seconds above
     its cost floor so
     type checking, linting, formatting, and audits fit in the remainder. A
     suite consuming the whole gate budget is a breach even when its own
     limit is satisfied.
   - **Ratchet:** the budget may shrink freely; it grows only as a reviewed
     change tracking genuinely-fast new checks. A project under budget MUST
     NOT relax toward the guide OR the cap — both are ceilings, never
     entitlements. Record the project's measured figure, never the cap.
     <!-- ralph-control: verification-budget-ratchet -->
   - **Optimize first:** when the project's measured controllable time
     exceeds a cap in this policy or the testing policy — a multi-million-line
     codebase whose full gate takes 40 minutes above its floor — the agent
     MUST first apply every reduction achievable within the run: run
     independent checks in parallel, cache with complete keys so only work a
     change can affect re-runs (rule 17), remove duplicate checks, and repair
     the slowest checks and the missing seams behind them. Leaving an
     achievable reduction undone is a violation, never a deferral.
   - **Over a cap after optimizing:** record the measured figure, with no
     headroom, as the enforced budget, so the gate fails the moment the
     project gets slower. Record the residual excess in
     `verification_budget_overage` in exactly this form, every field
     non-empty:
     `excess: <gate, fast-path, and suite seconds over each cap>; owner: <person or team>; plan: <reductions applied and reductions remaining>; review: <YYYY-MM-DD>`.
     Readiness rejects any other form, and blocks once the review date has
     passed: at review the agent optimizes again, re-measures, and records an
     excess no higher than before with a new date. The budget and the excess
     only shrink; every new default-lane cost MUST be offset by a recorded
     removal or speed-up while the overage remains; narrowing the gate,
     moving a check out of the default lane, or deselecting work to reach a
     cap stays forbidden. The overage blocks no unrelated change. A project
     within every cap records `none`.
10. A run is judged on **time to a correct, verified change**; speed never permits partial proof.
11. Prevent an extra pass first; within a pass, the cost of proving the change is the largest term; where they conflict, avoid the extra pass.
12. The four commitments are: **fast path first**, **a slow gate is a defect**, **do not leave the loop slower**, and **checks answer consistently**. Non-compliance is a defect: fix it within the requested run when that fits its budget, or raise it to an owner who can act. A change is never done on a partial check.
13. FAST PATH FIRST. Maintain a narrow check catching the change's likely
    failures and run it before the full gate. It MUST be SCOPED TO THE
    CHANGE — the modified files and the TRANSITIVE set depending on them —
    not a hand-picked subset that drifts out of relevance. When the full
    gate already finishes inside the fast-path cap, THE FULL GATE IS THE
    FAST PATH and no selection mechanism is required; small projects satisfy
    this by being fast, not by building machinery.
14. Any selection mechanism MUST FAIL SAFE: when impact cannot be
    determined it runs everything, never less. Fall back to the full set
    whenever any of these is in play, because each carries a dependency edge
    file-level analysis cannot see — reflection, dynamic dispatch, DI
    containers, plugin or entry-point registries; monkeypatching by string
    path; shared fixtures, factories, or per-directory test configuration,
    which apply by location rather than import; database migrations;
    code-generation inputs such as schemas, interface definitions, and
    templates; runtime resource files such as SQL, templates, i18n, and
    static assets; environment variables and feature flags, which change
    behaviour with NO file change; cross-language boundaries and generated
    client types; deletions and renames, where the importers that just broke
    are exactly what must run; lockfiles and transitive bumps; and
    configuration or build files. This list is a floor: an unmapped input is
    a reason to run everything.
15. Selection optimizes the FAST PATH ONLY. The full gate before done runs
    everything in its lane; a change is never verified on a selected subset.
16. Fast-path and full-gate costs MUST be measured invocation-to-answer
    including startup, and the gate MUST emit its observed duration so a
    regression is a number, not a feeling. Target 10 seconds of
    controllable time for the fast path, HARD CAP 30 seconds above the fast
    path's recorded cost floor (rule 9). Record the enforced limit as
    `fast_path_time_budget`: the measured fast-path time, never the cap; the
    fast path MUST fail when it exceeds that budget. A fast path already
    over the cap records its excess in `verification_budget_overage` on
    rule 9's terms. A project with a
    negligible floor is held to 10 and 30 seconds outright. A floor above
    the target is a measured fact to record and an owned reduction under
    rule 22, never a reason to widen the cap on controllable time. Added
    cost and any budget breach require an explicit choice and actionable
    escalation.
17. Parallelism and caching are the two legitimate levers for holding the
    budget as the project grows, permitted only when they cannot produce a
    FALSE GREEN. Parallelism requires independent, order-insensitive checks.
    Caching requires a key covering EVERY input that can change the answer —
    the same taxonomy as rule 14, since a key omitting an environment
    variable, generated source, lockfile, or resource file returns green for
    a run that never happened — PLUS the inputs that are not repository
    content at all: toolchain and interpreter versions, operating system and
    architecture, and invocation flags. Caching MUST also provide a
    documented bypass. Unlike selection, caching has no full-gate backstop,
    so an incomplete key is a correctness defect, not a latency one.
18. A check answering inconsistently is a defect: fix or raise it, never
    normalize reruns. Failure output MUST identify what broke so diagnosis
    is bounded.
19. Orientation every run would otherwise rebuild MUST be durable, cheap to
    read, and updated by the run that learns it.
20. Phase order, wait permissions, and phase-granted capabilities belong to
    workflow machinery outside this policy; the runtime owns their settings
    and reports them alongside phase time where available.
21. A SLOW GATE IS A DEFECT, not a cost of doing business. Superlinear
    growth or a hanging step is a hard indicator of a real problem, usually
    architectural — the testing policy's performance rule diagnoses it in
    full. Treat a gate performance regression as seriously as a failing
    test: fix the design. NEVER raise a budget to make a slow gate fit, and
    never inflate a cost floor to the same end: a floor is re-measured from
    its recorded command, never estimated.
22. The full gate remains required before done. Reducing recurring
    full-gate, fast-path, or setup cost is in scope when it fits the run
    budget without displacing the request; otherwise raise it where an owner
    can act. A raised cost must be actionable, not merely reported.

## Gate lanes

The effective portfolio owns five deliberate destinations: the deterministic
`default` lane, owned `triggered` profiles, `human` review, `operational`
monitoring, and preserved `one-off` evidence. Every non-default destination
MUST name its owner, trigger, marginal cost, evidence oracle, and retention
review condition; moving a check never hides or launders its cost.

Every check MUST sit in exactly one lane below, or in a lane declared by
another policy in this set (the testing policy's human-review and
one-off-evidence lanes are the common cases). Lanes keep the default gate
FAST and TRUSTED without pushing real checks into an unowned opt-in suite
where they decay silently.

1. DEFAULT GATE — the authoritative pre-merge entry point. Mandatory on
   every change, bound by rule 9, and deterministic and hermetic enough to
   run from a clean clone. A check that CAN meet those constraints MUST be
   here, new or existing. Budget pressure never qualifies a check for
   another lane: merely slow is a defect here, under rule 21.
2. NAMED PROFILE — a valuable, repeatable check that CANNOT meet the default
   gate's constraints: a live third-party service, a paid or rate-limited
   API, a device or platform matrix, fixed hardware for a timing
   measurement, a large dataset, a long soak, a release artifact, or output
   non-deterministic by nature that can only be scored against a recorded
   baseline. This list is EXHAUSTIVE; a new category requires amending this
   policy. Dataset size and device coverage qualify only when the SCALE or
   DEVICE is itself the behaviour under test.
   A profile is legitimate only with all five declared here: its command,
   its OWNER, its TRIGGER or SCHEDULE, that it FAILS HARD when it runs, and
   the DATE IT LAST RAN with the staleness horizon past which it is decayed.
   A profile past its horizon with no run evidence belongs in the DELETED
   lane — a check that never runs reports a safety it never verified.
   These categories are the single authoritative list; the testing policy
   routes into this lane rather than restating them, so a category added
   here is available there automatically.
3. DELETED — for a check with no owner, no trigger, no lane, or no evidence
   of ever running. Deleting a decayed check is honest; leaving it in an
   unobserved corner is not.

A check MUST NOT be moved into the named-profile lane to escape a red result
or a budget breach, and a new check MUST NOT be born there for the same
reason. Demotion is a design choice with an owner and a recorded reason,
never a way to make the gate green today. The declared owner MUST be a named
person or team, not a role the author occupies by default.

## Guard inventory

| Guard | Owner | Protected outcome | Detected condition | Claim scope | Enforcement | Retirement trigger |
|---|---|---|---|---|---|---|
| Bounded run budget | Verification owner | the full gate finishes inside the budget the Makefile declares | each gate target prints its elapsed time and the verify target fails when the budget is exceeded | each gate target prints its elapsed time | deterministic: make verify | when the budget moves |
| `verification-budget-ratchet` Verification budget ratchet | Verification owner | the verification budget shrinks freely and grows only through a reviewed change | the gate report records the elapsed time against the recorded budget | the gate report records the elapsed time | deterministic: make verify | when the budget is measured and recorded by the gate |
| No high-churn evidence in review | Verification owner | no volatile report and no generated inventory enters a human review | the review request names the durable artifact the reviewer reads | the review request names the durable artifact | advisory: no gate inspects a review request; review: at every review request | when the review request is a machine-readable artifact |
| Gate-lane coverage | Verification owner | every declared gate sits in a lane the portfolio records | the policy gate resolves every declared command to a gate target the Makefile defines | the policy gate resolves every declared command to a gate target | deterministic: make policy-validate | when the portfolio records a lane no gate uses |
| Public-behavior proof | Verification owner | a material behaviour claim names the command that produced it and that command's output | the gate report names the command that ran and the findings it printed | the gate report names the command that ran | deterministic: make verify | when a published artifact replaces the gate report |

## Project facts to resolve

The `RALPH-FACT:` lines below record verified project facts. Agents rely
on them when enforcing this policy and MUST keep them current as the
project evolves.

RALPH-FACT: authoritative_verify_command: make verify
RALPH-FACT: gate_prerequisites: python3 with pytest, ruff, mypy, and detect-secrets installed, the Rust toolchain (cargo and clippy), and the .NET 8.0 SDK
RALPH-FACT: gate_order: the Makefile verify target runs three concurrent groups — the scan group (policy-validate, bypass-audit, banned-constructs, comment-audit, deps, secret-scan), the suite group (lint, typecheck, test), and the .NET group (dotnet-build, then dotnet-format alongside dotnet-test)
RALPH-FACT: fast_path_command: make lint
RALPH-FACT: fast_path_selection_mechanism: none — the full gate finishes inside the fast-path cap, so the full gate is the fast path
RALPH-FACT: gate_parallelism_mechanism: the Makefile verify target starts the scan group, the suite group, and the .NET group as concurrent sub-makes, one gate at a time inside the scan and suite groups, and the .NET group builds first and then runs dotnet-format alongside dotnet-test, so the wall clock is the slowest group while every gate keeps its declared order and its own 30s budget
RALPH-FACT: gate_cache_mechanism_and_key: none — the gates are not cached
RALPH-FACT: gate_duration_report_location: the duration line each Make target prints after its command
RALPH-FACT: bypass_detection_lint_audit: make bypass-audit
RALPH-FACT: bypass_detection_typecheck_audit: make bypass-audit
RALPH-FACT: ci_integration_command: make verify
RALPH-FACT: required_verification_profiles: none — every check runs in the default gate
RALPH-FACT: verification_time_budget: 30 seconds for make verify, declared as VERIFY_BUDGET_SECONDS in the Makefile; the measured run took 28 seconds on 2026-11-24 with the Python, Rust, and .NET gates (including Unitas.Scripting and Unitas.Testing) wired into the default gate
RALPH-FACT: verification_cost_floor: full gate floor 1.43s (measured via `pytest --collect-only -q` at 0.27s, `cargo check --workspace --all-targets` at 0.10s, and `dotnet build bridge/Unitas.sln --configuration Release -p:AnalysisMode=All --no-restore` at 1.06s against `tools/verify.py`, `crates/unitas-core/src/lib.rs`, and `bridge/UnityEngine/Object.cs`), fast-path floor 0.10s (measured via `cargo check --workspace --all-targets` against `crates/unitas-core/src/lib.rs`), and default suite floor 0.36s (measured via `pytest --collect-only -q` at 0.27s and `cargo test --workspace --no-run` at 0.09s against `tools/test_verify.py` and `crates/unitas-core/src/lib.rs`) on Apple Silicon (macOS Darwin 25.5.0 arm64) on 2026-11-24
RALPH-FACT: fast_path_time_budget: 30 seconds enforced per step via VERIFY_BUDGET_SECONDS in the Makefile (`python3 tools/verify.py timed <step> 30 ...`) for `make lint`, which measured 0.18s warm on Apple Silicon on 2026-11-24
RALPH-FACT: verification_budget_overage: none — full gate controllable time is 26.57s (28.00s measured minus 1.43s cost floor, below the 120s cap), fast-path controllable time is 0.08s (0.18s measured minus 0.10s cost floor, below the 30s cap), and default suite controllable time is 7.27s (7.63s measured for `make test` minus 0.36s cost floor, below the 60s cap)
RALPH-FACT: verification_time_enforcement_mechanism: the timed gate wrapper fails the run when a gate exceeds the budget the Makefile declares
RALPH-FACT: gate_lane_review_cadence_and_owner: every policy remediation pass, owned by the repository maintainers

`required_verification_profiles` is the single home for the project's named
non-default profiles, including any declared by the testing policy. Record
all five fields for each: command, named owner, trigger or schedule,
fails-hard-when-run, and last-run date with staleness horizon.

## AI execution instructions

This section is the verification-specific procedure only. The five
generic rules (truthful evidence, fail-closed verification, bounded
execution, no weakening of checks, explicit unverified-claim
reporting), the precedence over domain policies, the
selective-applicability rule, and the claim-scope rule are owned by
the kernel starter (`execution-kernel.md`) and MUST be loaded from
there — do not re-derive them here.

On every change:

1. RUN the change-scoped fast path first, then the full authoritative gate
   before declaring done. A selected subset is never sufficient proof
   (rules 13, 15).
2. ENSURE every gate listed here is runnable in the environment. Document
   any that cannot run, and why.
3. RUN every `RALPH-COMMAND:` gate below, disclosing any result served
   from a cache rather than a fresh run (rule 17).
4. RECORD the observed full-gate duration when it changes materially, and
   justify any verification cost the change adds (rule 16).
5. PLACE any new check in a declared lane, naming the lane; outside the
   default gate, name its owner too. A check with no lane is not shipped.
6. UPDATE this policy when the entry point, gate order, selection or caching
   mechanism, profile set, or bypass-detection audit changes.

## High-churn evidence (B8)

Volatile reports (per-run test logs, mutation-analysis dumps, coverage
HTML, snapshot diffs) and large generated inventories go to ignored paths
under `/tmp` or to CI artifacts. Commit only compact, deterministic,
human-reviewable baselines whose diffs carry meaning: the
`verification_time_budget`, `fast_path_time_budget`, and
`verification_cost_floor` facts, the recorded gate-lane review with its
dated evidence, and reviewable gate results. A reviewer who pulls the change
must see a single small diff against a known baseline, not a thousand-line
inventory of auto-generated noise.

An agent MUST NOT:

* Add a "verification" command that does not exercise every default-gate
  check.
* Lower the recorded `verification_time_budget` or `fast_path_time_budget`
  below the project's measured figure, grow `verification_budget_overage`,
  record an overage without its measured figure, owner, and reduction plan,
  record a `verification_cost_floor`
  that was not measured or that counts check work, narrow the default gate to skip a documented check,
  or shrink a per-step timeout past the proven floor to make a slow
  or failing run pass. The kernel's no-weakening rule (see
  `execution-kernel.md` §Kernel rules 4) applies in full to every
  verification-specific choice below.
* Move a check into a profile, create a new check in a profile, or narrow
  the selected subset, to avoid a red result or a budget breach.
* Rely on a cache or selection mechanism that can report success without
  having proven the change.
* Hide bypasses via file-level disables or blanket silencers.
* Dismiss, defer, or excuse a failing gate as pre-existing, unrelated, or
  someone else's regression. Fix it, or stop and report it as an active
  blocker — never report work as verified while any gate is red (rule 6).
* Stash, bisect, or re-run against a clean tree merely to establish that a
  failure is pre-existing. The verdict is the same either way (rule 7).

## Verification

Run every gate below before claiming a change complies with this policy.

RALPH-COMMAND: make verify
RALPH-REVIEW: review that every declared profile has run within its staleness horizon, that no check has been demoted out of the default gate to avoid a red result, and that the recorded gate duration still matches reality; evidence: dated gate-lane review listing each profile with its last-run date; owner: the named person or team recorded in the gate_lane_review_cadence_and_owner fact

Expected result: exit 0 from the authoritative entry point within the
recorded budget. On failure report the failing gate and the category —
assertion, type, lint, policy, timeout, environmental. A `timeout` is a HARD
failure: fix the slow check, never the budget.

The gate-lane review runs on the cadence in
`gate_lane_review_cadence_and_owner`, and a review older than that cadence
is itself a finding — without it, nothing enforces the staleness horizon
that keeps a declared profile honest.

## Bypass detection

Lint and typecheck bypass detection MUST be part of the authoritative gate:

* Newly weakened global configuration (per-file-ignores,
  ignore_missing_imports, follow_imports = silent, ignore_errors,
  disable_error_code, and similar) is detected and reported.
* Inline suppressions are detected and reported.
* Commands claiming to verify the project while omitting required paths are
  detected and reported.
* A selection or caching mechanism that can shrink what actually ran is a
  bypass surface: it MUST be observable in gate output — what was selected,
  what was cached — so a shrinking gate cannot pass as a passing gate.

RALPH-COMMAND: make bypass-audit

Expected result: exit 0 (no bypass detected). On failure report the affected
file, line, and bypass category. Approved documented exceptions MUST be
listed under "Exceptions".

## Exceptions

A documented bypass (e.g. a generated file with a `// @ts-nocheck`
header) requires a documented reason, scope, owner, and removal or
review date. Undocumented bypasses are non-compliant.

## Maintenance triggers

Review this policy in the same workflow as any of:

* A gate is added or removed.
* The authoritative entry point changes.
* The fast-path command or its change-scoping mechanism changes.
* The gate's parallelism or caching mechanism changes.
* A named profile is added, retired, or has its owner or trigger changed.
* A named profile has not run within its declared staleness horizon.
* The observed gate duration moves materially, or the budget changes.
* The toolchain, build configuration, or reference machine changes: re-measure
  `verification_cost_floor`.
* The bypass-detection audit changes.

## Research basis

* publisher: Google Engineering Practices
  title: "Code Review: Speed of Code Reviews"
  http: https://google.github.io/eng-practices/review/reviewer/speed.html
  review date: 2026-07-11

* publisher: Google SRE Book
  title: "Monitoring Distributed Systems"
  http: https://sre.google/sre-book/monitoring-distributed-systems/
  review date: 2026-07-11

* publisher: Martin Fowler
  title: "Continuous Integration"
  http: https://martinfowler.com/articles/continuousIntegration.html
  review date: 2026-07-11

* publisher: DORA / Google Cloud
  title: "Capabilities: Continuous integration"
  http: https://dora.dev/capabilities/continuous-integration/
  review date: 2026-08-08

* publisher: Google Research
  title: "Taming Google-Scale Continuous Testing"
  http: https://research.google/pubs/taming-google-scale-continuous-testing/
  review date: 2026-08-08

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

* Policy id: `<!-- ralph-policy-id: verification-policy.md -->`
* Schema version: `<!-- ralph-policy-schema: v6 -->`
