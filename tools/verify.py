from __future__ import annotations

import ast
import contextlib
import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import time
import tokenize
import tomllib
from datetime import UTC, date, datetime
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator, Sequence

REPO_ROOT = Path(__file__).resolve().parent.parent
POLICY_DIR = REPO_ROOT / "docs" / "ralph-workflow-policy"
PORTFOLIO_PATH = POLICY_DIR / "policy-portfolio.toml"
AGENTS_MD = REPO_ROOT / "AGENTS.md"
MAKEFILE = REPO_ROOT / "Makefile"
SECRETS_BASELINE = REPO_ROOT / ".secrets.baseline"
CONFIG_FILES = ("pyproject.toml", "tools/dependency-registry.toml")
CARGO_LOCKFILE = Path("Cargo.lock")
NUGET_LOCKFILE_NAME = "packages.lock.json"
DEPENDENCY_REGISTRY = Path("tools") / "dependency-registry.toml"
UNSAFE_ZONES_REGISTRY = Path("tools") / "unsafe-zones.toml"
ZONE_RUST_DENIES = (
    "warnings",
    "unnecessary_transmutes",
    "static_mut_refs",
    "unsafe_op_in_unsafe_fn",
    "improper_ctypes",
    "improper_ctypes_definitions",
)
ZONE_CLIPPY_DENIES = (
    "unwrap_used",
    "expect_used",
    "panic",
    "todo",
    "unimplemented",
    "dbg_macro",
    "undocumented_unsafe_blocks",
    "missing_safety_doc",
    "multiple_unsafe_ops_per_block",
    "ptr_as_ptr",
    "cast_ptr_alignment",
    "as_ptr_cast_mut",
    "borrow_as_ptr",
    "transmute_ptr_to_ptr",
)
REGISTRY_ENTRY_KEYS = ("name", "version", "license", "advisory_reviewed", "note")
PERMISSIVE_LICENSES = frozenset(
    {
        "0BSD",
        "Apache-2.0",
        "BSD-2-Clause",
        "BSD-3-Clause",
        "BSL-1.0",
        "ISC",
        "MIT",
        "Unlicense",
        "Unicode-3.0",
        "Zlib",
    }
)
COMMENT_SUFFIXES = (".cfg", ".ini", ".j2", ".jinja", ".sh", ".toml", ".yaml", ".yml")
SOURCE_SUFFIXES = (
    ".cs",
    ".go",
    ".h",
    ".java",
    ".js",
    ".jsx",
    ".kt",
    ".py",
    ".rs",
    ".ts",
    ".tsx",
)
SKIP_DIR_NAMES = frozenset(
    {"__pycache__", "bin", "build", "dist", "node_modules", "obj", "target", "venv"}
)
APPLICABILITY_PATH = POLICY_DIR / "applicability.toml"
CONDITIONAL_DOMAINS = (
    "accessibility",
    "api-compatibility",
    "data-storage",
    "design-system",
    "memory-usage",
    "performance",
    "privacy",
    "reliability-observability",
    "release-deployment",
    "ux",
)
APPLICABILITY_PATTERN = re.compile(
    r'^([a-z][a-z-]+)\s*=\s*"(required|inactive); reason: ([^;]+); review trigger: ([^"]+)"$'
)
POLICY_FILES = (
    "agent-policy.md",
    "architecture-policy.md",
    "clean-code-policy.md",
    "dependency-policy.md",
    "documentation-policy.md",
    "execution-kernel.md",
    "gate-script-policy.md",
    "linting-policy.md",
    "security-policy.md",
    "testing-policy.md",
    "typechecking-policy.md",
    "verification-policy.md",
)
PLACEHOLDER_TOKENS = (
    "TODO",
    "TBD",
    "FIXME",
    "<REPLACE>",
    "{{",
    "REPLACE-ME",
    "PROJECT-FACT-UNRESOLVED",
    "<date>",
    "<trigger>",
)
APPROVED_GATE_TOOLS = frozenset(
    {
        "ansible",
        "bandit",
        "black",
        "bun",
        "bundle",
        "cargo",
        "composer",
        "detect-secrets",
        "docker",
        "docker-compose",
        "dotnet",
        "eslint",
        "flake8",
        "gitleaks",
        "go",
        "golangci-lint",
        "gosec",
        "gradle",
        "hadolint",
        "isort",
        "kubectl",
        "make",
        "mix",
        "mvn",
        "mypy",
        "node",
        "npm",
        "npx",
        "php",
        "pnpm",
        "prettier",
        "pylint",
        "pytest",
        "python",
        "python3",
        "ruff",
        "safety",
        "semgrep",
        "shellcheck",
        "swift",
        "terraform",
        "ts-node",
        "tsc",
        "uv",
        "yarn",
    }
)
FACT_PATTERN = re.compile(r"^RALPH-FACT:\s*([^:]+):\s*(.*)$")
TOML_STRING_FIELD = re.compile(r'^([A-Za-z_][A-Za-z0-9_-]*)\s*=\s*"([^"]*)"$')
COMMAND_PATTERN = re.compile(r"^RALPH-COMMAND:\s*(.*)$")
DETERMINISTIC_PATTERN = re.compile(r"^deterministic:\s*(\S.*)$")
ADVISORY_PATTERN = re.compile(r"^advisory:\s*([^;]+);\s*review:\s*(\S.*)$")
LINK_PATTERN = re.compile(r"`([^`\n]+)`")
PATH_TOKEN_PATTERN = re.compile(r"^[A-Za-z0-9_.\-/]+$")
ROOT_CITED_FILES = frozenset({"AGENTS.md", "CLAUDE.md", "Makefile", "pyproject.toml"})
REPOSITORY_PATH_ROOTS = ("docs/", "tools/")
GUARD_CELL_COUNT = 7
GUARD_ENFORCEMENT_INDEX = 5
MAKE_COMMAND_MIN_PARTS = 2
TIMED_COMMAND_MIN_ARGS = 4
GATE_CLI_ARG_COUNT = 2
BYPASS_SOURCE_PATTERNS: dict[str, tuple[tuple[str, re.Pattern[str]], ...]] = {
    ".js": (("eslint-disable", re.compile(r"(?m)^\s*//\s*eslint-disable")),),
    ".jsx": (("eslint-disable", re.compile(r"(?m)^\s*//\s*eslint-disable")),),
    ".ts": (
        ("eslint-disable", re.compile(r"(?m)^\s*//\s*eslint-disable")),
        ("typescript-ignore", re.compile(r"(?m)^\s*//\s*@ts-(ignore|nocheck)")),
    ),
    ".tsx": (
        ("eslint-disable", re.compile(r"(?m)^\s*//\s*eslint-disable")),
        ("typescript-ignore", re.compile(r"(?m)^\s*//\s*@ts-(ignore|nocheck)")),
    ),
    ".rs": (
        ("rust allow attribute", re.compile(r"#!?\[\s*allow\b")),
        ("rust expect attribute", re.compile(r"#!?\[\s*expect\b")),
        (
            "rust conditional lint attribute",
            re.compile(r"#!?\[\s*cfg_attr\s*\([^]]*\b(?:allow|expect)\b"),
        ),
    ),
    ".cs": (
        ("csharp pragma warning disable", re.compile(r"#\s*pragma\s+warning\s+disable\b")),
        ("csharp SuppressMessage", re.compile(r"\bSuppressMessage(?:Attribute)?\b")),
    ),
}
PYTHON_BYPASS_PREFIXES = (
    "noqa",
    "type: ignore",
    "type:ignore",
    "ruff: noqa",
    "fmt:",
    "pragma:",
    "pylint:",
    "pyright:",
)
BYPASS_CONFIG_PATTERNS = (
    ("ruff-per-file-ignores", re.compile(r"^\s*(extend-)?per-file-ignores\s*=", re.MULTILINE)),
    ("ruff-extend-ignore", re.compile(r"^\s*extend-ignore\s*=", re.MULTILINE)),
    ("mypy-disable-error-code", re.compile(r"^\s*disable_error_code\s*=", re.MULTILINE)),
    ("mypy-ignore-errors", re.compile(r"^\s*ignore_errors\s*=", re.MULTILINE)),
    ("mypy-ignore-missing", re.compile(r"^\s*ignore_missing_imports\s*=", re.MULTILINE)),
    (
        "mypy-follow-imports-silent",
        re.compile(r'^\s*follow_imports\s*=\s*"?silent"?', re.MULTILINE),
    ),
    (
        "mypy-warn-unused-ignores-false",
        re.compile(r"^\s*warn_unused_ignores\s*=\s*false\b", re.MULTILINE),
    ),
)
CATALOG_BANNED_RUFF_RULES = frozenset({"ANN401", "B006", "BLE001", "E722", "E731", "F403"})
CATALOG_BANNED_RUST_LINTS = frozenset(
    {
        "dbg_macro",
        "downcast_ref",
        "expect_used",
        "panic",
        "static_mut_refs",
        "todo",
        "unimplemented",
        "unnecessary_transmutes",
        "unsafe_code",
        "unwrap_used",
        "warnings",
    }
)
FORBIDDEN_CLIPPY_GROUPS = frozenset({"nursery", "restriction"})
TOML_KEY_TOKEN_RE = re.compile(
    r"""^\s*(?P<key>(?:"(?:[^"\\]|\\.)*"|'[^']*'|[A-Za-z0-9_-]+)"""
    r"""(?:\s*\.\s*(?:"(?:[^"\\]|\\.)*"|'[^']*'|[A-Za-z0-9_-]+))*)\s*="""
)
TOML_STRING_TOKEN_RE = re.compile(r"""(?:"(?:[^"\\]|\\.)*"|'[^']*')""")
REQUIRED_DIRECTORY_BUILD_PROPS: tuple[tuple[str, re.Pattern[str]], ...] = (
    ("Nullable=enable", re.compile(r"<Nullable>\s*enable\s*</Nullable>")),
    (
        "TreatWarningsAsErrors=true",
        re.compile(r"<TreatWarningsAsErrors>\s*true\s*</TreatWarningsAsErrors>", re.IGNORECASE),
    ),
    ("AnalysisMode=All", re.compile(r"<AnalysisMode>\s*All\s*</AnalysisMode>")),
    ("AnalysisLevel=latest-all", re.compile(r"<AnalysisLevel>\s*latest-all\s*</AnalysisLevel>")),
    (
        "EnforceCodeStyleInBuild=true",
        re.compile(r"<EnforceCodeStyleInBuild>\s*true\s*</EnforceCodeStyleInBuild>", re.IGNORECASE),
    ),
)
MSBUILD_WEAKENING_PATTERNS: tuple[tuple[str, re.Pattern[str]], ...] = (
    (
        "TreatWarningsAsErrors set to false",
        re.compile(
            r"""<TreatWarningsAsErrors(?:\s(?:[^<>"']|"[^"<]*"|'[^'<]*')*)?>"""
            r"\s*false\s*</TreatWarningsAsErrors>",
            re.IGNORECASE,
        ),
    ),
    ("NoWarn", re.compile(r"<NoWarn\b")),
    ("WarningsNotAsErrors", re.compile(r"<WarningsNotAsErrors\b")),
    (
        "Nullable weakened",
        re.compile(r"<Nullable>\s*(?:disable|warnings|annotations)\s*</Nullable>", re.IGNORECASE),
    ),
    (
        "EnforceCodeStyleInBuild set to false",
        re.compile(
            r"<EnforceCodeStyleInBuild>\s*false\s*</EnforceCodeStyleInBuild>", re.IGNORECASE
        ),
    ),
    (
        "AnalysisMode weakened",
        re.compile(
            r"<AnalysisMode>\s*(?:None|Default|Minimum|Recommended)\s*</AnalysisMode>",
            re.IGNORECASE,
        ),
    ),
)
COMPILE_REMOVE_RE = re.compile(r'<Compile\s+Remove\s*=\s*"([^"]+)"')
EDITORCONFIG_DOWNGRADE_RE = re.compile(
    r"^\s*(?:(?:dotnet_diagnostic\.[A-Za-z0-9_.-]+|dotnet_analyzer_diagnostic"
    r"(?:\.[A-Za-z0-9_.-]+)?)\.severity\s*=\s*(none|silent|suggestion)"
    r"|[A-Za-z0-9_.]+\s*=\s*[^#\n]*:\s*(none|silent|suggestion))\s*(?:#\s*(.*))?$",
    re.IGNORECASE,
)
BANNED_CALLS = frozenset({"compile", "eval", "exec", "__import__"})
BANNED_BUILTINS = frozenset({"globals", "locals", "vars"})
BROAD_EXCEPTIONS = frozenset({"BaseException", "Exception"})
MUTABLE_DEFAULTS = (ast.Dict, ast.List, ast.Set)
BANNED_SOURCE_PATTERNS: dict[str, tuple[tuple[str, re.Pattern[str]], ...]] = {
    ".rs": (
        ("unsafe block", re.compile(r"(?m)^\s*unsafe\b")),
        ("transmute", re.compile(r"\btransmute\b")),
        ("static mut", re.compile(r"\bstatic\s+mut\b")),
        ("banned allow attribute", re.compile(r"#\[\s*allow\(")),
        ("unwrap outside a test module", re.compile(r"\.unwrap\(\)")),
        ("expect outside a test module", re.compile(r"\.expect\(")),
        ("panic macro", re.compile(r"\bpanic!\s*\(")),
        (
            "any type downcast",
            re.compile(r"\bstd::any::Any\b|\bdyn\s+Any\b|\.downcast_(?:ref|mut)\b"),
        ),
    ),
    ".cs": (
        ("dynamic type", re.compile(r"\bdynamic\b")),
        ("unsafe block", re.compile(r"\bunsafe\b")),
        ("pragma warning disable", re.compile(r"#\s*pragma\s+warning\s+disable")),
        ("reflection API", re.compile(r"\bSystem\.Reflection\b|\bGetType\(\)")),
        (
            "raw generic type",
            re.compile(
                r"(?m)\b(?:List|Dictionary|IEnumerable|IList|ICollection|ISet|HashSet|Queue|Stack|ObservableCollection)\b(?!\s*<)"
            ),
        ),
        ("unchecked", re.compile(r"(?m)\bunchecked\b")),
        ("object parameter", re.compile(r"(?m)\(\s*(?:System\.)?object\s+\w+")),
    ),
}
MAKE_TARGET_RE = re.compile(r"(?m)^([A-Za-z0-9_.-]+):(?!=)")
BUDGET_RE = re.compile(r"(?m)^VERIFY_BUDGET_SECONDS\s*:?=\s*(\d+)")
BUDGET_FACT_RE = re.compile(r"^RALPH-FACT:\s*verification_time_budget:\s*(\d+)", re.MULTILINE)
SECRET_SCAN_EXCLUDES = (
    r"^\.agent/",
    r"^\.opencode/",
    r"^\.mypy_cache/",
    r"^\.pytest_cache/",
    r"^\.ruff_cache/",
    r"^\.secrets\.baseline$",
    r"(^|/)target/",
    r"^checkpoint\.json$",
    r"(^|/)bin/",
    r"(^|/)obj/",
)


