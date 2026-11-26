from __future__ import annotations

import ast
import contextlib
import importlib.util
import io
import json
import re
import signal
import subprocess
import sys
import time
from itertools import product
from typing import TYPE_CHECKING
from unittest import mock

from tools import verify, verify_perf, verify_samples

if TYPE_CHECKING:
    from collections.abc import Callable
    from pathlib import Path

SIGNALLED_EXIT_CODE = 143


def test_repository_gates_report_no_findings() -> None:
    for name in sorted(verify.GATES):
        findings = verify.GATES[name]()
        assert findings == [], f"{name} reported findings: {findings}"


def test_pytest_command_matches_the_xdist_availability() -> None:
    command = verify.pytest_command()
    assert command[1:4] == ["-m", "pytest", "-q"]
    available = importlib.util.find_spec("xdist") is not None
    assert ("-n" in command) == available
    if available:
        assert command[command.index("-n") + 1] == str(verify.PYTEST_WORKERS)


def test_placeholder_tokens_are_detected() -> None:
    corpus = {"testing-policy.md": "RALPH-FACT: test_command: REPLACE-ME"}
    findings = verify.check_placeholders(corpus)
    assert findings == ["testing-policy.md: placeholder token 'REPLACE-ME' remains"]


def test_duplicate_fact_keys_are_detected() -> None:
    corpus = {
        "testing-policy.md": "RALPH-FACT: test_command: a\nRALPH-FACT: test_command: b\n",
    }
    findings = verify.check_fact_keys(corpus)
    assert findings == ["testing-policy.md: RALPH-FACT key 'test_command' occurs 2 times"]


def test_unapproved_command_tool_is_detected() -> None:
    corpus = {"testing-policy.md": "RALPH-COMMAND: ./run-tests.sh\n"}
    findings = verify.check_commands(corpus)
    assert findings == ["testing-policy.md:1: RALPH-COMMAND tool './run-tests.sh' is not approved"]


def test_unknown_guard_enforcer_is_detected() -> None:
    corpus = {"testing-policy.md": _guard_table("deterministic: make nowhere")}
    findings = verify.check_guard_enforcement(corpus, ["make verify"])
    assert findings == ["testing-policy.md: guard row 1 references unknown enforcer 'make nowhere'"]


def test_advisory_guard_enforcement_is_accepted() -> None:
    corpus = {
        "testing-policy.md": _guard_table("advisory: no gate exists; review: when a gate lands")
    }
    assert verify.check_guard_enforcement(corpus, []) == []


def test_budget_consistency_reports_a_limit_the_makefile_does_not_enforce(
    tmp_path: Path,
) -> None:
    policy_dir = tmp_path / "policy"
    policy_dir.mkdir()
    (policy_dir / "verification-policy.md").write_text(
        "RALPH-FACT: authoritative_verify_command: make verify\n"
        "RALPH-FACT: verification_enforced_limit: 124 seconds\n"
        "RALPH-FACT: fast_path_command: make verify\n"
        "RALPH-FACT: fast_path_enforced_limit: 30 seconds\n",
        encoding="utf-8",
    )
    (policy_dir / "testing-policy.md").write_text(
        "RALPH-FACT: test_command: make test\nRALPH-FACT: suite_enforced_limit: 61 seconds\n",
        encoding="utf-8",
    )
    makefile = tmp_path / "Makefile"
    makefile.write_text(
        "VERIFY_BUDGET_SECONDS := 30\nSUITE_BUDGET_SECONDS := 61\n", encoding="utf-8"
    )
    with (
        mock.patch.object(verify, "MAKEFILE", makefile),
        mock.patch.object(verify, "POLICY_DIR", policy_dir),
    ):
        findings = verify.check_budget_consistency()
    assert findings == [
        "the Makefile enforces 30s for `make verify` but verification-policy.md "
        "records verification_enforced_limit 124s"
    ]


def test_repository_has_no_bypass_markers() -> None:
    assert verify.check_bypass() == []


def test_bypass_marker_in_source_is_detected(tmp_path: Path) -> None:
    source = tmp_path / "sample.py"
    source.write_text("value = 1  # noqa: E501\n", encoding="utf-8")
    findings = verify.bypass_findings_in(source)
    assert any("noqa" in finding for finding in findings)


def _write_clean_pyproject(root: Path) -> Path:
    path = root / "pyproject.toml"
    path.write_text(
        "# Governed by docs/ralph-workflow-policy/linting-policy.md\n"
        "[tool.ruff.lint]\n"
        'select = ["ALL"]\n'
        "ignore = [\n"
        '    "COM812", # reason: conflicts with ruff format\n'
        "]\n"
        "[tool.mypy]\n"
        "strict = true\n",
        encoding="utf-8",
    )
    return path


def _write_clean_rust_workspace(root: Path) -> Path:
    manifest = root / "Cargo.toml"
    manifest.write_text(
        "# Governed by docs/ralph-workflow-policy/linting-policy.md\n"
        "[workspace]\n"
        'members = ["crates/alpha"]\n'
        "[workspace.lints.rust]\n"
        'warnings = "deny"\n'
        'unsafe_code = "forbid"\n'
        'unnecessary_transmutes = "deny"\n'
        'static_mut_refs = "deny"\n'
        "[workspace.lints.clippy]\n"
        'all = { level = "deny", priority = -1 }\n'
        'pedantic = { level = "deny", priority = -1 }\n'
        'unwrap_used = "deny"\n'
        'expect_used = "deny"\n'
        'panic = "deny"\n'
        'todo = "deny"\n'
        'unimplemented = "deny"\n'
        'dbg_macro = "deny"\n'
        "# clean-code-policy.md bans doc comments\n"
        'missing_errors_doc = "allow"\n',
        encoding="utf-8",
    )
    member = root / "crates" / "alpha"
    member.mkdir(parents=True, exist_ok=True)
    (member / "Cargo.toml").write_text(
        "[package]\n"
        'name = "alpha"\n'
        'version = "0.1.0"\n'
        'edition = "2021"\n'
        "[lints]\n"
        "workspace = true\n",
        encoding="utf-8",
    )
    return manifest


ZONE_REGISTRY_ENTRY = (
    "[[zones]]\n"
    'path = "crates/zone-crate"\n'
    'language = "rust"\n'
    'purpose = "test zone"\n'
    'invariants = ["test invariant"]\n'
    "unsafe_block_count = 0\n"
)


def _write_zone_registry(root: Path, entry: str = ZONE_REGISTRY_ENTRY) -> Path:
    registry = root / "tools" / "unsafe-zones.toml"
    registry.parent.mkdir(parents=True, exist_ok=True)
    registry.write_text(entry, encoding="utf-8")
    return registry


def _write_zone_member(root: Path) -> Path:
    member = root / "crates" / "zone-crate"
    member.mkdir(parents=True, exist_ok=True)
    (member / "Cargo.toml").write_text(
        "[package]\n"
        'name = "zone-crate"\n'
        'version = "0.1.0"\n'
        'edition = "2024"\n'
        "[lints.rust]\n"
        'warnings = "deny"\n'
        'unsafe_code = "allow"\n'
        'unnecessary_transmutes = "deny"\n'
        'static_mut_refs = "deny"\n'
        'unsafe_op_in_unsafe_fn = "deny"\n'
        'improper_ctypes = "deny"\n'
        'improper_ctypes_definitions = "deny"\n'
        "[lints.clippy]\n"
        'all = { level = "deny", priority = -1 }\n'
        'pedantic = { level = "deny", priority = -1 }\n'
        'unwrap_used = "deny"\n'
        'expect_used = "deny"\n'
        'panic = "deny"\n'
        'todo = "deny"\n'
        'unimplemented = "deny"\n'
        'dbg_macro = "deny"\n'
        'missing_errors_doc = "allow"\n'
        'undocumented_unsafe_blocks = "deny"\n'
        'missing_safety_doc = "deny"\n'
        'multiple_unsafe_ops_per_block = "deny"\n'
        'ptr_as_ptr = "deny"\n'
        'cast_ptr_alignment = "deny"\n'
        'as_ptr_cast_mut = "deny"\n'
        'borrow_as_ptr = "deny"\n'
        'transmute_ptr_to_ptr = "deny"\n',
        encoding="utf-8",
    )
    return member


def test_bypass_audit_accepts_registered_zone_lint_table(tmp_path: Path) -> None:
    _write_clean_pyproject(tmp_path)
    _write_clean_rust_workspace(tmp_path)
    member = _write_zone_member(tmp_path)
    findings = verify.check_bypass(tmp_path)
    assert any("member crate must set [lints] workspace = true" in f for f in findings)
    _write_zone_registry(tmp_path)
    assert verify.check_bypass(tmp_path) == []
    manifest = member / "Cargo.toml"
    manifest.write_text(
        manifest.read_text(encoding="utf-8").replace(
            'unsafe_code = "allow"', 'unsafe_code = "deny"'
        ),
        encoding="utf-8",
    )
    assert verify.check_bypass(tmp_path) == []


