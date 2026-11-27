use unitas_asset::anim_import::{
    anim_import_outcome, import_animation_clip, AnimImportError, ImportedAnimationClip,
};
use unitas_asset::dependencies::{
    AssetImportState, ImportCandidate, ImportDiagnostic, ImportOutcome, ImportSession,
};
use unitas_asset::texture::artifact::texture_artifact_for_target;
use unitas_asset::texture::import::{
    apply_max_size, decode_texture, decode_texture_blocks, encode_texture_blocks,
    generate_mip_chain, import_texture, import_texture_for_target, srgb_to_linear,
    texture_import_outcome, DecodedTexture, FilterMode, SpriteMode, TextureCompressionQuality,
    TextureFormat, TextureImportError, TextureImportSettings, TextureImporterType,
    TexturePlatformFormat, TexturePlatformSettings, WrapMode,
};
use unitas_asset::texture::jpeg::decode_jpeg;
use unitas_asset::texture::png::decode_png;
use unitas_asset::texture::tga::decode_tga;

#[derive(Default)]
struct PngOptions {
    width: u32,
    height: u32,
    bit_depth: u8,
    color_type: u8,
    interlace: u8,
    palette: Option<Vec<u8>>,
    transparency: Option<Vec<u8>>,
    scanlines: Vec<Vec<u8>>,
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut checked = Vec::new();
    checked.extend_from_slice(kind);
    checked.extend_from_slice(data);
    let mut out = Vec::new();
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    out.extend_from_slice(&crc32(&checked).to_be_bytes());
    out
}

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

fn build_png(options: &PngOptions) -> Vec<u8> {
    let mut header = Vec::new();
    header.extend_from_slice(&options.width.to_be_bytes());
    header.extend_from_slice(&options.height.to_be_bytes());
    header.push(options.bit_depth);
    header.push(options.color_type);
    header.push(0);
    header.push(0);
    header.push(options.interlace);
    let mut png = vec![137u8, 80, 78, 71, 13, 10, 26, 10];
    png.extend_from_slice(&chunk(b"IHDR", &header));
    if let Some(palette) = &options.palette {
        png.extend_from_slice(&chunk(b"PLTE", palette));
    }
    if let Some(transparency) = &options.transparency {
        png.extend_from_slice(&chunk(b"tRNS", transparency));
    }
    let raw: Vec<u8> = options.scanlines.iter().flatten().copied().collect();
    png.extend_from_slice(&chunk(b"IDAT", &zlib_stored(&raw)));
    png.extend_from_slice(&chunk(b"IEND", &[]));
    png
}

fn paeth(left: u8, up: u8, upper_left: u8) -> u8 {
    let estimate = i16::from(left) + i16::from(up) - i16::from(upper_left);
    let left_distance = (estimate - i16::from(left)).abs();
    let up_distance = (estimate - i16::from(up)).abs();
    let upper_left_distance = (estimate - i16::from(upper_left)).abs();
    if left_distance <= up_distance && left_distance <= upper_left_distance {
        left
    } else if up_distance <= upper_left_distance {
        up
    } else {
        upper_left
    }
}

fn filter_row(filter: u8, raw: &[u8], previous: &[u8], bpp: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len() + 1);
    out.push(filter);
    for index in 0..raw.len() {
        let left = if index >= bpp { raw[index - bpp] } else { 0 };
        let up = previous[index];
        let upper_left = if index >= bpp {
            previous[index - bpp]
        } else {
            0
        };
        let value = match filter {
            0 => raw[index],
            1 => raw[index].wrapping_sub(left),
            2 => raw[index].wrapping_sub(up),
            3 => raw[index].wrapping_sub(((u16::from(left) + u16::from(up)) / 2) as u8),
            4 => raw[index].wrapping_sub(paeth(left, up, upper_left)),
            _ => raw[index],
        };
        out.push(value);
    }
    out
}

fn filtered_scanlines(rows: &[Vec<u8>], filters: &[u8], bpp: usize) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut previous = vec![0u8; rows.first().map_or(0, Vec::len)];
    for (index, row) in rows.iter().enumerate() {
        out.push(filter_row(filters[index], row, &previous, bpp));
        previous = row.clone();
    }
    out
}

fn tga_header(image_type: u8, width: u16, height: u16, pixel_depth: u8, descriptor: u8) -> Vec<u8> {
    let mut header = vec![0u8; 18];
    header[2] = image_type;
    header[12..14].copy_from_slice(&width.to_le_bytes());
    header[14..16].copy_from_slice(&height.to_le_bytes());
    header[16] = pixel_depth;
    header[17] = descriptor;
    header
}

fn decode_error(result: Result<DecodedTexture, TextureImportError>) -> TextureImportError {
    match result {
        Ok(texture) => {
            assert_eq!(texture.width, u32::MAX, "expected a decode failure");
            TextureImportError::InvalidSignature
        }
        Err(error) => error,
    }
}

#[test]
fn decodes_eight_bit_greyscale() -> Result<(), Box<dyn std::error::Error>> {
    let rows = vec![vec![0u8, 255], vec![64, 128]];
    let png = build_png(&PngOptions {
        width: 2,
        height: 2,
        bit_depth: 8,
        color_type: 0,
        scanlines: filtered_scanlines(&rows, &[0, 0], 1),
        ..PngOptions::default()
    });
    let texture = decode_png(&png)?;
    assert_eq!(texture.width, 2);
    assert_eq!(texture.height, 2);
    assert_eq!(texture.format, TextureFormat::R8);
    assert_eq!(texture.pixels, vec![0, 255, 64, 128]);
    assert_eq!(texture.pixel(1, 0), Some(&[255u8][..]));
    Ok(())
}

#[test]
fn decodes_eight_bit_true_colour() -> Result<(), Box<dyn std::error::Error>> {
    let rows = vec![vec![255u8, 0, 0, 0, 255, 0], vec![0, 0, 255, 255, 255, 255]];
    let png = build_png(&PngOptions {
        width: 2,
        height: 2,
        bit_depth: 8,
        color_type: 2,
        scanlines: filtered_scanlines(&rows, &[0, 0], 3),
        ..PngOptions::default()
    });
    let texture = decode_png(&png)?;
    assert_eq!(texture.format, TextureFormat::Rgb8);
    assert_eq!(
        texture.pixels,
        vec![255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255]
    );
    Ok(())
}

#[test]
fn decodes_eight_bit_rgba() -> Result<(), Box<dyn std::error::Error>> {
    let rows = vec![
        vec![1u8, 2, 3, 4, 5, 6, 7, 8],
        vec![9, 10, 11, 12, 13, 14, 15, 16],
    ];
    let png = build_png(&PngOptions {
        width: 2,
        height: 2,
        bit_depth: 8,
        color_type: 6,
        scanlines: filtered_scanlines(&rows, &[0, 0], 4),
        ..PngOptions::default()
    });
    let texture = decode_png(&png)?;
    assert_eq!(texture.format, TextureFormat::Rgba8);
    assert_eq!(texture.pixels, rows.concat());
    Ok(())
}

#[test]
fn decodes_indexed_png_with_palette_alpha() -> Result<(), Box<dyn std::error::Error>> {
    let rows = vec![vec![0u8, 1], vec![1, 0]];
    let png = build_png(&PngOptions {
        width: 2,
        height: 2,
        bit_depth: 8,
        color_type: 3,
        palette: Some(vec![255, 0, 0, 0, 255, 0]),
        transparency: Some(vec![0, 255]),
        scanlines: filtered_scanlines(&rows, &[0, 0], 1),
        ..PngOptions::default()
    });
    let texture = decode_png(&png)?;
    assert_eq!(texture.format, TextureFormat::Rgba8);
    assert_eq!(
        texture.pixels,
        vec![255, 0, 0, 0, 0, 255, 0, 255, 0, 255, 0, 255, 255, 0, 0, 0]
    );
    Ok(())
}

#[test]
fn decodes_greyscale_with_colour_key() -> Result<(), Box<dyn std::error::Error>> {
    let rows = vec![vec![10u8, 20]];
    let png = build_png(&PngOptions {
        width: 2,
        height: 1,
        bit_depth: 8,
        color_type: 0,
        transparency: Some(vec![10, 0]),
        scanlines: filtered_scanlines(&rows, &[0], 1),
        ..PngOptions::default()
    });
    let texture = decode_png(&png)?;
    assert_eq!(texture.format, TextureFormat::Rgba8);
    assert_eq!(texture.pixels, vec![10, 10, 10, 0, 20, 20, 20, 255]);
    Ok(())
}

#[test]
fn decodes_sixteen_bit_png_by_taking_the_high_byte() -> Result<(), Box<dyn std::error::Error>> {
    let rows = vec![vec![0x12u8, 0x34, 0xAB, 0xCD, 0xFF, 0x00]];
    let png = build_png(&PngOptions {
        width: 1,
        height: 1,
        bit_depth: 16,
        color_type: 2,
        scanlines: filtered_scanlines(&rows, &[0], 6),
        ..PngOptions::default()
    });
    let texture = decode_png(&png)?;
    assert_eq!(texture.format, TextureFormat::Rgb8);
    assert_eq!(texture.pixels, vec![0x12, 0xAB, 0xFF]);
    Ok(())
}

