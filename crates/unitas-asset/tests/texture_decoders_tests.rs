use std::path::Path;

use unitas_asset::database::{identify_asset_family, RecognizedAssetFamily};
use unitas_asset::dependencies::AssetImportState;
use unitas_asset::texture::hdr::decode_hdr;
use unitas_asset::texture::import::{
    decode_texture, import_texture, DecodedTexture, TextureFormat, TextureImportSettings,
    TextureImporterType,
};
use unitas_asset::texture::psd::decode_psd;

fn adler32(data: &[u8]) -> u32 {
    let mut low = 1u32;
    let mut high = 0u32;
    for byte in data {
        low = (low + u32::from(*byte)) % 65521;
        high = (high + low) % 65521;
    }
    (high << 16) | low
}

fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78u8, 0x01];
    let mut offset = 0usize;
    loop {
        let take = (data.len() - offset).min(65535);
        let final_block = offset + take >= data.len();
        out.push(u8::from(final_block));
        out.extend_from_slice(&(take as u16).to_le_bytes());
        out.extend_from_slice(&(!(take as u16)).to_le_bytes());
        out.extend_from_slice(&data[offset..offset + take]);
        offset += take;
        if final_block {
            break;
        }
    }
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

fn radiance_component(plane: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for chunk in plane.chunks(128) {
        out.push(chunk.len() as u8);
        out.extend_from_slice(chunk);
    }
    out
}

fn radiance_scanline(row: &[[u8; 4]], width: usize) -> Vec<u8> {
    let mut out = vec![2u8, 2u8, (width >> 8) as u8, width as u8];
    for channel in 0..4 {
        let plane: Vec<u8> = row.iter().map(|pixel| pixel[channel]).collect();
        out.extend_from_slice(&radiance_component(&plane));
    }
    out
}

fn radiance_flat_scanline(row: &[[u8; 4]]) -> Vec<u8> {
    let mut out = Vec::new();
    for pixel in row {
        out.extend_from_slice(pixel);
    }
    out
}

fn build_hdr(rows: &[Vec<[u8; 4]>], resolution: &str, flat: bool) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"#?RADIANCE\n");
    out.extend_from_slice(b"FORMAT=32-bit_rle_rgbe\n");
    out.extend_from_slice(b"\n");
    out.extend_from_slice(resolution.as_bytes());
    out.extend_from_slice(b"\n");
    for row in rows {
        if flat {
            out.extend_from_slice(&radiance_flat_scanline(row.as_slice()));
        } else {
            out.extend_from_slice(&radiance_scanline(row.as_slice(), row.len()));
        }
    }
    out
}

fn packbits_row(row: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for chunk in row.chunks(128) {
        out.push((chunk.len() - 1) as u8);
        out.extend_from_slice(chunk);
    }
    out
}

fn predicted_rows(planes: &[Vec<u8>], width: usize, height: usize, depth: u16) -> Vec<u8> {
    let mut out = Vec::new();
    for plane in planes {
        for row in 0..height {
            let start = row * width;
            if depth == 8 {
                out.push(plane[start]);
                for index in 1..width {
                    out.push(plane[start + index].wrapping_sub(plane[start + index - 1]));
                }
            } else {
                let sample = |index: usize| -> u16 {
                    u16::from_be_bytes([plane[(start + index) * 2], plane[(start + index) * 2 + 1]])
                };
                let first = sample(0);
                out.extend_from_slice(&first.to_be_bytes());
                for index in 1..width {
                    let delta = sample(index).wrapping_sub(sample(index - 1));
                    out.extend_from_slice(&delta.to_be_bytes());
                }
            }
        }
    }
    out
}

struct PsdOptions<'a> {
    width: usize,
    height: usize,
    channels: usize,
    depth: u16,
    compression: u16,
    planes: &'a [Vec<u8>],
    layer_section: &'a [u8],
}

