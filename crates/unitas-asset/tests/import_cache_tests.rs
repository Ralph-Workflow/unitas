use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use unitas_asset::audio_import::{cook_project_audio, default_audio_cache_dir, load_or_cook_audio};
use unitas_asset::model::{
    cook_project_meshes, default_mesh_cache_dir, default_mesh_cache_dir_for_path,
    load_or_cook_mesh, COOKED_MESH_DIR_NAME,
};
use unitas_asset::texture::{
    cook_project_textures, default_texture_cache_dir, load_or_cook_texture, COOKED_DIR_NAME,
};
use unitas_asset::video_import::{
    cook_project_videos, default_video_cache_dir, load_or_cook_video, COOKED_VIDEO_DIR_NAME,
};

type TestResult = Result<(), Box<dyn Error>>;

const SAMPLE_OBJ: &str =
    "o Hero\nv 0.0 0.0 0.0\nv 1.0 0.0 0.0\nv 0.0 1.0 0.0\nvn 0.0 0.0 1.0\nvt 0.0 0.0\nvt 1.0 0.0\nvt 0.0 1.0\nf 1/1/1 2/2/1 3/3/1\n";

fn temp_test_dir(name: &str) -> std::io::Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!("unitas-{name}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn minimal_tga_bytes(width: u16, height: u16, rgb: [u8; 3]) -> Vec<u8> {
    let mut bytes = vec![0u8; 18 + (width as usize) * (height as usize) * 3];
    bytes[2] = 2;
    bytes[12..14].copy_from_slice(&width.to_le_bytes());
    bytes[14..16].copy_from_slice(&height.to_le_bytes());
    bytes[16] = 24;
    bytes[17] = 0x20;
    for chunk in bytes[18..].chunks_exact_mut(3) {
        chunk[0] = rgb[2];
        chunk[1] = rgb[1];
        chunk[2] = rgb[0];
    }
    bytes
}

fn minimal_wav_bytes(channels: u16, sample_rate: u32, pcm_samples: &[i16]) -> Vec<u8> {
    let byte_rate = sample_rate * u32::from(channels) * 2;
    let block_align = channels * 2;
    let data_size = (pcm_samples.len() * 2) as u32;
    let riff_size = 4 + (8 + 16) + (8 + data_size);

    let mut bytes = Vec::with_capacity(44 + pcm_samples.len() * 2);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&riff_size.to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&byte_rate.to_le_bytes());
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_size.to_le_bytes());
    for sample in pcm_samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}

