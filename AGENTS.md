# AGENTS.md

unitas is a commercial-quality game engine, editor, and player that opens and runs
Unity 2022.3 LTS projects unchanged. This file holds the rules every agent working in
this repository follows. The product itself is described in
`docs/product-specification/`. Read it as the product's design documents, as a whole,
not as a checklist.

## Before you start

Read `docs/ralph-workflow-policy/execution-kernel.md`, then the policy files the
changed surface triggers. The corpus names its own triggers, so load no other policy
files. Product requirements come from the specification, not from the policy.

## Verification

`make verify` is the authoritative entry point. It runs the policy self-check, the
bypass audit, the unsafe-zone gate, the ban-catalog gate, the comment audit,
the dependency gate, the secret scan, the lint and format checks, the type
checks, and the Python, Rust, and C# test suites, and it fails when the run
exceeds the budget the Makefile declares.

Run the declared `RALPH-COMMAND:` gates and report the real exit status. A gate
that cannot run is a red gate.

## Product status

The real state of the product is the code on `main` and the results of its gates and
lanes. Read the code, not a summary of it.

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