def test_zone_gate_accepts_compliant_zone(tmp_path: Path) -> None:
    _write_clean_rust_workspace(tmp_path)
    _write_zone_member(tmp_path)
    _write_zone_registry(tmp_path)
    assert verify.check_zones(tmp_path) == []


def test_zone_gate_flags_lint_table_mismatch(tmp_path: Path) -> None:
    _write_clean_rust_workspace(tmp_path)
    member = _write_zone_member(tmp_path)
    _write_zone_registry(tmp_path)
    manifest = member / "Cargo.toml"
    manifest.write_text(
        manifest.read_text(encoding="utf-8").replace(
            'unsafe_code = "allow"', 'unsafe_code = "forbid"'
        ),
        encoding="utf-8",
    )
    findings = verify.check_zones(tmp_path)
    assert any("zone rust lint 'unsafe_code' must be 'allow'" in f for f in findings)
    manifest.write_text(
        manifest.read_text(encoding="utf-8").replace(
            'undocumented_unsafe_blocks = "deny"', 'undocumented_unsafe_blocks = "allow"'
        ),
        encoding="utf-8",
    )
    findings = verify.check_zones(tmp_path)
    assert any(
        "zone clippy lint 'undocumented_unsafe_blocks' must be 'deny'" in f for f in findings
    )


def test_zone_gate_flags_unsafe_outside_zone(tmp_path: Path) -> None:
    _write_clean_rust_workspace(tmp_path)
    _write_zone_member(tmp_path)
    _write_zone_registry(tmp_path)
    source = tmp_path / "crates" / "alpha" / "lib.rs"
    source.write_text(
        '#[unsafe(export_name = "alpha_free")]\n'
        'pub extern "C" fn alpha_free() {}\n'
        "pub fn leak() { unsafe { } }\n",
        encoding="utf-8",
    )
    findings = verify.check_zones(tmp_path)
    outside = [f for f in findings if "unsafe construct outside a registered unsafe zone" in f]
    assert outside == [
        f"{source.relative_to(tmp_path)}:3: unsafe construct outside a registered unsafe zone"
    ]


def test_zone_gate_flags_unsafe_block_count_mismatch(tmp_path: Path) -> None:
    _write_clean_rust_workspace(tmp_path)
    member = _write_zone_member(tmp_path)
    _write_zone_registry(
        tmp_path, ZONE_REGISTRY_ENTRY.replace("unsafe_block_count = 0", "unsafe_block_count = 1")
    )
    source = member / "lib.rs"
    source.write_text("pub fn zero() -> i32 { 0 }\n", encoding="utf-8")
    findings = verify.check_zones(tmp_path)
    assert any("registered unsafe block count 1 differs from actual 0" in f for f in findings)
    _write_zone_registry(tmp_path)
    source.write_text("pub fn leak() { unsafe { } }\n", encoding="utf-8")
    findings = verify.check_zones(tmp_path)
    assert any("registered unsafe block count 0 differs from actual 1" in f for f in findings)


def test_zone_gate_flags_link_time_symbol_alias(tmp_path: Path) -> None:
    _write_clean_rust_workspace(tmp_path)
    build = tmp_path / "crates" / "alpha" / "build.rs"
    build.write_text(
        'fn main() { println!("cargo:rustc-cdylib-link-arg=-Wl,-alias,_mangled,_stable"); }\n',
        encoding="utf-8",
    )
    findings = verify.check_zones(tmp_path)
    assert any("link-time symbol alias is banned" in f for f in findings)


def _write_clean_csharp_configs(root: Path) -> None:
    (root / "Directory.Build.props").write_text(
        "<!-- Governed by docs/ralph-workflow-policy/linting-policy.md -->\n"
        "<Project>\n"
        "  <PropertyGroup>\n"
        "    <Nullable>enable</Nullable>\n"
        "    <TreatWarningsAsErrors>true</TreatWarningsAsErrors>\n"
        "    <AnalysisMode>All</AnalysisMode>\n"
        "    <AnalysisLevel>latest-all</AnalysisLevel>\n"
        "    <EnforceCodeStyleInBuild>true</EnforceCodeStyleInBuild>\n"
        "  </PropertyGroup>\n"
        "</Project>\n",
        encoding="utf-8",
    )
    proj_dir = root / "bridge" / "Sample"
    proj_dir.mkdir(parents=True, exist_ok=True)
    (proj_dir / "Sample.csproj").write_text(
        '<Project Sdk="Microsoft.NET.Sdk">\n'
        "  <PropertyGroup>\n"
        "    <TargetFramework>net8.0</TargetFramework>\n"
        "  </PropertyGroup>\n"
        "</Project>\n",
        encoding="utf-8",
    )
    (root / ".editorconfig").write_text(
        "root = true\n"
        "[*.cs]\n"
        "# Unity API signature compatibility\n"
        "dotnet_diagnostic.CA1002.severity = none\n",
        encoding="utf-8",
    )


def test_bypass_audit_flags_ruff_select_without_all_and_ignore_without_reason(
    tmp_path: Path,
) -> None:
    _write_clean_pyproject(tmp_path)
    assert verify.check_bypass(tmp_path) == []
    (tmp_path / "pyproject.toml").write_text(
        '[tool.ruff.lint]\nselect = ["E", "F"]\n',
        encoding="utf-8",
    )
    findings = verify.check_bypass(tmp_path)
    assert any("select must contain 'ALL'" in finding for finding in findings)
    (tmp_path / "pyproject.toml").write_text(
        '[tool.ruff.lint]\nselect = ["ALL"]\nignore = [\n    "COM812",\n]\n',
        encoding="utf-8",
    )
    findings = verify.check_bypass(tmp_path)
    assert any("COM812" in finding and "lacks a reason comment" in finding for finding in findings)


def test_bypass_audit_requires_reasons_for_single_quoted_ruff_ignores(tmp_path: Path) -> None:
    cases = (
        ("ignore = ['COM812']", 3),
        ("ignore = [\n    'COM812',\n]", 4),
        ("ignore = [\n    'COM812', 'D203',\n]", 4),
        ("ignore = ['COM812',\n]", 3),
        ("ignore = [\n    'COM812']", 4),
    )
    for index, (ignore, line) in enumerate(cases):
        root = tmp_path / str(index)
        root.mkdir()
        path = root / "pyproject.toml"
        text = f"[tool.ruff.lint]\nselect = ['ALL']\n{ignore}\n"
        lines = text.splitlines(keepends=True)
        lines[line - 1] = lines[line - 1].rstrip() + " # formatter conflict\n"
        path.write_text("".join(lines), encoding="utf-8")
        assert verify.check_bypass(root) == [], repr(ignore)
        path.write_text(text, encoding="utf-8")
        rules = ("COM812", "D203") if "D203" in ignore else ("COM812",)
        assert verify.check_bypass(root) == [
            f"pyproject.toml:{line}: ruff ignore entry {rule!r} lacks a reason comment"
            for rule in rules
        ], repr(ignore)


def test_bypass_audit_requires_reasons_for_quoted_ruff_ignore_keys(tmp_path: Path) -> None:
    path = tmp_path / "pyproject.toml"
    for key in ('"ignore"', "'ignore'"):
        text = f"[tool.ruff.lint]\nselect = ['ALL']\n{key} = ['COM812']"
        path.write_text(f"{text} # formatter conflict\n", encoding="utf-8")
        assert verify.check_bypass(tmp_path) == [], key
        path.write_text(f"{text}\n", encoding="utf-8")
        assert verify.check_bypass(tmp_path) == [
            "pyproject.toml:3: ruff ignore entry 'COM812' lacks a reason comment"
        ], key


def test_bypass_audit_requires_reasons_for_escaped_ruff_ignore_keys(tmp_path: Path) -> None:
    path = tmp_path / "pyproject.toml"
    text = '[tool.ruff.lint]\nselect = ["ALL"]\n"\\u0069gnore" = ["COM812"]'
    path.write_text(f"{text} # formatter conflict\n", encoding="utf-8")
    assert verify.check_bypass(tmp_path) == []
    path.write_text(f"{text}\n", encoding="utf-8")
    assert verify.check_bypass(tmp_path) == [
        "pyproject.toml:3: ruff ignore entry 'COM812' lacks a reason comment"
    ]


