use unitas_asset::texture::import::{
    apply_max_size, decode_texture, generate_mip_chain, srgb_to_linear, DecodedTexture,
    FilterMode, SpriteMode, TextureFormat, TextureImportError, TextureImportSettings, WrapMode,
};
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
    let rows = vec![
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
    let rows = vec![
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
    let error = decode_error(decode_texture(&png, "hero.jpg"));
    assert!(matches!(error, TextureImportError::UnsupportedFormat(_)));
    Ok(())
}
