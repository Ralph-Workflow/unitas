<!-- ralph-policy-schema: v7 -->
<!-- ralph-policy-id: verification-policy.md -->
# Verification Policy

## Purpose and scope

This policy defines the authoritative verification entry point: every gate
that must pass before merge or release, the exact commands, the
prerequisites, the time budgets, the lanes a check may live in, and the
bypass-detection rules.

Loading trigger: a change to a gate command, a gate lane, a time budget,
or the authoritative verification entry point.

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
   by the gate itself, never by convention. The gate FAILS only when its
   controllable time exceeds the cap below, never because a run came in
   slower than the last measurement. Record the project's MEASURED gate time
   as `verification_time_budget`: the measurement, never the cap and never a
   guess. It is the figure each re-measurement is compared against, not a
   limit the gate fails on. Record the limit the gate enforces as
   `verification_enforced_limit`.
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
     size, and this cap is the limit the gate enforces — a gate slower than that
     destroys the edit/verify loop humans and agents depend on, so past
     ~120k LOC the cap, not the rate, binds. This cap is far stricter than
     commonly cited industry guidance for a commit build, because an agent
     BLOCKS on this gate many times per task, paying the cost per iteration
     rather than per commit — a five-iteration task reaches ten minutes at
     this cap.
   - **Shared hosts:** the gate enforces the cap, never the measurement, so a
     run slowed by other work on the same host (parallel agent runs, a busy
     CI runner) stays inside it. A gate that only passes on a quiet host
     enforces the wrong number and breaks rule 12's "checks answer
     consistently". Time limits are judged on contention-adjusted time:
     measured time minus the share of the host that other processes kept
     busy during the run, which the gate records alongside its timing. A run
     whose checks all pass but whose time crosses a limit on a contended host
     is inconclusive, not a defect: it is never re-recorded in this policy,
     never optimized, and never re-run to wait for a quiet host, and the work
     proceeds on its correctness result. Only an overrun on an uncontended
     run is a defect to optimize. NEVER re-run repeatedly waiting for a quiet
     host, and never treat a contended overrun as an architecture failure.
   - **One full gate at a time:** iterate on the narrowest check that covers
     the change and run the full gate once, before the commit. Never start a
     second full gate while one is running for the same worktree. Full gates
     on one host share host-wide slots, so a gate may wait for its turn
     before it starts; that wait is not gate time.
   - **Gates leave nothing running:** every check runs in its own process
     group and is reaped when it ends. No process, temporary directory,
     browser profile, container, or file watch survives a gate run, and the
     gate fails when one does: a leak slows every other run on the host, so
     it is a correctness defect, not a performance one.
     Moving a gate's enforced limit from its recorded measurement to the cap
     is not budget growth: the recorded measurement does not change.
   - **Nesting:** the suite is ONE STEP inside this budget, not a parallel
     allowance. The testing policy caps the default suite at 60 seconds above
     its cost floor so
     type checking, linting, formatting, and audits fit in the remainder. A
     suite consuming the whole gate budget is a breach even when its own
     limit is satisfied.
   - **Re-measurement:** re-measure on a warm, otherwise idle reference run
     whenever the toolchain or the gate changes and at each review; a new
     measurement above the recorded one is a regression to optimize under
     rule 12's "do not leave the loop slower". Growth is caught by
     re-measuring, never by failing every run a busy host slows. A project
     under its cap MUST NOT relax toward the guide OR the cap — both are
     ceilings, never entitlements. One run's timing is never a reason to
     re-record a figure: a commit whose only change is a refreshed
     measurement is churn. Record figures in whole seconds.
   - **Ratchet:** the recorded measurement may shrink freely; it grows only
     as a reviewed change tracking genuinely-fast new checks. Record the
     project's measured figure, never the cap.
     <!-- ralph-control: verification-budget-ratchet -->
   - **Optimize first:** when the project's measured controllable time
     exceeds a cap in this policy or the testing policy — a multi-million-line
     codebase whose full gate takes 40 minutes above its floor — the agent
     MUST first apply every reduction achievable within the run: run
     independent checks in parallel, cache with complete keys so only work a
     change can affect re-runs (rule 17), remove duplicate checks, and repair
     the slowest checks and the missing seams behind them. Leaving an
     achievable reduction undone is a violation, never a deferral.
   - **Over a cap after optimizing:** record the measured figure as the
     budget, and enforce it plus a 25% shared-host allowance, so the gate
     fails when the project really gets slower but never because the host
     was busy. Record the residual excess in
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
    path's recorded cost floor (rule 9). The fast path FAILS only when it
    exceeds the limit it enforces — for the recorded fast path, which IS the
    full gate, that limit is the full gate's `verification_enforced_limit` —
    never because a run came in slower than the recorded measurement. Record
    the measured fast-path time as `fast_path_time_budget`: the measurement
    each re-measurement is compared against, never the cap. A fast path
    already over the cap records its excess in `verification_budget_overage`
    and enforces its measured figure plus rule 9's 25% shared-host
    allowance. The cap is always measured above the recorded floor, never as
    a flat wall-clock figure. A floor above
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
   The date MUST name the durable source that carries it (a CI run id, a
   committed baseline, or the gate-lane review entry); a run whose only
   record lives in an ignored path is not recorded as run. A profile past
   its horizon with no run evidence belongs in the DELETED lane — a check
   that never runs reports a safety it never verified.
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
| Bounded run budget | Verification owner | the full gate finishes inside the enforced limit the Makefile declares | each timed gate step prints its elapsed time, is terminated at its enforced limit, and the verify target fails when that limit is exceeded | each gate target prints its elapsed time | deterministic: make verify | when the enforced limit moves |
| Gate leaves nothing running | Verification owner | no process, temporary directory, browser profile, container, or file watch survives a gate run | the timed gate runner starts every step in its own process group (`start_new_session=True`), sweeps the group with `SIGKILL` (`kill_timed_gate_group`) when the step ends or is terminated, and `make policy-validate` checks that `verification-policy.md` preserves the non-negotiable leak-free gate rule | survivors of one gate run on the host that ran it | deterministic: make verify | when the gate no longer starts processes |
| `verification-budget-ratchet` Verification budget ratchet | Verification owner | the verification budget shrinks freely and grows only through a reviewed change | the gate report records the elapsed time against the recorded budget | the gate report records the elapsed time | deterministic: make verify | when the budget is measured and recorded by the gate |
| No high-churn evidence in review | Verification owner | no volatile report and no generated inventory enters a human review | the review request names the durable artifact the reviewer reads | the review request names the durable artifact | advisory: no gate inspects a review request; review: at every review request | when the review request is a machine-readable artifact |
| Gate-lane coverage | Verification owner | every declared gate sits in a lane the portfolio records | the policy gate resolves every declared command to a gate target the Makefile defines | the policy gate resolves every declared command to a gate target | deterministic: make policy-validate | when the portfolio records a lane no gate uses |
| Public-behavior proof | Verification owner | a material behaviour claim names the command that produced it and that command's output | the gate report names the command that ran and the findings it printed | the gate report names the command that ran | deterministic: make verify | when a published artifact replaces the gate report |