def read_policy_corpus() -> dict[str, str]:
    return {name: (POLICY_DIR / name).read_text(encoding="utf-8") for name in POLICY_FILES}


def declared_commands(corpus: dict[str, str]) -> list[str]:
    values: list[str] = []
    for text in corpus.values():
        for line in text.splitlines():
            match = COMMAND_PATTERN.match(line)
            if match is not None:
                values.append(match.group(1).strip())
    return values


def check_placeholders(corpus: dict[str, str]) -> list[str]:
    findings: list[str] = []
    for name, text in corpus.items():
        for token in PLACEHOLDER_TOKENS:
            if token in text:
                findings.append(f"{name}: placeholder token {token!r} remains")
                break
    return findings


def check_fact_keys(corpus: dict[str, str]) -> list[str]:
    findings: list[str] = []
    for name, text in corpus.items():
        keys = [
            match.group(1).strip()
            for line in text.splitlines()
            if (match := FACT_PATTERN.match(line)) is not None
        ]
        if not keys:
            findings.append(f"{name}: declares no resolved RALPH-FACT line")
        findings.extend(
            f"{name}: RALPH-FACT key {key!r} occurs {keys.count(key)} times"
            for key in sorted(set(keys))
            if keys.count(key) != 1
        )
    return findings


def check_commands(corpus: dict[str, str]) -> list[str]:
    findings: list[str] = []
    for name, text in corpus.items():
        for index, line in enumerate(text.splitlines(), start=1):
            match = COMMAND_PATTERN.match(line)
            if match is None:
                continue
            value = match.group(1).strip()
            if not value:
                findings.append(f"{name}:{index}: RALPH-COMMAND is empty")
                continue
            tool = value.split()[0]
            if tool not in APPROVED_GATE_TOOLS:
                findings.append(f"{name}:{index}: RALPH-COMMAND tool {tool!r} is not approved")
    return findings


def make_targets() -> set[str]:
    text = MAKEFILE.read_text(encoding="utf-8")
    return {match.group(1) for match in MAKE_TARGET_RE.finditer(text)}


def check_make_targets(corpus: dict[str, str]) -> list[str]:
    findings: list[str] = []
    targets = make_targets()
    for name, text in corpus.items():
        for index, line in enumerate(text.splitlines(), start=1):
            match = COMMAND_PATTERN.match(line)
            if match is None:
                continue
            value = match.group(1).strip()
            parts = value.split()
            if len(parts) < MAKE_COMMAND_MIN_PARTS or parts[0] != "make":
                continue
            if parts[1] not in targets:
                findings.append(
                    f"{name}:{index}: RALPH-COMMAND names unknown gate target {parts[1]!r}"
                )
    return findings


def check_budget_consistency() -> list[str]:
    findings: list[str] = []
    declared = BUDGET_RE.search(MAKEFILE.read_text(encoding="utf-8"))
    if declared is None:
        return ["the Makefile declares no VERIFY_BUDGET_SECONDS"]
    recorded = BUDGET_FACT_RE.search(
        (POLICY_DIR / "verification-policy.md").read_text(encoding="utf-8")
    )
    if recorded is None:
        return ["verification-policy.md records no verification_time_budget"]
    if declared.group(1) != recorded.group(1):
        findings.append(
            f"the Makefile budget {declared.group(1)}s differs from the recorded "
            f"verification_time_budget {recorded.group(1)}s"
        )
    return findings


