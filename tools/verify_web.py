from __future__ import annotations

import http.server
import json
import subprocess
import sys
import threading
import time
from pathlib import Path
from typing import TYPE_CHECKING, ClassVar

if TYPE_CHECKING:
    import os
    import socket
    import socketserver
    from collections.abc import Sequence

REPO_ROOT = Path(__file__).resolve().parent.parent
LANE_DIR = REPO_ROOT / "target" / "web-lane"
APP_DIR = LANE_DIR / "app"
SELF_CHECK_DIR = LANE_DIR / "self-check"
REPORT_PATH = LANE_DIR / "report.json"
CONSOLE_LOG_PATH = LANE_DIR / "console.log"
SCREENSHOT_PATH = LANE_DIR / "screenshot.png"
DRIVER_PATH = REPO_ROOT / "tools" / "web_lane_driver.mjs"
PLAYER_BINARY = REPO_ROOT / "target" / "debug" / "unitas-player"
RUNTIME_PACKAGE = "unitas-runtime"
BUILD_PACKAGE = "unitas-build"
WEB_TARGET = "wasm32-unknown-unknown"
PLAYER_PACKAGE_FLAG = "--package-web"
WEB_INDEX_HTML = "index.html"
WEB_LOADER_JS = "unitas-loader.js"
SERVER_HOST = "127.0.0.1"
MIN_FRAMES = 60
FRAME_DEADLINE_SECONDS = 30
DEFAULT_BUDGET_SECONDS = 180.0
BUILD_BUDGET_SECONDS = 900.0
DRIVER_BUDGET_SECONDS = 90.0
BAD_CLI_EXIT_CODE = 2
VERDICT_ARG_COUNT = 2
POLICY_CITATION = "docs/ralph-workflow-policy/verification-policy.md § Gate lanes"
TIMEOUT_CITATION = "docs/ralph-workflow-policy/verification-policy.md § Default requirements"
GATE_SCRIPT_CITATION = "docs/ralph-workflow-policy/gate-script-policy.md § Failure output"

MIME_TYPES: dict[str, str] = {
    ".html": "text/html; charset=utf-8",
    ".js": "text/javascript; charset=utf-8",
    ".mjs": "text/javascript; charset=utf-8",
    ".wasm": "application/wasm",
    ".json": "application/json",
    ".css": "text/css; charset=utf-8",
    ".png": "image/png",
    ".svg": "image/svg+xml",
}
DEFAULT_CONTENT_TYPE = "application/octet-stream"

REPORT_FIELDS: tuple[str, ...] = (
    "lane",
    "ok",
    "url",
    "chrome",
    "frame_count",
    "min_frames",
    "first_frame_count",
    "elapsed_seconds",
    "frames_per_second",
    "booted",
    "error",
    "console_error_count",
    "console_errors",
    "exception_count",
    "exceptions",
    "requests",
    "screenshot",
    "console_log",
    "duration_seconds",
    "reason",
)

LANE_METADATA: tuple[tuple[str, str], ...] = (
    ("owner", "platforms"),
    (
        "trigger",
        "changes to the web bootstrap, web packaging, or the wasm runtime surface",
    ),
    ("lane", "make verify-web"),
    (
        "evidence",
        "the lane report at target/web-lane/report.json, the captured browser console log "
        "and screenshot, and the lane exit status",
    ),
    ("review", "when the web player gains managed scripting or rendering"),
)

SELF_CHECK_HTML = """<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>unitas web lane self-check</title>
</head>
<body>
<div id="unitas-status">Running</div>
<script>
window.__unitas = { booted: true, frameCount: 0, error: null };
let frames = 0;
setInterval(() => {
  frames += 1;
  window.__unitas.frameCount = frames;
}, 16);
</script>
</body>
</html>
"""


def help_text() -> str:
    metadata_lines = [f"  {key}: {value}" for key, value in LANE_METADATA]
    usage = (
        "usage: verify_web.py [--help] [--metadata] [--verdict <driver-verdict.json>]\n"
        "       verify_web.py --self-check"
    )
    return "\n".join(
        [
            usage,
            "lane metadata:",
            *metadata_lines,
        ]
    )


def metadata_text() -> str:
    return "\n".join(f"{key}: {value}" for key, value in LANE_METADATA)