def test_bypass_audit_requires_reasons_for_escaped_ruff_ignore_entries(tmp_path: Path) -> None:
    path = tmp_path / "pyproject.toml"
    path.write_text(
        '[tool.ruff.lint]\nselect = ["ALL"]\nignore = [\n    "\\u0043OM812",\n]\n',
        encoding="utf-8",
    )
    assert verify.check_bypass(tmp_path) == [
        "pyproject.toml:4: ruff ignore entry 'COM812' lacks a reason comment"
    ]
    path.write_text(
        '[tool.ruff.lint]\nselect = ["ALL"]\nignore = [\n'
        '    "\\u0043OM812", # reason: formatter conflict\n]\n',
        encoding="utf-8",
    )
    assert verify.check_bypass(tmp_path) == []


def test_bypass_audit_requires_reasons_for_ruff_ignore_entries_with_delimiters(
    tmp_path: Path,
) -> None:
    path = tmp_path / "pyproject.toml"
    for entries in ('["A#B", "COM812"]', '["A]B", "COM812"]'):
        path.write_text(
            f'[tool.ruff.lint]\nselect = ["ALL"]\nignore = {entries}\n',
            encoding="utf-8",
        )
        findings = verify.check_bypass(tmp_path)
        assert any(
            "ruff ignore entry 'COM812' lacks a reason comment" in finding for finding in findings
        ), entries


def test_bypass_audit_requires_reasons_for_escaped_rust_lint_keys(tmp_path: Path) -> None:
    _write_clean_pyproject(tmp_path)
    manifest = _write_clean_rust_workspace(tmp_path)
    text = manifest.read_text(encoding="utf-8")
    manifest.write_text(
        text.replace(
            '# clean-code-policy.md bans doc comments\nmissing_errors_doc = "allow"\n',
            '"missing_\\u0064ocs" = "allow"\n',
        ),
        encoding="utf-8",
    )
    assert verify.check_bypass(tmp_path) == [
        "Cargo.toml:18: lint 'missing_docs' set to 'allow' lacks a reason comment"
    ]
    manifest.write_text(
        text.replace(
            'missing_errors_doc = "allow"',
            '"missing_\\u0064ocs" = "allow" # clean-code-policy.md bans doc comments',
        ),
        encoding="utf-8",
    )
    assert verify.check_bypass(tmp_path) == []


def test_bypass_audit_flags_escaped_mypy_weakening_keys(tmp_path: Path) -> None:
    path = tmp_path / "pyproject.toml"
    path.write_text(
        '[tool.ruff.lint]\nselect = ["ALL"]\n[tool.mypy]\nstrict = true\n',
        encoding="utf-8",
    )
    assert verify.check_bypass(tmp_path) == []
    path.write_text(
        '[tool.ruff.lint]\nselect = ["ALL"]\n[tool.mypy]\nstrict = true\n'
        '"disable_\\u0065rror_code" = ["all"]\n',
        encoding="utf-8",
    )
    assert verify.check_bypass(tmp_path) == [
        "pyproject.toml:5: mypy-disable-error-code configuration bypass"
    ]


def test_bypass_audit_flags_quoted_and_escaped_mypy_values(tmp_path: Path) -> None:
    path = tmp_path / "pyproject.toml"
    for value in ("'silent'", '"sil\\u0065nt"'):
        path.write_text(
            '[tool.ruff.lint]\nselect = ["ALL"]\n[tool.mypy]\nstrict = true\n'
            f"follow_imports = {value}\n",
            encoding="utf-8",
        )
        assert verify.check_bypass(tmp_path) == [
            "pyproject.toml:5: mypy-follow-imports-silent configuration bypass"
        ], value
    path.write_text(
        '[tool.ruff.lint]\nselect = ["ALL"]\n[tool.mypy]\nstrict = true\n'
        'follow_imports = "normal"\n',
        encoding="utf-8",
    )
    assert verify.check_bypass(tmp_path) == []


def test_bypass_audit_flags_unopted_member_crate_and_missing_warnings_deny(
    tmp_path: Path,
) -> None:
    _write_clean_pyproject(tmp_path)
    _write_clean_rust_workspace(tmp_path)
    assert verify.check_bypass(tmp_path) == []
    (tmp_path / "crates" / "alpha" / "Cargo.toml").write_text(
        '[package]\nname = "alpha"\nversion = "0.1.0"\nedition = "2021"\n',
        encoding="utf-8",
    )
    findings = verify.check_bypass(tmp_path)
    assert any("member crate must set [lints] workspace = true" in finding for finding in findings)
    manifest = _write_clean_rust_workspace(tmp_path)
    text = manifest.read_text(encoding="utf-8")
    manifest.write_text(text.replace('warnings = "deny"\n', ""), encoding="utf-8")
    findings = verify.check_bypass(tmp_path)
    assert any("warnings = 'deny'" in finding for finding in findings)


def test_bypass_audit_accepts_registered_unsafe_zone_with_own_lints(tmp_path: Path) -> None:
    _write_clean_pyproject(tmp_path)
    _write_clean_rust_workspace(tmp_path)
    zone_manifest = tmp_path / "crates" / "alpha" / "Cargo.toml"
    zone_manifest.write_text(
        "[package]\n"
        'name = "alpha"\n'
        'version = "0.1.0"\n'
        'edition = "2024"\n'
        "[lints.rust]\n"
        'warnings = "deny"\n'
        'unnecessary_transmutes = "deny"\n'
        'static_mut_refs = "deny"\n'
        'unsafe_op_in_unsafe_fn = "deny"\n'
        'improper_ctypes = "deny"\n'
        'improper_ctypes_definitions = "deny"\n'
        "[lints.clippy]\n"
        'all = { level = "deny", priority = -1 }\n'
        'pedantic = { level = "deny", priority = -1 }\n'
        'unwrap_used = "deny"\n'
        'expect_used = "deny"\n'
        'panic = "deny"\n'
        'todo = "deny"\n'
        'unimplemented = "deny"\n'
        'dbg_macro = "deny"\n'
        'undocumented_unsafe_blocks = "deny"\n'
        'missing_safety_doc = "deny"\n'
        'multiple_unsafe_ops_per_block = "deny"\n'
        'ptr_as_ptr = "deny"\n'
        'cast_ptr_alignment = "deny"\n'
        'as_ptr_cast_mut = "deny"\n'
        'borrow_as_ptr = "deny"\n'
        'transmute_ptr_to_ptr = "deny"\n',
        encoding="utf-8",
    )
    findings = verify.check_bypass(tmp_path)
    assert any("member crate must set [lints] workspace = true" in finding for finding in findings)
    (tmp_path / "tools").mkdir(exist_ok=True)
    (tmp_path / "tools" / "unsafe-zones.toml").write_text(
        '[[zone]]\npath = "crates/alpha"\nlanguage = "rust"\npurpose = "ffi"\n'
        "invariants = []\nunsafe_blocks = 0\n",
        encoding="utf-8",
    )
    assert verify.check_bypass(tmp_path) == []
    zone_manifest.write_text(
        zone_manifest.read_text(encoding="utf-8").replace('unsafe_op_in_unsafe_fn = "deny"\n', ""),
        encoding="utf-8",
    )
    findings = verify.check_bypass(tmp_path)
    assert any("unsafe_op_in_unsafe_fn" in finding for finding in findings)


def test_bypass_audit_flags_missing_member_manifest(tmp_path: Path) -> None:
    _write_clean_pyproject(tmp_path)
    _write_clean_rust_workspace(tmp_path)
    assert verify.check_bypass(tmp_path) == []
    member = tmp_path / "crates" / "alpha" / "Cargo.toml"
    member.unlink()
    assert verify.check_bypass(tmp_path) == [
        f"{member.relative_to(tmp_path)}: workspace member Cargo.toml is missing"
    ]


def test_bypass_audit_checks_lint_inheritance_for_root_package(tmp_path: Path) -> None:
    _write_clean_pyproject(tmp_path)
    manifest = _write_clean_rust_workspace(tmp_path)
    text = manifest.read_text(encoding="utf-8") + '\n[package]\nname = "root"\nversion = "0.1.0"\n'
    manifest.write_text(text + "\n[lints]\nworkspace = true\n", encoding="utf-8")
    assert verify.check_bypass(tmp_path) == []
    manifest.write_text(text, encoding="utf-8")
    assert verify.check_bypass(tmp_path) == [
        "Cargo.toml: member crate must set [lints] workspace = true"
    ]


