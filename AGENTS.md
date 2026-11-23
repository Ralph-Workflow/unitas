# AGENTS.md

This repository holds the unitas project-policy corpus and the verification
tooling that enforces it.

## Before you start

Read `docs/ralph-workflow-policy/execution-kernel.md`, then the policy files the
changed surface triggers. The corpus names its own triggers; load nothing else.

## Verification

`make verify` is the authoritative entry point. It runs the policy self-check, the
bypass audit, the ban-catalog gate, the comment audit, the dependency gate, the
secret scan, the lint gate, the type check, and the test suite, and it fails when
the run exceeds the budget the Makefile declares.

Run the declared `RALPH-COMMAND:` gates and report the real exit status. A gate
that cannot run is a red gate.

## Product status

The Rust crates and the C# UnityEngine bridge are specified but not implemented.
Their gates are dated deferrals in the corpus; resolve a deferral when its review
trigger fires.

<!-- ralph-workflow-policy:begin v3 -->
Quality policies live under `docs/ralph-workflow-policy/`. Every agent role loads
them in this order: execution-kernel.md first, then its role slice, then any domain
policy the changed surface triggers. Load only the effective bounded portfolio: the
kernel, the selected profiles, the local tightening, and the valid exceptions.

The authoritative verification entry point is the gate the corpus declares; run
every declared `RALPH-COMMAND:` gate and report the real exit status. A check that
cannot run is a red gate.

Search nearest coverage before adding a control; record REMOVE, MERGE, REPLACE, or
KEEP; use the cheapest sufficient layer and keep the default-gate budget neutral.
Give non-default work an owner, trigger, lane, evidence, and review condition.
Telemetry, provenance, private-state inspection, and agent narration are not
correctness proof.

Resolve every `RALPH-PENDING:` line whose review trigger is met; leaving a
resolvable item is a violation. Dead code is prohibited; remove it — rewriting
later is better than retaining it. These are living documents: keep the facts
current without amending policy intent.
<!-- ralph-workflow-policy:end -->
