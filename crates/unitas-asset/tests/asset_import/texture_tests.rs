use unitas_asset::texture::import::{
    apply_max_size, decode_texture, generate_mip_chain, import_texture,
    import_texture_for_target, srgb_to_linear, DecodedTexture, FilterMode, SpriteMode,
    TextureCompressionQuality, TextureFormat, TextureImportError, TextureImportSettings,
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
        let upper_left = if index >= bpp { previous[index - bpp] } else { 0 };
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
    assert_eq!(texture.pixels, vec![255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255]);
    Ok(())
}

#[test]
fn decodes_eight_bit_rgba() -> Result<(), Box<dyn std::error::Error>> {
    let rows = vec![vec![1u8, 2, 3, 4, 5, 6, 7, 8], vec![9, 10, 11, 12, 13, 14, 15, 16]];
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
        srgb: false,
        max_size: 1024,
        mipmaps: false,
        sprite_mode: SpriteMode::Multiple,
        alpha_is_transparency: false,
        wrap_mode: WrapMode::Clamp,
        filter_mode: FilterMode::Trilinear,
        platform_settings: Vec::new(),
    };
    let text = settings.to_meta_text();
    let restored = TextureImportSettings::from_meta_text(&text)?;
    assert_eq!(restored, settings);
    assert_eq!(settings.round_trip()?, settings);
    Ok(())
}

#[test]
fn default_import_settings_survive_a_meta_text_round_trip() -> Result<(), Box<dyn std::error::Error>> {
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
        1, 1, 1, 1, 1, 255, 219, 0, 67, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 255, 192, 0, 17, 8, 0, 4, 0, 4, 3, 1, 17,
        0, 2, 17, 1, 3, 17, 1, 255, 196, 0, 31, 0, 0, 1, 5, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0,
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 255, 196, 0, 181, 16, 0, 2, 1, 3, 3, 2, 4, 3, 5, 5,
        4, 4, 0, 0, 1, 125, 1, 2, 3, 0, 4, 17, 5, 18, 33, 49, 65, 6, 19, 81, 97, 7, 34, 113, 20,
        50, 129, 145, 161, 8, 35, 66, 177, 193, 21, 82, 209, 240, 36, 51, 98, 114, 130, 9, 10, 22,
        23, 24, 25, 26, 37, 38, 39, 40, 41, 42, 52, 53, 54, 55, 56, 57, 58, 67, 68, 69, 70, 71, 72,
        73, 74, 83, 84, 85, 86, 87, 88, 89, 90, 99, 100, 101, 102, 103, 104, 105, 106, 115, 116,
        117, 118, 119, 120, 121, 122, 131, 132, 133, 134, 135, 136, 137, 138, 146, 147, 148, 149,
        150, 151, 152, 153, 154, 162, 163, 164, 165, 166, 167, 168, 169, 170, 178, 179, 180, 181,
        182, 183, 184, 185, 186, 194, 195, 196, 197, 198, 199, 200, 201, 202, 210, 211, 212, 213,
        214, 215, 216, 217, 218, 225, 226, 227, 228, 229, 230, 231, 232, 233, 234, 241, 242, 243,
        244, 245, 246, 247, 248, 249, 250, 255, 196, 0, 31, 1, 0, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0,
        0, 0, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 255, 196, 0, 181, 17, 0, 2, 1, 2, 4, 4,
        3, 4, 7, 5, 4, 4, 0, 1, 2, 119, 0, 1, 2, 3, 17, 4, 5, 33, 49, 6, 18, 65, 81, 7, 97, 113,
        19, 34, 50, 129, 8, 20, 66, 145, 161, 177, 193, 9, 35, 51, 82, 240, 21, 98, 114, 209, 10,
        22, 36, 52, 225, 37, 241, 23, 24, 25, 26, 38, 39, 40, 41, 42, 53, 54, 55, 56, 57, 58, 67,
        68, 69, 70, 71, 72, 73, 74, 83, 84, 85, 86, 87, 88, 89, 90, 99, 100, 101, 102, 103, 104,
        105, 106, 115, 116, 117, 118, 119, 120, 121, 122, 130, 131, 132, 133, 134, 135, 136, 137,
        138, 146, 147, 148, 149, 150, 151, 152, 153, 154, 162, 163, 164, 165, 166, 167, 168, 169,
        170, 178, 179, 180, 181, 182, 183, 184, 185, 186, 194, 195, 196, 197, 198, 199, 200, 201,
        202, 210, 211, 212, 213, 214, 215, 216, 217, 218, 226, 227, 228, 229, 230, 231, 232, 233,
        234, 242, 243, 244, 245, 246, 247, 248, 249, 250, 255, 218, 0, 12, 3, 1, 0, 2, 17, 3, 17,
        0, 63, 0, 252, 51, 253, 164, 63, 224, 171, 255, 0, 183, 255, 0, 236, 55, 241, 103, 89, 248,
        75, 240, 11, 246, 136, 241, 39, 133, 124, 35, 226, 61, 31, 194, 31, 19, 245, 123, 93, 15,
        71, 240, 183, 195, 136, 111, 60, 85, 226, 191, 9, 104, 182, 87, 215, 19, 104, 159, 5, 52,
        63, 133, 190, 19, 150, 27, 13, 43, 66, 209, 124, 59, 160, 121, 222, 28, 150, 239, 64, 240,
        134, 135, 225, 207, 4, 105, 87, 150, 222, 13, 240, 167, 133, 116, 29, 16, 225, 159, 115,
        192, 223, 163, 94, 26, 143, 238, 50, 236, 187, 194, 44, 207, 40, 202, 50, 202, 95, 238,
        121, 86, 93, 147, 120, 207, 226, 230, 85, 78, 149, 9, 212, 231, 198, 227, 113, 217, 182,
        35, 7, 137, 226, 110, 42, 226, 12, 231, 23, 154, 113, 47, 25, 113, 190, 123, 196, 252, 107,
        197, 121, 206, 115, 196, 252, 71, 155, 230, 152, 179, 52, 255, 0, 105, 241, 19, 199, 28,
        234, 191, 189, 154, 103, 127, 72, 31, 27, 222, 119, 138, 143, 238, 163, 154, 230, 92, 47,
        226, 87, 17, 112, 7, 250, 195, 138, 193, 209, 246, 121, 125, 62, 35, 226, 140, 7, 8, 96,
        120, 151, 143, 179, 156, 30, 19, 11, 140, 227, 223, 17, 179, 110, 47, 241, 51, 139, 39,
        154, 241, 223, 26, 241, 94, 127, 155, 255, 0, 255, 217,
    ]
}