def run_step(
    label: str,
    command: Sequence[str],
    budget_seconds: float,
) -> tuple[int, str]:
    started = time.monotonic()
    try:
        completed = subprocess.run(
            list(command),
            cwd=REPO_ROOT,
            check=False,
            timeout=budget_seconds,
            capture_output=True,
            text=True,
            errors="replace",
        )
    except subprocess.TimeoutExpired:
        elapsed = time.monotonic() - started
        sys.stderr.write(
            f"FAIL verify-web ({label}): timed out after {elapsed:.2f}s "
            f"(budget {budget_seconds:.0f}s); a slow check is a defect to diagnose, "
            f"never a budget to raise ({TIMEOUT_CITATION})\n"
        )
        return (1, "")
    elapsed = time.monotonic() - started
    output = completed.stdout + completed.stderr
    if output:
        sys.stdout.write(output if output.endswith("\n") else output + "\n")
    sys.stdout.write(f"web-step {label} took {elapsed:.2f}s (exit {completed.returncode})\n")
    if completed.returncode != 0:
        cmd_str = " ".join(command)
        sys.stderr.write(
            f"FAIL verify-web ({label}): command {cmd_str} exited "
            f"{completed.returncode}; every declared lane must pass hard "
            f"when run ({POLICY_CITATION})\n"
        )
    return (completed.returncode, output)


def content_type_for(path: Path) -> str:
    return MIME_TYPES.get(path.suffix.lower(), DEFAULT_CONTENT_TYPE)


def as_int(value: object, default: int) -> int:
    if isinstance(value, bool):
        return default
    if isinstance(value, int | float):
        return int(value)
    if isinstance(value, str):
        try:
            return int(value)
        except ValueError:
            return default
    return default


def as_float(value: object, default: float) -> float:
    if isinstance(value, bool):
        return default
    if isinstance(value, int | float):
        return float(value)
    if isinstance(value, str):
        try:
            return float(value)
        except ValueError:
            return default
    return default


def find_web_root(app_dir: Path) -> Path | None:
    if (app_dir / WEB_INDEX_HTML).is_file():
        return app_dir
    candidates = sorted(app_dir.glob(f"*/{WEB_INDEX_HTML}"))
    if candidates:
        return candidates[0].parent
    return None


def web_root_failure_reason(root: Path) -> str | None:
    if not (root / WEB_LOADER_JS).is_file():
        return (
            f"the packaged web app at {root} carries no {WEB_LOADER_JS} next to "
            f"{WEB_INDEX_HTML}; the web player cannot boot without its loader"
        )
    if not list(root.glob("*.wasm")):
        return (
            f"the packaged web app at {root} carries no wasm module next to "
            f"{WEB_INDEX_HTML}; the web player cannot boot without the module"
        )
    return None


def driver_command(
    url: str,
    screenshot: Path,
    console_log: Path,
    min_frames: int = MIN_FRAMES,
    timeout_seconds: int = FRAME_DEADLINE_SECONDS,
) -> tuple[str, ...]:
    return (
        "node",
        str(DRIVER_PATH),
        "--url",
        url,
        "--screenshot",
        str(screenshot),
        "--console-log",
        str(console_log),
        "--min-frames",
        str(min_frames),
        "--timeout-seconds",
        str(timeout_seconds),
    )


def parse_driver_verdict(output: str) -> dict[str, object] | None:
    for line in reversed(output.splitlines()):
        stripped = line.strip()
        if not stripped.startswith("{"):
            continue
        try:
            payload = json.loads(stripped)
        except ValueError:
            continue
        if isinstance(payload, dict):
            return payload
    return None


def build_report(
    verdict: dict[str, object],
    url: str,
    requests: Sequence[str],
) -> dict[str, object]:
    console_errors = verdict.get("console_errors")
    exceptions = verdict.get("exceptions")
    return {
        "lane": "verify-web",
        "ok": verdict.get("ok") is True,
        "url": url,
        "chrome": str(verdict.get("chrome", "")),
        "frame_count": as_int(verdict.get("frame_count"), 0),
        "min_frames": as_int(verdict.get("min_frames"), MIN_FRAMES),
        "first_frame_count": verdict.get("first_frame_count"),
        "elapsed_seconds": as_float(verdict.get("elapsed_seconds"), 0.0),
        "frames_per_second": as_float(verdict.get("frames_per_second"), 0.0),
        "booted": verdict.get("booted") is True,
        "error": verdict.get("error"),
        "console_error_count": as_int(verdict.get("console_error_count"), 0),
        "console_errors": list(console_errors) if isinstance(console_errors, list) else [],
        "exception_count": as_int(verdict.get("exception_count"), 0),
        "exceptions": list(exceptions) if isinstance(exceptions, list) else [],
        "requests": list(requests),
        "screenshot": str(verdict.get("screenshot", "")),
        "console_log": str(verdict.get("console_log", "")),
        "duration_seconds": as_float(verdict.get("duration_seconds"), 0.0),
        "reason": str(verdict.get("reason", "")),
    }