fn build_psd(options: &PsdOptions<'_>) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"8BPS");
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&[0u8; 6]);
    out.extend_from_slice(&(options.channels as u16).to_be_bytes());
    out.extend_from_slice(&(options.height as u32).to_be_bytes());
    out.extend_from_slice(&(options.width as u32).to_be_bytes());
    out.extend_from_slice(&options.depth.to_be_bytes());
    out.extend_from_slice(&3u16.to_be_bytes());
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&(options.layer_section.len() as u32).to_be_bytes());
    out.extend_from_slice(options.layer_section);
    out.extend_from_slice(&options.compression.to_be_bytes());
    let sample_bytes = usize::from(options.depth / 8);
    let row_size = options.width * sample_bytes;
    match options.compression {
        0 => {
            for plane in options.planes {
                out.extend_from_slice(plane);
            }
        }
        1 => {
            let rows = options.height * options.channels;
            let mut encoded: Vec<Vec<u8>> = Vec::with_capacity(rows);
            for plane in options.planes {
                for row in 0..options.height {
                    let start = row * row_size;
                    encoded.push(packbits_row(&plane[start..start + row_size]));
                }
            }
            for row in &encoded {
                out.extend_from_slice(&(row.len() as u16).to_be_bytes());
            }
            for row in &encoded {
                out.extend_from_slice(row);
            }
        }
        2 => {
            let mut raw = Vec::new();
            for plane in options.planes {
                raw.extend_from_slice(plane);
            }
            out.extend_from_slice(&zlib_stored(&raw));
        }
        3 => {
            let raw = predicted_rows(options.planes, options.width, options.height, options.depth);
            out.extend_from_slice(&zlib_stored(&raw));
        }
        other => out.extend_from_slice(&other.to_be_bytes()),
    }
    out
}

fn rgb_planes() -> Vec<Vec<u8>> {
    vec![
        vec![10u8, 20, 30, 40],
        vec![50u8, 60, 70, 80],
        vec![90u8, 100, 110, 120],
    ]
}

fn rgba_planes() -> Vec<Vec<u8>> {
    vec![
        vec![10u8, 20, 30, 40],
        vec![50u8, 60, 70, 80],
        vec![90u8, 100, 110, 120],
        vec![255u8, 128, 64, 0],
    ]
}

fn planes_16bit() -> Vec<Vec<u8>> {
    vec![
        vec![
            0x12u8, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0x11, 0x11, 0x22, 0x22, 0x33, 0x33,
            0x44, 0x44,
        ],
        vec![
            0x0Fu8, 0x1F, 0x2F, 0x3F, 0x4F, 0x5F, 0x6F, 0x7F, 0x01, 0x01, 0x02, 0x02, 0x03, 0x03,
            0x04, 0x04,
        ],
        vec![
            0x01u8, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
            0x0F, 0x10,
        ],
    ]
}

fn pixel(texture: &DecodedTexture, x: u32, y: u32) -> Vec<u8> {
    texture.pixel(x, y).map(<[u8]>::to_vec).unwrap_or_default()
}

#[test]
fn u4_psd_rle_composite_decodes_to_the_known_pixels() -> Result<(), Box<dyn std::error::Error>> {
    let planes = rgb_planes();
    let bytes = build_psd(&PsdOptions {
        width: 2,
        height: 2,
        channels: 3,
        depth: 8,
        compression: 1,
        planes: &planes,
        layer_section: &[],
    });

    let decoded = decode_psd(&bytes)?;
    assert_eq!(decoded.format, TextureFormat::Rgb8);
    assert_eq!((decoded.width, decoded.height), (2, 2));
    assert_eq!(pixel(&decoded, 0, 0), vec![10, 50, 90]);
    assert_eq!(pixel(&decoded, 1, 0), vec![20, 60, 100]);
    assert_eq!(pixel(&decoded, 0, 1), vec![30, 70, 110]);
    assert_eq!(pixel(&decoded, 1, 1), vec![40, 80, 120]);
    Ok(())
}

#[test]
fn u4_psd_rle_composite_of_a_layered_file_keeps_the_flattened_image(
) -> Result<(), Box<dyn std::error::Error>> {
    let planes = rgb_planes();
    let layer_section = vec![0xAAu8; 64];
    let bytes = build_psd(&PsdOptions {
        width: 2,
        height: 2,
        channels: 3,
        depth: 8,
        compression: 1,
        planes: &planes,
        layer_section: &layer_section,
    });

    let decoded = decode_psd(&bytes)?;
    assert_eq!(decoded.format, TextureFormat::Rgb8);
    assert_eq!(pixel(&decoded, 0, 0), vec![10, 50, 90]);
    assert_eq!(pixel(&decoded, 1, 1), vec![40, 80, 120]);
    Ok(())
}

