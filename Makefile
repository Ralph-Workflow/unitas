.PHONY: verify test lint typecheck policy-validate bypass-audit banned-constructs comment-audit secret-scan deps

VERIFY_BUDGET_SECONDS := 30
VERIFY_START := $(shell date +%s)

verify: policy-validate bypass-audit banned-constructs comment-audit deps secret-scan lint typecheck test
	@now=$$(date +%s); elapsed=$$(( now - $(VERIFY_START) )); test $$elapsed -le $(VERIFY_BUDGET_SECONDS) || { echo "verify exceeded the $(VERIFY_BUDGET_SECONDS)s budget"; exit 1; }; echo "all gates passed in $$elapsed s (budget $(VERIFY_BUDGET_SECONDS)s)"

test:
	@python3 tools/verify.py timed test $(VERIFY_BUDGET_SECONDS) pytest -q

lint:
	@python3 tools/verify.py timed lint $(VERIFY_BUDGET_SECONDS) ruff check .
	@python3 tools/verify.py timed format $(VERIFY_BUDGET_SECONDS) ruff format --check .

typecheck:
	@python3 tools/verify.py timed typecheck $(VERIFY_BUDGET_SECONDS) mypy

policy-validate:
	@python3 tools/verify.py timed policy $(VERIFY_BUDGET_SECONDS) python3 tools/verify.py policy

bypass-audit:
	@python3 tools/verify.py timed bypass $(VERIFY_BUDGET_SECONDS) python3 tools/verify.py bypass

banned-constructs:
	@python3 tools/verify.py timed banned $(VERIFY_BUDGET_SECONDS) python3 tools/verify.py banned

comment-audit:
	@python3 tools/verify.py timed comments $(VERIFY_BUDGET_SECONDS) python3 tools/verify.py comments

secret-scan:
	@python3 tools/verify.py timed secret $(VERIFY_BUDGET_SECONDS) python3 tools/verify.py secret

deps:
	@python3 tools/verify.py timed deps $(VERIFY_BUDGET_SECONDS) python3 tools/verify.py deps
