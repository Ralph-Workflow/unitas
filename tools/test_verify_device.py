from __future__ import annotations

import math
import struct
import wave
from typing import TYPE_CHECKING

from tools import verify_device

if TYPE_CHECKING:
    from collections.abc import Callable
    from pathlib import Path

DEVICE_LINE = (
    "device_verify=ok reference=tools/approved_gpu_reference.bmp size=320x240 "
    "frames_presented=240 mean_error=0.0000 tolerance=4.0000 audio_active=true "
    'audio_status="active" audio_frames=240 final_pos=(1.000,0.500,2.000) '
    'canvas=true canvas_texts="Score 0 | Combo x1 | Time 0.0" shadow=true'
)
GAME_SCORE = 2
CONTROL_FRAMES = 30
CHIME_SAMPLE_RATE = 48_000
CHIME_LEAD_IN_SECONDS = 0.5
CHIME_TONE_SECONDS = 0.25
CHIME_TEST_SECONDS = 1.0
CHIME_TONE_AMPLITUDE = 0.5


def write_capture(
    path: Path,
    tone_hz: float | None,
    lead_in: float = CHIME_LEAD_IN_SECONDS,
    tone_seconds: float = CHIME_TONE_SECONDS,
) -> None:
    count = int(CHIME_SAMPLE_RATE * CHIME_TEST_SECONDS)
    samples = [0.0] * count
    if tone_hz is not None:
        start = round(lead_in * CHIME_SAMPLE_RATE)
        window = round(tone_seconds * CHIME_SAMPLE_RATE)
        for index in range(start, start + window):
            phase = 2.0 * math.pi * tone_hz * index / CHIME_SAMPLE_RATE
            samples[index] = CHIME_TONE_AMPLITUDE * math.sin(phase)
    frames = struct.pack(f"<{count}h", *(int(value * 32767) for value in samples))
    with wave.open(str(path), "wb") as handle:
        handle.setnchannels(1)
        handle.setsampwidth(2)
        handle.setframerate(CHIME_SAMPLE_RATE)
        handle.writeframes(frames)


def assert_control_check_error(action: Callable[[], object], fragment: str) -> None:
    caught: str | None = None
    try:
        action()
    except verify_device.ControlCheckError as error:
        caught = str(error)
    assert caught is not None
    assert fragment in caught


def test_the_device_report_carries_the_generic_canvas_facts() -> None:
    report = verify_device.parse_device_report(DEVICE_LINE)
    assert report is not None
    assert report["canvas"] == "true"
    assert report["canvas_texts"] == "Score 0 | Combo x1 | Time 0.0"
    assert report["shadow"] == "true"
    assert "score" not in report
    assert "won" not in report
    assert "hud" not in report


def test_the_collected_chime_frame_is_read_from_the_player_log() -> None:
    lines = ["collect chime played pickup_frame=42", "unrelated output"]
    assert verify_device.pickup_frames(lines) == [42]
    assert verify_device.pickup_frames(["nothing here"]) == []


def test_a_silent_capture_fails_the_chime_check(tmp_path: Path) -> None:
    capture = tmp_path / "silent.wav"
    write_capture(capture, None)
    reason, summary = verify_device.chime_capture_reason(capture)
    assert reason is not None
    assert "no collect chime" in reason
    assert summary == ""


def test_a_capture_carrying_the_chime_band_passes_the_chime_check(tmp_path: Path) -> None:
    capture = tmp_path / "chime.wav"
    write_capture(capture, verify_device.CHIME_FREQUENCY_HZ)
    reason, summary = verify_device.chime_capture_reason(capture)
    assert reason is None
    assert "audio energy in the window" in summary


def test_a_capture_that_carries_the_band_before_the_injection_fails(tmp_path: Path) -> None:
    capture = tmp_path / "early.wav"
    write_capture(capture, verify_device.CHIME_FREQUENCY_HZ, lead_in=0.0, tone_seconds=0.75)
    reason, _summary = verify_device.chime_capture_reason(capture)
    assert reason is not None
    assert "carries audio before the injected" in reason


def test_a_missing_capture_fails_the_chime_check(tmp_path: Path) -> None:
    reason, summary = verify_device.chime_capture_reason(tmp_path / "absent.wav")
    assert reason is not None
    assert "wrote no audio capture" in reason
    assert summary == ""


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
    assert_control_check_error(lambda: verify_device.control_state(payload), "CoreCLR")


def test_the_control_state_requires_its_fields() -> None:
    payload = {"ok": True, "state": "playing"}
    assert_control_check_error(lambda: verify_device.control_state(payload), "frames")
