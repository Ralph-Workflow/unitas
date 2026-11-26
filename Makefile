export PATH := $(HOME)/.dotnet:$(PATH)
export DOTNET_ROOT := $(HOME)/.dotnet

.PHONY: verify verify-gates verify-scan-gates verify-suite-gates verify-dotnet-gates verify-device verify-unsafe verify-samples verify-perf test test-steps lint typecheck policy-validate bypass-audit banned-constructs comment-audit secret-scan deps runtime-build dotnet-build dotnet-format dotnet-test dotnet-test-run zone-gate

VERIFY_BUDGET_SECONDS := 30
SUITE_BUDGET_SECONDS := 25
VERIFY_START := $(shell date +%s)
DOTNET_BUILD_COMMAND := dotnet build bridge/Unitas.sln --configuration Release
DOTNET_RESTORE_COMMAND := dotnet restore bridge/Unitas.sln
EDITOR_TEST_BINARY := editor/Unitas.Editor.Tests/bin/Release/net8.0/Unitas.Editor.Tests.dll
EDITOR_TEST_BUNDLE := $(CURDIR)/target/editor-test-bundle
EDITOR_TEST_SHARD_FANOUT := 6
EDITOR_TEST_SHARDS := AuditTests,BuildToolsTests,BuiltinDefaultsTests,ChangeReviewTests,ClockTests,ExecuteMethodToolsTests,HeadlessCliTests,NativeToolBridgeTests,OperationStoreTests,OperationToolsTests,RegistryTests,ScopeTests,SessionTests,UntrustedContentTests \
	HttpTransportTests \
	ScenarioToolsTests \
	StdioTransportTests \
	PlaySessionLifecycleTests \
	PlayWorkerCrashTests \
	PlayLiveCaptureTests,PlayPersonOnlyGrantTests,PlayRuntimeDisposalTests \
	AuthoringToolsTests,ScenarioGlueDiscoveryTests

verify:
	@$(MAKE) --no-print-directory verify-scan-gates & p1=$$!; \
	$(MAKE) --no-print-directory verify-suite-gates & p2=$$!; \
	$(MAKE) --no-print-directory verify-dotnet-gates & p3=$$!; \
	rc=0; wait $$p1 || rc=1; wait $$p2 || rc=1; wait $$p3 || rc=1; \
	test $$rc -eq 0 || { echo "verify failed"; exit 1; }; \
	now=$$(date +%s); elapsed=$$(( now - $(VERIFY_START) )); test $$elapsed -le $(VERIFY_BUDGET_SECONDS) || { echo "verify exceeded the $(VERIFY_BUDGET_SECONDS)s budget"; exit 1; }; echo "all gates passed in $$elapsed s (budget $(VERIFY_BUDGET_SECONDS)s)"

verify-scan-gates:
	@$(MAKE) --no-print-directory policy-validate
	@$(MAKE) --no-print-directory bypass-audit
	@$(MAKE) --no-print-directory zone-gate
	@$(MAKE) --no-print-directory banned-constructs
	@$(MAKE) --no-print-directory comment-audit
	@$(MAKE) --no-print-directory deps
	@$(MAKE) --no-print-directory secret-scan

verify-suite-gates:
	@python3 tools/verify.py timed lint $(VERIFY_BUDGET_SECONDS) ruff check .
	@python3 tools/verify.py timed format $(VERIFY_BUDGET_SECONDS) ruff format --check .
	@python3 tools/verify.py timed fmt-rs $(VERIFY_BUDGET_SECONDS) cargo fmt --all -- --check
	@python3 tools/verify.py timed clippy $(VERIFY_BUDGET_SECONDS) cargo clippy --workspace --all-targets -- -D warnings
	@python3 tools/verify.py timed typecheck $(VERIFY_BUDGET_SECONDS) mypy
	@$(MAKE) --no-print-directory test

