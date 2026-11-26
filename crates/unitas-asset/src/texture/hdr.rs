use crate::texture::import::TextureFormat;
pub use crate::texture::import::{DecodedTexture, TextureImportError};

const SIGNATURE: &[u8] = b"#?";

pub fn has_hdr_signature(bytes: &[u8]) -> bool {
    bytes.len() >= SIGNATURE.len() && bytes.starts_with(SIGNATURE)
}

pub fn decode_hdr(bytes: &[u8]) -> Result<DecodedTexture, TextureImportError> {
    if !has_hdr_signature(bytes) {
        return Err(TextureImportError::InvalidSignature);
    }
    let mut cursor = 0usize;
    let mut format_name = String::new();
    loop {
        let line = read_line(bytes, &mut cursor)?;
        if line.is_empty() {
            break;
        }
        if let Some(value) = line.strip_prefix("FORMAT=") {
            format_name = value.to_string();
        }
    }
    if !format_name.is_empty() && format_name != "32-bit_rle_rgbe" {
        return Err(TextureImportError::UnsupportedFormat(format!(
            "Radiance format {format_name}"
        )));
    }
    let resolution = read_line(bytes, &mut cursor)?;
    let (width, height, flip_rows, flip_columns) = parse_resolution(&resolution)?;
    let pixel_count = (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| TextureImportError::CorruptData("HDR size overflow".to_string()))?;
    let mut values: Vec<[f32; 4]> = Vec::with_capacity(pixel_count);
    let mut planes = [
        vec![0u8; width as usize],
        vec![0u8; width as usize],
        vec![0u8; width as usize],
        vec![0u8; width as usize],
    ];
    for _ in 0..height {
        read_scanline(bytes, &mut cursor, width as usize, &mut planes)?;
        for pixel in planes[0]
            .iter()
            .zip(&planes[1])
            .zip(&planes[2])
            .zip(&planes[3])
        {
            let (((red, green), blue), exponent) = pixel;
            values.push(rgbe_to_float([*red, *green, *blue, *exponent]));
        }
    }
    let mut pixels = Vec::with_capacity(pixel_count * 16);
    for y in 0..height as usize {
        let source_y = if flip_rows {
            height as usize - 1 - y
        } else {
            y
        };
        for x in 0..width as usize {
            let source_x = if flip_columns {
                width as usize - 1 - x
            } else {
                x
            };
            let value = values[source_y * width as usize + source_x];
            pixels.extend_from_slice(&value[0].to_le_bytes());
            pixels.extend_from_slice(&value[1].to_le_bytes());
            pixels.extend_from_slice(&value[2].to_le_bytes());
            pixels.extend_from_slice(&value[3].to_le_bytes());
        }
    }
    Ok(DecodedTexture {
        width,
        height,
        format: TextureFormat::Rgba32F,
        pixels,
    })
}

fn read_line(bytes: &[u8], cursor: &mut usize) -> Result<String, TextureImportError> {
    if *cursor >= bytes.len() {
        return Err(TextureImportError::Truncated);
    }
    let start = *cursor;
    let end = match bytes[start..].iter().position(|byte| *byte == b'\n') {
        Some(offset) => start + offset,
        None => bytes.len(),
    };
    *cursor = end.saturating_add(1).min(bytes.len());
    Ok(String::from_utf8_lossy(&bytes[start..end])
        .trim_end_matches('\r')
        .to_string())
}