## Project facts to resolve

The `RALPH-FACT:` lines below record verified project facts. Agents rely
on them when enforcing this policy and MUST keep them current as the
project evolves.

RALPH-FACT: authoritative_verify_command: make verify
RALPH-FACT: gate_prerequisites: python3 with pytest, ruff, mypy, and detect-secrets installed, the Rust toolchain (cargo, clippy, and rustfmt), and the .NET 8.0 SDK
RALPH-FACT: gate_order: the Makefile verify target runs four concurrent top-level sub-make groups (`verify-scan-gates`, `verify-static-gates`, `verify-test-gates`, and `verify-dotnet-gates`) and waits for all four before checking total elapsed wall time against `VERIFY_BUDGET_SECONDS` (124s): (1) `verify-scan-gates` runs eight repository scan targets concurrently (`policy-validate`, `bypass-audit`, `zone-gate`, `banned-constructs`, `comment-audit`, `deps`, `secret-scan`, `size-gate`); (2) `verify-static-gates` runs five static checks concurrently (`ruff check .`, `ruff format --check .`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --target-dir target/clippy -- -D warnings`, and `mypy`) and aggregates their exit statuses so one red check does not hide the others; (3) `verify-test-gates` runs `python3 tools/verify.py pytest` and `python3 tools/verify.py cargo-test` concurrently (the same two steps `make test` runs under its 61s `SUITE_BUDGET_SECONDS` wrapper via `test-steps`), where `cargo-test` compiles all workspace test targets once (`cargo test --workspace --lib --tests --bins --no-run --message-format=json`, omitting doc-test targets because no crate declares doctests) and executes the compiled binaries across package shards with up to eight binaries per package running at once; clippy uses `target/clippy`, while `cargo-test` and `runtime-build` share the default `target/` directory and serialize on Cargo's artifact lock; (4) `verify-dotnet-gates` starts `runtime-build` (`cargo build -p unitas-runtime -p unitas-editor-abi -p unitas-build`) in the background (`& pr=$$!`), runs `dotnet-build` (`dotnet build bridge/Unitas.sln --configuration Release`) in the foreground, then launches `make dotnet-format` in the background (`& pf=$$!`) so the format pass overlaps the runtime build and the test shards, waits for `runtime-build` (`wait $$pr`), and then runs `dotnet-test-run`, which stages `target/editor-test-bundle` via `unitas-player --sample` when needed and runs `bridge-test-shards` (twelve concurrent shards of `Unitas.Bridge.Tests`), `Unitas.PlayerHost.Tests`, and `editor-test-shards` (`ScenarioToolsTests` in the background while `StdioTransportTests` runs first, then the remaining `EDITOR_TEST_SHARDS` up to `EDITOR_TEST_SHARD_FANOUT := 12`) concurrently, and finally waits for the format pass (`wait $$pf`); standalone `make lint` runs ruff check, ruff format, cargo fmt, cargo clippy, and `dotnet-format` sequentially, and standalone `make typecheck` runs mypy, `cargo check --workspace --all-targets`, and `dotnet-build` (which depends on `runtime-build`) sequentially
RALPH-FACT: fast_path_command: make verify
RALPH-FACT: fast_path_selection_mechanism: none — the full gate fits the fast-path cap in the recorded warm run; make lint remains the standalone lint and format target
RALPH-FACT: gate_parallelism_mechanism: the Makefile `verify` target launches four concurrent top-level sub-makes (`verify-scan-gates`, `verify-static-gates`, `verify-test-gates`, `verify-dotnet-gates`) and aggregates their exit codes; inside `verify-scan-gates`, all eight scan gates (`policy-validate`, `bypass-audit`, `zone-gate`, `banned-constructs`, `comment-audit`, `deps`, `secret-scan`, `size-gate`) run in parallel; inside `verify-static-gates`, all five static gates (`ruff check .`, `ruff format --check .`, `cargo fmt --all -- --check`, `cargo clippy` into `target/clippy`, and `mypy`) run in parallel; inside `verify-test-gates`, `python3 tools/verify.py pytest` (using `-n 4` when `xdist` is installed) and `python3 tools/verify.py cargo-test` (one `--no-run` workspace test compile followed by `ThreadPoolExecutor` execution across package shards with a per-package semaphore of 8) run in parallel; inside `verify-dotnet-gates`, `runtime-build` runs in the background while `dotnet-build` runs in the foreground, `dotnet-format` then runs in the background overlapping the runtime build and the test shards, and once both builds finish, `dotnet-test-run` overlaps `bridge-test-shards` (12 concurrent shards), `Unitas.PlayerHost.Tests`, and `editor-test-shards` (`ScenarioToolsTests` started alongside `StdioTransportTests`, then the remaining shards batched up to 12). Every timed step runs under `python3 tools/verify.py timed` in its own process group with a 124s limit (`VERIFY_BUDGET_SECONDS`, or 61s `SUITE_BUDGET_SECONDS` for `make test`) and one automatic re-run on a budget overrun, and the outer `verify` target enforces the 124s wall-clock limit across the whole run
RALPH-FACT: gate_cache_mechanism_and_key: the secret scan caches its findings in .ruff_cache/secret_scan_cache.json, keyed by a SHA-256 digest over the .secrets.baseline bytes, the detect-secrets tool version, the Python interpreter version, the platform, and the path and content of every scanned file; a cache hit is reported in the gate output as `secret scan: cache hit for N files (digest ...)`, and deleting the cache file forces a fresh scan; the dotnet format gate caches its pass in .ruff_cache/dotnet_format_cache.json, keyed by a SHA-256 digest over the dotnet format command, the reported dotnet version, and the path and content of every formatted file, and a cache hit is reported as `dotnet format: cache hit for N files (digest ...)`; both caches live inside the working tree and both are discarded when their key changes
RALPH-FACT: gate_duration_report_location: the duration line each Make target prints after its command
RALPH-FACT: bypass_detection_lint_audit: make bypass-audit
RALPH-FACT: bypass_detection_typecheck_audit: make bypass-audit
RALPH-FACT: ci_integration_command: make verify
RALPH-FACT: ci_integration_workflow: .github/workflows/verify.yml runs one verify job on ubuntu-24.04 with a 30-minute job timeout, on pull_request, push to main, and workflow_dispatch; it checks out the repository, pins Python 3.14 with pytest 9.0.3, ruff 0.15.16, mypy 2.1.0, and detect-secrets 1.5.0, installs Rust 1.93.0 with the rustfmt and clippy components and the .NET 8.0.428 SDK, then runs `make verify`; the workflow's FIRST REMOTE EXECUTION IS NOT YET OBSERVED — no CI run id carries it, so the lane's runtime on a runner is unmeasured; the recorded 26.24-second measurement and the 124-second enforced limit the job runs under both come from a WARM Apple Silicon host, so a cold runner may exceed that limit before any gate fails; review: at the first observed remote execution, record the run id and the observed lane time, then decide the cache and warm-up story (a restored cargo target cache) that keeps a cold runner inside the enforced limit
RALPH-FACT: required_verification_profiles: five named non-default lanes, each failing hard when run: `make verify-samples`, `make verify-benchmark`, `make verify-device`, `make verify-perf`, and `make verify-unsafe`

The named non-default profiles, each with the five required fields:

- `make verify-samples` — owner: editor and engine proof (the area that owns
  `samples/` and the playtest framework); trigger: changes to script
  compilation, the play pipeline, the player, or `samples/`; evidence:
  per-scenario report JSONs, the packaged-player step's `build-report.json`
  assertions and control-stdio replay, and lane exit status; fails hard when
  run; last run 2026-11-26, recorded in this tree by the re-approved
  directional-shadow S8 reference baselines under
  `samples/roll-a-ball/References/` and the lane exit status of that run;
  staleness horizon one milestone.
- `make verify-benchmark` — owner: editor and engine proof; trigger: changes
  to the editor's MCP tool surface, the agent-authoring benchmark, or the
  change-review policy; evidence: the lane report at
  `target/benchmark-lane/report.json`, its per-criterion OK/FAIL lines, and the
  lane exit status; fails hard when run; last run 2026-11-26, recorded in the
  gate-lane review entry below; staleness horizon one milestone.
- `make verify-device` — owner: presentation; trigger: every
  milestone touching presentation; evidence: lane exit status and captured
  GPU/audio/input artifacts; fails hard when run; last run 2026-11-26, recorded
  in this tree by the gate-lane review entry below; `tools/approved_gpu_reference.bmp`
  was re-approved on 2026-11-26 against the real shadowed device frame: the
  staged player's bundle now carries the project's `ProjectSettings/` directory
  next to `Assets/`, so the runtime resolves the `Standalone` quality level
  (`shadows: 2`, `shadowResolution: 2`, `shadowDistance: 50`) and the device
  frame carries the directional shadow instead of the shadows-off fallback; the
  re-approved file is the exact capture the lane produces today (two
  consecutive captures differ in 0 of 76 800 pixels, and the fresh capture
  matches the approved file with max_diff=0); against the prior pre-shadow
  baseline the shadowed capture differs in 3 834 of 76 800 pixels (4.99 %); the
  lane also asserts that the presented window frame carries the scene's uGUI
  canvas and the project shadows (`canvas=true`, `shadow=true` in the
  `device_verify=ok` line), so the windowed frame, not only the capture, is
  proven; the window sustained 34.2 FPS under shared-host contention and 38.9 FPS
  on an idle run against the 30.0 FPS interactive floor (`MIN_SUSTAINED_FPS`);
  the durable source for the shadow evidence is also the re-approved
  S8 reference set under `samples/roll-a-ball/References/`; staleness horizon
  one milestone.
- `make verify-perf` — owner: content and simulation (lane runner); trigger:
  every milestone touching import, scene load, physics, animation, scripting,
  presentation, or the editor; evidence: per-measurement OK/FAIL lines with
  value_ms against ceiling_ms, and value_bytes against ceiling_bytes for the
  PlayerLoop per-frame allocation measurement; fails hard when run; last run
  2026-11-26, recorded in the gate-lane review entry below; staleness horizon
  one milestone. The lane reports seventeen measurements: import throughput,
  scene load, physics step, 2D physics step (`physics2d_step`), particle eval, animation eval, the
  100,000-GameObject frame loop, `world_scale_100k` (the core scene graph
  holding 100,000 GameObjects, with handle lookup under churn, bulk hierarchy
  reparent, and change-tracking drain), the managed-to-native scripting call
  cost, the PlayerLoop per-frame managed allocation, `player_startup` (windowed
  player launch to first presented frame against the 3000.0 ms charter
  ceiling), `audio_callback_headroom` (audio mixer callback duration for a
  512-frame buffer at 48 kHz with eight active stereo voices against the
  10.667 ms buffer period ceiling), and the five editor and MCP
  measurements `editor_startup` (editor process start to a responsive MCP
  state), `editor_project_open` (opening `samples/roll-a-ball` through the
  editor's MCP surface), `editor_hierarchy_search_100k` (the hierarchy query at
  the 100,000-GameObject scale floor), `editor_property_edit_undo` (the
  property edit and its undo), and `editor_mcp_round_trip` (the MCP round
  trip). Each area owns the measurements of its own subsystems inside the
  shared lane.
- `make verify-unsafe` — owner: editor and engine proof (lane runner); every
  area that registers an unsafe zone owns that zone's stages; trigger: every
  change that touches a registered unsafe zone; evidence: per-stage exit
  statuses, with the sanitizer, Loom, fuzz, and Kani stages declared but not
  yet wired; fails hard when run; last run 2026-11-26, recorded in the gate-lane
  review entry below; staleness horizon one milestone.
- Lane, ownership, and policy records name subsystems or areas, never
  worktrees, branches, or agents.

### Gate-lane review 2026-11-26

| Profile | Last run | Durable source |
|---|---|---|
| `make verify-samples` | 2026-11-26 | the re-approved directional-shadow S8 reference baselines under `samples/roll-a-ball/References/` |
| `make verify-benchmark` | 2026-11-26 | this gate-lane review entry and the benchmark inputs under `samples/roll-a-ball/Benchmarks/agent-authoring/bonus-pickup/` |
| `make verify-device` | 2026-11-26 | this gate-lane review entry; `tools/approved_gpu_reference.bmp` was re-approved on 2026-11-26 against the real shadowed device frame: the staged player's bundle now carries the project's `ProjectSettings/` directory next to `Assets/`, so the runtime resolves the `Standalone` quality level and the device frame carries the directional shadow instead of the shadows-off fallback; the approved file is the exact capture the lane produces today (two consecutive captures differ in 0 of 76 800 pixels; the fresh capture matches the approved file with max_diff=0), and against the prior pre-shadow baseline the shadowed capture differs in 3 834 of 76 800 pixels (4.99 %); the lane also asserts that the presented window frame carries the scene's uGUI canvas and the project shadows |
| `make verify-perf` | 2026-11-26 | this gate-lane review entry |
| `make verify-unsafe` | 2026-11-26 | this gate-lane review entry; the zone-test stage passed and the miri stage reported that the miri toolchain is not installed |
RALPH-FACT: verification_time_budget: 26.24 seconds wall for `make verify` on an Apple Silicon host on 2026-11-25, of which 22.37 seconds is controllable time above the recorded 3.87-second full-gate cost floor; the full .NET formatter took 21.49s of that run. This is the measurement each re-measurement is compared against, not a limit the gate fails on
RALPH-FACT: verification_cost_floor: full gate floor 3.87s (measured via `pytest --collect-only -q` at 0.52s, `cargo check --workspace --all-targets` at 0.07s, and `dotnet build bridge/Unitas.sln --configuration Release --no-restore` with `AnalysisMode=All` in `Directory.Build.props` at 3.28s against `tools/verify.py`, `crates/unitas-core/src/lib.rs`, and `bridge/UnityEngine/Object.cs`), fast-path floor 3.87s (the recorded fast path is the full gate, so it carries the full gate floor, measured via the same `pytest --collect-only -q` at 0.52s, `cargo check --workspace --all-targets` at 0.07s, and `dotnet build bridge/Unitas.sln --configuration Release --no-restore` with `AnalysisMode=All` in `Directory.Build.props` at 3.28s against `tools/verify.py`, `crates/unitas-core/src/lib.rs`, and `bridge/UnityEngine/Object.cs`), and default suite floor 0.67s (measured via `pytest --collect-only -q` at 0.52s and `cargo test --workspace --no-run` at 0.15s against `tools/test_verify.py` and `crates/unitas-core/src/lib.rs`) on Apple Silicon (macOS Darwin 25.5.0 arm64) on 2026-11-25
RALPH-FACT: verification_contention_measurement: on 2026-11-25 the shared host ran other worktrees' gates at load average 39-44, and the same tree measured 9.28s for `cargo test --workspace --lib --tests --bins` alone while the gate's timed wrapper measured 37.89s-115.88s for the same command inside `make verify`; those contended runs sit inside the recorded 124s enforced limit and pass, and the gate fails only past it, after exactly one re-run. Reduction plan owned by editor and engine proof: verify-static-gates and verify-test-gates run concurrently as separate top-level sub-makes in separate cargo target directories, so their wall time is the slower sub-make rather than their sum, and the timing-sensitive `StdioTransportTests` editor shard runs alone before the batched shards so its in-process deadline is not starved by sibling shards; the first item is delivered (verify-static-gates's clippy builds into `target/clippy`, verify-test-gates's compile into the default target directory), and verify-dotnet-gates's bridge shards were balanced on 2026-11-26 by splitting the bridge suite's expensive classes into per-case entries, because verify-dotnet-gates was the lane that overran the enforced limit under shared-host contention, a planned move of the dotnet-format gate out of verify-dotnet-gates into verify-static-gates, recorded on 2026-11-26 because a cold format pass costs about 91 seconds, did not land: the Makefile still runs dotnet-format inside verify-dotnet-gates, launched in the background after dotnet-build so it overlaps runtime-build and dotnet-test-run instead of sitting on that group's critical path; owner: the repository maintainers; review trigger: the next time verify-dotnet-gates overruns the enforced limit under shared-host contention, re-evaluate moving dotnet-format into verify-static-gates
RALPH-FACT: fast_path_time_budget: 18 seconds for the recorded fast path (`make verify`) on an Apple Silicon host on 2026-11-25 (28s on the same host while other runs shared it); standalone make lint measured 27.28s on the same host (`ruff check .` 0.02s, `ruff format --check .` 0.02s, `cargo fmt --all -- --check` 0.87s, `cargo clippy --workspace --all-targets --target-dir target/clippy -- -D warnings` 0.20s, and `dotnet format bridge/Unitas.sln --verify-no-changes --no-restore` 25.03s) — the measurement each re-measurement is compared against, never the cap
RALPH-FACT: verification_enforced_limit: 124 seconds for `make verify`, the recorded 3.87-second full-gate cost floor plus the 2-minute cap of rule 9, declared as VERIFY_BUDGET_SECONDS in the Makefile and enforced by the timed gate runner, which terminates each step's process group at the limit and fails the run when the whole gate exceeds it; the gate fails only past this limit, never because a run came in slower than the recorded measurement, and a contended overrun gets exactly one re-run
RALPH-FACT: fast_path_enforced_limit: 124 seconds, the full gate's enforced limit, because the recorded fast path IS the full gate (`make verify`); the fast path fails only past that limit, never because a run came in slower than the recorded measurement, and a contended overrun gets exactly one re-run
RALPH-FACT: verification_budget_overage: none in the recorded warm runs — full gate controllable time was 22.37s (26.24s wall minus the recorded 3.87s cost floor), fast-path controllable time equaled full-gate controllable time because the fast path runs the full gate, and default suite controllable time was 7.27s (7.94s wall for make test minus the recorded 0.67s floor) on 2026-11-25, every figure under its cap. The shared-host runs that exceeded the old 30s measurement limit now run inside the recorded 124s enforced limit and pass
RALPH-FACT: verification_time_enforcement_mechanism: every timed gate step runs in its own process group (`start_new_session=True`) and is terminated and reaped (`kill_timed_gate_group`) when the step ends or at the enforced limit the Makefile declares, and the gate fails; the outer verify target additionally fails the run when the total wall time exceeds VERIFY_BUDGET_SECONDS; a step whose attempt OVERRUNS its budget is re-run ONCE with the same budget — the runner prints `gate <label> is re-run once with the same <budget>s budget`, and when that re-run fits the budget the gate passes and the runner prints `gate <label> passed on the re-run`; the re-run does NOT apply to a step that fails with a non-zero exit code, whose exit code is returned at once, and it does NOT apply to the outer verify target's whole-lane wall-clock check, which has no re-run; when the re-run overruns the budget again the gate fails with exit 1, and when that re-run was terminated at the limit the runner prints its budget guidance (a gate that overruns its budget is a defect to diagnose, never a budget to raise) to stderr; a re-run reuses the same budget, so a step that overruns twice can consume twice its budget against the whole-lane limit. Deferred work for rule 9's contention-adjusted timing, worktree-level single-full-gate lock, and post-run temporary-directory/process-survivor audit: owner: Verification owner; trigger: when the timed gate runner (`tools/verify.py`) or the outer `make verify` target is next modified; lane: `make verify` (default gate); evidence: `tools/test_verify.py` unit tests and `make verify` output recording contention share alongside elapsed time, rejecting concurrent `make verify` runs in the same worktree, and failing on surviving processes or temp directories; review condition: review on 2026-12-15 or when `tools/verify.py` timed-runner logic changes
RALPH-FACT: gate_lane_review_cadence_and_owner: every policy remediation pass, owned by the repository maintainers (the project maintainers recorded in docs/ralph-workflow-policy/policy-portfolio.toml for the default lane, with the area owners declared on each non-default Makefile target) and evidenced in this file by the dated `### Gate-lane review 2026-11-26` table above listing all five profiles and their durable sources
RALPH-FACT: durable_evidence_rule: a committed file never cites a worktree-local or append-only record (a `tmp/` or `.agent/` file, a run log, or an `*.ndjson` ledger) by line, row count, or excerpt, and a profile's last run is a calendar date plus the durable source that carries it (a CI run id, a committed baseline, or the gate-lane review entry); a run whose only record lives in an ignored path is not recorded as run

`required_verification_profiles` is the single home for the project's named
non-default profiles, including any declared by the testing policy. Record
all five fields for each: command, named owner, trigger or schedule,
fails-hard-when-run, and last-run date with staleness horizon and the
durable source that carries that date (see `durable_evidence_rule`).

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
RALPH-REVIEW: review that every declared profile has run within its staleness horizon, that no check has been demoted out of the default gate to avoid a red result, and that the recorded gate duration still matches reality; evidence: dated gate-lane review listing each profile with its last-run date and the durable source that carries that date; owner: the named person or team recorded in the gate_lane_review_cadence_and_owner fact

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
* Schema version: `<!-- ralph-policy-schema: v7 -->`