verify-dotnet-gates:
	@rc=0; \
	$(MAKE) --no-print-directory runtime-build & pr=$$!; \
	python3 tools/verify.py timed dotnet-restore $(VERIFY_BUDGET_SECONDS) $(DOTNET_RESTORE_COMMAND) || rc=1; \
	wait $$pr || rc=1; \
	python3 tools/verify.py timed dotnet-build $(VERIFY_BUDGET_SECONDS) $(DOTNET_BUILD_COMMAND) --no-restore || rc=1; \
	$(MAKE) --no-print-directory dotnet-format & pf=$$!; \
	$(MAKE) --no-print-directory dotnet-test-run || rc=1; \
	wait $$pf || rc=1; \
	exit $$rc

test:
	@python3 tools/verify.py timed test-suite $(SUITE_BUDGET_SECONDS) $(MAKE) --no-print-directory test-steps

test-steps:
	@python3 tools/verify.py timed test $(VERIFY_BUDGET_SECONDS) pytest -q & p1=$$!; \
	python3 tools/verify.py timed test-rs $(VERIFY_BUDGET_SECONDS) cargo test --workspace & p2=$$!; \
	rc=0; wait $$p1 || rc=1; wait $$p2 || rc=1; exit $$rc

lint:
	@python3 tools/verify.py timed lint $(VERIFY_BUDGET_SECONDS) ruff check .
	@python3 tools/verify.py timed format $(VERIFY_BUDGET_SECONDS) ruff format --check .
	@python3 tools/verify.py timed fmt-rs $(VERIFY_BUDGET_SECONDS) cargo fmt --all -- --check
	@python3 tools/verify.py timed clippy $(VERIFY_BUDGET_SECONDS) cargo clippy --workspace --all-targets -- -D warnings
	@$(MAKE) --no-print-directory dotnet-format

typecheck:
	@python3 tools/verify.py timed typecheck $(VERIFY_BUDGET_SECONDS) mypy
	@python3 tools/verify.py timed typecheck-rs $(VERIFY_BUDGET_SECONDS) cargo check --workspace --all-targets
	@$(MAKE) --no-print-directory dotnet-build

policy-validate:
	@python3 tools/verify.py timed policy $(VERIFY_BUDGET_SECONDS) python3 tools/verify.py policy

bypass-audit:
	@python3 tools/verify.py timed bypass $(VERIFY_BUDGET_SECONDS) python3 tools/verify.py bypass

banned-constructs:
	@python3 tools/verify.py timed banned $(VERIFY_BUDGET_SECONDS) python3 tools/verify.py banned

zone-gate:
	@python3 tools/verify.py timed zones $(VERIFY_BUDGET_SECONDS) python3 tools/verify.py zones

comment-audit:
	@python3 tools/verify.py timed comments $(VERIFY_BUDGET_SECONDS) python3 tools/verify.py comments

secret-scan:
	@python3 tools/verify.py timed secret $(VERIFY_BUDGET_SECONDS) python3 tools/verify.py secret

deps:
	@python3 tools/verify.py timed deps $(VERIFY_BUDGET_SECONDS) python3 tools/verify.py deps

runtime-build:
	@python3 tools/verify.py timed runtime-build $(VERIFY_BUDGET_SECONDS) cargo build -p unitas-runtime -p unitas-editor-abi -p unitas-build

dotnet-build: runtime-build
	@python3 tools/verify.py timed dotnet-build $(VERIFY_BUDGET_SECONDS) $(DOTNET_BUILD_COMMAND)

dotnet-format:
	@python3 tools/verify.py timed dotnet-format $(VERIFY_BUDGET_SECONDS) python3 tools/verify.py dotnet-format

dotnet-test: runtime-build
	@$(MAKE) --no-print-directory dotnet-test-run

