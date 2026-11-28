use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use unitas_asset::video_import::{
    COOKED_VIDEO_FORMAT_VERSION, CookedVideo, VideoContainerFormat, load_or_cook_video_with_cache,
};
use unitas_core::EntityId;
use unitas_runtime::{
    Runtime, VIDEO_STATE_BLOB_BYTES, decode_cooked_video, ffmpeg_libraries_available,
    test_read_path_buffer, test_write_path_buffer, unitas_runtime_destroy, unitas_runtime_new,
    unitas_runtime_path_buffer, unitas_runtime_video_advance, unitas_runtime_video_create,
    unitas_runtime_video_destroy, unitas_runtime_video_get_frame_rgba,
    unitas_runtime_video_get_state, unitas_runtime_video_load_clip, unitas_runtime_video_pause,
    unitas_runtime_video_play, unitas_runtime_video_prepare, unitas_runtime_video_set_frame,
    unitas_runtime_video_set_looping, unitas_runtime_video_set_playback_speed,
    unitas_runtime_video_set_send_frame_ready_events, unitas_runtime_video_set_target_entity,
    unitas_runtime_video_set_time, unitas_runtime_video_step_forward, unitas_runtime_video_stop,
    unitas_runtime_video_wait_prepared,
};

type TestResult = Result<(), Box<dyn Error>>;

fn generate_two_color_mp4(path: &Path) -> bool {
    Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=red:s=64x48:r=15:d=1",
            "-f",
            "lavfi",
            "-i",
            "color=c=blue:s=64x48:r=15:d=1",
            "-filter_complex",
            "[0:v][1:v]concat=n=2:v=1:a=0",
            "-c:v",
            "mpeg4",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(path)
        .output()
        .is_ok_and(|out| out.status.success())
}

#[test]
fn video_player_corrupt_clip_and_abi_error_paths_report_cleanly() -> TestResult {
    let mut runtime = Runtime::new()?;
    let player = runtime.create_video_player(EntityId::new(1));
    assert!(!runtime.load_video_clip(player, Path::new("/nonexistent/clip.mp4")));
    let snap = runtime
        .take_video_player_snapshot(player)
        .ok_or("expected snapshot for player")?;
    assert!(!snap.is_prepared);
    assert!(snap.events.error_received);
    assert!(snap.last_error.is_some());
    assert!(!runtime.diagnostics().is_empty());

    let corrupt = CookedVideo {
        format_version: COOKED_VIDEO_FORMAT_VERSION,
        container: VideoContainerFormat::Mp4,
        codec_tag: *b"mp4v",
        width: 64,
        height: 48,
        fps_num: 15,
        fps_den: 1,
        frame_count: 10,
        duration_micros: 1_000_000,
        payload: b"not a valid mp4 stream".to_vec(),
    };
    if ffmpeg_libraries_available() {
        assert!(decode_cooked_video(&corrupt).is_err());
    }

    assert!(runtime.destroy_video_player(player));
    assert!(!runtime.destroy_video_player(player));
    Ok(())
}

const VIDEO_SURFACE_SCENE_YAML: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &10
GameObject:
  m_Name: Main Camera
  m_TagString: MainCamera
--- !u!4 &11
Transform:
  m_GameObject: {fileID: 10}
  m_LocalPosition: {x: 0, y: 0, z: -5}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!20 &12
Camera:
  m_GameObject: {fileID: 10}
  orthographic: 1
  orthographic size: 0.2
  near clip plane: 0.1
  far clip plane: 100
  m_BackGroundColor: {r: 0, g: 0, b: 0, a: 1}
--- !u!1 &20
GameObject:
  m_Name: VideoSurface
--- !u!4 &21
Transform:
  m_GameObject: {fileID: 20}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
";

fn assert_cooked_two_color_video(video_path: &Path, cache_dir: &Path) -> TestResult {
    let cooked = load_or_cook_video_with_cache(video_path, Some(cache_dir))
        .ok_or("failed to cook two_color.mp4")?;
    assert_eq!(cooked.width, 64);
    assert_eq!(cooked.height, 48);
    assert_eq!(cooked.frame_count, 30);

    let decoded = decode_cooked_video(&cooked)?;
    assert_eq!(decoded.width, 64);
    assert_eq!(decoded.height, 48);
    assert_eq!(decoded.frames_rgba.len(), 30);
    let first_frame = &decoded.frames_rgba[0];
    let last_frame = &decoded.frames_rgba[29];
    assert_ne!(first_frame, last_frame);
    assert!(
        first_frame[0] > 180 && first_frame[2] < 80,
        "expected red first frame, got {:?}",
        &first_frame[0..4]
    );
    assert!(
        last_frame[2] > 180 && last_frame[0] < 80,
        "expected blue last frame, got {:?}",
        &last_frame[0..4]
    );
    Ok(())
}