def test_bypass_audit_requires_explicit_rust_catalog_denies(tmp_path: Path) -> None:
    cases = (
        ("rust", "unnecessary_transmutes"),
        ("rust", "static_mut_refs"),
        ("clippy", "unwrap_used"),
        ("clippy", "expect_used"),
        ("clippy", "panic"),
        ("clippy", "todo"),
        ("clippy", "unimplemented"),
        ("clippy", "dbg_macro"),
    )
    for table, lint in cases:
        root = tmp_path / lint
        root.mkdir()
        _write_clean_pyproject(root)
        manifest = _write_clean_rust_workspace(root)
        assert verify.check_bypass(root) == [], lint
        text = manifest.read_text(encoding="utf-8")
        manifest.write_text(text.replace(f'{lint} = "deny"\n', ""), encoding="utf-8")
        assert verify.check_bypass(root) == [
            f"Cargo.toml: [workspace.lints.{table}] must set {lint} = 'deny'"
        ], lint


def test_bypass_audit_requires_clippy_group_priority(tmp_path: Path) -> None:
    groups = ("all", "pedantic")
    entries = ('"deny"', '{ level = "deny", priority = 0 }', '{ level = "deny", priority = -1.0 }')
    for index, (group, entry) in enumerate(product(groups, entries)):
        root = tmp_path / str(index)
        root.mkdir()
        _write_clean_pyproject(root)
        manifest = _write_clean_rust_workspace(root)
        assert verify.check_bypass(root) == [], (group, entry)
        text = manifest.read_text(encoding="utf-8")
        original = f'{group} = {{ level = "deny", priority = -1 }}'
        manifest.write_text(text.replace(original, f"{group} = {entry}"), encoding="utf-8")
        assert verify.check_bypass(root) == [
            f"Cargo.toml: [workspace.lints.clippy] must set {group} priority = -1"
        ], (group, entry)


def test_bypass_audit_flags_allow_or_warn_for_catalog_banned_lint(tmp_path: Path) -> None:
    _write_clean_pyproject(tmp_path)
    manifest = _write_clean_rust_workspace(tmp_path)
    assert verify.check_bypass(tmp_path) == []
    text = manifest.read_text(encoding="utf-8")
    manifest.write_text(
        text.replace('unwrap_used = "deny"', 'unwrap_used = "warn"'), encoding="utf-8"
    )
    findings = verify.check_bypass(tmp_path)
    assert any("catalog-banned lint 'unwrap_used'" in finding for finding in findings)
    _write_clean_rust_workspace(tmp_path)
    (tmp_path / "pyproject.toml").write_text(
        "[tool.ruff.lint]\n"
        'select = ["ALL"]\n'
        "ignore = [\n"
        '    "ANN401", # reason: attempted catalog bypass\n'
        "]\n",
        encoding="utf-8",
    )
    findings = verify.check_bypass(tmp_path)
    assert any("catalog-banned rule 'ANN401'" in finding for finding in findings)


def test_bypass_audit_flags_msbuild_warning_weakenings(tmp_path: Path) -> None:
    _write_clean_pyproject(tmp_path)
    _write_clean_csharp_configs(tmp_path)
    assert verify.check_bypass(tmp_path) == []
    csproj = tmp_path / "bridge" / "Sample" / "Sample.csproj"
    csproj.write_text(
        '<Project Sdk="Microsoft.NET.Sdk">\n'
        "  <PropertyGroup>\n"
        "    <TreatWarningsAsErrors>false</TreatWarningsAsErrors>\n"
        "    <NoWarn>CA1002</NoWarn>\n"
        "    <WarningsNotAsErrors>CS8618</WarningsNotAsErrors>\n"
        "  </PropertyGroup>\n"
        "</Project>\n",
        encoding="utf-8",
    )
    findings = verify.check_bypass(tmp_path)
    assert any("TreatWarningsAsErrors set to false" in finding for finding in findings)
    assert any("NoWarn" in finding for finding in findings)
    assert any("WarningsNotAsErrors" in finding for finding in findings)


def test_bypass_audit_flags_conditional_msbuild_warnings_as_errors_false(tmp_path: Path) -> None:
    _write_clean_pyproject(tmp_path)
    _write_clean_csharp_configs(tmp_path)
    csproj = tmp_path / "bridge" / "Sample" / "Sample.csproj"
    text = (
        '<Project Sdk="Microsoft.NET.Sdk">\n'
        "  <PropertyGroup>\n"
        "    <TreatWarningsAsErrors Condition=\"'$(Configuration)' == 'Debug'\">true"
        "</TreatWarningsAsErrors>\n"
        "  </PropertyGroup>\n"
        "</Project>\n"
    )
    csproj.write_text(text, encoding="utf-8")
    assert verify.check_bypass(tmp_path) == []
    csproj.write_text(text.replace(">true<", ">false<"), encoding="utf-8")
    assert verify.check_bypass(tmp_path) == [
        "bridge/Sample/Sample.csproj:3: TreatWarningsAsErrors set to false configuration bypass"
    ]


def test_bypass_audit_flags_suppress_message_global_suppressions_and_rust_expect(
    tmp_path: Path,
) -> None:
    _write_clean_pyproject(tmp_path)
    (tmp_path / "Clean.cs").write_text(
        "namespace Sample { public class Clean {} }\n", encoding="utf-8"
    )
    (tmp_path / "clean.rs").write_text(
        "pub fn add(a: i32, b: i32) -> i32 { a + b }\n", encoding="utf-8"
    )
    assert verify.check_bypass(tmp_path) == []
    (tmp_path / "Clean.cs").write_text(
        '[System.Diagnostics.CodeAnalysis.SuppressMessage("Design", "CA1000")]\n'
        "public class Weakened {}\n",
        encoding="utf-8",
    )
    (tmp_path / "GlobalSuppressions.cs").write_text("namespace Sample {}\n", encoding="utf-8")
    (tmp_path / "clean.rs").write_text(
        "#[expect(clippy::unwrap_used)]\npub fn run() {}\n",
        encoding="utf-8",
    )
    findings = verify.check_bypass(tmp_path)
    assert any("SuppressMessage" in finding for finding in findings)
    assert any("GlobalSuppressions.cs" in finding for finding in findings)
    assert any("rust expect attribute" in finding for finding in findings)


def test_bypass_audit_flags_analyzer_severity_downgrade_without_reason(tmp_path: Path) -> None:
    _write_clean_pyproject(tmp_path)
    _write_clean_csharp_configs(tmp_path)
    assert verify.check_bypass(tmp_path) == []
    (tmp_path / ".editorconfig").write_text(
        "root = true\n[*.cs]\ndotnet_diagnostic.CA1002.severity = none\n",
        encoding="utf-8",
    )
    findings = verify.check_bypass(tmp_path)
    assert any("analyzer severity downgrade" in finding for finding in findings)
    (tmp_path / ".editorconfig").write_text("root = true\n", encoding="utf-8")
    (tmp_path / ".globalconfig").write_text(
        "is_global = true\n# justified downgrade\ndotnet_diagnostic.CA1031.severity = suggestion\n",
        encoding="utf-8",
    )
    assert verify.check_bypass(tmp_path) == []
    (tmp_path / ".globalconfig").write_text(
        "is_global = true\ndotnet_diagnostic.CA1031.severity = suggestion\n",
        encoding="utf-8",
    )
    findings = verify.check_bypass(tmp_path)
    assert any(
        ".globalconfig" in finding and "analyzer severity downgrade" in finding
        for finding in findings
    )


def test_bypass_audit_requires_reasons_for_bulk_analyzer_severity(tmp_path: Path) -> None:
    cases = (
        (".editorconfig", "[*.cs]", "dotnet_analyzer_diagnostic.category-Performance", "none"),
        (".globalconfig", "is_global = true", "dotnet_analyzer_diagnostic", "silent"),
    )
    for index, (filename, header, key, severity) in enumerate(cases):
        root = tmp_path / str(index)
        member = root / "bridge"
        member.mkdir(parents=True)
        _write_clean_pyproject(root)
        config = member / filename
        setting = f"{key}.severity = {severity}\n"
        config.write_text(f"{header}\n# reason for the downgrade\n{setting}", encoding="utf-8")
        assert verify.check_bypass(root) == [], filename
        config.write_text(f"{header}\n{setting}", encoding="utf-8")
        path = config.relative_to(root)
        assert verify.check_bypass(root) == [
            f"{path}:2: analyzer severity downgrade to {severity!r} lacks a reason comment"
        ], filename