#[test]
fn u4_psd_rle_composite_keeps_the_alpha_channel() -> Result<(), Box<dyn std::error::Error>> {
    let planes = rgba_planes();
    let bytes = build_psd(&PsdOptions {
        width: 2,
        height: 2,
        channels: 4,
        depth: 8,
        compression: 1,
        planes: &planes,
        layer_section: &[],
    });

    let decoded = decode_psd(&bytes)?;
    assert_eq!(decoded.format, TextureFormat::Rgba8);
    assert_eq!(pixel(&decoded, 0, 0), vec![10, 50, 90, 255]);
    assert_eq!(pixel(&decoded, 1, 0), vec![20, 60, 100, 128]);
    assert_eq!(pixel(&decoded, 0, 1), vec![30, 70, 110, 64]);
    assert_eq!(pixel(&decoded, 1, 1), vec![40, 80, 120, 0]);
    Ok(())
}

#[test]
fn u4_psd_16bit_rle_composite_keeps_the_high_byte_like_the_png_path(
) -> Result<(), Box<dyn std::error::Error>> {
    let planes = planes_16bit();
    let bytes = build_psd(&PsdOptions {
        width: 4,
        height: 2,
        channels: 3,
        depth: 16,
        compression: 1,
        planes: &planes,
        layer_section: &[],
    });

    let decoded = decode_psd(&bytes)?;
    assert_eq!(decoded.format, TextureFormat::Rgb8);
    assert_eq!((decoded.width, decoded.height), (4, 2));
    assert_eq!(pixel(&decoded, 0, 0), vec![0x12, 0x0F, 0x01]);
    assert_eq!(pixel(&decoded, 1, 0), vec![0x56, 0x2F, 0x03]);
    assert_eq!(pixel(&decoded, 0, 1), vec![0x11, 0x01, 0x09]);
    assert_eq!(pixel(&decoded, 3, 1), vec![0x44, 0x04, 0x0F]);
    Ok(())
}

#[test]
fn u4_psd_zip_composite_decodes_to_the_known_pixels() -> Result<(), Box<dyn std::error::Error>> {
    let planes = rgb_planes();
    let bytes = build_psd(&PsdOptions {
        width: 2,
        height: 2,
        channels: 3,
        depth: 8,
        compression: 2,
        planes: &planes,
        layer_section: &[],
    });

    let decoded = decode_psd(&bytes)?;
    assert_eq!(decoded.format, TextureFormat::Rgb8);
    assert_eq!(pixel(&decoded, 0, 0), vec![10, 50, 90]);
    assert_eq!(pixel(&decoded, 1, 1), vec![40, 80, 120]);
    Ok(())
}

#[test]
fn u4_psd_zip_prediction_composite_decodes_to_the_known_pixels(
) -> Result<(), Box<dyn std::error::Error>> {
    let planes = rgb_planes();
    let bytes = build_psd(&PsdOptions {
        width: 2,
        height: 2,
        channels: 3,
        depth: 8,
        compression: 3,
        planes: &planes,
        layer_section: &[],
    });

    let decoded = decode_psd(&bytes)?;
    assert_eq!(decoded.format, TextureFormat::Rgb8);
    assert_eq!(pixel(&decoded, 0, 0), vec![10, 50, 90]);
    assert_eq!(pixel(&decoded, 1, 0), vec![20, 60, 100]);
    assert_eq!(pixel(&decoded, 0, 1), vec![30, 70, 110]);
    assert_eq!(pixel(&decoded, 1, 1), vec![40, 80, 120]);
    Ok(())
}

#[test]
fn u4_psd_zip_prediction_16bit_composite_decodes_to_the_known_pixels(
) -> Result<(), Box<dyn std::error::Error>> {
    let planes = planes_16bit();
    let bytes = build_psd(&PsdOptions {
        width: 4,
        height: 2,
        channels: 3,
        depth: 16,
        compression: 3,
        planes: &planes,
        layer_section: &[],
    });

    let decoded = decode_psd(&bytes)?;
    assert_eq!(decoded.format, TextureFormat::Rgb8);
    assert_eq!(pixel(&decoded, 0, 0), vec![0x12, 0x0F, 0x01]);
    assert_eq!(pixel(&decoded, 3, 1), vec![0x44, 0x04, 0x0F]);
    Ok(())
}