def guard_rows(text: str) -> list[list[str]]:
    lines = text.splitlines()
    start = -1
    for index, line in enumerate(lines):
        if line.strip() == "## Guard inventory":
            start = index
            break
    if start == -1:
        return []
    rows: list[list[str]] = []
    for line in lines[start + 1 :]:
        stripped = line.strip()
        if stripped.startswith("#"):
            break
        if stripped.startswith("|"):
            rows.append([cell.strip() for cell in stripped.strip("|").split("|")])
        elif rows:
            break
    return rows[2:]


def check_guard_enforcement(corpus: dict[str, str], commands: Sequence[str]) -> list[str]:
    findings: list[str] = []
    known = set(commands)
    for name, text in corpus.items():
        for index, cells in enumerate(guard_rows(text), start=1):
            if len(cells) != GUARD_CELL_COUNT:
                findings.append(f"{name}: guard row {index} has {len(cells)} cells")
                continue
            enforcement = cells[GUARD_ENFORCEMENT_INDEX]
            deterministic = DETERMINISTIC_PATTERN.match(enforcement)
            if deterministic is not None:
                if deterministic.group(1) not in known:
                    findings.append(
                        f"{name}: guard row {index} references unknown enforcer "
                        f"{deterministic.group(1)!r}"
                    )
                continue
            if ADVISORY_PATTERN.match(enforcement) is None:
                findings.append(f"{name}: guard row {index} has malformed enforcement")
    return findings


def portfolio_controls() -> list[dict[str, object]]:
    parsed = tomllib.loads(PORTFOLIO_PATH.read_text(encoding="utf-8"))
    raw = parsed.get("controls", [])
    if not isinstance(raw, list):
        return []
    return [item for item in raw if isinstance(item, dict)]


def check_portfolio(commands: Sequence[str]) -> list[str]:
    content = PORTFOLIO_PATH.read_text(encoding="utf-8")
    findings = [
        f"policy-portfolio.toml: placeholder token {token!r} remains"
        for token in PLACEHOLDER_TOKENS
        if token in content
    ]
    known = set(commands)
    for control in portfolio_controls():
        control_id = control.get("id", "unknown")
        enforcement = control.get("enforcement")
        if not isinstance(enforcement, str):
            findings.append(
                f"policy-portfolio.toml: control {control_id!r} declares no enforcement"
            )
            continue
        deterministic = DETERMINISTIC_PATTERN.match(enforcement.strip())
        if deterministic is not None:
            if deterministic.group(1) not in known:
                findings.append(
                    f"policy-portfolio.toml: control {control_id!r} references unknown "
                    f"enforcer {deterministic.group(1)!r}"
                )
            continue
        if ADVISORY_PATTERN.match(enforcement.strip()) is None:
            findings.append(
                f"policy-portfolio.toml: control {control_id!r} enforcement is malformed"
            )
    return findings


def check_agents_md() -> list[str]:
    findings: list[str] = []
    content = AGENTS_MD.read_text(encoding="utf-8")
    begin = content.count("<!-- ralph-workflow-policy:begin v3 -->")
    end = content.count("<!-- ralph-workflow-policy:end -->")
    if begin != 1:
        findings.append(f"AGENTS.md declares {begin} managed block beginnings")
    if end != 1:
        findings.append(f"AGENTS.md declares {end} managed block endings")
    if begin == 1 and end == 1:
        start = content.index("<!-- ralph-workflow-policy:begin v3 -->")
        stop = content.index("<!-- ralph-workflow-policy:end -->")
        body = content[start:stop]
        if "docs/ralph-workflow-policy/" not in body:
            findings.append("AGENTS.md managed block does not cite the canonical policy directory")
    return findings


def check_links(corpus: dict[str, str]) -> list[str]:
    findings: list[str] = []
    for name, text in corpus.items():
        for token in LINK_PATTERN.findall(text):
            if not PATH_TOKEN_PATTERN.match(token):
                continue
            if "/" not in token and token not in ROOT_CITED_FILES:
                continue
            if "/" in token and not token.startswith(REPOSITORY_PATH_ROOTS):
                continue
            if (REPO_ROOT / token).exists():
                continue
            if (POLICY_DIR / token).exists():
                continue
            findings.append(f"{name}: cited path {token!r} does not resolve")
    return findings


def check_applicability() -> list[str]:
    findings: list[str] = []
    if not APPLICABILITY_PATH.exists():
        return findings
    seen: set[str] = set()
    for index, raw_line in enumerate(
        APPLICABILITY_PATH.read_text(encoding="utf-8").splitlines(), start=1
    ):
        line = raw_line.strip()
        if not line or line.startswith("#") or line == "[domains]":
            continue
        match = APPLICABILITY_PATTERN.fullmatch(line)
        if match is None:
            findings.append(f"applicability.toml:{index}: malformed applicability decision")
            continue
        domain = match.group(1)
        if domain not in CONDITIONAL_DOMAINS:
            findings.append(f"applicability.toml:{index}: unknown conditional domain {domain!r}")
        if domain in seen:
            findings.append(f"applicability.toml:{index}: duplicate decision for {domain!r}")
        seen.add(domain)
    return findings


def check_policy() -> list[str]:
    corpus = read_policy_corpus()
    commands = declared_commands(corpus)
    findings: list[str] = []
    findings.extend(check_placeholders(corpus))
    findings.extend(check_fact_keys(corpus))
    findings.extend(check_commands(corpus))
    findings.extend(check_guard_enforcement(corpus, commands))
    findings.extend(check_portfolio(commands))
    findings.extend(check_make_targets(corpus))
    findings.extend(check_budget_consistency())
    findings.extend(check_applicability())
    findings.extend(check_agents_md())
    findings.extend(check_links(corpus))
    return findings


def iter_source_files(root: Path = REPO_ROOT) -> Iterator[Path]:
    for walk_root, dirnames, filenames in os.walk(root):
        dirnames[:] = [
            name
            for name in sorted(dirnames)
            if not name.startswith(".") and name not in SKIP_DIR_NAMES
        ]
        for filename in sorted(filenames):
            path = Path(walk_root) / filename
            if path.suffix in SOURCE_SUFFIXES:
                yield path


def display_path(path: Path, root: Path = REPO_ROOT) -> str:
    try:
        return str(path.relative_to(root))
    except ValueError:
        return str(path)


def iter_named_files(
    root: Path, names: frozenset[str], suffixes: tuple[str, ...] = ()
) -> Iterator[Path]:
    for walk_root, dirnames, filenames in os.walk(root):
        dirnames[:] = [
            name
            for name in sorted(dirnames)
            if name not in SKIP_DIR_NAMES and (name in names or not name.startswith("."))
        ]
        for filename in sorted(filenames):
            path = Path(walk_root) / filename
            if filename in names or (suffixes and path.suffix in suffixes):
                yield path


def python_comment_tokens(path: Path) -> list[tuple[int, str]]:
    source = path.read_text(encoding="utf-8")
    try:
        tokens = list(tokenize.generate_tokens(iter(source.splitlines(keepends=True)).__next__))
    except (tokenize.TokenError, IndentationError, SyntaxError):
        return []
    return [(token.start[0], token.string) for token in tokens if token.type == tokenize.COMMENT]


def bypass_findings_in(path: Path, root: Path = REPO_ROOT) -> list[str]:
    relative = display_path(path, root)
    findings: list[str] = []
    if path.name == "GlobalSuppressions.cs":
        findings.append(f"{relative}:1: GlobalSuppressions.cs suppression file")
    if path.suffix == ".py":
        for lineno, comment in python_comment_tokens(path):
            body = comment.lstrip("#").strip().lower()
            for prefix in PYTHON_BYPASS_PREFIXES:
                if body.startswith(prefix):
                    findings.append(f"{relative}:{lineno}: python {prefix} suppression marker")
                    break
        return findings
    patterns = BYPASS_SOURCE_PATTERNS.get(path.suffix, ())
    if not patterns:
        return findings
    text = path.read_text(encoding="utf-8", errors="replace")
    for label, pattern in patterns:
        for match in pattern.finditer(text):
            line = text.count("\n", 0, match.start()) + 1
            findings.append(f"{relative}:{line}: {label} suppression marker")
    return findings


def has_preceding_reason_comment(lines: Sequence[str], index: int) -> bool:
    cursor = index - 1
    while cursor >= 0:
        stripped = lines[cursor].strip()
        if not stripped:
            return False
        if stripped.startswith("#"):
            return bool(stripped.lstrip("#").strip())
        return False
    return False


def toml_index_outside_string(text: str, stop: str) -> int | None:
    quote = ""
    index = 0
    while index < len(text):
        char = text[index]
        if quote:
            if char == "\\" and quote == '"':
                index += 2
                continue
            if char == quote:
                quote = ""
            index += 1
            continue
        if char in "\"'":
            quote = char
            index += 1
            continue
        if char in stop:
            return index
        index += 1
    return None