fn parse_resolution(line: &str) -> Result<(u32, u32, bool, bool), TextureImportError> {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    if tokens.len() != 4 {
        return Err(TextureImportError::CorruptData(format!(
            "Radiance resolution line {line} is malformed"
        )));
    }
    let mut width = None;
    let mut height = None;
    let mut flip_rows = false;
    let mut flip_columns = false;
    for pair in tokens.chunks_exact(2) {
        let (axis, positive) = match parse_axis(pair[0]) {
            Some(axis) => axis,
            None => {
                return Err(TextureImportError::CorruptData(format!(
                    "Radiance resolution line {line} is malformed"
                )))
            }
        };
        let value: u32 = match pair[1].parse() {
            Ok(value) => value,
            Err(_) => {
                return Err(TextureImportError::CorruptData(format!(
                    "Radiance resolution line {line} is malformed"
                )))
            }
        };
        if value == 0 {
            return Err(TextureImportError::CorruptData(
                "Radiance image has a zero sized axis".to_string(),
            ));
        }
        match axis {
            'Y' => {
                height = Some(value);
                flip_rows = positive;
            }
            'X' => {
                width = Some(value);
                flip_columns = !positive;
            }
            _ => {
                return Err(TextureImportError::CorruptData(format!(
                    "Radiance resolution line {line} is malformed"
                )))
            }
        }
    }
    match (width, height) {
        (Some(width), Some(height)) => Ok((width, height, flip_rows, flip_columns)),
        _ => Err(TextureImportError::CorruptData(format!(
            "Radiance resolution line {line} is malformed"
        ))),
    }
}

fn parse_axis(spec: &str) -> Option<(char, bool)> {
    let mut chars = spec.chars();
    let sign = chars.next()?;
    let axis = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    let positive = match sign {
        '+' => true,
        '-' => false,
        _ => return None,
    };
    Some((axis, positive))
}

fn read_scanline(
    bytes: &[u8],
    cursor: &mut usize,
    width: usize,
    planes: &mut [Vec<u8>; 4],
) -> Result<(), TextureImportError> {
    let head = take(bytes, cursor, 4)?;
    if head[0] == 2 && head[1] == 2 && head[2] & 0x80 == 0 {
        let stored = (usize::from(head[2]) << 8) | usize::from(head[3]);
        if stored != width {
            return Err(TextureImportError::CorruptData(format!(
                "Radiance scanline holds {stored} pixels, expected {width}"
            )));
        }
        for plane in planes.iter_mut() {
            read_component(bytes, cursor, plane)?;
        }
        return Ok(());
    }
    for (channel, plane) in planes.iter_mut().enumerate() {
        plane[0] = head[channel];
    }
    for index in 1..width {
        let pixel = take(bytes, cursor, 4)?;
        for (channel, plane) in planes.iter_mut().enumerate() {
            plane[index] = pixel[channel];
        }
    }
    Ok(())
}

fn read_component(
    bytes: &[u8],
    cursor: &mut usize,
    plane: &mut [u8],
) -> Result<(), TextureImportError> {
    let mut filled = 0usize;
    while filled < plane.len() {
        let header = take(bytes, cursor, 1)?[0];
        if header > 128 {
            let count = usize::from(header) - 128;
            let value = take(bytes, cursor, 1)?[0];
            let take_count = count.min(plane.len() - filled);
            plane[filled..filled + take_count].fill(value);
            filled += take_count;
        } else {
            let count = usize::from(header);
            if count == 0 {
                return Err(TextureImportError::CorruptData(
                    "Radiance scanline component holds an empty literal run".to_string(),
                ));
            }
            let literal = take(bytes, cursor, count)?;
            let take_count = count.min(plane.len() - filled);
            plane[filled..filled + take_count].copy_from_slice(&literal[..take_count]);
            filled += take_count;
        }
    }
    Ok(())
}

fn take<'a>(
    bytes: &'a [u8],
    cursor: &mut usize,
    count: usize,
) -> Result<&'a [u8], TextureImportError> {
    let end = cursor
        .checked_add(count)
        .ok_or(TextureImportError::Truncated)?;
    if end > bytes.len() {
        return Err(TextureImportError::Truncated);
    }
    let slice = &bytes[*cursor..end];
    *cursor = end;
    Ok(slice)
}

fn rgbe_to_float(rgbe: [u8; 4]) -> [f32; 4] {
    let exponent = i32::from(rgbe[3]);
    if exponent == 0 {
        return [0.0, 0.0, 0.0, 1.0];
    }
    let scale = 2.0f32.powi(exponent - 136);
    [
        (f32::from(rgbe[0]) + 0.5) * scale,
        (f32::from(rgbe[1]) + 0.5) * scale,
        (f32::from(rgbe[2]) + 0.5) * scale,
        1.0,
    ]
}