#[test]
fn u4_psd_import_builds_the_mip_chain_and_srgb_flag_of_the_png_path(
) -> Result<(), Box<dyn std::error::Error>> {
    let planes = rgb_planes();
    let bytes = build_psd(&PsdOptions {
        width: 2,
        height: 2,
        channels: 3,
        depth: 8,
        compression: 1,
        planes: &planes,
        layer_section: &[],
    });

    let settings = TextureImportSettings::default();
    let imported = import_texture(&bytes, "Assets/Textures/hero.psd", &settings)?;
    assert_eq!(imported.source.format, TextureFormat::Rgb8);
    assert_eq!(imported.mips.len(), 2);
    assert_eq!((imported.mips[0].width, imported.mips[0].height), (2, 2));
    assert_eq!((imported.mips[1].width, imported.mips[1].height), (1, 1));
    assert!(imported.settings.srgb);
    assert!(!imported.settings.is_linear());
    assert!(!imported.settings.is_normal_map());
    assert!(imported.notes.is_empty());

    let normal_settings = TextureImportSettings {
        texture_type: TextureImporterType::NormalMap,
        ..TextureImportSettings::default()
    };
    let normal_import = import_texture(&bytes, "Assets/Textures/hero.psd", &normal_settings)?;
    assert!(normal_import.settings.is_normal_map());
    assert!(normal_import.settings.is_linear());
    Ok(())
}

#[test]
fn u4_hdr_rle_scanlines_decode_to_the_known_float_pixels() -> Result<(), Box<dyn std::error::Error>>
{
    let rows = vec![
        vec![[128u8, 128, 128, 128], [255, 255, 255, 129]],
        vec![[0u8, 0, 0, 0], [64, 32, 16, 128]],
    ];
    let bytes = build_hdr(&rows, "-Y 2 +X 2", false);

    let decoded = decode_hdr(&bytes)?;
    assert_eq!(decoded.format, TextureFormat::Rgba32F);
    assert_eq!((decoded.width, decoded.height), (2, 2));
    let first = decoded.pixel_f32(0, 0).unwrap_or([0.0; 4]);
    assert!(
        (first[0] - 0.501_953_1).abs() < 0.000_001
            && (first[1] - 0.501_953_1).abs() < 0.000_001
            && (first[2] - 0.501_953_1).abs() < 0.000_001
            && (first[3] - 1.0).abs() < 0.000_001,
        "expected the first pixel to decode to 0.501953125, got {first:?}"
    );
    let second = decoded.pixel_f32(1, 0).unwrap_or([0.0; 4]);
    assert!(
        (second[0] - 1.996_093_8).abs() < 0.000_001,
        "expected the second pixel to decode to 1.99609375, got {second:?}"
    );
    let third = decoded.pixel_f32(0, 1).unwrap_or([0.0; 4]);
    assert_eq!(third, [0.0, 0.0, 0.0, 1.0]);
    let fourth = decoded.pixel_f32(1, 1).unwrap_or([0.0; 4]);
    assert!(
        (fourth[0] - 0.251_953_13).abs() < 0.000_001
            && (fourth[1] - 0.126_953_13).abs() < 0.000_001
            && (fourth[2] - 0.064_453_125).abs() < 0.000_001,
        "expected the fourth pixel to decode to the scaled components, got {fourth:?}"
    );
    Ok(())
}

#[test]
fn u4_hdr_flat_scanlines_decode_to_the_known_float_pixels() -> Result<(), Box<dyn std::error::Error>>
{
    let rows = vec![vec![[128u8, 128, 128, 128], [255, 255, 255, 129]]];
    let bytes = build_hdr(&rows, "-Y 1 +X 2", true);

    let decoded = decode_hdr(&bytes)?;
    assert_eq!(decoded.format, TextureFormat::Rgba32F);
    let first = decoded.pixel_f32(0, 0).unwrap_or([0.0; 4]);
    assert!((first[0] - 0.501_953_1).abs() < 0.000_001);
    let second = decoded.pixel_f32(1, 0).unwrap_or([0.0; 4]);
    assert!((second[0] - 1.996_093_8).abs() < 0.000_001);
    Ok(())
}

#[test]
fn u4_hdr_import_builds_a_float_mip_chain() -> Result<(), Box<dyn std::error::Error>> {
    let rows = vec![
        vec![[128u8, 128, 128, 128], [255, 255, 255, 129]],
        vec![[0u8, 0, 0, 0], [64, 32, 16, 128]],
    ];
    let bytes = build_hdr(&rows, "-Y 2 +X 2", false);

    let imported = import_texture(
        &bytes,
        "Assets/Textures/sky.hdr",
        &TextureImportSettings::default(),
    )?;
    assert_eq!(imported.source.format, TextureFormat::Rgba32F);
    assert_eq!(imported.mips.len(), 2);
    assert_eq!((imported.mips[0].width, imported.mips[0].height), (2, 2));
    assert_eq!(imported.mips[1].format, TextureFormat::Rgba32F);
    assert!(imported.notes.is_empty());
    Ok(())
}