#[test]
fn decodes_every_scanline_filter() -> Result<(), Box<dyn std::error::Error>> {
    let rows = vec![
        vec![10u8, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120],
        vec![1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
    ];
    let expected = rows.concat();
    for filter in 0..=4u8 {
        let png = build_png(&PngOptions {
            width: 4,
            height: 2,
            bit_depth: 8,
            color_type: 2,
            scanlines: filtered_scanlines(&rows, &[filter, filter], 3),
            ..PngOptions::default()
        });
        let texture = decode_png(&png)?;
        assert_eq!(texture.pixels, expected, "scanline filter {filter}");
    }
    Ok(())
}

#[test]
fn reports_interlaced_png_as_unsupported() -> Result<(), Box<dyn std::error::Error>> {
    let rows = vec![vec![1u8, 2, 3, 4]];
    let png = build_png(&PngOptions {
        width: 2,
        height: 2,
        bit_depth: 8,
        color_type: 2,
        interlace: 1,
        scanlines: filtered_scanlines(&rows, &[0], 3),
        ..PngOptions::default()
    });
    let error = decode_error(decode_png(&png));
    assert!(matches!(error, TextureImportError::InterlacedNotSupported));
    Ok(())
}

#[test]
fn reports_unsupported_bit_depth() -> Result<(), Box<dyn std::error::Error>> {
    let rows = vec![vec![1u8, 2]];
    let png = build_png(&PngOptions {
        width: 2,
        height: 1,
        bit_depth: 4,
        color_type: 0,
        scanlines: filtered_scanlines(&rows, &[0], 1),
        ..PngOptions::default()
    });
    let error = decode_error(decode_png(&png));
    assert!(matches!(error, TextureImportError::UnsupportedBitDepth(4)));
    Ok(())
}

#[test]
fn reports_truncated_png() -> Result<(), Box<dyn std::error::Error>> {
    let rows = vec![vec![1u8, 2, 3]];
    let png = build_png(&PngOptions {
        width: 1,
        height: 1,
        bit_depth: 8,
        color_type: 2,
        scanlines: filtered_scanlines(&rows, &[0], 3),
        ..PngOptions::default()
    });
    let truncated = &png[..png.len() - 10];
    let error = decode_error(decode_png(truncated));
    assert!(matches!(error, TextureImportError::Truncated));
    Ok(())
}

#[test]
fn reports_a_non_png_signature() -> Result<(), Box<dyn std::error::Error>> {
    let error = decode_error(decode_png(&[1, 2, 3, 4, 5, 6, 7, 8, 9]));
    assert!(matches!(error, TextureImportError::InvalidSignature));
    Ok(())
}

#[test]
fn decodes_uncompressed_tga_true_colour() -> Result<(), Box<dyn std::error::Error>> {
    let mut tga = tga_header(2, 2, 2, 24, 0x20);
    tga.extend_from_slice(&[0, 0, 255, 0, 255, 0]);
    tga.extend_from_slice(&[255, 0, 0, 255, 255, 255]);
    let texture = decode_tga(&tga)?;
    assert_eq!(texture.width, 2);
    assert_eq!(texture.height, 2);
    assert_eq!(texture.format, TextureFormat::Rgb8);
    assert_eq!(
        texture.pixels,
        vec![255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255]
    );
    Ok(())
}

#[test]
fn decodes_tga_with_alpha() -> Result<(), Box<dyn std::error::Error>> {
    let mut tga = tga_header(2, 2, 1, 32, 0x28);
    tga.extend_from_slice(&[10, 20, 30, 40]);
    tga.extend_from_slice(&[50, 60, 70, 80]);
    let texture = decode_tga(&tga)?;
    assert_eq!(texture.format, TextureFormat::Rgba8);
    assert_eq!(texture.pixels, vec![30, 20, 10, 40, 70, 60, 50, 80]);
    Ok(())
}

#[test]
fn decodes_sixteen_bit_tga() -> Result<(), Box<dyn std::error::Error>> {
    let mut tga = tga_header(2, 2, 1, 16, 0x28);
    tga.extend_from_slice(&0xFC00u16.to_le_bytes());
    tga.extend_from_slice(&0x7C00u16.to_le_bytes());
    let texture = decode_tga(&tga)?;
    assert_eq!(texture.format, TextureFormat::Rgba8);
    assert_eq!(texture.pixels, vec![255, 0, 0, 255, 255, 0, 0, 0]);
    Ok(())
}

#[test]
fn decodes_run_length_encoded_tga() -> Result<(), Box<dyn std::error::Error>> {
    let mut tga = tga_header(10, 4, 1, 24, 0x20);
    tga.push(0x82);
    tga.extend_from_slice(&[0, 0, 255]);
    tga.push(0x00);
    tga.extend_from_slice(&[255, 255, 0]);
    let texture = decode_tga(&tga)?;
    assert_eq!(texture.width, 4);
    assert_eq!(
        texture.pixels,
        vec![255, 0, 0, 255, 0, 0, 255, 0, 0, 0, 255, 255]
    );
    Ok(())
}

#[test]
fn decodes_colour_mapped_tga() -> Result<(), Box<dyn std::error::Error>> {
    let mut tga = vec![0u8; 18];
    tga[1] = 1;
    tga[2] = 1;
    tga[5..7].copy_from_slice(&2u16.to_le_bytes());
    tga[7] = 24;
    tga[12..14].copy_from_slice(&2u16.to_le_bytes());
    tga[14..16].copy_from_slice(&1u16.to_le_bytes());
    tga[16] = 8;
    tga[17] = 0x20;
    tga.extend_from_slice(&[0, 0, 255, 0, 255, 0]);
    tga.extend_from_slice(&[0, 1]);
    let texture = decode_tga(&tga)?;
    assert_eq!(texture.format, TextureFormat::Rgb8);
    assert_eq!(texture.pixels, vec![255, 0, 0, 0, 255, 0]);
    Ok(())
}

#[test]
fn decodes_greyscale_tga() -> Result<(), Box<dyn std::error::Error>> {
    let mut tga = tga_header(3, 2, 2, 8, 0x20);
    tga.extend_from_slice(&[10, 20, 30, 40]);
    let texture = decode_tga(&tga)?;
    assert_eq!(texture.format, TextureFormat::R8);
    assert_eq!(texture.pixels, vec![10, 20, 30, 40]);
    Ok(())
}

#[test]
fn flips_bottom_up_tga_to_top_left_origin() -> Result<(), Box<dyn std::error::Error>> {
    let mut tga = tga_header(2, 2, 2, 24, 0x00);
    tga.extend_from_slice(&[0, 0, 255]);
    tga.extend_from_slice(&[0, 255, 0]);
    tga.extend_from_slice(&[255, 0, 0]);
    tga.extend_from_slice(&[255, 255, 255]);
    let texture = decode_tga(&tga)?;
    assert_eq!(
        texture.pixels,
        vec![0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255, 0]
    );
    Ok(())
}

#[test]
fn flips_right_to_left_tga_to_top_left_origin() -> Result<(), Box<dyn std::error::Error>> {
    let mut tga = tga_header(2, 2, 2, 24, 0x10);
    tga.extend_from_slice(&[0, 255, 0]);
    tga.extend_from_slice(&[0, 0, 255]);
    tga.extend_from_slice(&[255, 255, 255]);
    tga.extend_from_slice(&[255, 0, 0]);
    let texture = decode_tga(&tga)?;
    assert_eq!(
        texture.pixels,
        vec![0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255, 0]
    );
    Ok(())
}

#[test]
fn generates_mip_chain_with_box_filter() -> Result<(), Box<dyn std::error::Error>> {
    let rows = [
        vec![0u8, 10, 20, 30],
        vec![40, 50, 60, 70],
        vec![80, 90, 100, 110],
        vec![120, 130, 140, 150],
    ];
    let texture = DecodedTexture {
        width: 4,
        height: 4,
        format: TextureFormat::R8,
        pixels: rows.concat(),
    };
    let chain = generate_mip_chain(&texture);
    assert_eq!(chain.len(), 3);
    assert_eq!((chain[0].width, chain[0].height), (4, 4));
    assert_eq!((chain[1].width, chain[1].height), (2, 2));
    assert_eq!((chain[2].width, chain[2].height), (1, 1));
    assert_eq!(chain[1].pixels, vec![25, 45, 105, 125]);
    assert_eq!(chain[2].pixels, vec![75]);
    Ok(())
}

#[test]
fn limits_imported_size_to_the_configured_maximum() -> Result<(), Box<dyn std::error::Error>> {
    let rows = [
        vec![0u8, 10, 20, 30],
        vec![40, 50, 60, 70],
        vec![80, 90, 100, 110],
        vec![120, 130, 140, 150],
    ];
    let texture = DecodedTexture {
        width: 4,
        height: 4,
        format: TextureFormat::R8,
        pixels: rows.concat(),
    };
    let limited = apply_max_size(&texture, 2);
    assert_eq!((limited.width, limited.height), (2, 2));
    assert_eq!(limited.pixels, vec![25, 45, 105, 125]);
    Ok(())
}

#[test]
fn converts_srgb_values_to_linear() -> Result<(), Box<dyn std::error::Error>> {
    let settings = TextureImportSettings::default();
    assert!(!settings.is_linear());
    assert!((srgb_to_linear(255) - 1.0).abs() < 0.0001);
    assert!(srgb_to_linear(0).abs() < 0.0001);
    assert!((srgb_to_linear(128) - 0.2158).abs() < 0.001);
    assert!((settings.to_linear(128) - 0.2158).abs() < 0.001);
    let linear = TextureImportSettings {
        srgb: false,
        ..TextureImportSettings::default()
    };
    assert!(linear.is_linear());
    Ok(())
}

#[test]
fn import_settings_survive_a_meta_text_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let settings = TextureImportSettings {
        texture_type: TextureImporterType::NormalMap,
        srgb: false,
        max_size: 1024,
        mipmaps: false,
        sprite_mode: SpriteMode::Multiple,
        alpha_is_transparency: false,
        wrap_mode: WrapMode::Clamp,
        filter_mode: FilterMode::Trilinear,
        platform_settings: Vec::new(),
        sprite_slices: Vec::new(),
    };
    assert!(settings.is_normal_map());
    assert!(settings.is_linear());
    let text = settings.to_meta_text();
    let restored = TextureImportSettings::from_meta_text(&text)?;
    assert_eq!(restored, settings);
    assert_eq!(settings.round_trip()?, settings);
    Ok(())
}

#[test]
fn default_import_settings_survive_a_meta_text_round_trip() -> Result<(), Box<dyn std::error::Error>>
{
    let settings = TextureImportSettings::default();
    let restored = TextureImportSettings::from_meta_text(&settings.to_meta_text())?;
    assert_eq!(restored, settings);
    let polygon = TextureImportSettings {
        sprite_mode: SpriteMode::Polygon,
        ..TextureImportSettings::default()
    };
    assert_eq!(polygon.round_trip()?, polygon);
    Ok(())
}