def decode_toml_key_path(token: str) -> list[str]:
    try:
        parsed = tomllib.loads(f"{token} = 0\n")
    except tomllib.TOMLDecodeError:
        return []
    path: list[str] = []
    current: object = parsed
    while isinstance(current, dict) and len(current) == 1:
        key, value = next(iter(current.items()))
        path.append(key)
        current = value
    return path


def decode_toml_string(token: str) -> str | None:
    try:
        parsed = tomllib.loads(f"k = {token}\n")
    except tomllib.TOMLDecodeError:
        return None
    value = parsed.get("k")
    return value if isinstance(value, str) else None


def decode_toml_strings(text: str) -> list[str]:
    values: list[str] = []
    for match in TOML_STRING_TOKEN_RE.finditer(text):
        value = decode_toml_string(match.group(0))
        if value is not None:
            values.append(value)
    return values


def toml_value_text(value: str) -> str:
    match = TOML_STRING_TOKEN_RE.fullmatch(value.strip())
    if match is None:
        return value
    decoded = decode_toml_string(match.group(0))
    if decoded is None:
        return value
    escaped = decoded.replace("\\", "\\\\").replace('"', '\\"')
    return f'"{escaped}"'


def check_ruff_ignore_reasons(text: str, display: str) -> list[str]:
    findings: list[str] = []
    lines = text.splitlines()
    in_ignore = False
    for index, raw_line in enumerate(lines):
        comment_index = toml_index_outside_string(raw_line, "#")
        code = raw_line if comment_index is None else raw_line[:comment_index]
        comment = "" if comment_index is None else raw_line[comment_index + 1 :]
        if not in_ignore:
            start = TOML_KEY_TOKEN_RE.match(code)
            if start is None:
                continue
            path = decode_toml_key_path(start.group("key"))
            if not path or path[-1] != "ignore":
                continue
            code = code[start.end() :].lstrip()
            if not code.startswith("["):
                continue
        close_index = toml_index_outside_string(code, "]")
        items = code if close_index is None else code[:close_index]
        in_ignore = close_index is None
        if not comment.strip() and not has_preceding_reason_comment(lines, index):
            findings.extend(
                f"{display}:{index + 1}: ruff ignore entry {rule!r} lacks a reason comment"
                for rule in decode_toml_strings(items)
            )
    return findings


def check_pyproject_lint_table(lint: dict[str, object], text: str, display: str) -> list[str]:
    findings: list[str] = []
    select = lint.get("select")
    if not isinstance(select, list) or "ALL" not in select:
        findings.append(f"{display}: ruff select must contain 'ALL'")
    findings.extend(
        f"{display}: ruff {key} configuration bypass"
        for key in ("per-file-ignores", "extend-per-file-ignores", "extend-ignore")
        if key in lint
    )
    ignore = lint.get("ignore")
    if ignore is not None:
        if not isinstance(ignore, list):
            findings.append(f"{display}: ruff ignore must be a list")
        else:
            findings.extend(
                f"{display}: ruff ignore weakens catalog-banned rule {item!r}"
                for item in ignore
                if isinstance(item, str) and item in CATALOG_BANNED_RUFF_RULES
            )
            findings.extend(check_ruff_ignore_reasons(text, display))
    return findings


def check_lint_configuration(root: Path = REPO_ROOT) -> list[str]:
    path = root / "pyproject.toml"
    display = display_path(path, root)
    if not path.exists():
        return [f"{display} is missing"]
    text = path.read_text(encoding="utf-8")
    try:
        parsed = tomllib.loads(text)
    except tomllib.TOMLDecodeError as error:
        return [f"{display}: invalid TOML: {error}"]
    tool = parsed.get("tool", {})
    if not isinstance(tool, dict):
        return [f"{display} declares no tool table"]
    ruff_table = tool.get("ruff", {})
    if not isinstance(ruff_table, dict):
        return [f"{display} declares no ruff configuration"]
    lint = ruff_table.get("lint", {})
    if not isinstance(lint, dict):
        return [f"{display} declares no ruff lint configuration"]
    findings = check_pyproject_lint_table(lint, text, display)
    mypy_table = tool.get("mypy")
    if isinstance(mypy_table, dict) and mypy_table.get("strict") is not True:
        findings.append(f"{display}: mypy strict must be true")
    return findings


def rust_lint_level(entry: object) -> str:
    if isinstance(entry, str):
        return entry
    if isinstance(entry, dict):
        level = entry.get("level")
        if isinstance(level, str):
            return level
    return ""


def toml_key_lines(text: str) -> list[tuple[int, str, str]]:
    entries: list[tuple[int, str, str]] = []
    for index, raw_line in enumerate(text.splitlines()):
        match = TOML_KEY_TOKEN_RE.match(raw_line)
        if match is None:
            continue
        path = decode_toml_key_path(match.group("key"))
        if path:
            entries.append((index, path[-1], toml_value_text(raw_line[match.end() :])))
    return entries


def is_toml_key_line(text: str) -> bool:
    match = TOML_KEY_TOKEN_RE.match(text)
    return match is not None and bool(decode_toml_key_path(match.group("key")))


def toml_key_line_map(text: str) -> dict[str, int]:
    mapping: dict[str, int] = {}
    for index, key, _ in toml_key_lines(text):
        mapping.setdefault(key, index)
    return mapping


def toml_entry_has_reason(lines: Sequence[str], line_index: int) -> bool:
    raw_line = lines[line_index]
    comment_index = toml_index_outside_string(raw_line, "#")
    if comment_index is not None and raw_line[comment_index + 1 :].strip():
        return True
    cursor = line_index - 1
    while cursor >= 0:
        stripped = lines[cursor].strip()
        if not stripped:
            return False
        if stripped.startswith("#"):
            if stripped.lstrip("#").strip():
                return True
            cursor -= 1
            continue
        if is_toml_key_line(stripped):
            cursor -= 1
            continue
        return False
    return False


def check_rust_lint_table(
    table: dict[str, object],
    text: str,
    display: str,
    table_name: str,
) -> list[str]:
    findings: list[str] = []
    lines = text.splitlines()
    key_lines = toml_key_line_map(text)
    for key, value in table.items():
        level = rust_lint_level(value)
        line_index = key_lines.get(key, 0)
        lineno = line_index + 1
        if table_name == "clippy" and key in FORBIDDEN_CLIPPY_GROUPS and level != "allow":
            findings.append(
                f"{display}:{lineno}: clippy group {key!r} must not be enabled as a whole group"
            )
        if level in ("allow", "warn"):
            if key in CATALOG_BANNED_RUST_LINTS:
                findings.append(f"{display}:{lineno}: catalog-banned lint {key!r} set to {level!r}")
            elif not toml_entry_has_reason(lines, line_index):
                findings.append(
                    f"{display}:{lineno}: lint {key!r} set to {level!r} lacks a reason comment"
                )
    return findings


def check_workspace_cargo_toml(
    workspace: dict[str, object],
    text: str,
    display: str,
) -> tuple[list[str], set[str]]:
    findings: list[str] = []
    members_raw = workspace.get("members", [])
    members = (
        {item for item in members_raw if isinstance(item, str)}
        if isinstance(members_raw, list)
        else set()
    )
    lints = workspace.get("lints")
    if not isinstance(lints, dict):
        findings.append(f"{display}: workspace declares no [workspace.lints] table")
        return findings, members
    rust_lints = lints.get("rust")
    if not isinstance(rust_lints, dict):
        findings.append(f"{display}: workspace declares no [workspace.lints.rust] table")
    else:
        findings.extend(
            f"{display}: [workspace.lints.rust] must set {lint} = 'deny'"
            for lint in ("warnings", "unnecessary_transmutes", "static_mut_refs")
            if rust_lint_level(rust_lints.get(lint)) != "deny"
        )
        if rust_lint_level(rust_lints.get("unsafe_code")) != "forbid":
            findings.append(f"{display}: [workspace.lints.rust] must set unsafe_code = 'forbid'")
        findings.extend(check_rust_lint_table(rust_lints, text, display, "rust"))
    clippy_lints = lints.get("clippy")
    if not isinstance(clippy_lints, dict):
        findings.append(f"{display}: workspace declares no [workspace.lints.clippy] table")
    else:
        for group in ("all", "pedantic"):
            entry = clippy_lints.get(group)
            if rust_lint_level(entry) != "deny":
                findings.append(f"{display}: [workspace.lints.clippy] must set {group} = 'deny'")
                continue
            priority = entry.get("priority") if isinstance(entry, dict) else None
            if not isinstance(priority, int) or priority != -1:
                findings.append(
                    f"{display}: [workspace.lints.clippy] must set {group} priority = -1"
                )
        denies = ("unwrap_used", "expect_used", "panic", "todo", "unimplemented", "dbg_macro")
        findings.extend(
            f"{display}: [workspace.lints.clippy] must set {lint} = 'deny'"
            for lint in denies
            if rust_lint_level(clippy_lints.get(lint)) != "deny"
        )
        findings.extend(check_rust_lint_table(clippy_lints, text, display, "clippy"))
    return findings, members


