from __future__ import annotations

import contextlib
import io
import json
import urllib.request
from typing import TYPE_CHECKING

from tools import verify_web

if TYPE_CHECKING:
    from pathlib import Path

BAD_CLI_EXIT_CODE = 2
MIN_FRAMES = 60
COERCED_FRAME_COUNT = 60
COERCED_ELAPSED_SECONDS = 1.5
HTTP_OK = 200


def ok_verdict(frame_count: int = MIN_FRAMES) -> dict[str, object]:
    return {
        "ok": True,
        "reason": "",
        "url": "http://127.0.0.1:8000/index.html",
        "chrome": "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "frame_count": frame_count,
        "min_frames": MIN_FRAMES,
        "first_frame_count": 1,
        "elapsed_seconds": 1.5,
        "frames_per_second": 40.0,
        "booted": True,
        "error": None,
        "console_error_count": 0,
        "console_errors": [],
        "exception_count": 0,
        "exceptions": [],
        "console": [],
        "screenshot": "target/web-lane/screenshot.png",
        "screenshot_error": "",
        "console_log": "target/web-lane/console.log",
        "duration_seconds": 2.0,
    }


def test_the_mime_map_covers_the_packaged_web_assets() -> None:
    assert verify_web.content_type_for(verify_web.LANE_DIR / "index.html") == (
        "text/html; charset=utf-8"
    )
    assert verify_web.content_type_for(verify_web.LANE_DIR / "unitas-loader.js") == (
        "text/javascript; charset=utf-8"
    )
    assert verify_web.content_type_for(verify_web.LANE_DIR / "module.wasm") == "application/wasm"
    assert verify_web.content_type_for(verify_web.LANE_DIR / "data.JSON") == "application/json"


def test_the_mime_map_falls_back_to_octet_stream_for_unknown_suffixes() -> None:
    assert verify_web.content_type_for(verify_web.LANE_DIR / "blob.bin") == (
        verify_web.DEFAULT_CONTENT_TYPE
    )


def test_the_driver_command_names_the_url_screenshot_console_log_and_thresholds() -> None:
    command = verify_web.driver_command(
        "http://127.0.0.1:9/index.html",
        verify_web.SCREENSHOT_PATH,
        verify_web.CONSOLE_LOG_PATH,
    )
    assert command[0] == "node"
    assert command[1] == str(verify_web.DRIVER_PATH)
    assert command[command.index("--url") + 1] == "http://127.0.0.1:9/index.html"
    assert command[command.index("--screenshot") + 1] == str(verify_web.SCREENSHOT_PATH)
    assert command[command.index("--console-log") + 1] == str(verify_web.CONSOLE_LOG_PATH)
    assert command[command.index("--min-frames") + 1] == str(verify_web.MIN_FRAMES)
    assert command[command.index("--timeout-seconds") + 1] == str(verify_web.FRAME_DEADLINE_SECONDS)


def test_the_driver_verdict_parser_reads_the_last_json_line() -> None:
    verdict = {"ok": True, "frame_count": 61}
    output = "chrome noise\nnoise {not json}\n" + json.dumps(verdict) + "\n"
    assert verify_web.parse_driver_verdict(output) == verdict


def test_the_driver_verdict_parser_rejects_output_without_a_json_object() -> None:
    assert verify_web.parse_driver_verdict("chrome noise only\n[]") is None


def test_the_lane_report_carries_every_declared_field() -> None:
    report = verify_web.build_report(ok_verdict(), "http://127.0.0.1:8000/index.html", [])
    for field in verify_web.REPORT_FIELDS:
        assert field in report
    assert report["lane"] == "verify-web"
    assert report["ok"] is True
    assert report["booted"] is True
    assert report["requests"] == []


def test_the_lane_report_coerces_scalar_fields_from_string_verdicts() -> None:
    verdict = ok_verdict()
    verdict["frame_count"] = "60"
    verdict["console_error_count"] = "0"
    verdict["elapsed_seconds"] = "1.5"
    report = verify_web.build_report(verdict, "http://127.0.0.1:8000/index.html", [])
    assert report["frame_count"] == COERCED_FRAME_COUNT
    assert report["console_error_count"] == 0
    assert report["elapsed_seconds"] == COERCED_ELAPSED_SECONDS


def test_a_complete_lane_report_passes_the_contract() -> None:
    report = verify_web.build_report(ok_verdict(), "http://127.0.0.1:8000/index.html", [])
    assert verify_web.report_failure_reason(report) is None


def test_a_lane_report_missing_fields_fails_the_contract() -> None:
    report = verify_web.build_report(ok_verdict(), "http://127.0.0.1:8000/index.html", [])
    del report["screenshot"]
    reason = verify_web.report_failure_reason(report)
    assert reason is not None
    assert "screenshot" in reason


def test_a_lane_report_that_never_booted_fails_the_contract() -> None:
    verdict = ok_verdict()
    verdict["ok"] = False
    verdict["booted"] = False
    verdict["reason"] = "the web player never reported a boot within 30s"
    report = verify_web.build_report(verdict, "http://127.0.0.1:8000/index.html", [])
    reason = verify_web.report_failure_reason(report)
    assert reason is not None
    assert "boot" in reason