#[test]
fn dispatches_decoding_by_file_name() -> Result<(), Box<dyn std::error::Error>> {
    let rows = vec![vec![1u8, 2, 3]];
    let png = build_png(&PngOptions {
        width: 1,
        height: 1,
        bit_depth: 8,
        color_type: 2,
        scanlines: filtered_scanlines(&rows, &[0], 3),
        ..PngOptions::default()
    });
    let texture = decode_texture(&png, "hero.PNG")?;
    assert_eq!(texture.pixels, vec![1, 2, 3]);
    let mut tga = tga_header(3, 1, 1, 8, 0x20);
    tga.extend_from_slice(&[77]);
    let texture = decode_texture(&tga, "hero.tga")?;
    assert_eq!(texture.pixels, vec![77]);
    let jpeg = baseline_jpeg_4x4();
    let texture = decode_texture(&jpeg, "hero.jpg")?;
    assert_eq!((texture.width, texture.height), (4, 4));
    assert_eq!(texture.format, TextureFormat::Rgba8);
    let texture_jpeg = decode_texture(&jpeg, "hero.JPEG")?;
    assert_eq!(texture_jpeg, texture);
    let texture_by_signature = decode_texture(&jpeg, "hero.bin")?;
    assert_eq!(texture_by_signature, texture);
    let error = decode_error(decode_texture(&png, "hero.bmp"));
    assert!(matches!(error, TextureImportError::UnsupportedFormat(_)));
    Ok(())
}

fn baseline_jpeg_4x4() -> Vec<u8> {
    vec![
        255, 216, 255, 224, 0, 16, 74, 70, 73, 70, 0, 1, 1, 0, 0, 1, 0, 1, 0, 0, 255, 219, 0, 67,
        0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 255, 219, 0, 67, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 255, 192, 0, 17, 8, 0, 4, 0, 4, 3, 1, 17, 0,
        2, 17, 1, 3, 17, 1, 255, 196, 0, 31, 0, 0, 1, 5, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0,
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 255, 196, 0, 181, 16, 0, 2, 1, 3, 3, 2, 4, 3, 5, 5, 4,
        4, 0, 0, 1, 125, 1, 2, 3, 0, 4, 17, 5, 18, 33, 49, 65, 6, 19, 81, 97, 7, 34, 113, 20, 50,
        129, 145, 161, 8, 35, 66, 177, 193, 21, 82, 209, 240, 36, 51, 98, 114, 130, 9, 10, 22, 23,
        24, 25, 26, 37, 38, 39, 40, 41, 42, 52, 53, 54, 55, 56, 57, 58, 67, 68, 69, 70, 71, 72, 73,
        74, 83, 84, 85, 86, 87, 88, 89, 90, 99, 100, 101, 102, 103, 104, 105, 106, 115, 116, 117,
        118, 119, 120, 121, 122, 131, 132, 133, 134, 135, 136, 137, 138, 146, 147, 148, 149, 150,
        151, 152, 153, 154, 162, 163, 164, 165, 166, 167, 168, 169, 170, 178, 179, 180, 181, 182,
        183, 184, 185, 186, 194, 195, 196, 197, 198, 199, 200, 201, 202, 210, 211, 212, 213, 214,
        215, 216, 217, 218, 225, 226, 227, 228, 229, 230, 231, 232, 233, 234, 241, 242, 243, 244,
        245, 246, 247, 248, 249, 250, 255, 196, 0, 31, 1, 0, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0,
        0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 255, 196, 0, 181, 17, 0, 2, 1, 2, 4, 4, 3, 4,
        7, 5, 4, 4, 0, 1, 2, 119, 0, 1, 2, 3, 17, 4, 5, 33, 49, 6, 18, 65, 81, 7, 97, 113, 19, 34,
        50, 129, 8, 20, 66, 145, 161, 177, 193, 9, 35, 51, 82, 240, 21, 98, 114, 209, 10, 22, 36,
        52, 225, 37, 241, 23, 24, 25, 26, 38, 39, 40, 41, 42, 53, 54, 55, 56, 57, 58, 67, 68, 69,
        70, 71, 72, 73, 74, 83, 84, 85, 86, 87, 88, 89, 90, 99, 100, 101, 102, 103, 104, 105, 106,
        115, 116, 117, 118, 119, 120, 121, 122, 130, 131, 132, 133, 134, 135, 136, 137, 138, 146,
        147, 148, 149, 150, 151, 152, 153, 154, 162, 163, 164, 165, 166, 167, 168, 169, 170, 178,
        179, 180, 181, 182, 183, 184, 185, 186, 194, 195, 196, 197, 198, 199, 200, 201, 202, 210,
        211, 212, 213, 214, 215, 216, 217, 218, 226, 227, 228, 229, 230, 231, 232, 233, 234, 242,
        243, 244, 245, 246, 247, 248, 249, 250, 255, 218, 0, 12, 3, 1, 0, 2, 17, 3, 17, 0, 63, 0,
        252, 51, 253, 164, 63, 224, 171, 255, 0, 183, 255, 0, 236, 55, 241, 103, 89, 248, 75, 240,
        11, 246, 136, 241, 39, 133, 124, 35, 226, 61, 31, 194, 31, 19, 245, 123, 93, 15, 71, 240,
        183, 195, 136, 111, 60, 85, 226, 191, 9, 104, 182, 87, 215, 19, 104, 159, 5, 52, 63, 133,
        190, 19, 150, 27, 13, 43, 66, 209, 124, 59, 160, 121, 222, 28, 150, 239, 64, 240, 134, 135,
        225, 207, 4, 105, 87, 150, 222, 13, 240, 167, 133, 116, 29, 16, 225, 159, 115, 192, 223,
        163, 94, 26, 143, 238, 50, 236, 187, 194, 44, 207, 40, 202, 50, 202, 95, 238, 121, 86, 93,
        147, 120, 207, 226, 230, 85, 78, 149, 9, 212, 231, 198, 227, 113, 217, 182, 35, 7, 137,
        226, 110, 42, 226, 12, 231, 23, 154, 113, 47, 25, 113, 190, 123, 196, 252, 107, 197, 121,
        206, 115, 196, 252, 71, 155, 230, 152, 179, 52, 255, 0, 105, 241, 19, 199, 28, 234, 191,
        189, 154, 103, 127, 72, 31, 27, 222, 119, 138, 143, 238, 163, 154, 230, 92, 47, 226, 87,
        17, 112, 7, 250, 195, 138, 193, 209, 246, 121, 125, 62, 35, 226, 140, 7, 8, 96, 120, 151,
        143, 179, 156, 30, 19, 11, 140, 227, 223, 17, 179, 110, 47, 241, 51, 139, 39, 154, 241,
        223, 26, 241, 94, 127, 155, 255, 0, 255, 217,
    ]
}

fn progressive_jpeg_4x4() -> Vec<u8> {
    vec![
        255, 216, 255, 224, 0, 16, 74, 70, 73, 70, 0, 1, 1, 0, 0, 1, 0, 1, 0, 0, 255, 219, 0, 67,
        0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 255, 219, 0, 67, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 255, 194, 0, 17, 8, 0, 4, 0, 4, 3, 1, 17, 0,
        2, 17, 1, 3, 17, 1, 255, 196, 0, 20, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8,
        255, 196, 0, 20, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 218, 0, 12, 3,
        1, 0, 2, 16, 3, 16, 0, 0, 1, 12, 159, 255, 196, 0, 22, 16, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 6, 5, 7, 255, 218, 0, 8, 1, 1, 0, 1, 5, 2, 73, 171, 191, 13, 91, 255, 196,
        0, 32, 17, 0, 2, 2, 2, 0, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 5, 3, 6, 1, 2, 7, 18, 20, 21,
        33, 34, 35, 255, 218, 0, 8, 1, 3, 1, 1, 63, 1, 43, 233, 98, 188, 27, 191, 146, 141, 226, 5,
        223, 173, 151, 30, 184, 40, 149, 118, 86, 40, 59, 132, 176, 233, 202, 62, 172, 90, 64, 162,
        6, 79, 204, 134, 40, 166, 125, 99, 45, 189, 153, 182, 74, 122, 233, 169, 229, 255, 0, 255,
        196, 0, 30, 17, 0, 2, 3, 0, 1, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 5, 2, 3, 6, 1, 7, 17,
        33, 34, 51, 255, 218, 0, 8, 1, 2, 1, 1, 63, 1, 89, 227, 13, 211, 90, 225, 232, 56, 249, 18,
        68, 16, 104, 252, 69, 28, 61, 158, 184, 88, 198, 28, 203, 189, 215, 94, 93, 148, 216, 205,
        171, 3, 45, 41, 147, 151, 103, 51, 116, 212, 195, 25, 177, 44, 171, 127, 255, 196, 0, 26,
        16, 0, 3, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 3, 4, 5, 1, 6, 17, 255, 218, 0, 8,
        1, 1, 0, 6, 63, 2, 118, 78, 7, 162, 166, 89, 40, 76, 154, 109, 20, 38, 92, 238, 29, 85, 72,
        144, 50, 234, 113, 81, 151, 39, 120, 10, 66, 103, 71, 217, 250, 72, 145, 19, 196, 163, 24,
        229, 149, 9, 255, 196, 0, 20, 16, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255,
        218, 0, 8, 1, 1, 0, 1, 63, 33, 49, 29, 227, 106, 233, 159, 118, 63, 255, 218, 0, 12, 3, 1,
        0, 2, 0, 3, 0, 0, 0, 16, 31, 255, 196, 0, 21, 17, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 65, 255, 218, 0, 8, 1, 3, 1, 1, 63, 16, 170, 157, 40, 96, 90, 164, 159, 255, 0,
        255, 196, 0, 20, 17, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 218, 0, 8, 1,
        2, 1, 1, 63, 16, 37, 255, 0, 122, 238, 17, 175, 253, 255, 196, 0, 20, 16, 1, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 218, 0, 8, 1, 1, 0, 1, 63, 16, 54, 133, 198, 52, 98,
        229, 247, 83, 255, 217,
    ]
}