def load_unsafe_zone_paths(root: Path = REPO_ROOT) -> set[str]:
    registry = root / UNSAFE_ZONES_REGISTRY
    if not registry.exists():
        return set()
    try:
        parsed = tomllib.loads(registry.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError):
        return set()
    zones = parsed.get("zone", [])
    if not isinstance(zones, list):
        return set()
    return {
        zone["path"]
        for zone in zones
        if isinstance(zone, dict) and isinstance(zone.get("path"), str)
    }


def check_zone_cargo_toml(
    parsed: dict[str, object],
    text: str,
    display: str,
) -> list[str]:
    findings: list[str] = []
    package = parsed.get("package")
    edition = package.get("edition") if isinstance(package, dict) else None
    if edition != "2024":
        findings.append(f"{display}: zone crate must set package edition = '2024'")
    lints = parsed.get("lints")
    if not isinstance(lints, dict) or lints.get("workspace") is True:
        findings.append(
            f"{display}: zone crate must declare its own [lints.rust] and [lints.clippy] tables"
        )
        return findings
    rust_lints = lints.get("rust")
    if not isinstance(rust_lints, dict):
        findings.append(f"{display}: zone crate declares no [lints.rust] table")
    else:
        findings.extend(
            f"{display}: zone [lints.rust] must set {lint} = 'deny'"
            for lint in ZONE_RUST_DENIES
            if rust_lint_level(rust_lints.get(lint)) != "deny"
        )
        if rust_lint_level(rust_lints.get("unsafe_code")) == "forbid":
            findings.append(f"{display}: zone [lints.rust] must not forbid unsafe_code")
        findings.extend(check_rust_lint_table(rust_lints, text, display, "rust"))
    clippy_lints = lints.get("clippy")
    if not isinstance(clippy_lints, dict):
        findings.append(f"{display}: zone crate declares no [lints.clippy] table")
    else:
        for group in ("all", "pedantic"):
            entry = clippy_lints.get(group)
            if rust_lint_level(entry) != "deny":
                findings.append(f"{display}: zone [lints.clippy] must set {group} = 'deny'")
                continue
            priority = entry.get("priority") if isinstance(entry, dict) else None
            if not isinstance(priority, int) or priority != -1:
                findings.append(f"{display}: zone [lints.clippy] must set {group} priority = -1")
        findings.extend(
            f"{display}: zone [lints.clippy] must set {lint} = 'deny'"
            for lint in ZONE_CLIPPY_DENIES
            if rust_lint_level(clippy_lints.get(lint)) != "deny"
        )
        findings.extend(check_rust_lint_table(clippy_lints, text, display, "clippy"))
    return findings


def check_member_cargo_toml(
    parsed: dict[str, object],
    text: str,
    display: str,
    zones: set[str] | None = None,
) -> list[str]:
    crate_dir = display.rsplit("/", 1)[0] if "/" in display else ""
    if zones and crate_dir in zones:
        return check_zone_cargo_toml(parsed, text, display)
    findings: list[str] = []
    lints = parsed.get("lints")
    if lints != {"workspace": True}:
        findings.append(f"{display}: member crate must set [lints] workspace = true")
    if isinstance(lints, dict):
        for table_name in ("rust", "clippy"):
            subtable = lints.get(table_name)
            if isinstance(subtable, dict):
                findings.extend(check_rust_lint_table(subtable, text, display, table_name))
    return findings


def check_rust_tool_configs(root: Path) -> list[str]:
    findings: list[str] = []
    config_names = frozenset({"clippy.toml", ".clippy.toml", "rustfmt.toml", ".rustfmt.toml"})
    for config_path in sorted(iter_named_files(root, config_names)):
        display = display_path(config_path, root)
        text = config_path.read_text(encoding="utf-8")
        try:
            parsed = tomllib.loads(text)
        except tomllib.TOMLDecodeError as error:
            findings.append(f"{display}: invalid TOML: {error}")
            continue
        if parsed.get("disable_all_formatting") is True:
            findings.append(f"{display}: disable_all_formatting bypass is forbidden")
        findings.extend(
            f"{display}: {key} bypass is forbidden"
            for key in ("allow-expect-in-tests", "allow-unwrap-in-tests", "allow-panic-in-tests")
            if parsed.get(key) is True
        )
    return findings


def check_rust_configuration(root: Path = REPO_ROOT) -> list[str]:
    findings: list[str] = []
    zones = load_unsafe_zone_paths(root)
    cargo_files = sorted(iter_named_files(root, frozenset({"Cargo.toml"})))
    expected_members: set[Path] = set()
    for path in cargo_files:
        display = display_path(path, root)
        text = path.read_text(encoding="utf-8")
        try:
            parsed = tomllib.loads(text)
        except tomllib.TOMLDecodeError as error:
            findings.append(f"{display}: invalid TOML: {error}")
            continue
        workspace = parsed.get("workspace")
        if isinstance(workspace, dict):
            ws_findings, members = check_workspace_cargo_toml(workspace, text, display)
            findings.extend(ws_findings)
            expected_members.update(
                (path.parent / member / "Cargo.toml").resolve() for member in members
            )
        if "package" in parsed:
            findings.extend(check_member_cargo_toml(parsed, text, display, zones))
    findings.extend(
        f"{display_path(member_manifest, root)}: workspace member Cargo.toml is missing"
        for member_manifest in sorted(expected_members)
        if not member_manifest.exists()
    )
    findings.extend(check_rust_tool_configs(root))
    return findings


def check_msbuild_file(path: Path, root: Path, *, is_csproj: bool) -> list[str]:
    display = display_path(path, root)
    text = path.read_text(encoding="utf-8")
    findings: list[str] = []
    for label, pattern in MSBUILD_WEAKENING_PATTERNS:
        for match in pattern.finditer(text):
            line = text.count("\n", 0, match.start()) + 1
            findings.append(f"{display}:{line}: {label} configuration bypass")
    if is_csproj:
        for match in COMPILE_REMOVE_RE.finditer(text):
            removed = match.group(1)
            if not removed.startswith("Fixtures"):
                line = text.count("\n", 0, match.start()) + 1
                findings.append(
                    f"{display}:{line}: Compile Remove={removed!r} excludes non-fixture source"
                )
    return findings


def check_editorconfig_files(root: Path) -> list[str]:
    findings: list[str] = []
    for config_path in sorted(
        iter_named_files(root, frozenset({".editorconfig", ".globalconfig"}))
    ):
        display = display_path(config_path, root)
        lines = config_path.read_text(encoding="utf-8").splitlines()
        for index, raw_line in enumerate(lines):
            stripped = raw_line.strip()
            if not stripped or stripped.startswith("#"):
                continue
            match = EDITORCONFIG_DOWNGRADE_RE.match(raw_line)
            if match is None:
                continue
            severity = match.group(1) or match.group(2)
            inline_reason = (match.group(3) or "").strip()
            if not inline_reason and not has_preceding_reason_comment(lines, index):
                findings.append(
                    f"{display}:{index + 1}: analyzer severity downgrade to {severity!r} "
                    "lacks a reason comment"
                )
    return findings


def check_csharp_configuration(root: Path = REPO_ROOT) -> list[str]:
    findings: list[str] = []
    props_files = sorted(iter_named_files(root, frozenset({"Directory.Build.props"})))
    csproj_files = sorted(iter_named_files(root, frozenset(), (".csproj",)))
    root_props = root / "Directory.Build.props"
    if csproj_files and not root_props.exists():
        findings.append("Directory.Build.props: missing root Directory.Build.props for C# projects")
    if root_props.exists():
        props_text = root_props.read_text(encoding="utf-8")
        props_display = display_path(root_props, root)
        findings.extend(
            f"{props_display}: missing required {label}"
            for label, pattern in REQUIRED_DIRECTORY_BUILD_PROPS
            if pattern.search(props_text) is None
        )
    for path in props_files:
        findings.extend(check_msbuild_file(path, root, is_csproj=False))
    for path in csproj_files:
        findings.extend(check_msbuild_file(path, root, is_csproj=True))
    findings.extend(check_editorconfig_files(root))
    return findings


def check_bypass(root: Path = REPO_ROOT) -> list[str]:
    findings: list[str] = []
    for path in iter_source_files(root):
        findings.extend(bypass_findings_in(path, root))
    for name in CONFIG_FILES:
        path = root / name
        if not path.exists():
            continue
        display = display_path(path, root)
        text = path.read_text(encoding="utf-8")
        key_lines = toml_key_lines(text)
        for label, pattern in BYPASS_CONFIG_PATTERNS:
            for index, key, value in key_lines:
                if pattern.search(f"{key} = {value}") is not None:
                    findings.append(f"{display}:{index + 1}: {label} configuration bypass")
    findings.extend(check_lint_configuration(root))
    findings.extend(check_rust_configuration(root))
    findings.extend(check_csharp_configuration(root))
    return findings