fn synthetic_ivf_bytes(
    width: u16,
    height: u16,
    fps_num: u32,
    fps_den: u32,
    frames: u32,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(32);
    out.extend_from_slice(b"DKIF");
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(b"VP80");
    out.extend_from_slice(&width.to_le_bytes());
    out.extend_from_slice(&height.to_le_bytes());
    out.extend_from_slice(&fps_num.to_le_bytes());
    out.extend_from_slice(&fps_den.to_le_bytes());
    out.extend_from_slice(&frames.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out
}

fn assert_no_pid_segment(path: &Path) {
    let pid_prefix = format!("{}_", std::process::id());
    for component in path.components() {
        let piece = component.as_os_str().to_string_lossy();
        assert!(
            !piece.starts_with(&pid_prefix),
            "cache path {path:?} must not contain pid prefix {pid_prefix}"
        );
    }
}

#[test]
fn default_import_cache_dirs_reside_under_project_library_and_use_pid_free_fallback() -> TestResult
{
    let root = temp_test_dir("default-import-cache-dirs")?;
    let project = root.join("Project");
    fs::create_dir_all(project.join("Assets"))?;
    fs::create_dir_all(project.join("ProjectSettings"))?;

    let expected_base = project.join("Library").join("unitas").join("cache");
    assert_eq!(
        default_texture_cache_dir(&project),
        default_texture_cache_dir(&project)
    );
    assert_eq!(
        default_texture_cache_dir(&project),
        expected_base.join("texture")
    );
    assert_eq!(
        default_mesh_cache_dir(&project),
        default_mesh_cache_dir(&project)
    );
    assert_eq!(default_mesh_cache_dir(&project), expected_base.join("mesh"));
    assert_eq!(
        default_audio_cache_dir(&project),
        default_audio_cache_dir(&project)
    );
    assert_eq!(
        default_audio_cache_dir(&project),
        expected_base.join("audio")
    );
    assert_eq!(
        default_video_cache_dir(&project),
        default_video_cache_dir(&project)
    );
    assert_eq!(
        default_video_cache_dir(&project),
        expected_base.join("video")
    );

    let missing_project = root.join("NonExistentProject");
    let tex_fallback = default_texture_cache_dir(&missing_project);
    let mesh_fallback = default_mesh_cache_dir(&missing_project);
    let audio_fallback = default_audio_cache_dir(&missing_project);
    let video_fallback = default_video_cache_dir(&missing_project);

    assert_eq!(tex_fallback, default_texture_cache_dir(&missing_project));
    assert_eq!(mesh_fallback, default_mesh_cache_dir(&missing_project));
    assert_eq!(audio_fallback, default_audio_cache_dir(&missing_project));
    assert_eq!(video_fallback, default_video_cache_dir(&missing_project));

    assert!(tex_fallback.starts_with(std::env::temp_dir().join("unitas_texture_import_cache")));
    assert!(mesh_fallback.starts_with(std::env::temp_dir().join("unitas_mesh_import_cache")));
    assert!(audio_fallback.starts_with(std::env::temp_dir().join("unitas_audio_import_cache")));
    assert!(video_fallback.starts_with(std::env::temp_dir().join("unitas_video_import_cache")));

    assert_no_pid_segment(&tex_fallback);
    assert_no_pid_segment(&mesh_fallback);
    assert_no_pid_segment(&audio_fallback);
    assert_no_pid_segment(&video_fallback);

    let standalone_mesh = root.join("standalone").join("Hero.obj");
    let fp = unitas_asset::dependencies::ImportFingerprint {
        source: 42,
        metadata: 0,
        settings: 0,
        importer_version: 1,
        dependencies: 0,
    };
    let mesh_path_fallback = default_mesh_cache_dir_for_path(&standalone_mesh, &fp);
    assert_eq!(
        mesh_path_fallback,
        default_mesh_cache_dir_for_path(&standalone_mesh, &fp)
    );
    assert_no_pid_segment(&mesh_path_fallback);

    fs::remove_dir_all(&root)?;
    Ok(())
}

fn setup_test_project(project: &Path) -> TestResult {
    let tex_dir = project.join("Assets").join("Textures");
    let mesh_dir = project.join("Assets").join("Models");
    let audio_dir = project.join("Assets").join("Audio");
    let video_dir = project.join("Assets").join("Video");
    fs::create_dir_all(&tex_dir)?;
    fs::create_dir_all(&mesh_dir)?;
    fs::create_dir_all(&audio_dir)?;
    fs::create_dir_all(&video_dir)?;
    fs::create_dir_all(project.join("ProjectSettings"))?;

    fs::write(
        tex_dir.join("Albedo.tga"),
        minimal_tga_bytes(4, 4, [200, 100, 50]),
    )?;
    fs::write(
        tex_dir.join("Albedo.tga.meta"),
        "fileFormatVersion: 2\nguid: 11111111111111111111111111111111\n",
    )?;
    fs::write(mesh_dir.join("Hero.obj"), SAMPLE_OBJ.as_bytes())?;
    fs::write(
        mesh_dir.join("Hero.obj.meta"),
        "fileFormatVersion: 2\nguid: 22222222222222222222222222222222\n",
    )?;
    fs::write(
        audio_dir.join("Chime.wav"),
        minimal_wav_bytes(1, 22050, &[0, 8192, -8192, 0]),
    )?;
    fs::write(
        audio_dir.join("Chime.wav.meta"),
        "fileFormatVersion: 2\nguid: 33333333333333333333333333333333\n",
    )?;
    fs::write(
        video_dir.join("Clip.ivf"),
        synthetic_ivf_bytes(64, 48, 24, 1, 12),
    )?;
    fs::write(
        video_dir.join("Clip.ivf.meta"),
        "fileFormatVersion: 2\nguid: 44444444444444444444444444444444\n",
    )?;
    Ok(())
}

#[test]
fn second_cook_session_reuses_persistent_library_cache_across_importers() -> TestResult {
    let root = temp_test_dir("persistent-library-cache-reuse")?;
    let project = root.join("Project");
    setup_test_project(&project)?;

    let out_one = root.join("OutOne");
    let (_, tex_first) = cook_project_textures(
        &project,
        &default_texture_cache_dir(&project),
        &out_one.join(COOKED_DIR_NAME),
    )?;
    let (_, mesh_first) = cook_project_meshes(
        &project,
        &default_mesh_cache_dir(&project),
        &out_one.join(COOKED_MESH_DIR_NAME),
    )?;
    let (_, audio_first) = cook_project_audio(
        &project,
        &default_audio_cache_dir(&project),
        &out_one.join(COOKED_DIR_NAME),
    )?;
    let (_, video_first) = cook_project_videos(
        &project,
        &default_video_cache_dir(&project),
        &out_one.join(COOKED_VIDEO_DIR_NAME),
    )?;
    assert_eq!((tex_first.imported, tex_first.reused), (1, 0));
    assert_eq!((mesh_first.imported, mesh_first.reused), (1, 0));
    assert_eq!((audio_first.imported, audio_first.reused), (1, 0));
    assert_eq!((video_first.imported, video_first.reused), (1, 0));

    assert!(project
        .join("Library/unitas/cache/texture/import-cache.json")
        .is_file());
    assert!(project
        .join("Library/unitas/cache/mesh/import-cache.json")
        .is_file());
    assert!(project
        .join("Library/unitas/cache/audio/import-cache.json")
        .is_file());
    assert!(project
        .join("Library/unitas/cache/video/import-cache.json")
        .is_file());

    let out_two = root.join("OutTwo");
    let (_, tex_second) = cook_project_textures(
        &project,
        &default_texture_cache_dir(&project),
        &out_two.join(COOKED_DIR_NAME),
    )?;
    let (_, mesh_second) = cook_project_meshes(
        &project,
        &default_mesh_cache_dir(&project),
        &out_two.join(COOKED_MESH_DIR_NAME),
    )?;
    let (_, audio_second) = cook_project_audio(
        &project,
        &default_audio_cache_dir(&project),
        &out_two.join(COOKED_DIR_NAME),
    )?;
    let (_, video_second) = cook_project_videos(
        &project,
        &default_video_cache_dir(&project),
        &out_two.join(COOKED_VIDEO_DIR_NAME),
    )?;
    assert_eq!((tex_second.imported, tex_second.reused), (0, 1));
    assert_eq!((mesh_second.imported, mesh_second.reused), (0, 1));
    assert_eq!((audio_second.imported, audio_second.reused), (0, 1));
    assert_eq!((video_second.imported, video_second.reused), (0, 1));

    assert!(load_or_cook_texture(&project.join("Assets/Textures/Albedo.tga")).is_some());
    assert!(load_or_cook_mesh(&project.join("Assets/Models/Hero.obj")).is_some());
    assert!(load_or_cook_audio(&project.join("Assets/Audio/Chime.wav")).is_some());
    assert!(load_or_cook_video(&project.join("Assets/Video/Clip.ivf")).is_some());

    fs::remove_dir_all(&root)?;
    Ok(())
}
