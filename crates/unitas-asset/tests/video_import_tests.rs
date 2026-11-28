use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use unitas_asset::video_import::{
    build_video_candidate, cook_project_videos, cook_video_candidates, import_video,
    is_supported_video_extension, is_video_source_path, load_or_cook_video_with_cache,
    probe_video_stream, video_import_outcome, CookedVideo, CookedVideoManifest,
    VideoContainerFormat, VideoImportError, VideoImportSettings, COOKED_VIDEO_MAGIC,
};

type TestResult = Result<(), Box<dyn Error>>;

fn synthetic_mp4_bytes(width: u16, height: u16, fps: u32, frame_count: u32) -> Vec<u8> {
    let mut ftyp = Vec::new();
    ftyp.extend_from_slice(&20u32.to_be_bytes());
    ftyp.extend_from_slice(b"ftypisom");
    ftyp.extend_from_slice(&0u32.to_be_bytes());
    ftyp.extend_from_slice(b"isom");

    let timescale = fps.max(1) * 1000;
    let sample_delta = 1000u32;
    let duration = u64::from(frame_count) * u64::from(sample_delta);

    let mut mvhd = Vec::new();
    mvhd.extend_from_slice(&32u32.to_be_bytes());
    mvhd.extend_from_slice(b"mvhd");
    mvhd.extend_from_slice(&[0, 0, 0, 0]);
    mvhd.extend_from_slice(&0u32.to_be_bytes());
    mvhd.extend_from_slice(&0u32.to_be_bytes());
    mvhd.extend_from_slice(&timescale.to_be_bytes());
    mvhd.extend_from_slice(&(duration as u32).to_be_bytes());
    mvhd.extend_from_slice(&0x0001_0000u32.to_be_bytes());

    let mut tkhd = vec![0u8; 92];
    tkhd[0..4].copy_from_slice(&92u32.to_be_bytes());
    tkhd[4..8].copy_from_slice(b"tkhd");
    let width_fixed = u32::from(width) << 16;
    let height_fixed = u32::from(height) << 16;
    tkhd[84..88].copy_from_slice(&width_fixed.to_be_bytes());
    tkhd[88..92].copy_from_slice(&height_fixed.to_be_bytes());

    let mut mdhd = Vec::new();
    mdhd.extend_from_slice(&32u32.to_be_bytes());
    mdhd.extend_from_slice(b"mdhd");
    mdhd.extend_from_slice(&[0, 0, 0, 0]);
    mdhd.extend_from_slice(&0u32.to_be_bytes());
    mdhd.extend_from_slice(&0u32.to_be_bytes());
    mdhd.extend_from_slice(&timescale.to_be_bytes());
    mdhd.extend_from_slice(&(duration as u32).to_be_bytes());
    mdhd.extend_from_slice(&0u32.to_be_bytes());

    let mut hdlr = Vec::new();
    hdlr.extend_from_slice(&24u32.to_be_bytes());
    hdlr.extend_from_slice(b"hdlr");
    hdlr.extend_from_slice(&[0, 0, 0, 0]);
    hdlr.extend_from_slice(&0u32.to_be_bytes());
    hdlr.extend_from_slice(b"vide");
    hdlr.extend_from_slice(&0u32.to_be_bytes());

    let mut stsd = vec![0u8; 16 + 40];
    let stsd_len = stsd.len() as u32;
    stsd[0..4].copy_from_slice(&stsd_len.to_be_bytes());
    stsd[4..8].copy_from_slice(b"stsd");
    stsd[12..16].copy_from_slice(&1u32.to_be_bytes());
    stsd[16..20].copy_from_slice(&40u32.to_be_bytes());
    stsd[20..24].copy_from_slice(b"avc1");
    stsd[16 + 32..16 + 34].copy_from_slice(&width.to_be_bytes());
    stsd[16 + 34..16 + 36].copy_from_slice(&height.to_be_bytes());

    let mut stts = Vec::new();
    stts.extend_from_slice(&24u32.to_be_bytes());
    stts.extend_from_slice(b"stts");
    stts.extend_from_slice(&[0, 0, 0, 0]);
    stts.extend_from_slice(&1u32.to_be_bytes());
    stts.extend_from_slice(&frame_count.to_be_bytes());
    stts.extend_from_slice(&sample_delta.to_be_bytes());

    let mut stsz = Vec::new();
    stsz.extend_from_slice(&20u32.to_be_bytes());
    stsz.extend_from_slice(b"stsz");
    stsz.extend_from_slice(&[0, 0, 0, 0]);
    stsz.extend_from_slice(&256u32.to_be_bytes());
    stsz.extend_from_slice(&frame_count.to_be_bytes());

    let mut stbl = Vec::new();
    let stbl_len = (8 + stsd.len() + stts.len() + stsz.len()) as u32;
    stbl.extend_from_slice(&stbl_len.to_be_bytes());
    stbl.extend_from_slice(b"stbl");
    stbl.extend_from_slice(&stsd);
    stbl.extend_from_slice(&stts);
    stbl.extend_from_slice(&stsz);

    let mut minf = Vec::new();
    let minf_len = (8 + stbl.len()) as u32;
    minf.extend_from_slice(&minf_len.to_be_bytes());
    minf.extend_from_slice(b"minf");
    minf.extend_from_slice(&stbl);

    let mut mdia = Vec::new();
    let mdia_len = (8 + mdhd.len() + hdlr.len() + minf.len()) as u32;
    mdia.extend_from_slice(&mdia_len.to_be_bytes());
    mdia.extend_from_slice(b"mdia");
    mdia.extend_from_slice(&mdhd);
    mdia.extend_from_slice(&hdlr);
    mdia.extend_from_slice(&minf);

    let mut trak = Vec::new();
    let trak_len = (8 + tkhd.len() + mdia.len()) as u32;
    trak.extend_from_slice(&trak_len.to_be_bytes());
    trak.extend_from_slice(b"trak");
    trak.extend_from_slice(&tkhd);
    trak.extend_from_slice(&mdia);

    let mut moov = Vec::new();
    let moov_len = (8 + mvhd.len() + trak.len()) as u32;
    moov.extend_from_slice(&moov_len.to_be_bytes());
    moov.extend_from_slice(b"moov");
    moov.extend_from_slice(&mvhd);
    moov.extend_from_slice(&trak);

    let mut out = ftyp;
    out.extend_from_slice(&moov);
    out
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

#[test]
fn mp4_and_ivf_stream_probing_and_cooked_roundtrip_work_offline() -> TestResult {
    assert!(is_supported_video_extension("mp4"));
    assert!(is_supported_video_extension("webm"));
    assert!(is_supported_video_extension("mov"));
    assert!(is_video_source_path(Path::new("Assets/Video/intro.mp4")));
    assert!(!is_video_source_path(Path::new("Assets/Audio/intro.wav")));

    let mp4_bytes = synthetic_mp4_bytes(320, 180, 30, 60);
    let meta = probe_video_stream(&mp4_bytes, "intro.mp4")?;
    assert_eq!(meta.container, VideoContainerFormat::Mp4);
    assert_eq!(meta.codec_name(), "avc1");
    assert_eq!(meta.width, 320);
    assert_eq!(meta.height, 180);
    assert_eq!(meta.fps_num, 30);
    assert_eq!(meta.fps_den, 1);
    assert_eq!(meta.frame_count, 60);
    assert_eq!(meta.duration_micros, 2_000_000);

    let settings = VideoImportSettings::from_meta_text(
        "fileFormatVersion: 2\nguid: 00000000000000000000000000000001\nVideoClipImporter:\n  keepAlpha: 1\n  deinterlace: 0\n  flipHorizontally: 0\n  flipVertically: 1\n  importAudio: 1\n",
    )?;
    assert!(settings.keep_alpha);
    assert!(settings.flip_vertically);
    assert!(!settings.flip_horizontally);

    let imported = import_video(&mp4_bytes, "intro.mp4", &settings)?;
    let outcome = video_import_outcome("Assets/Video/intro.mp4", Ok(&imported));
    assert!(outcome.diagnostics.is_empty());

    let cooked = CookedVideo::from_import(&imported);
    let encoded = cooked.encode();
    assert!(encoded.starts_with(&COOKED_VIDEO_MAGIC));
    let decoded = CookedVideo::decode(&encoded)?;
    assert_eq!(decoded, cooked);
    assert!((decoded.frame_rate() - 30.0).abs() < 1e-6);
    assert!((decoded.duration_seconds() - 2.0).abs() < 1e-6);

    let ivf_bytes = synthetic_ivf_bytes(160, 120, 24, 1, 48);
    let ivf_meta = probe_video_stream(&ivf_bytes, "clip.ivf")?;
    assert_eq!(ivf_meta.container, VideoContainerFormat::Ivf);
    assert_eq!(ivf_meta.codec_name(), "VP80");
    assert_eq!(ivf_meta.width, 160);
    assert_eq!(ivf_meta.height, 120);
    assert_eq!(ivf_meta.frame_count, 48);

    assert!(matches!(
        probe_video_stream(b"not a video container at all", "bad.mp4"),
        Err(VideoImportError::UnsupportedContainer(_))
    ));
    Ok(())
}

#[test]
fn cook_project_videos_and_manifest_roundtrip() -> TestResult {
    let temp = std::env::temp_dir().join(format!(
        "unitas_asset_video_cook_test_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&temp);
    let project_root = temp.join("project");
    let video_dir = project_root.join("Assets/Video");
    fs::create_dir_all(&video_dir)?;
    fs::create_dir_all(project_root.join("ProjectSettings"))?;

    let video_path = video_dir.join("testsrc.mp4");
    let ffmpeg_ok = Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=64x48:rate=15:duration=2",
            "-c:v",
            "mpeg4",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(&video_path)
        .output()
        .is_ok_and(|out| out.status.success());

    if !ffmpeg_ok {
        fs::write(&video_path, synthetic_mp4_bytes(64, 48, 15, 30))?;
    }
    fs::write(
        video_dir.join("testsrc.mp4.meta"),
        "fileFormatVersion: 2\nguid: 00000000000000000000000000000002\nVideoClipImporter:\n  keepAlpha: 0\n",
    )?;

    let cache_dir = temp.join("cache");
    let cooked_out = temp.join("cooked_out");
    let (manifest, report) = cook_project_videos(&project_root, &cache_dir, &cooked_out)?;
    assert_eq!(report.imported, 1);
    let entry = manifest
        .get_by_asset_path("Assets/Video/testsrc.mp4")
        .ok_or("missing cooked video entry")?;
    assert_eq!(entry.guid, "00000000000000000000000000000002");
    assert_eq!(entry.width, 64);
    assert_eq!(entry.height, 48);
    assert_eq!(entry.fps_num / entry.fps_den.max(1), 15);
    assert_eq!(entry.frame_count, 30);

    let reloaded_manifest = CookedVideoManifest::load_from_dir(&cooked_out)
        .ok_or("failed to reload cooked video manifest")?;
    assert_eq!(reloaded_manifest, manifest);

    let cooked = load_or_cook_video_with_cache(&video_path, Some(&cache_dir))
        .ok_or("failed to load cooked video from cache")?;
    assert_eq!(cooked.width, 64);
    assert_eq!(cooked.height, 48);
    assert_eq!(cooked.frame_count, 30);

    let candidate = build_video_candidate(&video_path, PathBuf::from("Assets/Video/testsrc.mp4"))?;
    let (_, cached_report) =
        cook_video_candidates(std::slice::from_ref(&candidate), &cache_dir, None)?;
    assert_eq!(cached_report.reused, 1);

    let _ = fs::remove_dir_all(&temp);
    Ok(())
}