fn grayscale_jpeg_2x2() -> Vec<u8> {
    vec![
        255, 216, 255, 224, 0, 16, 74, 70, 73, 70, 0, 1, 1, 0, 0, 1, 0, 1, 0, 0, 255, 219, 0, 67,
        0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 255, 192, 0, 11, 8, 0, 2, 0, 2, 1, 1, 17, 0, 255, 196, 0, 31, 0, 0, 1, 5, 1,
        1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 255, 196, 0, 181,
        16, 0, 2, 1, 3, 3, 2, 4, 3, 5, 5, 4, 4, 0, 0, 1, 125, 1, 2, 3, 0, 4, 17, 5, 18, 33, 49, 65,
        6, 19, 81, 97, 7, 34, 113, 20, 50, 129, 145, 161, 8, 35, 66, 177, 193, 21, 82, 209, 240,
        36, 51, 98, 114, 130, 9, 10, 22, 23, 24, 25, 26, 37, 38, 39, 40, 41, 42, 52, 53, 54, 55,
        56, 57, 58, 67, 68, 69, 70, 71, 72, 73, 74, 83, 84, 85, 86, 87, 88, 89, 90, 99, 100, 101,
        102, 103, 104, 105, 106, 115, 116, 117, 118, 119, 120, 121, 122, 131, 132, 133, 134, 135,
        136, 137, 138, 146, 147, 148, 149, 150, 151, 152, 153, 154, 162, 163, 164, 165, 166, 167,
        168, 169, 170, 178, 179, 180, 181, 182, 183, 184, 185, 186, 194, 195, 196, 197, 198, 199,
        200, 201, 202, 210, 211, 212, 213, 214, 215, 216, 217, 218, 225, 226, 227, 228, 229, 230,
        231, 232, 233, 234, 241, 242, 243, 244, 245, 246, 247, 248, 249, 250, 255, 218, 0, 8, 1, 1,
        0, 0, 63, 0, 254, 171, 190, 17, 254, 198, 159, 178, 4, 127, 10, 126, 24, 199, 31, 236, 167,
        251, 54, 162, 39, 195, 223, 5, 162, 34, 124, 12, 248, 96, 168, 136, 190, 27, 211, 85, 85,
        85, 124, 46, 2, 170, 128, 2, 168, 0, 0, 0, 3, 21, 255, 217,
    ]
}

fn assert_rgba_within_tolerance(actual: &[u8], expected: [u8; 4], tolerance: u8) {
    assert_eq!(actual.len(), 4);
    for channel in 0..4 {
        let diff = actual[channel].abs_diff(expected[channel]);
        assert!(
            diff <= tolerance,
            "channel {channel}: actual {} vs expected {} (diff {diff} > {tolerance})",
            actual[channel],
            expected[channel]
        );
    }
}

#[test]
fn decodes_baseline_jpeg_to_rgba8() -> Result<(), Box<dyn std::error::Error>> {
    let jpeg = baseline_jpeg_4x4();
    let texture = decode_jpeg(&jpeg)?;
    assert_eq!(texture.width, 4);
    assert_eq!(texture.height, 4);
    assert_eq!(texture.format, TextureFormat::Rgba8);
    assert_eq!(texture.pixels.len(), 4 * 4 * 4);
    let expected = [
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 255],
        [255, 255, 255, 255],
        [255, 255, 0, 255],
        [255, 0, 255, 255],
        [0, 255, 255, 255],
        [128, 128, 128, 255],
        [64, 0, 0, 255],
        [0, 64, 0, 255],
        [0, 0, 64, 255],
        [64, 64, 64, 255],
        [192, 64, 32, 255],
        [32, 192, 64, 255],
        [64, 32, 192, 255],
        [0, 0, 0, 255],
    ];
    for (index, target) in expected.iter().enumerate() {
        let x = (index % 4) as u32;
        let y = (index / 4) as u32;
        let pixel = texture.pixel(x, y).unwrap_or(&[]);
        assert_rgba_within_tolerance(pixel, *target, 4);
    }
    Ok(())
}

#[test]
fn decodes_progressive_jpeg_to_rgba8() -> Result<(), Box<dyn std::error::Error>> {
    let jpeg = progressive_jpeg_4x4();
    let texture = decode_jpeg(&jpeg)?;
    assert_eq!(texture.width, 4);
    assert_eq!(texture.height, 4);
    assert_eq!(texture.format, TextureFormat::Rgba8);
    let baseline = decode_jpeg(&baseline_jpeg_4x4())?;
    assert_eq!(texture.pixels, baseline.pixels);
    Ok(())
}

#[test]
fn decodes_grayscale_jpeg_to_rgba8() -> Result<(), Box<dyn std::error::Error>> {
    let jpeg = grayscale_jpeg_2x2();
    let texture = decode_jpeg(&jpeg)?;
    assert_eq!(texture.width, 2);
    assert_eq!(texture.height, 2);
    assert_eq!(texture.format, TextureFormat::Rgba8);
    let expected = [16u8, 96, 176, 240];
    for (index, luma) in expected.iter().enumerate() {
        let x = (index % 2) as u32;
        let y = (index / 2) as u32;
        let pixel = texture.pixel(x, y).unwrap_or(&[]);
        assert_rgba_within_tolerance(pixel, [*luma, *luma, *luma, 255], 3);
    }
    Ok(())
}

#[test]
fn applies_srgb_mipmaps_and_sprite_settings_to_decoded_jpeg(
) -> Result<(), Box<dyn std::error::Error>> {
    let texture = decode_texture(&baseline_jpeg_4x4(), "atlas.jpg")?;
    let settings = TextureImportSettings {
        texture_type: TextureImporterType::Sprite,
        srgb: true,
        max_size: 4,
        mipmaps: true,
        sprite_mode: SpriteMode::Multiple,
        alpha_is_transparency: true,
        wrap_mode: WrapMode::Clamp,
        filter_mode: FilterMode::Bilinear,
        platform_settings: Vec::new(),
        sprite_slices: Vec::new(),
    };
    assert!(!settings.is_linear());
    let red_pixel = texture.pixel(0, 0).unwrap_or(&[0, 0, 0, 0]);
    assert!((settings.to_linear(red_pixel[0]) - 1.0).abs() < 0.05);
    assert!(settings.to_linear(red_pixel[1]) < 0.01);
    let mips = settings.build_mip_chain(&texture);
    assert_eq!(mips.len(), 3);
    assert_eq!((mips[0].width, mips[0].height), (4, 4));
    assert_eq!((mips[1].width, mips[1].height), (2, 2));
    assert_eq!((mips[2].width, mips[2].height), (1, 1));
    assert_eq!(mips[2].format, TextureFormat::Rgba8);
    let restored = settings.round_trip()?;
    assert_eq!(restored.sprite_mode, SpriteMode::Multiple);
    Ok(())
}

#[test]
fn reports_malformed_and_truncated_jpeg_errors() -> Result<(), Box<dyn std::error::Error>> {
    let non_jpeg = decode_error(decode_jpeg(&[0x89, b'P', b'N', b'G']));
    assert!(matches!(non_jpeg, TextureImportError::InvalidSignature));

    let too_short = decode_error(decode_jpeg(&[0xFF, 0xD8]));
    assert!(matches!(too_short, TextureImportError::InvalidSignature));

    let signature_only = decode_error(decode_jpeg(&[0xFF, 0xD8, 0xFF]));
    assert!(matches!(
        signature_only,
        TextureImportError::Truncated | TextureImportError::CorruptData(_)
    ));

    let valid = baseline_jpeg_4x4();
    let truncated = decode_error(decode_jpeg(&valid[..40]));
    assert!(matches!(
        truncated,
        TextureImportError::Truncated | TextureImportError::CorruptData(_)
    ));

    let corrupt = decode_error(decode_texture(
        &[0xFF, 0xD8, 0xFF, 0x00, 0x12, 0x34],
        "bad.jpg",
    ));
    assert!(matches!(
        corrupt,
        TextureImportError::Truncated | TextureImportError::CorruptData(_)
    ));
    Ok(())
}

fn build_gif_frame(
    width: u16,
    height: u16,
    min_code_size: u8,
    lzw_codes: &[(u16, usize)],
) -> Vec<u8> {
    let mut frame = vec![0x2Cu8, 0, 0, 0, 0];
    frame.extend_from_slice(&width.to_le_bytes());
    frame.extend_from_slice(&height.to_le_bytes());
    frame.push(0x00);
    frame.push(min_code_size);
    let mut bit_buffer: u64 = 0;
    let mut bit_len: usize = 0;
    let mut sub_block = Vec::new();
    for (code, code_bits) in lzw_codes {
        bit_buffer |= u64::from(*code) << bit_len;
        bit_len += *code_bits;
        while bit_len >= 8 {
            sub_block.push((bit_buffer & 0xFF) as u8);
            bit_buffer >>= 8;
            bit_len -= 8;
        }
    }
    if bit_len > 0 {
        sub_block.push((bit_buffer & 0xFF) as u8);
    }
    frame.push(sub_block.len() as u8);
    frame.extend_from_slice(&sub_block);
    frame.push(0x00);
    frame
}

fn two_frame_animated_gif_2x2() -> Vec<u8> {
    let mut gif = Vec::new();
    gif.extend_from_slice(b"GIF89a");
    gif.extend_from_slice(&2u16.to_le_bytes());
    gif.extend_from_slice(&2u16.to_le_bytes());
    gif.push(0x81);
    gif.push(0x00);
    gif.push(0x00);
    gif.extend_from_slice(&[255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 0]);
    gif.extend_from_slice(&[0x21, 0xF9, 0x04, 0x01, 0x0A, 0x00, 0x03, 0x00]);
    gif.extend_from_slice(&build_gif_frame(
        2,
        2,
        2,
        &[(4, 3), (0, 3), (1, 3), (2, 3), (3, 4), (5, 4)],
    ));
    gif.extend_from_slice(&[0x21, 0xF9, 0x04, 0x00, 0x0A, 0x00, 0x00, 0x00]);
    gif.extend_from_slice(&build_gif_frame(
        2,
        2,
        2,
        &[(4, 3), (2, 3), (2, 3), (2, 3), (2, 4), (5, 4)],
    ));
    gif.push(0x3B);
    gif
}