def mutable_default_findings(
    node: ast.FunctionDef | ast.AsyncFunctionDef, display: str
) -> list[str]:
    defaults = list(node.args.defaults) + [
        default for default in node.args.kw_defaults if default is not None
    ]
    return [
        f"{display}:{node.lineno}: mutable default argument"
        for default in defaults
        if isinstance(default, MUTABLE_DEFAULTS)
    ]


def banned_call_findings(node: ast.Call, relative: str) -> list[str]:
    findings: list[str] = []
    if isinstance(node.func, ast.Name):
        if node.func.id in BANNED_CALLS:
            findings.append(f"{relative}:{node.lineno}: banned call {node.func.id}")
        if node.func.id == "print":
            findings.append(f"{relative}:{node.lineno}: print in library code")
    elif isinstance(node.func, ast.Attribute) and isinstance(node.func.value, ast.Name):
        owner = node.func.value
        if node.func.attr == "cast" and owner.id == "typing":
            findings.append(f"{relative}:{node.lineno}: typing.cast")
        if node.func.attr == "environ" and owner.id == "os":
            findings.append(f"{relative}:{node.lineno}: os.environ outside configuration")
    return findings


def banned_except_findings(node: ast.ExceptHandler, relative: str) -> list[str]:
    if node.type is None:
        return [f"{relative}:{node.lineno}: bare except"]
    if isinstance(node.type, ast.Name) and node.type.id in BROAD_EXCEPTIONS:
        return [f"{relative}:{node.lineno}: broad except"]
    return []


def is_named_lambda(node: ast.AST) -> bool:
    return (
        isinstance(node, ast.Assign)
        and isinstance(node.value, ast.Lambda)
        and isinstance(node.targets[0], ast.Name)
    )


def banned_node_findings(node: ast.AST, relative: str) -> list[str]:
    findings: list[str] = []
    if isinstance(node, ast.Call):
        findings.extend(banned_call_findings(node, relative))
    elif isinstance(node, ast.Attribute) and node.attr == "Any":
        findings.append(f"{relative}:{node.lineno}: Any annotation")
    elif isinstance(node, ast.Name) and node.id in BANNED_BUILTINS:
        findings.append(f"{relative}:{node.lineno}: banned namespace access {node.id}")
    elif isinstance(node, ast.Assert):
        findings.append(f"{relative}:{node.lineno}: assert used outside a test module")
    elif isinstance(node, ast.ImportFrom) and any(alias.name == "*" for alias in node.names):
        findings.append(f"{relative}:{node.lineno}: star import")
    elif isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
        findings.extend(mutable_default_findings(node, relative))
    elif isinstance(node, ast.ExceptHandler):
        findings.extend(banned_except_findings(node, relative))
    elif is_named_lambda(node) and isinstance(node, ast.Assign):
        findings.append(f"{relative}:{node.lineno}: named lambda")
    return findings


def banned_findings(display: str, tree: ast.Module) -> list[str]:
    findings: list[str] = []
    for node in ast.walk(tree):
        findings.extend(banned_node_findings(node, display))
    return findings


def banned_source_findings(path: Path) -> list[str]:
    relative = display_path(path)
    patterns = BANNED_SOURCE_PATTERNS.get(path.suffix, ())
    if not patterns:
        return []
    text = path.read_text(encoding="utf-8", errors="replace")
    findings: list[str] = []
    for label, pattern in patterns:
        for match in pattern.finditer(text):
            line = text.count("\n", 0, match.start()) + 1
            findings.append(f"{relative}:{line}: {label}")
    return findings


def check_banned(root: Path = REPO_ROOT) -> list[str]:
    findings: list[str] = []
    for path in iter_source_files(root):
        relative = display_path(path)
        if path.suffix == ".py":
            if path.name.startswith("test_"):
                continue
            tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
            findings.extend(banned_findings(str(relative), tree))
            continue
        findings.extend(banned_source_findings(path))
    return findings


def python_docstrings(tree: ast.Module) -> list[tuple[int, str]]:
    found: list[tuple[int, str]] = []
    holders = (ast.Module, ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef)
    for node in ast.walk(tree):
        if not isinstance(node, holders):
            continue
        body = getattr(node, "body", [])
        if not body:
            continue
        first = body[0]
        if isinstance(first, ast.Expr) and isinstance(first.value, ast.Constant):
            value = first.value.value
            if isinstance(value, str):
                found.append((first.lineno, "docstring"))
    return found


def strip_quoted(text: str) -> str:
    out: list[str] = []
    quote = ""
    for char in text:
        if quote:
            if char == quote:
                quote = ""
            continue
        if char in ("'", '"'):
            quote = char
            continue
        out.append(char)
    return "".join(out)


def python_comment_findings_in(path: Path, relative: str) -> list[str]:
    findings: list[str] = []
    source = path.read_text(encoding="utf-8")
    try:
        tokens = list(tokenize.generate_tokens(iter(source.splitlines(keepends=True)).__next__))
    except (tokenize.TokenError, IndentationError, SyntaxError):
        return findings
    for token in tokens:
        if token.type != tokenize.COMMENT:
            continue
        if token.start[0] == 1 and token.string.startswith("#!"):
            continue
        findings.append(f"{relative}:{token.start[0]}: comment token")
    tree = ast.parse(source, filename=str(path))
    for lineno, label in python_docstrings(tree):
        findings.append(f"{relative}:{lineno}: {label}")
    return findings


def comment_findings_in(path: Path, root: Path = REPO_ROOT) -> list[str]:
    relative = display_path(path, root)
    if path.suffix == ".py":
        return python_comment_findings_in(path, relative)
    if path.suffix not in COMMENT_SUFFIXES and path.name != "Makefile":
        return []
    findings: list[str] = []
    for index, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        stripped = strip_quoted(line).strip()
        if not stripped.startswith("#"):
            continue
        if index == 1 and stripped.startswith("#!"):
            continue
        findings.append(f"{relative}:{index}: comment token")
    return findings


def check_comments(root: Path = REPO_ROOT) -> list[str]:
    findings: list[str] = []
    for path in iter_source_files(root):
        findings.extend(comment_findings_in(path, root))
    makefile = root / "Makefile"
    if makefile.exists():
        findings.extend(comment_findings_in(makefile, root))
    return findings


def local_package_names() -> frozenset[str]:
    names = {
        path.name
        for path in REPO_ROOT.iterdir()
        if path.is_dir() and (path / "__init__.py").exists()
    }
    return frozenset(names)


def python_import_findings(path: Path, allowed: frozenset[str]) -> list[str]:
    findings: list[str] = []
    relative = path.relative_to(REPO_ROOT)
    tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            for alias in node.names:
                root = alias.name.split(".")[0]
                if root not in sys.stdlib_module_names and root not in allowed:
                    findings.append(f"{relative}:{node.lineno}: third-party import {alias.name}")
        elif isinstance(node, ast.ImportFrom) and node.level == 0 and node.module:
            root = node.module.split(".")[0]
            if root not in sys.stdlib_module_names and root not in allowed:
                findings.append(f"{relative}:{node.lineno}: third-party import {node.module}")
    return findings


def check_dependencies() -> list[str]:
    findings: list[str] = []
    allowed = local_package_names()
    for path in iter_source_files():
        if path.suffix == ".py":
            findings.extend(python_import_findings(path, allowed))
    for manifest, lock in (("Cargo.toml", "Cargo.lock"), ("package.json", "package-lock.json")):
        if (REPO_ROOT / manifest).exists() != (REPO_ROOT / lock).exists():
            findings.append(f"{manifest} and {lock} must be committed together")
    findings.extend(check_dependency_registry())
    findings.extend(check_nuget_registry())
    return findings


def split_license_expression(expression: str, operator: str) -> list[str]:
    parts: list[str] = []
    depth = 0
    current = ""
    index = 0
    while index < len(expression):
        character = expression[index]
        if character == "(":
            depth += 1
        elif character == ")":
            depth -= 1
        if depth == 0 and expression.startswith(operator, index):
            parts.append(current)
            current = ""
            index += len(operator)
            continue
        current += character
        index += 1
    parts.append(current)
    return [part.strip() for part in parts if part.strip()]


def license_identifier_allowed(text: str) -> bool:
    stripped = text.strip()
    if not stripped:
        return False
    if stripped.startswith("(") and stripped.endswith(")"):
        return license_allowed(stripped[1:-1])
    return stripped.split(" WITH ", 1)[0].strip() in PERMISSIVE_LICENSES


def license_allowed(expression: str) -> bool:
    if not expression.strip():
        return False
    normalized = expression.replace("/", " OR ")
    return any(
        all(
            license_identifier_allowed(part)
            for part in split_license_expression(alternative, "AND")
        )
        for alternative in split_license_expression(normalized, "OR")
    )


def toml_block_start_lines(text: str, header: str) -> list[int]:
    return [
        index for index, line in enumerate(text.splitlines(), start=1) if line.strip() == header
    ]


def toml_field_lines(text: str, field: str) -> dict[str, int]:
    lines: dict[str, int] = {}
    for index, line in enumerate(text.splitlines(), start=1):
        match = TOML_STRING_FIELD.match(line.strip())
        if match is None or match.group(1) != field:
            continue
        lines.setdefault(match.group(2), index)
    return lines