def test_bypass_audit_scans_clippy_toml_and_rustfmt_toml(tmp_path: Path) -> None:
    cases = ((".", ""), ("crates/alpha", "."))
    for index, (directory, prefix) in enumerate(cases):
        root = tmp_path / str(index)
        root.mkdir()
        _write_clean_pyproject(root)
        _write_clean_rust_workspace(root)
        member = root / directory
        clippy = member / f"{prefix}clippy.toml"
        rustfmt = member / f"{prefix}rustfmt.toml"
        clippy.write_text("cognitive-complexity-threshold = 25\n", encoding="utf-8")
        rustfmt.write_text("max_width = 100\n", encoding="utf-8")
        assert verify.check_bypass(root) == [], directory
        clippy.write_text("allow-unwrap-in-tests = true\n", encoding="utf-8")
        rustfmt.write_text("disable_all_formatting = true\n", encoding="utf-8")
        assert verify.check_bypass(root) == [
            f"{clippy.relative_to(root)}: allow-unwrap-in-tests bypass is forbidden",
            f"{rustfmt.relative_to(root)}: disable_all_formatting bypass is forbidden",
        ], directory


def test_comment_audit_allows_config_comments_and_flags_source_and_makefile_comments(
    tmp_path: Path,
) -> None:
    _write_clean_pyproject(tmp_path)
    (tmp_path / "Makefile").write_text("verify:\n\t@echo ok\n", encoding="utf-8")
    (tmp_path / "clean.py").write_text("VALUE = 1\n", encoding="utf-8")
    assert verify.check_comments(tmp_path) == []
    (tmp_path / "Makefile").write_text(
        "# comment in makefile\nverify:\n\t@echo ok\n", encoding="utf-8"
    )
    (tmp_path / "clean.py").write_text("# comment in python\nVALUE = 1\n", encoding="utf-8")
    findings = verify.check_comments(tmp_path)
    assert any("Makefile:1: comment token" in finding for finding in findings)
    assert any("clean.py:1: comment token" in finding for finding in findings)


def test_banned_construct_is_detected() -> None:
    tree = ast.parse("def run() -> None:\n    eval('1')\n")
    findings = verify.banned_findings("sample.py", tree)
    assert any("banned call eval" in finding for finding in findings)


def test_banned_raw_generic_type_is_detected(tmp_path: Path) -> None:
    source = tmp_path / "sample.cs"
    source.write_text("class C { List items; }\n", encoding="utf-8")
    findings = verify.banned_source_findings(source)
    assert any("raw generic type" in finding for finding in findings)
    source.write_text("class C { List<string> items; }\n", encoding="utf-8")
    assert verify.banned_source_findings(source) == []


def test_banned_any_downcast_is_detected(tmp_path: Path) -> None:
    source = tmp_path / "sample.rs"
    source.write_text("fn f(value: &std::any::Any) {}\n", encoding="utf-8")
    findings = verify.banned_source_findings(source)
    assert any("any type downcast" in finding for finding in findings)
    source.write_text("fn f(value: &str) {}\n", encoding="utf-8")
    assert verify.banned_source_findings(source) == []


def test_banned_unchecked_is_detected(tmp_path: Path) -> None:
    source = tmp_path / "sample.cs"
    source.write_text("class C { int x = unchecked(1); }\n", encoding="utf-8")
    findings = verify.banned_source_findings(source)
    assert any("unchecked" in finding for finding in findings)
    source.write_text("class C { int x = checked(1); }\n", encoding="utf-8")
    assert verify.banned_source_findings(source) == []


def test_banned_object_parameter_is_detected(tmp_path: Path) -> None:
    source = tmp_path / "sample.cs"
    source.write_text("class C { void Log(object item) {} }\n", encoding="utf-8")
    findings = verify.banned_source_findings(source)
    assert any("object parameter" in finding for finding in findings)
    source.write_text("class C { void Log(System.object item) {} }\n", encoding="utf-8")
    findings = verify.banned_source_findings(source)
    assert any("object parameter" in finding for finding in findings)


def test_banned_object_parameter_allows_unity_engine_object(tmp_path: Path) -> None:
    source = tmp_path / "sample.cs"
    source.write_text(
        "namespace UnityEngine\n"
        "{\n"
        "    public class Object\n"
        "    {\n"
        "        public string name { get; set; }\n"
        "        public void SetParent(Transform parent) {}\n"
        "    }\n"
        "    public class Component : Object\n"
        "    {\n"
        "        public void Compare(UnityEngine.Object other) {}\n"
        "    }\n"
        "}\n",
        encoding="utf-8",
    )
    assert verify.banned_source_findings(source) == []


def test_banned_source_skips_obj_and_bin_directories(tmp_path: Path) -> None:
    assert {"bin", "obj"}.issubset(verify.SKIP_DIR_NAMES)
    bridge_dir = tmp_path / "bridge" / "UnityEngine"
    obj_dir = bridge_dir / "obj"
    bin_dir = bridge_dir / "bin"
    bridge_dir.mkdir(parents=True)
    obj_dir.mkdir(parents=True)
    bin_dir.mkdir(parents=True)
    content = "class C { int x = unchecked(1); }\n"
    (bridge_dir / "Valid.cs").write_text(content, encoding="utf-8")
    (obj_dir / "Generated.cs").write_text(content, encoding="utf-8")
    (bin_dir / "Compiled.cs").write_text(content, encoding="utf-8")
    findings = verify.check_banned(root=tmp_path)
    assert any("Valid.cs" in finding for finding in findings)
    assert not any("Generated.cs" in finding for finding in findings)
    assert not any("Compiled.cs" in finding for finding in findings)


def test_banned_unsafe_is_exempt_inside_a_zone_and_reported_outside_one(
    tmp_path: Path,
) -> None:
    _write_zone_registry(tmp_path)
    zone = tmp_path / "crates" / "zone-crate"
    zone.mkdir(parents=True)
    (zone / "lib.rs").write_text(
        "pub fn leak() {\n"
        "    unsafe {}\n"
        "}\n"
        "\n"
        "pub fn widen(value: u8) -> u16 {\n"
        "    unsafe { std::mem::transmute(value) }\n"
        "}\n",
        encoding="utf-8",
    )
    outside = tmp_path / "crates" / "alpha"
    outside.mkdir(parents=True)
    (outside / "lib.rs").write_text("pub fn leak() {\n    unsafe {}\n}\n", encoding="utf-8")
    findings = verify.check_banned(tmp_path)
    assert "crates/alpha/lib.rs:2: unsafe block" in findings
    assert "crates/zone-crate/lib.rs:2: unsafe block" not in findings
    assert "crates/zone-crate/lib.rs:6: transmute" in findings


def test_dependency_gate_allows_stdlib_only() -> None:
    assert verify.check_dependencies() == []


def _write_lockfile(root: Path, packages: list[tuple[str, str]]) -> Path:
    path = root / "Cargo.lock"
    blocks = [
        "[[package]]\n"
        f'name = "{name}"\n'
        f'version = "{version}"\n'
        'source = "registry+https://github.com/rust-lang/crates.io-index"\n'
        for name, version in packages
    ]
    path.write_text("\n".join(blocks), encoding="utf-8")
    return path


def _write_registry(
    root: Path,
    entries: list[tuple[str, str, str | None, str]],
    section: str = "package",
) -> Path:
    path = root / "tools" / "dependency-registry.toml"
    path.parent.mkdir(parents=True, exist_ok=True)
    blocks = []
    for name, version, expression, reviewed in entries:
        block = f'[[{section}]]\nname = "{name}"\nversion = "{version}"\n'
        if expression is not None:
            block += f'license = "{expression}"\n'
        block += f'advisory_reviewed = "{reviewed}"\n'
        blocks.append(block)
    path.write_text("\n".join(blocks), encoding="utf-8")
    return path


def _write_nuget_lockfile(root: Path, packages: list[tuple[str, str]]) -> Path:
    path = root / "editor" / "Unitas.Editor" / "packages.lock.json"
    path.parent.mkdir(parents=True, exist_ok=True)
    locked = {name: {"type": "Transitive", "resolved": version} for name, version in packages}
    path.write_text(
        json.dumps({"version": 1, "dependencies": {"net8.0": locked}}), encoding="utf-8"
    )
    return path


def test_dependency_registry_reports_unlisted_package(tmp_path: Path) -> None:
    _write_lockfile(tmp_path, [("alpha", "1.0.0")])
    _write_registry(tmp_path, [("beta", "2.0.0", "MIT", "2026-01-01")])
    findings = verify.check_dependency_registry(root=tmp_path)
    assert any("alpha 1.0.0" in finding and "no entry" in finding for finding in findings)


def test_dependency_registry_reports_disallowed_license(tmp_path: Path) -> None:
    _write_lockfile(tmp_path, [("alpha", "1.0.0")])
    _write_registry(tmp_path, [("alpha", "1.0.0", "GPL-3.0-only", "2026-01-01")])
    findings = verify.check_dependency_registry(root=tmp_path)
    assert any("GPL-3.0-only" in finding for finding in findings)
    _write_registry(tmp_path, [("alpha", "1.0.0", "MIT AND GPL-3.0-only", "2026-01-01")])
    findings = verify.check_dependency_registry(root=tmp_path)
    assert any("MIT AND GPL-3.0-only" in finding for finding in findings)
    _write_registry(
        tmp_path, [("alpha", "1.0.0", "MIT OR Apache-2.0 OR LGPL-2.1-or-later", "2026-01-01")]
    )
    assert verify.check_dependency_registry(root=tmp_path) == []