#[test]
fn prj004_outcome14_pict_is_reported_unsupported() {
    let pict_bytes = vec![0u8; 528];
    let error = decode_error(decode_texture(&pict_bytes, "Assets/Textures/legacy.pict"));
    assert_eq!(
        error,
        TextureImportError::UnsupportedFormat("PICT (Assets/Textures/legacy.pict)".to_string())
    );
    let message = error.to_string();
    assert!(
        message.contains("PICT") && message.contains("Assets/Textures/legacy.pict"),
        "expected PICT and file name in unsupported error: {message}"
    );

    let pct_error = decode_error(decode_texture(&pict_bytes, "icon.pct"));
    assert_eq!(
        pct_error,
        TextureImportError::UnsupportedFormat("PICT (icon.pct)".to_string())
    );
    let import_error =
        match import_texture(&pict_bytes, "icon.pic", &TextureImportSettings::default()) {
            Ok(_) => TextureImportError::InvalidSignature,
            Err(error) => error,
        };
    assert_eq!(
        import_error,
        TextureImportError::UnsupportedFormat("PICT (icon.pic)".to_string())
    );
}

#[test]
fn prj004_outcome14_gif_imports_its_first_frame_and_reports_the_exclusion(
) -> Result<(), Box<dyn std::error::Error>> {
    let gif = two_frame_animated_gif_2x2();
    let settings = TextureImportSettings::default();
    let imported = import_texture(&gif, "Assets/Sprites/explosion.gif", &settings)?;
    assert_eq!(imported.texture.width, 2);
    assert_eq!(imported.texture.height, 2);
    assert_eq!(imported.texture.format, TextureFormat::Rgba8);
    assert_eq!(imported.frame_count, 2);
    assert_eq!(imported.excluded_frames, 1);
    assert_eq!(
        imported.texture.pixels,
        vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 0,]
    );
    assert!(
        imported.notes.iter().any(|note| {
            note.contains("GIF") && note.contains("first frame") && note.contains("excluded")
        }),
        "expected GIF first-frame exclusion notice in notes: {:?}",
        imported.notes
    );

    let decoded = decode_texture(&gif, "explosion.gif")?;
    assert_eq!(decoded, imported.texture);
    Ok(())
}

#[test]
fn prj004_outcome14_per_target_compression_overrides_survive_reimport(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut settings = TextureImportSettings {
        texture_type: TextureImporterType::Sprite,
        srgb: true,
        max_size: 8,
        mipmaps: true,
        sprite_mode: SpriteMode::Multiple,
        alpha_is_transparency: true,
        wrap_mode: WrapMode::Clamp,
        filter_mode: FilterMode::Bilinear,
        platform_settings: Vec::new(),
        sprite_slices: Vec::new(),
    };
    settings.set_platform_override(TexturePlatformSettings {
        build_target: "Standalone".to_string(),
        overridden: true,
        max_texture_size: 4,
        texture_compression: TextureCompressionQuality::HighQuality,
        format: TexturePlatformFormat::Bc7,
        compression_quality: 80,
        crunched_compression: false,
    });
    settings.set_platform_override(TexturePlatformSettings {
        build_target: "Android".to_string(),
        overridden: true,
        max_texture_size: 2,
        texture_compression: TextureCompressionQuality::Normal,
        format: TexturePlatformFormat::Astc6x6,
        compression_quality: 65,
        crunched_compression: true,
    });
    settings.set_platform_override(TexturePlatformSettings {
        build_target: "iPhone".to_string(),
        overridden: false,
        max_texture_size: 2,
        texture_compression: TextureCompressionQuality::LowQuality,
        format: TexturePlatformFormat::Astc4x4,
        compression_quality: 50,
        crunched_compression: false,
    });

    let meta_text = settings.to_meta_text();
    assert!(meta_text.contains("platformSettings:"));
    assert!(meta_text.contains("buildTarget: Standalone"));
    assert!(meta_text.contains("buildTarget: Android"));

    let restored = TextureImportSettings::from_meta_text(&meta_text)?;
    assert_eq!(restored, settings);
    assert_eq!(restored.effective_max_size(None), 8);
    assert_eq!(restored.effective_max_size(Some("Standalone")), 4);
    assert_eq!(restored.effective_max_size(Some("Android")), 2);
    assert_eq!(
        restored.effective_max_size(Some("iPhone")),
        8,
        "non-overridden target falls back to default max_size"
    );

    let source = baseline_jpeg_4x4();
    let first_import = import_texture_for_target(&source, "atlas.jpg", &restored, Some("Android"))?;
    assert_eq!(
        (first_import.texture.width, first_import.texture.height),
        (2, 2)
    );
    assert_eq!(first_import.applied_target.as_deref(), Some("Android"));
    assert_eq!(
        first_import.applied_format,
        Some(TexturePlatformFormat::Astc6x6)
    );

    let reimported_settings = restored.round_trip()?;
    let second_import =
        import_texture_for_target(&source, "atlas.jpg", &reimported_settings, Some("Android"))?;
    assert_eq!(first_import, second_import);
    Ok(())
}

fn half_bits(value: f32) -> [u8; 2] {
    let bits = value.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let exponent = i32::try_from((bits >> 23) & 0xff).unwrap_or(0) - 127 + 15;
    let mantissa = ((bits >> 13) & 0x03ff) as u16;
    if exponent <= 0 {
        return (sign | (mantissa >> 3)).to_le_bytes();
    }
    if exponent >= 31 {
        return (sign | 0x7c00).to_le_bytes();
    }
    (sign | ((exponent as u16) << 10) | mantissa).to_le_bytes()
}

fn exr_zip_compress(raw: &[u8]) -> Vec<u8> {
    let half = raw.len().div_ceil(2);
    let mut reordered = vec![0u8; raw.len()];
    let mut first = 0usize;
    let mut second = half;
    let mut index = 0usize;
    while index < raw.len() {
        reordered[first] = raw[index];
        first += 1;
        index += 1;
        if index >= raw.len() {
            break;
        }
        reordered[second] = raw[index];
        second += 1;
        index += 1;
    }
    let mut predicted = reordered.clone();
    let mut previous = i32::from(reordered[0]);
    for index in 1..predicted.len() {
        let current = i32::from(reordered[index]);
        predicted[index] = ((current - previous + 128 + 256) & 0xff) as u8;
        previous = current;
    }
    zlib_stored(&predicted)
}

