<!-- ralph-workflow-policy:begin v3 -->
This project participates in Ralph Workflow's project-policy-readiness
preflight. Before ordinary planning or development begins, the canonical
quality-policy files under `docs/ralph-workflow-policy/` must exist, be customized for
this project's languages, frameworks, and commands, and pass the
deterministic validator. All agent roles MUST load policies in this order:
execution-kernel.md first, then their role slice, then any triggered domain
policy before changing the project. Use the effective bounded policy
portfolio: load its invariant kernel, selected profiles, local tightening,
and valid exceptions instead of applying every conditional domain.

The remediation agent MUST, in order:

1. Inspect the project's actual languages, frameworks, package managers,
   and existing docs; create and maintain the canonical policy files under
   `docs/ralph-workflow-policy/`, replacing every starter placeholder with verified
   project evidence.
2. Migrate existing policy-like content into the matching canonical file,
   leaving a `ralph-workflow-policy:migrated ->` marker at the old
   location so the validator can clear the RWP-MIGRATE-UNRECONCILED
   finding.
3. Configure executable gates for testing, type checking, linting,
   dependency, and verification; document each gate as a RALPH-COMMAND
   line (or RALPH-INAPPLICABLE with reason).
4. Update CLAUDE.md (if present) to point Claude-compatible agents at
   this AGENTS.md, then run every declared verification gate and report
   the outcome.

Dead code is prohibited; remove it — rewriting later is better than
retaining it. The remediation agent MUST NOT mark any policy complete
while any RALPH-FACT placeholder token, RALPH-COMMAND without a real
value, or unresolved RALPH-LANG coverage remains. The readiness preflight
is byte-exact deterministic; near-miss prose, extra whitespace, or case
changes do not satisfy any requirement.
<!-- ralph-workflow-policy:end -->
