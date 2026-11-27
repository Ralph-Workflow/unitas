from __future__ import annotations

import pytest

from tools import verify_device

DEVICE_LINE = (
    'device_verify=ok reference=tools/approved_gpu_reference.bmp size=320x240 '
    "frames_presented=240 mean_error=0.0000 tolerance=4.0000 audio_active=true "
    'audio_status="active" audio_frames=240 final_pos=(1.000,0.500,2.000) '
    'canvas=true canvas_texts="Score 0 | Combo x1 | Time 0.0" shadow=true'
)
GAME_SCORE = 2
CONTROL_FRAMES = 30


def test_the_device_report_carries_the_generic_canvas_facts() -> None:
    report = verify_device.parse_device_report(DEVICE_LINE)
    assert report is not None
    assert report["canvas"] == "true"
    assert report["canvas_texts"] == "Score 0 | Combo x1 | Time 0.0"
    assert report["shadow"] == "true"
    assert "score" not in report
    assert "won" not in report
    assert "hud" not in report


def test_the_canvas_texts_split_on_the_reported_separator() -> None:
    report = verify_device.parse_device_report(DEVICE_LINE)
    assert report is not None
    assert verify_device.canvas_texts(report) == ["Score 0", "Combo x1", "Time 0.0"]


def test_a_complete_device_report_passes_the_contract() -> None:
    report = verify_device.parse_device_report(DEVICE_LINE)
    assert verify_device.device_failure_reason(report) is None


def test_a_device_report_without_the_canvas_fails_the_contract() -> None:
    line = DEVICE_LINE.replace("canvas=true", "canvas=false")
    report = verify_device.parse_device_report(line)
    reason = verify_device.device_failure_reason(report)
    assert reason is not None
    assert "canvas" in reason


def test_a_device_report_without_canvas_text_fails_the_contract() -> None:
    line = DEVICE_LINE.replace('canvas_texts="Score 0 | Combo x1 | Time 0.0"', 'canvas_texts=""')
    report = verify_device.parse_device_report(line)
    reason = verify_device.device_failure_reason(report)
    assert reason is not None
    assert "canvas_texts" in reason


def test_a_device_report_without_the_shadow_fails_the_contract() -> None:
    line = DEVICE_LINE.replace("shadow=true", "shadow=false")
    report = verify_device.parse_device_report(line)
    reason = verify_device.device_failure_reason(report)
    assert reason is not None
    assert "shadow" in reason


def test_a_device_report_without_a_line_fails_the_contract() -> None:
    reason = verify_device.device_failure_reason(verify_device.parse_device_report(""))
    assert reason is not None
    assert "device verification line" in reason


def test_the_game_progress_marker_is_read_from_the_player_log() -> None:
    lines = ["Run started", "Pickup collected score 1", "Pickup collected score 2"]
    assert verify_device.game_score(lines) == GAME_SCORE
    assert verify_device.game_progress_reason(lines) is None


def test_a_player_log_without_the_progress_marker_fails_the_progress_check() -> None:
    reason = verify_device.game_progress_reason(["Run started"])
    assert reason is not None
    assert "collected pickup" in reason


def test_the_control_state_carries_the_generic_fields() -> None:
    payload = {"ok": True, "state": "playing", "frames": CONTROL_FRAMES, "script_host": "coreclr"}
    assert verify_device.control_state(payload) == CONTROL_FRAMES


def test_the_control_state_requires_the_coreclr_script_host() -> None:
    payload = {"ok": True, "state": "playing", "frames": CONTROL_FRAMES, "script_host": "none"}
    with pytest.raises(verify_device.ControlCheckError, match="CoreCLR"):
        verify_device.control_state(payload)


def test_the_control_state_requires_its_fields() -> None:
    payload = {"ok": True, "state": "playing"}
    with pytest.raises(verify_device.ControlCheckError, match="frames"):
        verify_device.control_state(payload)