fn exr_attribute(bytes: &mut Vec<u8>, name: &str, kind: &str, value: &[u8]) {
    bytes.extend_from_slice(name.as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(kind.as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(&(value.len() as u32).to_le_bytes());
    bytes.extend_from_slice(value);
}

fn build_exr_zip(width: u32, height: u32, pixels: &[(f32, f32, f32, f32)]) -> Vec<u8> {
    let mut raw = Vec::new();
    for component in [3usize, 2, 1, 0] {
        for y in 0..height as usize {
            for x in 0..width as usize {
                let pixel = pixels[y * width as usize + x];
                let value = match component {
                    0 => pixel.0,
                    1 => pixel.1,
                    2 => pixel.2,
                    _ => pixel.3,
                };
                raw.extend_from_slice(&half_bits(value));
            }
        }
    }
    let payload = exr_zip_compress(&raw);

    let mut bytes = Vec::new();
    bytes.extend_from_slice(&[0x76u8, 0x2f, 0x31, 0x01]);
    bytes.extend_from_slice(&2u32.to_le_bytes());

    let mut channels = Vec::new();
    for name in ["A", "B", "G", "R"] {
        channels.extend_from_slice(name.as_bytes());
        channels.push(0);
        channels.extend_from_slice(&1i32.to_le_bytes());
        channels.push(0);
        channels.extend_from_slice(&[0u8, 0, 0]);
        channels.extend_from_slice(&1i32.to_le_bytes());
        channels.extend_from_slice(&1i32.to_le_bytes());
    }
    channels.push(0);
    exr_attribute(&mut bytes, "channels", "chlist", &channels);
    exr_attribute(&mut bytes, "compression", "compression", &[3u8]);

    let mut window = Vec::new();
    window.extend_from_slice(&0i32.to_le_bytes());
    window.extend_from_slice(&0i32.to_le_bytes());
    window.extend_from_slice(&i32::try_from(width - 1).unwrap_or(0).to_le_bytes());
    window.extend_from_slice(&i32::try_from(height - 1).unwrap_or(0).to_le_bytes());
    exr_attribute(&mut bytes, "dataWindow", "box2i", &window);
    exr_attribute(&mut bytes, "displayWindow", "box2i", &window);
    exr_attribute(&mut bytes, "lineOrder", "lineOrder", &[0u8]);
    exr_attribute(
        &mut bytes,
        "pixelAspectRatio",
        "float",
        &1.0f32.to_le_bytes(),
    );
    let mut center = Vec::new();
    center.extend_from_slice(&0.0f32.to_le_bytes());
    center.extend_from_slice(&0.0f32.to_le_bytes());
    exr_attribute(&mut bytes, "screenWindowCenter", "v2f", &center);
    exr_attribute(
        &mut bytes,
        "screenWindowWidth",
        "float",
        &1.0f32.to_le_bytes(),
    );
    bytes.push(0);

    let offset_table = bytes.len();
    bytes.extend_from_slice(&0u64.to_le_bytes());
    let chunk = bytes.len();
    bytes.extend_from_slice(&0i32.to_le_bytes());
    bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&payload);
    let offset = u64::try_from(chunk - offset_table).unwrap_or(0);
    bytes[offset_table..offset_table + 8].copy_from_slice(&offset.to_le_bytes());
    bytes
}

#[test]
fn prj004_outcome14_exr_imports_with_its_float_channels() -> Result<(), Box<dyn std::error::Error>>
{
    let pixels = [
        (1.0f32, 0.0f32, 0.0f32, 1.0f32),
        (0.0, 1.0, 0.0, 1.0),
        (0.0, 0.0, 1.0, 1.0),
        (2.0, 2.0, 2.0, 0.5),
    ];
    let bytes = build_exr_zip(2, 2, &pixels);

    let decoded = decode_texture(&bytes, "Assets/Textures/sky.exr")?;
    assert_eq!((decoded.width, decoded.height), (2, 2));
    assert_eq!(decoded.format, TextureFormat::Rgba32F);
    let first = decoded.pixel_f32(0, 0).unwrap_or([0.0; 4]);
    assert_eq!(first, [1.0, 0.0, 0.0, 1.0]);
    let second = decoded.pixel_f32(1, 0).unwrap_or([0.0; 4]);
    assert_eq!(second, [0.0, 1.0, 0.0, 1.0]);
    let third = decoded.pixel_f32(0, 1).unwrap_or([0.0; 4]);
    assert_eq!(third, [0.0, 0.0, 1.0, 1.0]);
    let fourth = decoded.pixel_f32(1, 1).unwrap_or([0.0; 4]);
    assert_eq!(fourth, [2.0, 2.0, 2.0, 0.5]);

    let settings = TextureImportSettings {
        max_size: 1,
        mipmaps: false,
        ..TextureImportSettings::default()
    };
    let imported = import_texture(&bytes, "Assets/Textures/sky.exr", &settings)?;
    assert_eq!(imported.source.format, TextureFormat::Rgba32F);
    assert_eq!((imported.source.width, imported.source.height), (2, 2));
    assert_eq!((imported.texture.width, imported.texture.height), (1, 1));
    let averaged = imported.texture.pixel_f32(0, 0).unwrap_or([0.0; 4]);
    assert!(
        (averaged[0] - 0.75).abs() < 0.0001
            && (averaged[1] - 0.75).abs() < 0.0001
            && (averaged[2] - 0.75).abs() < 0.0001
            && (averaged[3] - 0.875).abs() < 0.0001,
        "expected the downscaled float pixel to average the four source pixels: {averaged:?}"
    );
    Ok(())
}

#[test]
fn prj004_outcome14_sprite_multiple_slices_keep_their_names_rects_pivots_and_identities(
) -> Result<(), Box<dyn std::error::Error>> {
    let meta_text = "TextureImporter:\n  sRGBTexture: 1\n  maxTextureSize: 2048\n  mipmapEnabled: 1\n  spriteMode: 2\n  alphaIsTransparency: 1\n  wrapMode: 0\n  filterMode: 1\n  spriteSheet:\n    serializedVersion: 2\n    sprites:\n    - serializedVersion: 2\n      name: Hero_Slice_A\n      rect:\n        serializedVersion: 2\n        x: 0\n        y: 0\n        width: 8\n        height: 8\n      pivot: {x: 0.25, y: 0.75}\n      border: {x: 1, y: 2, z: 3, w: 4}\n    - serializedVersion: 2\n      name: Tile_Slice_B\n      rect:\n        serializedVersion: 2\n        x: 8\n        y: 0\n        width: 8\n        height: 8\n      pivot: {x: 0.5, y: 0.5}\n      border: {x: 0, y: 0, z: 0, w: 0}\n";
    let settings = TextureImportSettings::from_meta_text(meta_text)?;
    assert_eq!(settings.sprite_mode, SpriteMode::Multiple);
    assert_eq!(settings.sprite_slices.len(), 2);
    let hero = &settings.sprite_slices[0];
    assert_eq!(hero.name, "Hero_Slice_A");
    assert_eq!(hero.sub_asset_id(), "Hero_Slice_A");
    assert!(hero.unity_local_file_id() >= 213_000_000);
    assert_eq!((hero.x, hero.y, hero.width, hero.height), (0, 0, 8, 8));
    assert!((hero.pivot_x - 0.25).abs() < 0.0001);
    assert!((hero.pivot_y - 0.75).abs() < 0.0001);
    assert!((hero.border_left - 1.0).abs() < 0.0001);
    assert!((hero.border_top - 4.0).abs() < 0.0001);
    let tile = &settings.sprite_slices[1];
    assert_eq!(tile.name, "Tile_Slice_B");
    assert_ne!(hero.unity_local_file_id(), tile.unity_local_file_id());
    assert_eq!((tile.x, tile.y, tile.width, tile.height), (8, 0, 8, 8));

    let restored = settings.round_trip()?;
    assert_eq!(restored, settings);
    assert_eq!(restored.sprite_slices[0].name, "Hero_Slice_A");

    let rows = vec![
        vec![10u8, 20, 30, 40, 50, 60],
        vec![70, 80, 90, 100, 110, 120],
    ];
    let png = build_png(&PngOptions {
        width: 2,
        height: 2,
        bit_depth: 8,
        color_type: 2,
        scanlines: filtered_scanlines(&rows, &[0, 0], 3),
        ..PngOptions::default()
    });
    let imported = import_texture(&png, "Assets/Sprites/Hero.png", &settings)?;
    assert_eq!(imported.sprites.len(), 2);
    assert_eq!(imported.sprites[0].name, "Hero_Slice_A");
    assert_eq!(imported.sprites[0].sub_asset_id(), "Hero_Slice_A");
    assert_eq!(imported.sprites[1].name, "Tile_Slice_B");

    let reimported = import_texture(&png, "Assets/Sprites/Hero.png", &restored)?;
    assert_eq!(reimported.sprites, imported.sprites);
    Ok(())
}

#[test]
fn prj004_outcome6_source_preview_and_runtime_texture_are_distinguishable(
) -> Result<(), Box<dyn std::error::Error>> {
    let rows = vec![
        vec![255u8, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 0],
        vec![0, 0, 255, 255, 255, 255, 0, 0, 255, 0, 255, 255],
        vec![255, 255, 0, 0, 255, 255, 255, 0, 255, 255, 0, 0],
        vec![0, 255, 255, 0, 0, 255, 0, 255, 255, 255, 255, 255],
    ];
    let png = build_png(&PngOptions {
        width: 4,
        height: 4,
        bit_depth: 8,
        color_type: 2,
        scanlines: filtered_scanlines(&rows, &[0, 0, 0, 0], 3),
        ..PngOptions::default()
    });
    let settings = TextureImportSettings {
        max_size: 2,
        mipmaps: false,
        ..TextureImportSettings::default()
    };
    let imported = import_texture(&png, "Assets/Textures/Source.png", &settings)?;
    assert_eq!((imported.source.width, imported.source.height), (4, 4));
    assert_eq!(
        imported.source.pixels,
        rows.concat(),
        "the source preview keeps every source pixel"
    );
    assert_eq!((imported.texture.width, imported.texture.height), (2, 2));
    assert_ne!(
        imported.texture.pixels, imported.source.pixels,
        "the runtime texture differs from the source preview"
    );
    Ok(())
}

fn import_candidate(path: &str, guid: &str, source: Vec<u8>) -> ImportCandidate {
    ImportCandidate {
        path: std::path::PathBuf::from(path),
        guid: guid.to_string(),
        source,
        metadata: String::new(),
        settings: "TextureImporter".to_string(),
        importer_version: "1".to_string(),
        dependencies: Vec::new(),
    }
}

fn texture_importer(candidate: &ImportCandidate) -> ImportOutcome {
    let path = candidate.path.display().to_string();
    match import_texture(&candidate.source, &path, &TextureImportSettings::default()) {
        Ok(imported) => texture_import_outcome(&path, Ok(&imported)),
        Err(error) => texture_import_outcome(&path, Err(&error)),
    }
}

#[test]
fn prj004_outcome19_corrupt_textures_end_as_a_partial_success_and_an_interrupted_import_reuses_unchanged_results(
) -> Result<(), Box<dyn std::error::Error>> {
    let rows = vec![vec![1u8, 2, 3, 4, 5, 6], vec![7, 8, 9, 10, 11, 12]];
    let good = build_png(&PngOptions {
        width: 2,
        height: 2,
        bit_depth: 8,
        color_type: 2,
        scanlines: filtered_scanlines(&rows, &[0, 0], 3),
        ..PngOptions::default()
    });
    let corrupt = vec![0x89u8, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x01];

    let candidates = vec![
        import_candidate(
            "Assets/Textures/Good_A.png",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            good.clone(),
        ),
        import_candidate(
            "Assets/Textures/Corrupt_1.png",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            corrupt.clone(),
        ),
        import_candidate(
            "Assets/Textures/Corrupt_2.png",
            "cccccccccccccccccccccccccccccccc",
            corrupt.clone(),
        ),
        import_candidate(
            "Assets/Textures/Corrupt_3.png",
            "dddddddddddddddddddddddddddddddd",
            corrupt.clone(),
        ),
        import_candidate(
            "Assets/Textures/Good_B.png",
            "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
            good.clone(),
        ),
    ];

    let mut session = ImportSession::new();
    let first = session.run(&candidates, &mut texture_importer);
    assert_eq!(first.imported, 5, "the import continues past every failure");
    assert_eq!(first.failed, 3);
    assert_eq!(first.succeeded, 2);
    assert!(
        first.partial_success,
        "expected the import to end as a partial success"
    );
    for path in [
        "Assets/Textures/Corrupt_1.png",
        "Assets/Textures/Corrupt_2.png",
        "Assets/Textures/Corrupt_3.png",
    ] {
        let record = first
            .record(std::path::Path::new(path))
            .ok_or("expected a record for the corrupt texture")?;
        assert_eq!(record.state, AssetImportState::Failed);
        assert_eq!(record.state.label(), "failed");
        assert!(
            record.consequence().is_some(),
            "expected a stated consequence for {path}"
        );
    }
    let good_record = first
        .record(std::path::Path::new("Assets/Textures/Good_A.png"))
        .ok_or("expected a record for the imported texture")?;
    assert_eq!(good_record.state, AssetImportState::Ready);
    assert!(good_record.state.is_fully_usable());

    let mut interrupted = ImportSession::new();
    let first_batch = &candidates[..2];
    let interrupted_report = interrupted.run(first_batch, &mut texture_importer);
    assert_eq!(interrupted_report.imported, 2);
    assert_eq!(interrupted_report.failed, 1);

    let resumed = interrupted.run(&candidates, &mut texture_importer);
    assert_eq!(resumed.reused, 2, "unchanged results are reused");
    assert_eq!(
        resumed.imported, 3,
        "the remaining assets are imported again"
    );
    assert!(resumed.partial_success);
    let reused = resumed
        .record(std::path::Path::new("Assets/Textures/Good_A.png"))
        .ok_or("expected a record for the reused texture")?;
    assert!(reused.reused);
    assert!(reused.reason.contains("unchanged"));
    let reused_failure = resumed
        .record(std::path::Path::new("Assets/Textures/Corrupt_1.png"))
        .ok_or("expected a record for the reused failure")?;
    assert!(reused_failure.reused);
    assert_eq!(reused_failure.state, AssetImportState::Failed);
    assert!(
        reused_failure.consequence().is_some(),
        "the retry keeps the prior diagnostic history"
    );
    let imported_again = resumed.imported_again();
    assert_eq!(imported_again.len(), 3);
    assert!(imported_again
        .iter()
        .all(|record| record.reason == "first import"));
    Ok(())
}

#[test]
fn prj004_outcome5_every_asset_that_is_not_fully_usable_has_a_state_and_a_consequence(
) -> Result<(), Box<dyn std::error::Error>> {
    let gif = two_frame_animated_gif_2x2();
    let corrupt = vec![0x89u8, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x01];
    let candidates = vec![
        import_candidate(
            "Assets/Textures/Explosion.gif",
            "ffffffffffffffffffffffffffffffff",
            gif,
        ),
        import_candidate(
            "Assets/Textures/Broken.png",
            "11111111111111111111111111111111",
            corrupt,
        ),
    ];

    let mut session = ImportSession::new();
    let report = session.run(&candidates, &mut texture_importer);
    assert_eq!(report.failed, 1);

    let limited = report
        .record(std::path::Path::new("Assets/Textures/Explosion.gif"))
        .ok_or("expected a record for the animated GIF")?;
    assert_eq!(limited.state, AssetImportState::ReadyWithLimitations);
    assert!(!limited.state.is_fully_usable());
    assert_eq!(limited.state.label(), "ready with limitations");
    assert!(
        limited.consequence().is_some(),
        "expected a stated consequence for the partially supported GIF"
    );
    assert!(limited
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.issue.contains("excluded")));

    let failed = report
        .record(std::path::Path::new("Assets/Textures/Broken.png"))
        .ok_or("expected a record for the corrupt texture")?;
    assert_eq!(failed.state, AssetImportState::Failed);
    assert!(failed.consequence().is_some());

    let missing = ImportOutcome::missing(ImportDiagnostic::new(
        "Assets/Textures/Gone.png",
        "the source file is missing",
        "the asset cannot be imported until the source file returns",
    ));
    assert_eq!(missing.state, AssetImportState::Missing);
    assert!(!missing.state.is_fully_usable());
    let blocked = ImportOutcome::blocked_by_dependency(ImportDiagnostic::new(
        "Assets/Textures/Dependent.png",
        "the dependency of the asset did not import",
        "the asset cannot be imported until its dependency imports",
    ));
    assert_eq!(blocked.state, AssetImportState::BlockedByDependency);
    let trust = ImportOutcome::blocked_by_trust(ImportDiagnostic::new(
        "Assets/Scripts/Tool.cs",
        "the project is not trusted",
        "the asset is not imported until the project is trusted",
    ));
    assert_eq!(trust.state, AssetImportState::BlockedByTrust);
    for state in [
        AssetImportState::NotYetEvaluated,
        AssetImportState::Processing,
        AssetImportState::Ready,
        AssetImportState::ReadyWithLimitations,
        AssetImportState::Failed,
        AssetImportState::Missing,
        AssetImportState::BlockedByDependency,
        AssetImportState::BlockedByTrust,
    ] {
        assert!(!state.label().is_empty());
    }
    Ok(())
}