#[test]
fn ffmpeg_worker_decodes_cooked_video_and_renders_first_and_later_frames_to_scene_capture()
-> TestResult {
    if !ffmpeg_libraries_available() {
        eprintln!("skipping FFmpeg decode test: FFmpeg shared libraries not available on host");
        return Ok(());
    }
    let temp =
        std::env::temp_dir().join(format!("unitas_runtime_video_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp);
    let assets_dir = temp.join("Assets");
    fs::create_dir_all(&assets_dir)?;
    fs::create_dir_all(temp.join("ProjectSettings"))?;

    let video_path = assets_dir.join("two_color.mp4");
    if !generate_two_color_mp4(&video_path) {
        let _ = fs::remove_dir_all(&temp);
        eprintln!("skipping FFmpeg decode test: ffmpeg CLI not available to generate test clip");
        return Ok(());
    }

    assert_cooked_two_color_video(&video_path, &temp.join("cache"))?;

    let scene_path = assets_dir.join("VideoScene.unity");
    fs::write(&scene_path, VIDEO_SURFACE_SCENE_YAML)?;

    let mut runtime = Runtime::new()?;
    runtime.load_scene(&scene_path)?;
    let surface_entity = runtime
        .find_entity_by_name("VideoSurface")
        .ok_or("missing VideoSurface entity in loaded scene")?;
    let player = runtime.create_video_player(surface_entity);
    assert!(runtime.set_video_player_send_frame_ready_events(player, true));
    assert!(runtime.load_video_clip(player, &video_path));
    assert!(runtime.prepare_video_player(player));
    assert!(runtime.wait_video_player_prepared(player, Duration::from_secs(5)));

    let prepared_snap = runtime
        .take_video_player_snapshot(player)
        .ok_or("missing prepared snapshot")?;
    assert!(prepared_snap.is_prepared);
    assert!(prepared_snap.events.prepare_completed);
    assert!(prepared_snap.events.frame_ready);
    assert_eq!(prepared_snap.current_frame, 0);

    assert!(runtime.play_video_player(player));
    let capture_first = runtime.capture_scene_frame(64, 64)?;
    let center_first = capture_first.image.get_pixel(32, 32);
    assert!(
        center_first[0] > 150 && center_first[2] < 90,
        "expected captured GPU surface to show red first frame, got {center_first:?}"
    );

    assert!(runtime.advance_video_player(player, 1.5));
    let mid_snap = runtime
        .take_video_player_snapshot(player)
        .ok_or("missing mid snapshot")?;
    assert!(mid_snap.current_frame >= 20);
    assert!(mid_snap.events.frame_ready);

    let capture_later = runtime.capture_scene_frame(64, 64)?;
    let center_later = capture_later.image.get_pixel(32, 32);
    assert!(
        center_later[2] > 150 && center_later[0] < 90,
        "expected captured GPU surface to show blue later frame, got {center_later:?}"
    );
    assert_ne!(center_first, center_later);

    let _ = fs::remove_dir_all(&temp);
    Ok(())
}

fn configure_and_load_abi_video(handle: u64, player: u64, video_path: &Path) -> TestResult {
    let path_bytes = video_path.to_string_lossy().as_bytes().to_vec();
    let buf_capacity = path_bytes.len().max(VIDEO_STATE_BLOB_BYTES);
    let buf_ptr = unitas_runtime_path_buffer(handle, buf_capacity as u64);
    assert_ne!(buf_ptr, 0);
    test_write_path_buffer(handle, &path_bytes)?;
    assert_eq!(unitas_runtime_video_set_target_entity(handle, player, 0), 0);
    assert_eq!(unitas_runtime_video_set_looping(handle, player, 1), 0);
    assert_eq!(
        unitas_runtime_video_set_playback_speed(handle, player, 1.0),
        0
    );
    assert_eq!(
        unitas_runtime_video_set_send_frame_ready_events(handle, player, 1),
        0
    );
    assert_eq!(
        unitas_runtime_video_load_clip(handle, player, buf_ptr, path_bytes.len() as u64),
        0
    );
    assert_eq!(unitas_runtime_video_prepare(handle, player), 0);
    assert_eq!(unitas_runtime_video_wait_prepared(handle, player, 5000), 0);
    Ok(())
}

#[test]
fn abi_video_roundtrip_controls_playback_events_and_frame_buffer() -> TestResult {
    if !ffmpeg_libraries_available() {
        eprintln!("skipping FFmpeg ABI decode test: FFmpeg shared libraries not available on host");
        return Ok(());
    }
    let temp = std::env::temp_dir().join(format!(
        "unitas_runtime_abi_video_test_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&temp);
    fs::create_dir_all(&temp)?;
    let video_path = temp.join("abi_clip.mp4");
    if !generate_two_color_mp4(&video_path) {
        let _ = fs::remove_dir_all(&temp);
        eprintln!("skipping FFmpeg ABI decode test: ffmpeg CLI not available");
        return Ok(());
    }

    let handle = unitas_runtime_new();
    assert_ne!(handle, 0);
    let player = unitas_runtime_video_create(handle, 1);
    assert_ne!(player, 0);

    configure_and_load_abi_video(handle, player, &video_path)?;
    assert_eq!(unitas_runtime_video_play(handle, player), 0);
    assert_eq!(unitas_runtime_video_advance(handle, player, 1.5), 0);
    let rgba_len = unitas_runtime_video_get_frame_rgba(handle, player);
    assert_eq!(rgba_len, 64 * 48 * 4);
    let mut first_pixel = [0u8; 4];
    test_read_path_buffer(handle, &mut first_pixel)?;
    assert!(
        first_pixel[2] > 180 && first_pixel[0] < 80,
        "expected blue RGBA pixel from path buffer at t=1.5s, got {first_pixel:?}"
    );

    assert_eq!(unitas_runtime_video_set_frame(handle, player, 0), 0);
    assert_eq!(unitas_runtime_video_step_forward(handle, player), 0);
    assert_eq!(unitas_runtime_video_set_time(handle, player, 0.0), 0);
    assert_eq!(unitas_runtime_video_pause(handle, player), 0);
    let state_ptr = unitas_runtime_path_buffer(handle, VIDEO_STATE_BLOB_BYTES as u64);
    assert_eq!(unitas_runtime_video_get_state(handle, player, state_ptr), 0);
    assert_eq!(unitas_runtime_video_stop(handle, player), 0);
    assert_eq!(unitas_runtime_video_load_clip(handle, player, 0, 10), -2);
    assert_eq!(unitas_runtime_video_destroy(handle, player), 0);
    assert_eq!(unitas_runtime_destroy(handle), 0);

    let _ = fs::remove_dir_all(&temp);
    Ok(())
}