fn progressive_jpeg_4x4() -> Vec<u8> {
    vec![
        255, 216, 255, 224, 0, 16, 74, 70, 73, 70, 0, 1, 1, 0, 0, 1, 0, 1, 0, 0, 255, 219, 0, 67,
        0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 255, 219, 0, 67, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 255, 194, 0, 17, 8, 0, 4, 0, 4, 3, 1, 17,
        0, 2, 17, 1, 3, 17, 1, 255, 196, 0, 20, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        8, 255, 196, 0, 20, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 218, 0, 12,
        3, 1, 0, 2, 16, 3, 16, 0, 0, 1, 12, 159, 255, 196, 0, 22, 16, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 6, 5, 7, 255, 218, 0, 8, 1, 1, 0, 1, 5, 2, 73, 171, 191, 13, 91, 255,
        196, 0, 32, 17, 0, 2, 2, 2, 0, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 5, 3, 6, 1, 2, 7, 18,
        20, 21, 33, 34, 35, 255, 218, 0, 8, 1, 3, 1, 1, 63, 1, 43, 233, 98, 188, 27, 191, 146, 141,
        226, 5, 223, 173, 151, 30, 184, 40, 149, 118, 86, 40, 59, 132, 176, 233, 202, 62, 172, 90,
        64, 162, 6, 79, 204, 134, 40, 166, 125, 99, 45, 189, 153, 182, 74, 122, 233, 169, 229, 255,
        0, 255, 196, 0, 30, 17, 0, 2, 3, 0, 1, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 5, 2, 3, 6, 1,
        7, 17, 33, 34, 51, 255, 218, 0, 8, 1, 2, 1, 1, 63, 1, 89, 227, 13, 211, 90, 225, 232, 56,
        249, 18, 68, 16, 104, 252, 69, 28, 61, 158, 184, 88, 198, 28, 203, 189, 215, 94, 93, 148,
        216, 205, 171, 3, 45, 41, 147, 151, 103, 51, 116, 212, 195, 25, 177, 44, 171, 127, 255,
        196, 0, 26, 16, 0, 3, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 3, 4, 5, 1, 6, 17, 255,
        218, 0, 8, 1, 1, 0, 6, 63, 2, 118, 78, 7, 162, 166, 89, 40, 76, 154, 109, 20, 38, 92, 238,
        29, 85, 72, 144, 50, 234, 113, 81, 151, 39, 120, 10, 66, 103, 71, 217, 250, 72, 145, 19,
        196, 163, 24, 229, 149, 9, 255, 196, 0, 20, 16, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 255, 218, 0, 8, 1, 1, 0, 1, 63, 33, 49, 29, 227, 106, 233, 159, 118, 63, 255, 218,
        0, 12, 3, 1, 0, 2, 0, 3, 0, 0, 0, 16, 31, 255, 196, 0, 21, 17, 1, 1, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 65, 255, 218, 0, 8, 1, 3, 1, 1, 63, 16, 170, 157, 40, 96, 90, 164,
        159, 255, 0, 255, 196, 0, 20, 17, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255,
        218, 0, 8, 1, 2, 1, 1, 63, 16, 37, 255, 0, 122, 238, 17, 175, 253, 255, 196, 0, 20, 16, 1,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 218, 0, 8, 1, 1, 0, 1, 63, 16, 54,
        133, 198, 52, 98, 229, 247, 83, 255, 217,
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
        0, 0, 63, 0, 254, 171, 190, 17, 254, 198, 159, 178, 4, 127, 10, 126, 24, 199, 31, 236,
        167, 251, 54, 162, 39, 195, 223, 5, 162, 34, 124, 12, 248, 96, 168, 136, 190, 27, 211, 85,
        85, 85, 124, 46, 2, 170, 128, 2, 168, 0, 0, 0, 3, 21, 255, 217,
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
        srgb: true,
        max_size: 4,
        mipmaps: true,
        sprite_mode: SpriteMode::Multiple,
        alpha_is_transparency: true,
        wrap_mode: WrapMode::Clamp,
        filter_mode: FilterMode::Bilinear,
        platform_settings: Vec::new(),
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

    let corrupt = decode_error(decode_texture(&[0xFF, 0xD8, 0xFF, 0x00, 0x12, 0x34], "bad.jpg"));
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
    gif.extend_from_slice(&[
        255, 0, 0,
        0, 255, 0,
        0, 0, 255,
        255, 255, 0,
    ]);
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
fn reports_pict_as_unsupported_by_name() {
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
    let import_error = match import_texture(&pict_bytes, "icon.pic", &TextureImportSettings::default()) {
        Ok(_) => TextureImportError::InvalidSignature,
        Err(error) => error,
    };
    assert_eq!(
        import_error,
        TextureImportError::UnsupportedFormat("PICT (icon.pic)".to_string())
    );
}

#[test]
fn imports_gif_first_frame_and_reports_excluded_later_frames(
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
        vec![
            255, 0, 0, 255,
            0, 255, 0, 255,
            0, 0, 255, 255,
            255, 255, 0, 0,
        ]
    );
    assert!(
        imported.notes.iter().any(|note| {
            note.contains("GIF")
                && note.contains("first frame")
                && note.contains("excluded")
        }),
        "expected GIF first-frame exclusion notice in notes: {:?}",
        imported.notes
    );

    let decoded = decode_texture(&gif, "explosion.gif")?;
    assert_eq!(decoded, imported.texture);
    Ok(())
}

#[test]
fn per_target_texture_compression_overrides_survive_meta_round_trip_and_reimport(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut settings = TextureImportSettings {
        srgb: true,
        max_size: 8,
        mipmaps: true,
        sprite_mode: SpriteMode::Multiple,
        alpha_is_transparency: true,
        wrap_mode: WrapMode::Clamp,
        filter_mode: FilterMode::Bilinear,
        platform_settings: Vec::new(),
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
    assert_eq!((first_import.texture.width, first_import.texture.height), (2, 2));
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