def test_dependency_registry_reports_missing_license(tmp_path: Path) -> None:
    _write_lockfile(tmp_path, [("alpha", "1.0.0")])
    _write_registry(tmp_path, [("alpha", "1.0.0", None, "2026-01-01")])
    findings = verify.check_dependency_registry(root=tmp_path)
    assert any("declares no license" in finding for finding in findings)


def test_dependency_registry_reports_unusable_advisory_date(tmp_path: Path) -> None:
    _write_lockfile(tmp_path, [("alpha", "1.0.0")])
    _write_registry(tmp_path, [("alpha", "1.0.0", "MIT", "")])
    findings = verify.check_dependency_registry(root=tmp_path)
    assert any("no advisory_reviewed date" in finding for finding in findings)
    _write_registry(tmp_path, [("alpha", "1.0.0", "MIT", "not-a-date")])
    findings = verify.check_dependency_registry(root=tmp_path)
    assert any("is not an ISO date" in finding for finding in findings)
    _write_registry(tmp_path, [("alpha", "1.0.0", "MIT", "2999-12-31")])
    findings = verify.check_dependency_registry(root=tmp_path)
    assert any("is in the future" in finding for finding in findings)


def test_dependency_registry_accepts_compliant_entries(tmp_path: Path) -> None:
    _write_lockfile(tmp_path, [("alpha", "1.0.0"), ("beta", "2.0.0")])
    _write_registry(
        tmp_path,
        [
            ("alpha", "1.0.0", "MIT OR Apache-2.0", "2026-01-01"),
            ("beta", "2.0.0", "BSD-3-Clause", "2026-01-01"),
        ],
    )
    assert verify.check_dependency_registry(root=tmp_path) == []


def test_nuget_gate_reports_unlisted_package(tmp_path: Path) -> None:
    _write_nuget_lockfile(tmp_path, [("Alpha", "1.0.0")])
    _write_registry(tmp_path, [("Beta", "2.0.0", "MIT", "2026-01-01")], "nuget")
    findings = verify.check_nuget_registry(root=tmp_path)
    assert any("Alpha 1.0.0" in finding and "no entry" in finding for finding in findings)


def test_nuget_gate_reports_disallowed_license(tmp_path: Path) -> None:
    _write_nuget_lockfile(tmp_path, [("Alpha", "1.0.0")])
    _write_registry(tmp_path, [("Alpha", "1.0.0", "GPL-3.0-only", "2026-01-01")], "nuget")
    findings = verify.check_nuget_registry(root=tmp_path)
    assert any("GPL-3.0-only" in finding for finding in findings)


def test_nuget_gate_reports_future_advisory_date(tmp_path: Path) -> None:
    _write_nuget_lockfile(tmp_path, [("Alpha", "1.0.0")])
    _write_registry(tmp_path, [("Alpha", "1.0.0", "MIT", "2999-12-31")], "nuget")
    findings = verify.check_nuget_registry(root=tmp_path)
    assert any("is in the future" in finding for finding in findings)


def test_nuget_gate_accepts_compliant_entries(tmp_path: Path) -> None:
    _write_nuget_lockfile(tmp_path, [("Alpha", "1.0.0"), ("Beta", "2.0.0")])
    _write_registry(
        tmp_path,
        [
            ("Alpha", "1.0.0", "MIT OR Apache-2.0", "2026-01-01"),
            ("Beta", "2.0.0", "BSD-3-Clause", "2026-01-01"),
        ],
        "nuget",
    )
    assert verify.check_nuget_registry(root=tmp_path) == []


def test_nuget_gate_ignores_repository_without_lockfiles(tmp_path: Path) -> None:
    assert verify.check_nuget_registry(root=tmp_path) == []
    project_dir = tmp_path / "editor" / "Unitas.Editor"
    obj_dir = project_dir / "obj"
    agent_dir = tmp_path / ".agent"
    obj_dir.mkdir(parents=True)
    agent_dir.mkdir()
    content = json.dumps(
        {"version": 1, "dependencies": {"net8.0": {"Alpha": {"resolved": "1.0.0"}}}}
    )
    (obj_dir / "packages.lock.json").write_text(content, encoding="utf-8")
    (agent_dir / "packages.lock.json").write_text(content, encoding="utf-8")
    assert verify.check_nuget_registry(root=tmp_path) == []


def test_secret_delta_reports_new_findings() -> None:
    baseline = {"a.py": [{"type": "Hex High Entropy String", "line_number": 3}]}
    scanned = {
        "a.py": [{"type": "Hex High Entropy String", "line_number": 3}],
        "b.py": [{"type": "Keyword Detector", "line_number": 7}],
    }
    assert verify.secret_delta(baseline, scanned) == [
        "secret finding not in baseline: b.py Keyword Detector"
    ]


def test_secret_scan_excludes_build_output_but_not_sources() -> None:
    build_output = (
        "target/CACHEDIR.TAG",
        "crates/unitas-asset/target/debug/deps/libunitas_asset.rlib",
        "checkpoint.json",
        "bin/Debug/app.dll",
        "obj/Debug/app.dll",
        "bridge/UnityEngine/bin/Release/netstandard2.1/UnityEngine.dll",
        "bridge/UnityEngine/obj/Release/netstandard2.1/project.assets.json",
        "tools/__pycache__/verify.cpython-314.pyc",
    )
    for path in build_output:
        assert any(re.search(pattern, path) for pattern in verify.SECRET_SCAN_EXCLUDES)
    sources = (
        "crates/unitas-core/src/scene_graph.rs",
        "bridge/UnityEngine/GameObject.cs",
        "tools/verify.py",
    )
    for path in sources:
        assert not any(re.search(pattern, path) for pattern in verify.SECRET_SCAN_EXCLUDES)


def test_secret_scan_excludes_agent_scratch_directories() -> None:
    scratch = (
        ".agent/tmp/development_prompt.md",
        ".opencode/state.json",
        ".omo/run-continuation/ses_deb337c08ffe2K5MiExtomhIVY.json",
    )
    for path in scratch:
        assert any(re.search(pattern, path) for pattern in verify.SECRET_SCAN_EXCLUDES)


def test_secret_scan_skips_agent_scratch_tree(tmp_path: Path) -> None:
    scratch = tmp_path / ".omo" / "run-continuation"
    scratch.mkdir(parents=True)
    (scratch / "ses_deb337c08ffe2K5MiExtomhIVY.json").write_text(
        '{"sessionID": "0Z3mQm9Qb2F0YQ=="}', encoding="utf-8"
    )
    (tmp_path / "crates").mkdir()
    (tmp_path / "crates" / "lib.rs").write_text("fn main() {}\n", encoding="utf-8")

    files = verify.secret_scan_files(tmp_path)

    assert "crates/lib.rs" in files
    assert not any(path.startswith(".omo/") for path in files)


def test_secret_scan_skips_generated_player_bundles(tmp_path: Path) -> None:
    bundle = tmp_path / "roll-a-ball"
    bundle.mkdir()
    (bundle / verify.PLAYER_BUNDLE_MANIFEST_FILENAME).write_text("{}\n", encoding="utf-8")
    (bundle / "RollABall.app").mkdir()
    (bundle / "RollABall.app" / "Resources").mkdir()
    (bundle / "RollABall.app" / "Resources" / "bundle.manifest").write_text(
        "{}\n", encoding="utf-8"
    )
    source_dir = tmp_path / "crates"
    source_dir.mkdir()
    (source_dir / "lib.rs").write_text("fn main() {}\n", encoding="utf-8")

    files = verify.secret_scan_files(tmp_path)

    assert "crates/lib.rs" in files
    assert not any(path.startswith("roll-a-ball/") for path in files)


def test_secret_scan_keeps_first_party_directories_that_hold_a_bundle_manifest(
    tmp_path: Path,
) -> None:
    fixture = tmp_path / "crates" / "fixture"
    fixture.mkdir(parents=True)
    (fixture / verify.PLAYER_BUNDLE_MANIFEST_FILENAME).write_text("{}\n", encoding="utf-8")
    (fixture / "scene.unity").write_text("%YAML 1.1\n", encoding="utf-8")

    files = verify.secret_scan_files(tmp_path)

    assert "crates/fixture/scene.unity" in files


