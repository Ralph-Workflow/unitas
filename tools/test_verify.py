from __future__ import annotations

import ast
import json
import re
from pathlib import Path

from tools import verify


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
    locked = {
        name: {"type": "Transitive", "resolved": version} for name, version in packages
    }
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


def _guard_table(enforcement: str) -> str:
    header = (
        "| Guard | Owner | Protected outcome | Detected condition | Claim scope | "
        "Enforcement | Retirement trigger |"
    )
    separator = "|---|---|---|---|---|---|---|"
    row = f"| G | O | p | d | c | {enforcement} | r |"
    return "## Guard inventory\n\n" + header + "\n" + separator + "\n" + row + "\n"