def report_failure_reason(report: dict[str, object]) -> str | None:
    missing = [name for name in REPORT_FIELDS if name not in report]
    if missing:
        return f"the lane report lacks fields: {', '.join(missing)}"
    frame_count = report["frame_count"]
    min_frames = report["min_frames"]
    console_error_count = report["console_error_count"]
    if not isinstance(frame_count, int):
        return "the lane report frame_count is not an integer"
    if not isinstance(min_frames, int):
        return "the lane report min_frames is not an integer"
    if not isinstance(console_error_count, int):
        return "the lane report console_error_count is not an integer"
    checks: tuple[tuple[bool, str], ...] = (
        (report["ok"] is not True, str(report["reason"]) or "the browser driver failed"),
        (
            report["booted"] is not True,
            "the web player never reported a boot; the wasm module did not start",
        ),
        (
            frame_count < min_frames,
            f"the web player advanced {frame_count} frames; at least {min_frames} are required",
        ),
        (
            console_error_count > 0,
            f"the browser console reported {console_error_count} error(s): "
            f"{report['console_errors']}",
        ),
        (
            as_int(report["exception_count"], 0) > 0,
            f"the page raised {report['exception_count']} uncaught exception(s): "
            f"{report['exceptions']}",
        ),
    )
    for failed, reason in checks:
        if failed:
            return reason
    return None


class WebRequestHandler(http.server.SimpleHTTPRequestHandler):
    root_dir: ClassVar[Path] = Path()
    request_log: ClassVar[list[str]] = []

    def __init__(
        self,
        request: socket.socket | tuple[bytes, socket.socket],
        client_address: tuple[str, int],
        server: socketserver.BaseServer,
        *,
        directory: str | os.PathLike[str] | None = None,
    ) -> None:
        super().__init__(
            request,
            client_address,
            server,
            directory=directory or str(self.root_dir),
        )

    def guess_type(self, path: str | os.PathLike[str]) -> str:
        return content_type_for(Path(path))

    def log_request(self, code: str | int = "-", size: str | int = "-") -> None:
        self.request_log.append(f"{self.command} {self.path} -> {code} ({size} bytes)")