def test_a_lane_report_below_the_frame_floor_fails_the_contract() -> None:
    report = verify_web.build_report(ok_verdict(frame_count=41), "u", [])
    reason = verify_web.report_failure_reason(report)
    assert reason is not None
    assert "41 frames" in reason


def test_a_lane_report_with_console_errors_fails_the_contract() -> None:
    verdict = ok_verdict()
    verdict["ok"] = True
    verdict["reason"] = ""
    verdict["console_error_count"] = 1
    verdict["console_errors"] = [{"type": "error", "text": "wasm boot failed"}]
    report = verify_web.build_report(verdict, "u", [])
    reason = verify_web.report_failure_reason(report)
    assert reason is not None
    assert "wasm boot failed" in reason


def test_a_lane_report_with_uncaught_exceptions_fails_the_contract() -> None:
    verdict = ok_verdict()
    verdict["ok"] = True
    verdict["reason"] = ""
    verdict["exception_count"] = 1
    verdict["exceptions"] = ["ReferenceError: missing"]
    report = verify_web.build_report(verdict, "u", [])
    reason = verify_web.report_failure_reason(report)
    assert reason is not None
    assert "ReferenceError" in reason


def test_the_web_root_is_found_next_to_the_index_or_one_level_down(tmp_path: Path) -> None:
    flat = tmp_path / "flat"
    flat.mkdir()
    (flat / verify_web.WEB_INDEX_HTML).write_text("<html></html>", encoding="utf-8")
    assert verify_web.find_web_root(flat) == flat
    nested = tmp_path / "nested" / "unitas-web"
    nested.mkdir(parents=True)
    (nested / verify_web.WEB_INDEX_HTML).write_text("<html></html>", encoding="utf-8")
    assert verify_web.find_web_root(tmp_path / "nested") == nested
    empty = tmp_path / "empty"
    empty.mkdir()
    assert verify_web.find_web_root(empty) is None


def test_the_packaged_app_needs_its_loader_and_wasm_module_next_to_the_index(
    tmp_path: Path,
) -> None:
    root = tmp_path / "app"
    root.mkdir()
    (root / verify_web.WEB_INDEX_HTML).write_text("<html></html>", encoding="utf-8")
    reason = verify_web.web_root_failure_reason(root)
    assert reason is not None
    assert verify_web.WEB_LOADER_JS in reason
    (root / verify_web.WEB_LOADER_JS).write_text("// loader", encoding="utf-8")
    reason = verify_web.web_root_failure_reason(root)
    assert reason is not None
    assert "wasm" in reason
    (root / "unitas.wasm").write_bytes(b"\0asm")
    assert verify_web.web_root_failure_reason(root) is None


def test_the_fixture_server_serves_the_app_and_records_requests(tmp_path: Path) -> None:
    (tmp_path / verify_web.WEB_INDEX_HTML).write_text("<html>ok</html>", encoding="utf-8")
    server, port, request_log = verify_web.start_server(tmp_path)
    try:
        assert port > 0
        with urllib.request.urlopen(f"http://{verify_web.SERVER_HOST}:{port}/index.html") as (
            response
        ):
            assert response.status == HTTP_OK
            assert response.headers.get_content_type() == "text/html"
            assert response.read() == b"<html>ok</html>"
    finally:
        server.shutdown()
        server.server_close()
    assert any("index.html" in line and "200" in line for line in request_log)


def test_verify_web_main_passes_metadata_and_help_and_fails_unknown_flags() -> None:
    stdout = io.StringIO()
    with contextlib.redirect_stdout(stdout):
        assert verify_web.main(["verify_web.py", "--metadata"]) == 0
    assert "lane: make verify-web" in stdout.getvalue()
    assert "owner: platforms" in stdout.getvalue()
    stdout = io.StringIO()
    with contextlib.redirect_stdout(stdout):
        assert verify_web.main(["verify_web.py", "--help"]) == 0
    assert "usage: verify_web.py" in stdout.getvalue()
    stderr = io.StringIO()
    with contextlib.redirect_stderr(stderr):
        assert verify_web.main(["verify_web.py", "--unknown-flag"]) == BAD_CLI_EXIT_CODE
    assert "usage: verify_web.py" in stderr.getvalue()


def test_the_verdict_command_passes_a_sound_verdict_and_fails_a_broken_one(
    tmp_path: Path,
) -> None:
    sound = tmp_path / "verdict.json"
    sound.write_text(json.dumps(ok_verdict()), encoding="utf-8")
    stdout = io.StringIO()
    with contextlib.redirect_stdout(stdout):
        assert verify_web.main(["verify_web.py", "--verdict", str(sound)]) == 0
    assert "OK verify-web (verdict)" in stdout.getvalue()
    broken = tmp_path / "broken.json"
    broken.write_text(json.dumps({"ok": False}), encoding="utf-8")
    stderr = io.StringIO()
    with contextlib.redirect_stderr(stderr):
        assert verify_web.main(["verify_web.py", "--verdict", str(broken)]) == 1
    assert "FAIL verify-web (verdict)" in stderr.getvalue()
    stderr = io.StringIO()
    with contextlib.redirect_stderr(stderr):
        assert verify_web.main(["verify_web.py", "--verdict", str(tmp_path / "absent.json")]) == 1
    assert "unreadable" in stderr.getvalue()