def test_dotnet_format_files_skip_build_output_but_keep_sources(tmp_path: Path) -> None:
    project = tmp_path / "bridge" / "App"
    (project / "obj" / "Debug").mkdir(parents=True)
    (project / "bin" / "Release").mkdir(parents=True)
    (project / "App.csproj").write_text("<Project />\n", encoding="utf-8")
    (project / "Main.cs").write_text("class Main {}\n", encoding="utf-8")
    (project / "obj" / "Debug" / "Generated.cs").write_text("class G {}\n", encoding="utf-8")
    (project / "bin" / "Release" / "Compiled.cs").write_text("class C {}\n", encoding="utf-8")
    (tmp_path / "editor").mkdir()
    (tmp_path / ".editorconfig").write_text("root = true\n", encoding="utf-8")
    files = verify.dotnet_format_files(tmp_path)
    assert "bridge/App/Main.cs" in files
    assert "bridge/App/App.csproj" in files
    assert ".editorconfig" in files
    assert not any("obj" in name or "bin" in name for name in files)


def _fake_dotnet_run(
    calls: list[list[str]], exit_code: int = 0
) -> Callable[..., subprocess.CompletedProcess[str]]:
    def fake_run(command: list[str], **_kwargs: object) -> subprocess.CompletedProcess[str]:
        calls.append(list(command))
        if list(command)[-1] == "--version":
            return subprocess.CompletedProcess(command, 0, stdout="8.0.428\n", stderr="")
        return subprocess.CompletedProcess(command, exit_code)

    return fake_run


def test_dotnet_format_cache_hit_skips_the_formatter(tmp_path: Path) -> None:
    (tmp_path / "bridge").mkdir()
    (tmp_path / "bridge" / "Unitas.sln").write_text("sln\n", encoding="utf-8")
    cache = tmp_path / "cache.json"
    digest_calls: list[list[str]] = []
    with mock.patch.object(subprocess, "run", _fake_dotnet_run(digest_calls)):
        digest = verify.dotnet_format_digest(verify.dotnet_format_files(tmp_path), tmp_path)
    cache.write_text(json.dumps({"digest": digest}), encoding="utf-8")

    calls: list[list[str]] = []
    with mock.patch.object(subprocess, "run", _fake_dotnet_run(calls)):
        assert verify.run_dotnet_format(tmp_path, cache) == 0
    assert all(command[-1] == "--version" for command in calls)


def test_dotnet_format_runs_and_caches_on_success(tmp_path: Path) -> None:
    (tmp_path / "bridge").mkdir()
    (tmp_path / "bridge" / "Unitas.sln").write_text("sln\n", encoding="utf-8")
    cache = tmp_path / "cache.json"
    calls: list[list[str]] = []

    with mock.patch.object(subprocess, "run", _fake_dotnet_run(calls)):
        assert verify.run_dotnet_format(tmp_path, cache) == 0
    assert any(command[1:] == list(verify.DOTNET_FORMAT_COMMAND[1:]) for command in calls)
    assert json.loads(cache.read_text(encoding="utf-8"))["digest"]


def test_dotnet_format_failure_is_not_cached(tmp_path: Path) -> None:
    (tmp_path / "bridge").mkdir()
    (tmp_path / "bridge" / "Unitas.sln").write_text("sln\n", encoding="utf-8")
    cache = tmp_path / "cache.json"
    calls: list[list[str]] = []

    with mock.patch.object(subprocess, "run", _fake_dotnet_run(calls, exit_code=1)):
        assert verify.run_dotnet_format(tmp_path, cache) == 1
    assert not cache.exists()


def test_dotnet_format_digest_tracks_source_content(tmp_path: Path) -> None:
    source = tmp_path / "bridge" / "App" / "Main.cs"
    source.parent.mkdir(parents=True)
    source.write_text("class Main {}\n", encoding="utf-8")
    calls: list[list[str]] = []
    with mock.patch.object(subprocess, "run", _fake_dotnet_run(calls)):
        before = verify.dotnet_format_digest(verify.dotnet_format_files(tmp_path), tmp_path)
        source.write_text("class Main { int x; }\n", encoding="utf-8")
        after = verify.dotnet_format_digest(verify.dotnet_format_files(tmp_path), tmp_path)
    assert before != after


def _guard_table(enforcement: str) -> str:
    header = (
        "| Guard | Owner | Protected outcome | Detected condition | Claim scope | "
        "Enforcement | Retirement trigger |"
    )
    separator = "|---|---|---|---|---|---|---|"
    row = f"| G | O | p | d | c | {enforcement} | r |"
    return "## Guard inventory\n\n" + header + "\n" + separator + "\n" + row + "\n"


def test_bypass_audit_rejects_reason_comment_inside_toml_string(tmp_path: Path) -> None:
    _write_clean_pyproject(tmp_path)
    manifest = _write_clean_rust_workspace(tmp_path)
    assert verify.check_bypass(tmp_path) == []
    text = manifest.read_text(encoding="utf-8")
    manifest.write_text(
        text.replace(
            '# clean-code-policy.md bans doc comments\nmissing_errors_doc = "allow"',
            '"missing#docs" = "allow"',
        ),
        encoding="utf-8",
    )
    assert verify.check_bypass(tmp_path) == [
        "Cargo.toml:18: lint 'missing#docs' set to 'allow' lacks a reason comment"
    ]


def test_bypass_audit_flags_conditional_rust_lint_attribute(tmp_path: Path) -> None:
    _write_clean_pyproject(tmp_path)
    (tmp_path / "clean.rs").write_text(
        "pub fn add(a: i32, b: i32) -> i32 { a + b }\n", encoding="utf-8"
    )
    assert verify.check_bypass(tmp_path) == []
    nested = '#[cfg_attr(any(feature = "a", feature = "b"), allow(dead_code))]\n'
    (tmp_path / "clean.rs").write_text(
        '#[cfg_attr(feature = "extra", expect(clippy::unwrap_used))]\npub fn run() {}\n',
        encoding="utf-8",
    )
    assert verify.check_bypass(tmp_path) == [
        "clean.rs:1: rust conditional lint attribute suppression marker"
    ]
    (tmp_path / "clean.rs").write_text(f"{nested}pub fn run() {{}}\n", encoding="utf-8")
    assert verify.check_bypass(tmp_path) == [
        "clean.rs:1: rust conditional lint attribute suppression marker"
    ]
    (tmp_path / "clean.rs").write_text(
        '#[cfg_attr(feature = "extra", derive(Debug))]\npub fn run() {}\n', encoding="utf-8"
    )
    assert verify.check_bypass(tmp_path) == []


def test_timed_gate_passes_a_command_that_finishes_inside_its_budget() -> None:
    exit_code = verify.run_timed(["timed", "probe", "30", sys.executable, "-c", "pass"])
    assert exit_code == 0


def test_timed_gate_terminates_a_command_past_its_budget(tmp_path: Path) -> None:
    marker = tmp_path / "grandchild.txt"
    grandchild_record_delay_seconds = 5
    grandchild = (
        f"import pathlib, time\ntime.sleep({grandchild_record_delay_seconds})\n"
        f"pathlib.Path({str(marker)!r}).write_text('alive')\n"
    )
    script = (
        "import subprocess, sys, time\n"
        f"subprocess.Popen([sys.executable, '-c', {grandchild!r}])\n"
        "time.sleep(30)\n"
    )
    exit_code = verify.run_timed(["timed", "probe", "0.3", sys.executable, "-c", script])
    assert exit_code == 1
    time.sleep(grandchild_record_delay_seconds + 1.0)
    assert not marker.exists()


def test_timed_gate_timeout_cites_the_time_budget_policy() -> None:
    stderr = io.StringIO()
    with contextlib.redirect_stderr(stderr):
        exit_code = verify.run_timed(
            ["timed", "probe", "0.3", sys.executable, "-c", "import time; time.sleep(30)"]
        )
    assert exit_code == 1
    output = stderr.getvalue()
    assert "gate-script-policy.md" in output
    assert "testing-policy.md" in output


def test_timed_gate_terminates_its_child_when_the_runner_is_signalled(tmp_path: Path) -> None:
    marker = tmp_path / "child.txt"
    release = tmp_path / "release.txt"
    script = (
        "import pathlib, time\n"
        f"pathlib.Path({str(marker)!r}).write_text('started')\n"
        f"release = pathlib.Path({str(release)!r})\n"
        "while not release.exists():\n"
        "    time.sleep(0.05)\n"
        f"pathlib.Path({str(marker)!r}).write_text('finished')\n"
    )
    runner = subprocess.Popen(
        [sys.executable, "tools/verify.py", "timed", "probe", "30", sys.executable, "-c", script],
        cwd=verify.REPO_ROOT,
    )
    deadline = time.monotonic() + 20
    while not marker.exists() and time.monotonic() < deadline:
        time.sleep(0.05)
    assert marker.exists(), "the timed gate never started its child"
    runner.terminate()
    assert runner.wait(timeout=10) == SIGNALLED_EXIT_CODE
    release.write_text("release", encoding="utf-8")
    time.sleep(1.5)
    assert marker.read_text(encoding="utf-8") == "started"