def lockfile_registry_packages(text: str) -> list[tuple[str, str]]:
    parsed = tomllib.loads(text)
    raw = parsed.get("package", [])
    packages: list[tuple[str, str]] = []
    if not isinstance(raw, list):
        return packages
    for item in raw:
        if not isinstance(item, dict):
            continue
        name = item.get("name")
        version = item.get("version")
        if not isinstance(name, str) or not isinstance(version, str):
            continue
        if "source" not in item:
            continue
        packages.append((name, version))
    return packages


def dependency_registry_entries(text: str, section: str = "package") -> list[dict[str, str]]:
    parsed = tomllib.loads(text)
    raw = parsed.get(section, [])
    entries: list[dict[str, str]] = []
    if not isinstance(raw, list):
        return entries
    block_lines = toml_block_start_lines(text, f"[[{section}]]")
    for index, item in enumerate(raw):
        if not isinstance(item, dict):
            continue
        entry: dict[str, str] = {}
        for key in REGISTRY_ENTRY_KEYS:
            value = item.get(key)
            entry[key] = value if isinstance(value, str) else ""
        entry["line"] = str(block_lines[index] if index < len(block_lines) else 1)
        entries.append(entry)
    return entries


def registry_entry_findings(
    display: str, name: str, version: str, entry: dict[str, str]
) -> list[str]:
    findings: list[str] = []
    line = entry["line"]
    expression = entry["license"]
    if not expression:
        findings.append(
            f"{display}:{line}: registry entry for {name} {version} declares no license"
        )
    elif not license_allowed(expression):
        findings.append(
            f"{display}:{line}: license {expression!r} for {name} {version} is not "
            "in the permissive allowlist"
        )
    reviewed = entry["advisory_reviewed"]
    if not reviewed:
        findings.append(
            f"{display}:{line}: registry entry for {name} {version} records no "
            "advisory_reviewed date"
        )
        return findings
    try:
        parsed = date.fromisoformat(reviewed)
    except ValueError:
        findings.append(
            f"{display}:{line}: advisory_reviewed {reviewed!r} for {name} {version} "
            "is not an ISO date"
        )
        return findings
    if parsed > datetime.now(UTC).date():
        findings.append(
            f"{display}:{line}: advisory_reviewed {reviewed!r} for {name} {version} "
            "is in the future"
        )
    return findings


def check_dependency_registry(root: Path = REPO_ROOT) -> list[str]:
    findings: list[str] = []
    lock_path = root / CARGO_LOCKFILE
    registry_path = root / DEPENDENCY_REGISTRY
    lock_display = display_path(lock_path)
    registry_display = display_path(registry_path)
    if not lock_path.exists():
        return [f"{lock_display}: the dependency registry needs a Cargo.lock to check"]
    if not registry_path.exists():
        return [f"{registry_display}: the dependency registry is missing"]
    try:
        lock_text = lock_path.read_text(encoding="utf-8")
        registry_text = registry_path.read_text(encoding="utf-8")
    except OSError as error:
        return [f"{registry_display}: the dependency registry could not be read: {error}"]
    try:
        locked = lockfile_registry_packages(lock_text)
    except tomllib.TOMLDecodeError as error:
        return [f"{lock_display}: the lockfile is not valid TOML: {error}"]
    try:
        entries = dependency_registry_entries(registry_text)
    except tomllib.TOMLDecodeError as error:
        return [f"{registry_display}: the dependency registry is not valid TOML: {error}"]
    recorded: dict[tuple[str, str], dict[str, str]] = {}
    for entry in entries:
        key = (entry["name"], entry["version"])
        if key in recorded:
            findings.append(
                f"{registry_display}:{entry['line']}: duplicate registry entry for "
                f"{entry['name']} {entry['version']}"
            )
            continue
        recorded[key] = entry
    lock_lines = toml_field_lines(lock_text, "name")
    for name, version in locked:
        matched = recorded.get((name, version))
        if matched is None:
            findings.append(
                f"{lock_display}:{lock_lines.get(name, 1)}: registry package {name} "
                f"{version} has no entry in {registry_display}"
            )
            continue
        findings.extend(registry_entry_findings(registry_display, name, version, matched))
    return findings


def iter_nuget_lockfiles(root: Path = REPO_ROOT) -> Iterator[Path]:
    for walk_root, dirnames, filenames in os.walk(root):
        dirnames[:] = [
            name
            for name in sorted(dirnames)
            if not name.startswith(".") and name not in SKIP_DIR_NAMES
        ]
        if NUGET_LOCKFILE_NAME in filenames:
            yield Path(walk_root) / NUGET_LOCKFILE_NAME


def nuget_lockfile_packages(text: str) -> list[tuple[str, str]]:
    parsed = json.loads(text)
    frameworks = parsed.get("dependencies", {}) if isinstance(parsed, dict) else {}
    packages: list[tuple[str, str]] = []
    seen: set[tuple[str, str]] = set()
    if not isinstance(frameworks, dict):
        return packages
    for framework in sorted(frameworks):
        locked = frameworks[framework]
        if not isinstance(locked, dict):
            continue
        for name in sorted(locked):
            item = locked[name]
            if not isinstance(item, dict):
                continue
            resolved = item.get("resolved")
            if not isinstance(resolved, str):
                continue
            if (name, resolved) in seen:
                continue
            seen.add((name, resolved))
            packages.append((name, resolved))
    return packages


def nuget_lock_lines(text: str) -> dict[str, int]:
    lines: dict[str, int] = {}
    for index, line in enumerate(text.splitlines(), start=1):
        match = re.match(r'^\s*"([^"]+)"\s*:', line)
        if match is not None:
            lines.setdefault(match.group(1), index)
    return lines


def index_registry_entries(
    registry_display: str, entries: Sequence[dict[str, str]]
) -> tuple[dict[tuple[str, str], dict[str, str]], list[str]]:
    recorded: dict[tuple[str, str], dict[str, str]] = {}
    findings: list[str] = []
    for entry in entries:
        key = (entry["name"], entry["version"])
        if key in recorded:
            findings.append(
                f"{registry_display}:{entry['line']}: duplicate registry entry for "
                f"{entry['name']} {entry['version']}"
            )
            continue
        recorded[key] = entry
    return recorded, findings


def check_nuget_registry(root: Path = REPO_ROOT) -> list[str]:
    lock_paths = sorted(iter_nuget_lockfiles(root))
    if not lock_paths:
        return []
    registry_path = root / DEPENDENCY_REGISTRY
    registry_display = display_path(registry_path)
    if not registry_path.exists():
        return [f"{registry_display}: the dependency registry is missing"]
    try:
        registry_text = registry_path.read_text(encoding="utf-8")
    except OSError as error:
        return [f"{registry_display}: the dependency registry could not be read: {error}"]
    try:
        entries = dependency_registry_entries(registry_text, "nuget")
    except tomllib.TOMLDecodeError as error:
        return [f"{registry_display}: the dependency registry is not valid TOML: {error}"]
    recorded, findings = index_registry_entries(registry_display, entries)
    for lock_path in lock_paths:
        lock_display = display_path(lock_path)
        try:
            lock_text = lock_path.read_text(encoding="utf-8")
            locked = nuget_lockfile_packages(lock_text)
        except (OSError, json.JSONDecodeError) as error:
            findings.append(f"{lock_display}: the lockfile could not be parsed: {error}")
            continue
        lock_lines = nuget_lock_lines(lock_text)
        for name, version in locked:
            matched = recorded.get((name, version))
            if matched is None:
                findings.append(
                    f"{lock_display}:{lock_lines.get(name, 1)}: locked NuGet package "
                    f"{name} {version} has no entry in {registry_display}"
                )
                continue
            findings.extend(registry_entry_findings(registry_display, name, version, matched))
    return findings


def secret_finding_keys(results: object) -> set[tuple[str, str, str]]:
    keys: set[tuple[str, str, str]] = set()
    if not isinstance(results, dict):
        return keys
    for filename, findings in results.items():
        if not isinstance(findings, list):
            continue
        for finding in findings:
            if not isinstance(finding, dict):
                continue
            kind = str(finding.get("type", "unknown"))
            digest = str(finding.get("hashed_secret", ""))
            line = str(finding.get("line_number", ""))
            keys.add((str(filename), kind, digest or line))
    return keys


def secret_delta(baseline_results: object, scanned_results: object) -> list[str]:
    baseline_keys = secret_finding_keys(baseline_results)
    scanned_keys = secret_finding_keys(scanned_results)
    return [
        f"secret finding not in baseline: {key[0]} {key[1]}"
        for key in sorted(scanned_keys - baseline_keys)
    ]


SECRET_SCAN_CACHE = REPO_ROOT / ".ruff_cache" / "secret_scan_cache.json"


def secret_scan_files(root: Path = REPO_ROOT) -> list[str]:
    files: list[str] = []
    for walk_root, dirnames, filenames in os.walk(root):
        rel_dir = os.path.relpath(walk_root, root)
        prefix = "" if rel_dir == "." else rel_dir + "/"
        dirnames[:] = [
            name
            for name in sorted(dirnames)
            if name != ".git"
            and not any(re.search(pattern, prefix + name + "/") for pattern in SECRET_SCAN_EXCLUDES)
        ]
        for filename in sorted(filenames):
            rel_file = prefix + filename
            if not any(re.search(pattern, rel_file) for pattern in SECRET_SCAN_EXCLUDES):
                files.append(rel_file)
    return files