#[test]
fn encodes_and_decodes_bcn_etc2_and_astc_compressed_texture_blocks_for_platform_targets(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut rows = Vec::with_capacity(8);
    for y in 0..8u8 {
        let mut row = Vec::with_capacity(8 * 4);
        for x in 0..8u8 {
            let step = x + y;
            let r = 30u8.saturating_add(step.saturating_mul(12));
            let g = 50u8.saturating_add(step.saturating_mul(12));
            let b = 70u8.saturating_add(step.saturating_mul(12));
            let a = 110u8.saturating_add(step.saturating_mul(8));
            row.extend_from_slice(&[r, g, b, a]);
        }
        rows.push(row);
    }
    let rgba_source = rows.concat();
    let png = build_png(&PngOptions {
        width: 8,
        height: 8,
        bit_depth: 8,
        color_type: 6,
        scanlines: filtered_scanlines(&rows, &[0; 8], 4),
        ..PngOptions::default()
    });

    let cases = [
        (TexturePlatformFormat::Dxt1, 32usize, false, 18u8),
        (TexturePlatformFormat::Dxt5, 64usize, true, 18u8),
        (TexturePlatformFormat::Bc7, 64usize, true, 8u8),
        (TexturePlatformFormat::Etc2Rgb4, 32usize, false, 18u8),
        (TexturePlatformFormat::Etc2Rgba8, 64usize, true, 18u8),
        (TexturePlatformFormat::Astc4x4, 64usize, true, 24u8),
        (TexturePlatformFormat::Astc6x6, 64usize, true, 32u8),
        (TexturePlatformFormat::Astc8x8, 16usize, true, 36u8),
    ];

    for (format, expected_mip0_bytes, checks_alpha, max_tol) in cases {
        let encoded = encode_texture_blocks(&rgba_source, 8, 8, format)
            .ok_or("expected compressed block payload")?;
        assert_eq!(encoded.len(), expected_mip0_bytes, "format {format:?}");
        let decoded = decode_texture_blocks(&encoded, 8, 8, format)
            .ok_or("expected block decode to succeed")?;
        assert_eq!(decoded.len(), rgba_source.len());
        for px in 0..(8 * 8) {
            let base = px * 4;
            for c in 0..3 {
                let diff = decoded[base + c].abs_diff(rgba_source[base + c]);
                assert!(
                    diff <= max_tol,
                    "format {format:?} pixel {px} channel {c}: {} vs {} (diff {diff} > {max_tol})",
                    decoded[base + c],
                    rgba_source[base + c]
                );
            }
            if checks_alpha {
                let diff_a = decoded[base + 3].abs_diff(rgba_source[base + 3]);
                assert!(
                    diff_a <= max_tol,
                    "format {format:?} pixel {px} alpha: {} vs {} (diff {diff_a} > {max_tol})",
                    decoded[base + 3],
                    rgba_source[base + 3]
                );
            } else {
                assert_eq!(decoded[base + 3], 255);
            }
        }

        let mut settings = TextureImportSettings {
            max_size: 8,
            mipmaps: true,
            ..TextureImportSettings::default()
        };
        settings.set_platform_override(TexturePlatformSettings {
            build_target: "TestTarget".to_string(),
            overridden: true,
            max_texture_size: 8,
            texture_compression: TextureCompressionQuality::HighQuality,
            format,
            compression_quality: 100,
            crunched_compression: false,
        });
        let imported = import_texture_for_target(&png, "hero.png", &settings, Some("TestTarget"))?;
        assert_eq!(imported.encoded_format, format);
        assert_eq!(
            imported.compressed_data.as_ref().map(Vec::len),
            Some(expected_mip0_bytes)
        );
        assert_eq!(imported.compressed_mip_data.len(), imported.mips.len());

        let artifact =
            texture_artifact_for_target(&png, "hero.png", &settings, Some("TestTarget"))?;
        assert_eq!(artifact.encoded_format, format);
        assert_eq!(artifact.compressed_data, imported.compressed_data);
        assert_eq!(artifact.compressed_mip_data, imported.compressed_mip_data);
    }

    let mut auto_settings = TextureImportSettings::default();
    auto_settings.set_platform_override(TexturePlatformSettings {
        build_target: "Standalone".to_string(),
        overridden: true,
        max_texture_size: 8,
        texture_compression: TextureCompressionQuality::HighQuality,
        format: TexturePlatformFormat::Automatic,
        compression_quality: 80,
        crunched_compression: false,
    });
    auto_settings.set_platform_override(TexturePlatformSettings {
        build_target: "Android".to_string(),
        overridden: true,
        max_texture_size: 8,
        texture_compression: TextureCompressionQuality::Normal,
        format: TexturePlatformFormat::Automatic,
        compression_quality: 50,
        crunched_compression: false,
    });
    auto_settings.set_platform_override(TexturePlatformSettings {
        build_target: "iPhone".to_string(),
        overridden: true,
        max_texture_size: 8,
        texture_compression: TextureCompressionQuality::LowQuality,
        format: TexturePlatformFormat::Automatic,
        compression_quality: 50,
        crunched_compression: false,
    });

    let standalone_import =
        import_texture_for_target(&png, "hero.png", &auto_settings, Some("Standalone"))?;
    assert_eq!(standalone_import.encoded_format, TexturePlatformFormat::Bc7);
    assert!(standalone_import.compressed_data.is_some());

    let android_import =
        import_texture_for_target(&png, "hero.png", &auto_settings, Some("Android"))?;
    assert_eq!(
        android_import.encoded_format,
        TexturePlatformFormat::Astc6x6
    );
    assert!(android_import.compressed_data.is_some());

    let ios_import = import_texture_for_target(&png, "hero.png", &auto_settings, Some("iPhone"))?;
    assert_eq!(ios_import.encoded_format, TexturePlatformFormat::Astc8x8);
    assert!(ios_import.compressed_data.is_some());

    let default_import = import_texture(&png, "hero.png", &auto_settings)?;
    assert_eq!(
        default_import.encoded_format,
        TexturePlatformFormat::Automatic
    );
    assert!(default_import.compressed_data.is_none());

    Ok(())
}