dotnet-test-run:
	@rc=0; staging=""; \
	if [ ! -f "$(EDITOR_TEST_BUNDLE)/bundle.manifest" ] || [ -n "$$(find samples/roll-a-ball bridge/Unitas.PlayerHost/bin/Release/net8.0 -type f -newer "$(EDITOR_TEST_BUNDLE)/bundle.manifest" -print -quit 2>/dev/null)" ]; then \
		rm -rf "$(EDITOR_TEST_BUNDLE)"; \
		"$(CURDIR)/target/debug/unitas-player" --sample "$(EDITOR_TEST_BUNDLE)" --frames 1 > /dev/null 2>&1 & staging=$$!; \
	fi; \
	python3 tools/verify.py timed dotnet-test $(VERIFY_BUDGET_SECONDS) env "UNITAS_RUNTIME_LIBRARY=$(CURDIR)/target/debug/libunitas_runtime.dylib" bridge/Unitas.Bridge.Tests/bin/Release/net8.0/Unitas.Bridge.Tests & p1=$$!; \
	python3 tools/verify.py timed dotnet-test-playerhost $(VERIFY_BUDGET_SECONDS) env "UNITAS_RUNTIME_LIBRARY=$(CURDIR)/target/debug/libunitas_runtime.dylib" bridge/Unitas.PlayerHost.Tests/bin/Release/net8.0/Unitas.PlayerHost.Tests & p3=$$!; \
	if [ -n "$$staging" ]; then wait $$staging || rc=1; fi; \
	python3 tools/verify.py timed dotnet-test-editor $(VERIFY_BUDGET_SECONDS) $(MAKE) --no-print-directory editor-test-shards & p2=$$!; \
	wait $$p1 || rc=1; wait $$p2 || rc=1; wait $$p3 || rc=1; exit $$rc

editor-test-shards:
	@rc=0; pids=""; running=0; \
	for shard in $(EDITOR_TEST_SHARDS); do \
		UNITAS_PLAYER_BUNDLE_DIR="$(EDITOR_TEST_BUNDLE)" dotnet $(EDITOR_TEST_BINARY) $$shard & pids="$$pids $$!"; \
		running=$$((running+1)); \
		if [ $$running -ge $(EDITOR_TEST_SHARD_FANOUT) ]; then \
			for pid in $$pids; do wait $$pid || rc=1; done; pids=""; running=0; \
		fi; \
	done; \
	for pid in $$pids; do wait $$pid || rc=1; done; \
	exit $$rc

verify-unsafe:
	@echo "owner: editor and engine proof"
	@echo "trigger: every change that touches a registered unsafe zone"
	@echo "lane: make verify-unsafe (non-default)"
	@echo "evidence: per-stage exit statuses below; sanitizer, Loom, fuzz, and Kani stages are declared but not yet wired"
	@echo "review: at every change that touches a zone, by an agent other than its author"
	@if cargo +nightly miri --version >/dev/null 2>&1; then \
		echo "stage miri: cargo +nightly miri test -p unitas-editor-abi"; \
		cargo +nightly miri test -p unitas-editor-abi; \
	else \
		echo "stage miri: not run (miri toolchain not installed)"; \
	fi
	@echo "stage zone tests: cargo test -p unitas-editor-abi"
	@cargo test -p unitas-editor-abi

verify-device:
	@echo "owner: presentation and player"
	@echo "trigger: every milestone touching presentation"
	@echo "lane: make verify-device"
	@echo "evidence: lane exit status and captured GPU/audio/input artifacts"
	@echo "review: when the player gains a new platform"
	@python3 tools/verify_device.py

verify-perf:
	@echo "owner: content and simulation"
	@echo "trigger: every milestone touching import, scene load, physics, or animation"
	@echo "lane: make verify-perf"
	@echo "evidence: per-measurement OK/FAIL lines with value_ms against ceiling_ms"
	@echo "review: when a perf measurement is added or a ceiling moves"
	@python3 tools/verify_perf.py
verify-samples:
	@echo "owner: editor and engine proof"
	@echo "trigger: changes to bridge script compilation/play pipeline or samples/"
	@echo "lane: make verify-samples"
	@echo "evidence: scenario report JSONs, packaged-player build-report assertions, and lane exit status"
	@echo "review: when a second sample game gains real scripts"
	@python3 tools/verify_samples.py
