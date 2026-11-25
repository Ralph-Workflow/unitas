from __future__ import annotations

import ast
import json
import re
import subprocess
from itertools import product
from typing import TYPE_CHECKING
from unittest import mock

from tools import verify

if TYPE_CHECKING:
    from collections.abc import Callable
    from pathlib import Path


def test_repository_gates_report_no_findings() -> None:
    for name in sorted(verify.GATES):
        findings = verify.GATES[name]()
        assert findings == [], f"{name} reported findings: {findings}"


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