const HERO_RUN_ANIM_YAML: &str = r"%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!74 &7400000
AnimationClip:
  m_ObjectHideFlags: 0
  m_Name: HeroRun
  m_Legacy: 0
  m_Compressed: 0
  m_UseHighQualityCurve: 1
  m_RotationCurves:
  - curve:
      serializedVersion: 2
      m_Curve:
      - serializedVersion: 3
        time: 0
        value: {x: 0, y: 0, z: 0, w: 1}
        inSlope: {x: 0, y: 0, z: 0, w: 0}
        outSlope: {x: 0, y: 1, z: 0, w: 0}
        weightedMode: 0
        inWeight: {x: 0.33333334, y: 0.33333334, z: 0.33333334,
          w: 0.33333334}
        outWeight: {x: 0.33333334, y: 0.33333334, z: 0.33333334, w: 0.33333334}
      - serializedVersion: 3
        time: 1
        value: {x: 0, y: 0.70710677, z: 0, w: 0.70710677}
        inSlope: {x: 0, y: 0, z: 0, w: 0}
        outSlope: {x: 0, y: 0, z: 0, w: 0}
        weightedMode: 0
        inWeight: {x: 0.33333334, y: 0.33333334, z: 0.33333334, w: 0.33333334}
        outWeight: {x: 0.33333334, y: 0.33333334, z: 0.33333334, w: 0.33333334}
      m_PreInfinity: 2
      m_PostInfinity: 2
      m_RotationOrder: 4
    path: Rig/Spine
  m_CompressedRotationCurves: []
  m_EulerCurves:
  - curve:
      serializedVersion: 2
      m_Curve:
      - serializedVersion: 3
        time: 0
        value: {x: 0, y: 0, z: 0}
        inSlope: {x: 0, y: 90, z: 0}
        outSlope: {x: 0, y: 90, z: 0}
        weightedMode: 0
        inWeight: {x: 0.33333334, y: 0.33333334, z: 0.33333334}
        outWeight: {x: 0.33333334, y: 0.33333334, z: 0.33333334}
      - serializedVersion: 3
        time: 1
        value: {x: 0, y: 90, z: 0}
        inSlope: {x: 0, y: 90, z: 0}
        outSlope: {x: 0, y: 90, z: 0}
        weightedMode: 0
        inWeight: {x: 0.33333334, y: 0.33333334, z: 0.33333334}
        outWeight: {x: 0.33333334, y: 0.33333334, z: 0.33333334}
      m_PreInfinity: 2
      m_PostInfinity: 2
      m_RotationOrder: 4
    path: Rig/Head
  m_PositionCurves:
  - curve:
      serializedVersion: 2
      m_Curve:
      - serializedVersion: 3
        time: 0
        value: {x: 0, y: 1, z: 0}
        inSlope: {x: 2, y: 0, z: 0}
        outSlope: {x: 2, y: 0, z: 0}
        weightedMode: 0
        inWeight: {x: 0.33333334, y: 0.33333334, z: 0.33333334}
        outWeight: {x: 0.33333334, y: 0.33333334, z: 0.33333334}
      - serializedVersion: 3
        time: 1
        value: {x: 2, y: 1, z: 0}
        inSlope: {x: 2, y: 0, z: 0}
        outSlope: {x: 2, y: 0, z: 0}
        weightedMode: 0
        inWeight: {x: 0.33333334, y: 0.33333334, z: 0.33333334}
        outWeight: {x: 0.33333334, y: 0.33333334, z: 0.33333334}
      m_PreInfinity: 2
      m_PostInfinity: 2
      m_RotationOrder: 4
    path: Rig/Root
  m_ScaleCurves:
  - curve:
      serializedVersion: 2
      m_Curve:
      - serializedVersion: 3
        time: 0
        value: {x: 1, y: 1, z: 1}
        inSlope: {x: 0, y: 0, z: 0}
        outSlope: {x: 0, y: 0, z: 0}
        weightedMode: 0
        inWeight: {x: 0.33333334, y: 0.33333334, z: 0.33333334}
        outWeight: {x: 0.33333334, y: 0.33333334, z: 0.33333334}
      m_PreInfinity: 2
      m_PostInfinity: 2
      m_RotationOrder: 4
    path: Rig/Root
  m_FloatCurves:
  - curve:
      serializedVersion: 2
      m_Curve:
      - serializedVersion: 3
        time: 0
        value: 0
        inSlope: 10
        outSlope: 10
        weightedMode: 0
        inWeight: 0.33333334
        outWeight: 0.33333334
      - serializedVersion: 3
        time: 1
        value: 10
        inSlope: 10
        outSlope: 10
        weightedMode: 0
        inWeight: 0.33333334
        outWeight: 0.33333334
      m_PreInfinity: 2
      m_PostInfinity: 2
      m_RotationOrder: 4
    attribute: m_Intensity
    path: Rig/Torch
    classID: 108
    script: {fileID: 0}
  m_PPtrCurves:
  - curve:
      - time: 0
        value: {fileID: 21300000, guid: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, type: 3}
      - time: 0.5
        value: {fileID: 21300002, guid: bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, type: 3}
    attribute: m_Sprite
    path: Rig/Sprite
    classID: 212
    script: {fileID: 0}
  m_SampleRate: 60
  m_WrapMode: 2
  m_AnimationClipSettings:
    serializedVersion: 2
    m_StartTime: 0
    m_StopTime: 1
    m_OrientationOffsetY: 0
    m_Level: 0
    m_CycleOffset: 0
    m_HasAdditiveReferencePose: 0
    m_LoopTime: 1
    m_LoopBlend: 1
    m_LoopBlendOrientation: 0
    m_LoopBlendPositionY: 1
    m_LoopBlendPositionXZ: 0
    m_KeepOriginalOrientation: 1
    m_KeepOriginalPositionY: 1
    m_KeepOriginalPositionXZ: 0
    m_HeightFromFeet: 0
    m_Mirror: 0
  m_Events:
  - time: 0.5
    functionName: OnFootstep
    data: LeftFoot
    objectReferenceParameter: {fileID: 0}
    floatParameter: 0.75
    intParameter: 3
messageOptions: 0
";

fn hero_run_anim_yaml() -> &'static str {
    HERO_RUN_ANIM_YAML
}

#[test]
fn imports_unity_yaml_anim_clip_curves_settings_and_events(
) -> Result<(), Box<dyn std::error::Error>> {
    let anim_yaml = hero_run_anim_yaml();

    let clip = import_animation_clip(anim_yaml.as_bytes(), "Assets/Animations/HeroRun.anim")?;
    assert_anim_clip_header(&clip);
    assert_anim_clip_curves(&clip);
    assert_anim_clip_settings_and_events(&clip);

    let outcome = anim_import_outcome("Assets/Animations/HeroRun.anim", Ok(&clip));
    assert_eq!(outcome.state, AssetImportState::Ready);

    let non_anim = "%YAML 1.1\n--- !u!21 &2100000\nMaterial:\n  m_Name: TestMat\n";
    let err = match import_animation_clip(non_anim.as_bytes(), "TestMat.anim") {
        Ok(_) => AnimImportError::InvalidUtf8,
        Err(error) => error,
    };
    assert_eq!(err, AnimImportError::MissingAnimationClipDocument);
    let failed_outcome = anim_import_outcome("TestMat.anim", Err(&err));
    assert_eq!(failed_outcome.state, AssetImportState::Failed);

    let bad_rate =
        "%YAML 1.1\n--- !u!74 &7400000\nAnimationClip:\n  m_Name: Bad\n  m_SampleRate: 0\n";
    let rate_err = match import_animation_clip(bad_rate.as_bytes(), "Bad.anim") {
        Ok(_) => AnimImportError::InvalidUtf8,
        Err(error) => error,
    };
    assert!(matches!(rate_err, AnimImportError::InvalidSampleRate(_)));

    Ok(())
}

fn assert_anim_clip_header(clip: &ImportedAnimationClip) {
    assert_eq!(clip.name, "HeroRun");
    assert!(!clip.legacy);
    assert!(!clip.compressed);
    assert!(clip.use_high_quality_curve);
    assert!((clip.sample_rate - 60.0).abs() < 1e-9);
    assert_eq!(clip.wrap_mode, 2);
}

fn assert_anim_clip_curves(clip: &ImportedAnimationClip) {
    assert_eq!(clip.position_curves.len(), 1);
    assert_eq!(clip.position_curves[0].path, "Rig/Root");
    let mid_pos = clip.position_curves[0].sample(0.5);
    assert!((mid_pos[0] - 1.0).abs() < 1e-6);
    assert!((mid_pos[1] - 1.0).abs() < 1e-6);
    assert!(mid_pos[2].abs() < 1e-6);

    assert_eq!(clip.rotation_curves.len(), 1);
    assert_eq!(clip.rotation_curves[0].path, "Rig/Spine");
    let mid_rot = clip.rotation_curves[0].sample(0.5);
    let quat_norm = (mid_rot[0] * mid_rot[0]
        + mid_rot[1] * mid_rot[1]
        + mid_rot[2] * mid_rot[2]
        + mid_rot[3] * mid_rot[3])
        .sqrt();
    assert!((quat_norm - 1.0).abs() < 1e-6);

    assert_eq!(clip.euler_curves.len(), 1);
    assert_eq!(clip.euler_curves[0].path, "Rig/Head");
    let mid_euler = clip.euler_curves[0].sample(0.5);
    assert!((mid_euler[1] - 45.0).abs() < 1e-6);

    assert_eq!(clip.scale_curves.len(), 1);
    assert_eq!(clip.scale_curves[0].sample(0.5), [1.0, 1.0, 1.0]);

    assert_eq!(clip.float_curves.len(), 1);
    assert_eq!(clip.float_curves[0].path, "Rig/Torch");
    assert_eq!(clip.float_curves[0].attribute, "m_Intensity");
    assert_eq!(clip.float_curves[0].class_id, 108);
    assert!((clip.float_curves[0].sample(0.5) - 5.0).abs() < 1e-6);

    assert_eq!(clip.pptr_curves.len(), 1);
    assert_eq!(clip.pptr_curves[0].attribute, "m_Sprite");
    assert_eq!(clip.pptr_curves[0].class_id, 212);
    assert_eq!(clip.pptr_curves[0].keyframes.len(), 2);
    assert_eq!(
        clip.pptr_curves[0].keyframes[0].guid.as_deref(),
        Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
    );
    assert_eq!(clip.pptr_curves[0].keyframes[1].file_id, 21300002);
}

fn assert_anim_clip_settings_and_events(clip: &ImportedAnimationClip) {
    assert!(clip.settings.loop_time);
    assert!(clip.settings.loop_blend);
    assert!((clip.settings.stop_time - 1.0).abs() < 1e-9);

    assert_eq!(clip.events.len(), 1);
    assert_eq!(clip.events[0].function_name, "OnFootstep");
    assert_eq!(clip.events[0].data, "LeftFoot");
    assert!((clip.events[0].float_parameter - 0.75).abs() < 1e-9);
    assert_eq!(clip.events[0].int_parameter, 3);
}