#[test]
fn u4_psd_and_hdr_dispatch_through_the_shared_texture_decoder(
) -> Result<(), Box<dyn std::error::Error>> {
    let planes = rgb_planes();
    let psd_bytes = build_psd(&PsdOptions {
        width: 2,
        height: 2,
        channels: 3,
        depth: 8,
        compression: 1,
        planes: &planes,
        layer_section: &[],
    });
    let hdr_bytes = build_hdr(
        &[vec![[128u8, 128, 128, 128], [255, 255, 255, 129]]],
        "-Y 1 +X 2",
        false,
    );

    let psd = decode_texture(&psd_bytes, "Assets/Textures/hero.PSD")?;
    assert_eq!(psd.format, TextureFormat::Rgb8);
    assert_eq!(pixel(&psd, 0, 0), vec![10, 50, 90]);

    let hdr = decode_texture(&hdr_bytes, "Assets/Textures/sky.HDR")?;
    assert_eq!(hdr.format, TextureFormat::Rgba32F);
    Ok(())
}

#[test]
fn u4_psd_and_hdr_are_recognized_as_texture_assets() -> Result<(), Box<dyn std::error::Error>> {
    let psd = identify_asset_family(Path::new("Assets/Textures/hero.psd"), &[], None)
        .ok_or("expected a family for hero.psd")?;
    assert_eq!(psd.family, RecognizedAssetFamily::Texture2D);
    assert_eq!(psd.outcome.state, AssetImportState::Ready);

    let hdr = identify_asset_family(Path::new("Assets/Textures/sky.hdr"), &[], None)
        .ok_or("expected a family for sky.hdr")?;
    assert_eq!(hdr.family, RecognizedAssetFamily::Texture2D);
    assert_eq!(hdr.outcome.state, AssetImportState::Ready);
    Ok(())
}

#[test]
fn u4_psd_and_hdr_reject_files_that_are_not_their_format() {
    let psd_error = decode_psd(b"not a psd file").err();
    assert!(psd_error.is_some());
    let hdr_error = decode_hdr(b"not an hdr file").err();
    assert!(hdr_error.is_some());
}

fn build_psd_with_encoded_rows(
    width: usize,
    height: usize,
    channels: usize,
    rows: &[Vec<u8>],
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"8BPS");
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&[0u8; 6]);
    out.extend_from_slice(&(channels as u16).to_be_bytes());
    out.extend_from_slice(&(height as u32).to_be_bytes());
    out.extend_from_slice(&(width as u32).to_be_bytes());
    out.extend_from_slice(&8u16.to_be_bytes());
    out.extend_from_slice(&3u16.to_be_bytes());
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&1u16.to_be_bytes());
    for row in rows {
        out.extend_from_slice(&(row.len() as u16).to_be_bytes());
    }
    for row in rows {
        out.extend_from_slice(row);
    }
    out
}

#[test]
fn u4_psd_skips_the_packbits_no_op_byte_and_keeps_the_row() -> Result<(), Box<dyn std::error::Error>>
{
    let bytes = build_psd_with_encoded_rows(
        2,
        1,
        3,
        &[
            vec![0x80u8, 0x01, 0x0A, 0x0B],
            vec![0x01u8, 0x32, 0x3C],
            vec![0x01u8, 0x5A, 0x64],
        ],
    );

    let decoded = decode_psd(&bytes)?;
    assert_eq!(pixel(&decoded, 0, 0), vec![0x0A, 0x32, 0x5A]);
    assert_eq!(pixel(&decoded, 1, 0), vec![0x0B, 0x3C, 0x64]);
    Ok(())
}

#[test]
fn u4_hdr_rejects_a_scanline_component_with_an_empty_literal_run() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"#?RADIANCE\n");
    bytes.extend_from_slice(b"FORMAT=32-bit_rle_rgbe\n");
    bytes.extend_from_slice(b"\n");
    bytes.extend_from_slice(b"-Y 1 +X 2\n");
    bytes.extend_from_slice(&[2u8, 2, 0, 2]);
    bytes.push(0u8);
    assert!(
        decode_hdr(&bytes).is_err(),
        "an empty literal run must be reported instead of read forever"
    );
}