def detect_secrets_version() -> str:
    command = ["detect-secrets", "--version"]
    try:
        completed = subprocess.run(
            command,
            cwd=REPO_ROOT,
            check=False,
            capture_output=True,
            text=True,
        )
    except OSError:
        return "detect-secrets:unavailable"
    if completed.returncode != 0:
        return "detect-secrets:unavailable"
    return completed.stdout.strip() or "detect-secrets:unknown"


def secret_scan_digest(files: Sequence[str], root: Path = REPO_ROOT) -> str:
    hasher = hashlib.sha256()
    hasher.update(SECRETS_BASELINE.read_bytes())
    hasher.update(detect_secrets_version().encode("utf-8"))
    hasher.update(sys.version.encode("utf-8"))
    hasher.update(platform.platform().encode("utf-8"))
    for rel_file in files:
        hasher.update(rel_file.encode("utf-8"))
        hasher.update(b"\0")
        with contextlib.suppress(OSError):
            hasher.update((root / rel_file).read_bytes())
        hasher.update(b"\0")
    return hasher.hexdigest()


def secret_scan_chunks(files: Sequence[str], count: int) -> list[list[str]]:
    chunks: list[list[str]] = [[] for _ in range(count)]
    loads = [0] * count
    for relative in files:
        index = loads.index(min(loads))
        chunks[index].append(relative)
        loads[index] += 1
    return [chunk for chunk in chunks if chunk]


def check_secrets() -> list[str]:
    if not SECRETS_BASELINE.exists():
        return ["missing .secrets.baseline"]
    files = secret_scan_files()
    if not files:
        return []
    digest = secret_scan_digest(files)
    if SECRET_SCAN_CACHE.exists():
        with contextlib.suppress(OSError, json.JSONDecodeError):
            cached = json.loads(SECRET_SCAN_CACHE.read_text(encoding="utf-8"))
            if (
                isinstance(cached, dict)
                and cached.get("digest") == digest
                and isinstance(cached.get("findings"), list)
            ):
                sys.stdout.write(
                    f"secret scan: cache hit for {len(files)} files (digest {digest[:12]})\n"
                )
                return [str(item) for item in cached["findings"]]
    chunks = secret_scan_chunks(files, 4)
    runs = []
    for chunk in chunks:
        command = ["detect-secrets", "scan", *chunk]
        runs.append(
            subprocess.Popen(
                command,
                cwd=REPO_ROOT,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
            )
        )
    scanned: dict[str, object] = {}
    for process in runs:
        stdout, stderr = process.communicate()
        if process.returncode != 0:
            process.kill()
            return [f"detect-secrets scan failed: {stderr.strip()}"]
        scanned.update(json.loads(stdout).get("results", {}))
    sys.stdout.write(f"secret scan: {len(files)} files in {len(chunks)} chunks\n")
    baseline = json.loads(SECRETS_BASELINE.read_text(encoding="utf-8"))
    findings = secret_delta(baseline.get("results", {}), scanned)
    with contextlib.suppress(OSError):
        SECRET_SCAN_CACHE.parent.mkdir(parents=True, exist_ok=True)
        SECRET_SCAN_CACHE.write_text(
            json.dumps({"digest": digest, "findings": findings}),
            encoding="utf-8",
        )
    return findings


DOTNET_FORMAT_CACHE = REPO_ROOT / ".ruff_cache" / "dotnet_format_cache.json"
DOTNET_FORMAT_COMMAND = (
    "dotnet",
    "format",
    "bridge/Unitas.sln",
    "--verify-no-changes",
    "--no-restore",
)
DOTNET_FORMAT_SOURCE_SUFFIXES = (".cs", ".csproj", ".sln", ".editorconfig", ".props", ".targets")
DOTNET_FORMAT_ROOT_CONFIGS = (".editorconfig", "Directory.Build.props", "global.json")
DOTNET_FORMAT_ANCESTOR_CONFIGS = (".editorconfig", "Directory.Build.props")


def _dotnet_format_project_files(root: Path) -> list[str]:
    files: list[str] = []
    for base in ("bridge", "editor"):
        base_root = root / base
        if not base_root.is_dir():
            continue
        for walk_root, dirnames, filenames in os.walk(base_root):
            dirnames[:] = [name for name in sorted(dirnames) if name not in ("obj", "bin")]
            files.extend(
                str(Path(walk_root, filename).relative_to(root))
                for filename in sorted(filenames)
                if filename.endswith(DOTNET_FORMAT_SOURCE_SUFFIXES)
            )
    return files


def _dotnet_format_config_files(root: Path) -> list[str]:
    files = [name for name in DOTNET_FORMAT_ROOT_CONFIGS if (root / name).exists()]
    ancestor = root.parent
    while True:
        files.extend(
            str(ancestor / name)
            for name in DOTNET_FORMAT_ANCESTOR_CONFIGS
            if (ancestor / name).exists()
        )
        if ancestor.parent == ancestor:
            break
        ancestor = ancestor.parent
    return files


def dotnet_format_files(root: Path = REPO_ROOT) -> list[str]:
    return sorted(_dotnet_format_project_files(root) + _dotnet_format_config_files(root))


def _dotnet_executable() -> str:
    return shutil.which("dotnet") or "dotnet"


def dotnet_format_digest(files: Sequence[str], root: Path = REPO_ROOT) -> str:
    hasher = hashlib.sha256()
    hasher.update(" ".join(DOTNET_FORMAT_COMMAND).encode("utf-8"))
    try:
        completed = subprocess.run(
            [_dotnet_executable(), "--version"],
            cwd=root,
            check=False,
            capture_output=True,
            text=True,
        )
        hasher.update(completed.stdout.strip().encode("utf-8"))
    except OSError:
        hasher.update(b"dotnet:unavailable")
    for rel_file in files:
        hasher.update(rel_file.encode("utf-8"))
        hasher.update(b"\0")
        with contextlib.suppress(OSError):
            hasher.update((root / rel_file).read_bytes())
        hasher.update(b"\0")
    return hasher.hexdigest()


def run_dotnet_format(root: Path = REPO_ROOT, cache: Path = DOTNET_FORMAT_CACHE) -> int:
    files = dotnet_format_files(root)
    digest = dotnet_format_digest(files, root)
    if cache.exists():
        with contextlib.suppress(OSError, json.JSONDecodeError):
            cached = json.loads(cache.read_text(encoding="utf-8"))
            if isinstance(cached, dict) and cached.get("digest") == digest:
                sys.stdout.write(
                    f"dotnet format: cache hit for {len(files)} files (digest {digest[:12]})\n"
                )
                return 0
    try:
        completed = subprocess.run(
            [_dotnet_executable(), *DOTNET_FORMAT_COMMAND[1:]], cwd=root, check=False
        )
    except OSError as error:
        sys.stderr.write(f"dotnet format could not start: {error}\n")
        return 1
    if completed.returncode != 0:
        return completed.returncode
    with contextlib.suppress(OSError):
        cache.parent.mkdir(parents=True, exist_ok=True)
        cache.write_text(json.dumps({"digest": digest}), encoding="utf-8")
    return 0


GATES: dict[str, Callable[[], list[str]]] = {
    "banned": check_banned,
    "bypass": check_bypass,
    "comments": check_comments,
    "deps": check_dependencies,
    "policy": check_policy,
    "secret": check_secrets,
}


def run_timed(argv: Sequence[str]) -> int:
    if len(argv) < TIMED_COMMAND_MIN_ARGS:
        sys.stderr.write("usage: verify.py timed <label> <budget-seconds> <command...>\n")
        return 2
    label = argv[1]
    budget = float(argv[2])
    command = list(argv[3:])
    started = time.monotonic()
    completed = subprocess.run(command, cwd=REPO_ROOT, check=False)
    elapsed = time.monotonic() - started
    sys.stdout.write(f"gate {label} took {elapsed:.2f}s (exit {completed.returncode})\n")
    if completed.returncode != 0:
        return completed.returncode
    if elapsed > budget:
        sys.stdout.write(f"gate {label} exceeded the {budget:.0f}s budget\n")
        return 1
    return 0


def main(argv: Sequence[str]) -> int:
    if len(argv) > 1 and argv[1] == "timed":
        return run_timed(argv[1:])
    if len(argv) > 1 and argv[1] == "dotnet-format":
        return run_dotnet_format()
    if len(argv) != GATE_CLI_ARG_COUNT or argv[1] not in GATES:
        sys.stderr.write("usage: verify.py {" + "|".join(sorted(GATES)) + "}\n")
        return 2
    name = argv[1]
    findings = GATES[name]()
    for finding in findings:
        sys.stdout.write(f"FAIL {name}: {finding}\n")
    if findings:
        sys.stdout.write(f"{name}: {len(findings)} finding(s)\n")
        return 1
    sys.stdout.write(f"OK {name}\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