def test_timed_gate_runner_is_killable_when_its_parent_blocks_sigterm(tmp_path: Path) -> None:
    marker = tmp_path / "child.txt"
    release = tmp_path / "release.txt"
    script = (
        "import pathlib, time\n"
        f"pathlib.Path({str(marker)!r}).write_text('started')\n"
        f"release = pathlib.Path({str(release)!r})\n"
        "while not release.exists():\n"
        "    time.sleep(0.05)\n"
        f"pathlib.Path({str(marker)!r}).write_text('finished')\n"
    )
    blocked_in_parent = signal.pthread_sigmask(signal.SIG_BLOCK, {signal.SIGTERM})
    try:
        runner = subprocess.Popen(
            [
                sys.executable,
                "tools/verify.py",
                "timed",
                "probe",
                "30",
                sys.executable,
                "-c",
                script,
            ],
            cwd=verify.REPO_ROOT,
        )
    finally:
        signal.pthread_sigmask(signal.SIG_SETMASK, blocked_in_parent)
    deadline = time.monotonic() + 20
    while not marker.exists() and time.monotonic() < deadline:
        time.sleep(0.05)
    assert marker.exists(), "the timed gate never started its child"
    runner.terminate()
    assert runner.wait(timeout=10) == SIGNALLED_EXIT_CODE
    release.write_text("release", encoding="utf-8")


_MEASUREMENT_COUNT_WORDS = {
    "eight": 8,
    "nine": 9,
    "ten": 10,
    "eleven": 11,
    "twelve": 12,
    "thirteen": 13,
    "fourteen": 14,
    "fifteen": 15,
    "sixteen": 16,
    "seventeen": 17,
    "eighteen": 18,
    "nineteen": 19,
    "twenty": 20,
}

_EDITOR_PERF_MEASUREMENTS = (
    "editor_startup",
    "editor_project_open",
    "editor_hierarchy_search_100k",
    "editor_property_edit_undo",
    "editor_mcp_round_trip",
)


def test_verify_perf_lane_registers_the_editor_measurements() -> None:
    registered = {name for name, _ in verify_perf.MEASUREMENTS}
    for name in _EDITOR_PERF_MEASUREMENTS:
        assert name in registered
        assert name in verify_perf.EDITOR_MEASUREMENT_CEILINGS_MS
    assert set(verify_perf.EDITOR_MEASUREMENT_CEILINGS_MS) == set(_EDITOR_PERF_MEASUREMENTS)


def test_verify_perf_lane_count_matches_the_policy_fact() -> None:
    policy = (
        verify_perf.REPO_ROOT / "docs/ralph-workflow-policy/verification-policy.md"
    ).read_text(encoding="utf-8")
    for name in _EDITOR_PERF_MEASUREMENTS:
        assert f"`{name}`" in policy
    match = re.search(r"The lane reports (\w+) measurements:", policy)
    assert match is not None
    word = match.group(1)
    assert word in _MEASUREMENT_COUNT_WORDS, f"the policy states an unknown count: {word}"
    assert _MEASUREMENT_COUNT_WORDS[word] == len(verify_perf.MEASUREMENTS)


def test_samples_lane_lists_s8_visual_scenarios() -> None:
    s8 = (
        "s8_title_checkpoint.scenario.json",
        "s8_arena_countdown.scenario.json",
        "s8_arena_x5_checkpoint.scenario.json",
        "s8_results_gold_checkpoint.scenario.json",
    )
    entries = {
        scenario: (extra_args, needs_persist)
        for _, scenario, extra_args, needs_persist in verify_samples.SCENARIOS
    }
    for scenario in s8:
        assert scenario in entries
        assert entries[scenario] == ((), False)
        assert (verify_samples.SCENARIO_DIR / scenario).is_file()
    assert not set(s8) & set(verify_samples.EDITOR_SCENARIOS)


_TIMED_GATE_CHILD_SOURCE = """import pathlib
import sys
import time

marker = pathlib.Path(sys.argv[1])
deadline = time.monotonic() + 30
while time.monotonic() < deadline:
    with marker.open("a", encoding="utf-8") as handle:
        handle.write("x")
    time.sleep(0.05)
"""

_TIMED_GATE_PARENT_SOURCE = """import subprocess
import sys
import time

subprocess.Popen([sys.executable, sys.argv[1], sys.argv[2]])
time.sleep(30)
"""


_TIMED_GATE_BOUND_SECONDS = 10
_TIMED_GATE_ATTEMPTS = 2
_RED_GATE_EXIT_CODE = 3


def _run_timed(argv: list[str]) -> tuple[int, str]:
    buffer = io.StringIO()
    with contextlib.redirect_stdout(buffer):
        status = verify.run_timed(argv)
    return status, buffer.getvalue()


def test_timed_gate_fails_red_when_the_command_outlives_its_budget() -> None:
    started = time.monotonic()
    status, output = _run_timed(
        ["timed", "slow-gate", "1", sys.executable, "-c", "import time; time.sleep(30)"]
    )
    elapsed = time.monotonic() - started
    assert status == 1
    assert elapsed < _TIMED_GATE_BOUND_SECONDS
    assert "gate slow-gate exceeded the 1s budget and was terminated" in output


def test_timed_gate_reports_elapsed_time_for_a_passing_gate() -> None:
    status, output = _run_timed(
        ["timed", "fast-gate", "30", sys.executable, "-c", "print('gate ran')"]
    )
    assert status == 0
    assert "gate fast-gate took " in output
    assert "exceeded the 30s budget" not in output


def test_timed_gate_fails_a_command_that_would_succeed_past_its_budget() -> None:
    status, output = _run_timed(
        ["timed", "overrun-gate", "1", sys.executable, "-c", "import time; time.sleep(3)"]
    )
    assert status == 1
    assert "gate overrun-gate exceeded the 1s budget and was terminated" in output


def test_timed_gate_reruns_once_and_passes_when_the_re_run_fits_the_budget(
    tmp_path: Path,
) -> None:
    marker = tmp_path / "attempted.txt"
    script = (
        "import pathlib, sys, time\n"
        "marker = pathlib.Path(sys.argv[1])\n"
        "if not marker.exists():\n"
        "    marker.write_text('first', encoding='utf-8')\n"
        "    time.sleep(30)\n"
        "print('second')\n"
    )
    status, output = _run_timed(
        ["timed", "contended-gate", "1", sys.executable, "-c", script, str(marker)]
    )
    assert status == 0
    assert "gate contended-gate exceeded the 1s budget and was terminated" in output
    assert "gate contended-gate is re-run once with the same 1s budget" in output
    assert "gate contended-gate passed on the re-run" in output


def test_timed_gate_fails_when_the_re_run_overruns_the_budget_again() -> None:
    stderr = io.StringIO()
    with contextlib.redirect_stderr(stderr):
        status, output = _run_timed(
            ["timed", "slow-gate", "1", sys.executable, "-c", "import time; time.sleep(30)"]
        )
    assert status == 1
    terminations = output.count("gate slow-gate exceeded the 1s budget and was terminated")
    assert terminations == _TIMED_GATE_ATTEMPTS
    assert "gate slow-gate is re-run once with the same 1s budget" in output
    assert "gate-script-policy.md" in stderr.getvalue()


def test_timed_gate_does_not_rerun_a_command_that_exits_non_zero() -> None:
    stderr = io.StringIO()
    with contextlib.redirect_stderr(stderr):
        status, output = _run_timed(
            ["timed", "red-gate", "30", sys.executable, "-c", "raise SystemExit(3)"]
        )
    assert status == _RED_GATE_EXIT_CODE
    assert "is re-run once" not in output


def test_timed_gate_kills_the_process_tree_it_started(tmp_path: Path) -> None:
    child = tmp_path / "child.py"
    parent = tmp_path / "parent.py"
    marker = tmp_path / "grandchild.log"
    child.write_text(_TIMED_GATE_CHILD_SOURCE, encoding="utf-8")
    parent.write_text(_TIMED_GATE_PARENT_SOURCE, encoding="utf-8")
    status = verify.run_timed(
        ["timed", "tree-gate", "1", sys.executable, str(parent), str(child), str(marker)]
    )
    assert status == 1
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline and not marker.exists():
        time.sleep(0.05)
    assert marker.exists()
    time.sleep(0.2)
    size = marker.stat().st_size
    assert size > 0
    time.sleep(0.5)
    assert marker.stat().st_size == size