def start_server(root: Path) -> tuple[http.server.ThreadingHTTPServer, int, list[str]]:
    access_log: list[str] = []

    class RootedWebRequestHandler(WebRequestHandler):
        root_dir: ClassVar[Path] = root
        request_log: ClassVar[list[str]] = access_log

    server = http.server.ThreadingHTTPServer((SERVER_HOST, 0), RootedWebRequestHandler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    port = int(server.server_address[1])
    return (server, port, access_log)


def write_report(report: dict[str, object]) -> None:
    LANE_DIR.mkdir(parents=True, exist_ok=True)
    REPORT_PATH.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")


def run_build_gates() -> int:
    rc, _ = run_step(
        "build-wasm",
        ("cargo", "build", "-p", RUNTIME_PACKAGE, "--target", WEB_TARGET),
        BUILD_BUDGET_SECONDS,
    )
    if rc != 0:
        return rc
    rc, _ = run_step(
        "build-player",
        ("cargo", "build", "-p", BUILD_PACKAGE, "--bin", "unitas-player"),
        BUILD_BUDGET_SECONDS,
    )
    return rc


def run_driver(url: str, screenshot: Path, console_log: Path) -> tuple[int, str]:
    return run_step(
        "web-driver",
        driver_command(url, screenshot, console_log),
        DRIVER_BUDGET_SECONDS,
    )


def prepare_packaged_app(budget_seconds: float) -> Path | None:
    rc, _ = run_step(
        "package-web",
        (str(PLAYER_BINARY), PLAYER_PACKAGE_FLAG, str(APP_DIR)),
        budget_seconds,
    )
    if rc != 0:
        return None
    root = find_web_root(APP_DIR)
    if root is None:
        sys.stderr.write(
            f"FAIL verify-web (package-web): the packaged web app at {APP_DIR} carries no "
            f"{WEB_INDEX_HTML}; the packaging step must stage the web player "
            f"({POLICY_CITATION})\n"
        )
        return None
    reason = web_root_failure_reason(root)
    if reason is not None:
        sys.stderr.write(f"FAIL verify-web (package-web): {reason} ({POLICY_CITATION})\n")
        return None
    return root


def run_web_lane(budget_seconds: float = DEFAULT_BUDGET_SECONDS) -> int:
    sys.stdout.write(metadata_text() + "\n")
    rc = run_build_gates()
    if rc != 0:
        return rc
    root = prepare_packaged_app(budget_seconds)
    if root is None:
        return 1
    server, port, request_log = start_server(root)
    url = f"http://{SERVER_HOST}:{port}/{WEB_INDEX_HTML}"
    try:
        _rc, output = run_driver(url, SCREENSHOT_PATH, CONSOLE_LOG_PATH)
    finally:
        server.shutdown()
        server.server_close()
    verdict = parse_driver_verdict(output)
    if verdict is None:
        sys.stderr.write(
            "FAIL verify-web (web-driver): the browser driver reported no verdict; "
            f"a driver that cannot report is a broken driver ({GATE_SCRIPT_CITATION})\n"
        )
        write_report(build_report({}, url, request_log))
        return 1
    report = build_report(verdict, url, request_log)
    write_report(report)
    failure = report_failure_reason(report)
    if failure is not None:
        sys.stderr.write(f"FAIL verify-web (web-driver): {failure} ({POLICY_CITATION})\n")
        return 1
    sys.stdout.write(
        f"OK verify-web (web-driver): the packaged web player booted the wasm module in "
        f"headless Chrome and advanced {report['frame_count']} frames "
        f"(at least {report['min_frames']} required), with {report['console_error_count']} "
        f"console error(s); the first frame to the {report['min_frames']}th frame took "
        f"{report['elapsed_seconds']:.3f}s ({report['frames_per_second']:.1f} frames per "
        f"second observed, not a threshold)\n"
    )
    sys.stdout.write("OK verify-web\n")
    return 0


def run_self_check() -> int:
    SELF_CHECK_DIR.mkdir(parents=True, exist_ok=True)
    (SELF_CHECK_DIR / WEB_INDEX_HTML).write_text(SELF_CHECK_HTML, encoding="utf-8")
    server, port, _request_log = start_server(SELF_CHECK_DIR)
    url = f"http://{SERVER_HOST}:{port}/{WEB_INDEX_HTML}"
    try:
        _rc, output = run_driver(
            url,
            LANE_DIR / "self-check.png",
            LANE_DIR / "self-check-console.log",
        )
    finally:
        server.shutdown()
        server.server_close()
    verdict = parse_driver_verdict(output)
    if verdict is None:
        sys.stderr.write(
            "FAIL verify-web (self-check): the browser driver reported no verdict; "
            f"a driver that cannot report is a broken driver ({GATE_SCRIPT_CITATION})\n"
        )
        return 1
    report = build_report(verdict, url, [])
    failure = report_failure_reason(report)
    if failure is not None:
        sys.stderr.write(f"FAIL verify-web (self-check): {failure} ({POLICY_CITATION})\n")
        return 1
    sys.stdout.write(
        f"OK verify-web (self-check): the browser driver drove a stub page to "
        f"{report['frame_count']} frames with {report['console_error_count']} console "
        f"error(s); the self-check proves the CDP path only, never the packaged player\n"
    )
    return 0


def run_verdict_command(path: Path) -> int:
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        sys.stderr.write(f"FAIL verify-web (verdict): the verdict file is unreadable: {error}\n")
        return 1
    if not isinstance(payload, dict):
        sys.stderr.write("FAIL verify-web (verdict): the verdict file carries no JSON object\n")
        return 1
    report = build_report(payload, str(payload.get("url", "")), [])
    failure = report_failure_reason(report)
    if failure is not None:
        sys.stderr.write(f"FAIL verify-web (verdict): {failure} ({POLICY_CITATION})\n")
        return 1
    sys.stdout.write(
        f"OK verify-web (verdict): the browser verdict advanced {report['frame_count']} "
        f"frames with {report['console_error_count']} console error(s)\n"
    )
    return 0


def main(argv: Sequence[str]) -> int:
    args = list(argv[1:])
    if not args:
        return run_web_lane()
    if args in (["--help"], ["-h"]):
        sys.stdout.write(help_text() + "\n")
        return 0
    if args == ["--metadata"]:
        sys.stdout.write(metadata_text() + "\n")
        return 0
    if args == ["--self-check"]:
        return run_self_check()
    if args[0] == "--verdict" and len(args) == VERDICT_ARG_COUNT:
        return run_verdict_command(Path(args[1]))
    sys.stderr.write(help_text() + "\n")
    return BAD_CLI_EXIT_CODE


if __name__ == "__main__":
    sys.exit(main(sys.argv))
