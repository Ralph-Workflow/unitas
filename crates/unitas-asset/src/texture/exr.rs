use crate::inflate::inflate_zlib;
use crate::texture::import::{DecodedTexture, TextureFormat, TextureImportError};

const EXR_MAGIC: [u8; 4] = [0x76, 0x2f, 0x31, 0x01];

const FLAG_TILED: u32 = 0x0200;
const FLAG_DEEP: u32 = 0x0800;
const FLAG_MULTI_PART: u32 = 0x1000;

const COMPRESSION_NONE: u8 = 0;
const COMPRESSION_ZIPS: u8 = 1;
const COMPRESSION_ZIP: u8 = 3;

const PIXEL_TYPE_UINT: i32 = 0;
const PIXEL_TYPE_HALF: i32 = 1;
const PIXEL_TYPE_FLOAT: i32 = 2;

struct ExrChannel {
    name: String,
    pixel_type: i32,
    x_sampling: i32,
    y_sampling: i32,
}

struct ExrFrame {
    y_min: i32,
    width: usize,
    height: usize,
    rows_per_block: usize,
}

struct ExrHeader {
    channels: Vec<ExrChannel>,
    compression: u8,
    window: (i32, i32, i32, i32),
    cursor: usize,
}

impl ExrChannel {
    fn bytes_per_sample(&self) -> usize {
        match self.pixel_type {
            PIXEL_TYPE_HALF => 2,
            _ => 4,
        }
    }
}

pub fn decode_exr(bytes: &[u8]) -> Result<DecodedTexture, TextureImportError> {
    let header = read_exr_header(bytes)?;
    let rows_per_block = match header.compression {
        COMPRESSION_NONE | COMPRESSION_ZIPS => 1,
        COMPRESSION_ZIP => 16,
        other => {
            return Err(TextureImportError::UnsupportedFormat(format!(
                "EXR compression {other} is not supported"
            )))
        }
    };

    let (x_min, y_min, x_max, y_max) = header.window;
    if x_max < x_min || y_max < y_min {
        return Err(TextureImportError::CorruptData(
            "EXR data window is empty".to_string(),
        ));
    }
    let width = (x_max - x_min + 1) as usize;
    let height = (y_max - y_min + 1) as usize;
    let block_count = height.div_ceil(rows_per_block);

    let offsets = read_exr_offset_table(bytes, header.cursor, block_count)?;

    let mut values = vec![0.0f32; width * height * 4];
    for pixel in values.chunks_exact_mut(4) {
        pixel[3] = 1.0;
    }
    decode_exr_blocks(
        bytes,
        &header,
        &offsets,
        &ExrFrame {
            y_min,
            width,
            height,
            rows_per_block,
        },
        &mut values,
    )?;

    let mut pixels = Vec::with_capacity(values.len() * 4);
    for value in values {
        pixels.extend_from_slice(&value.to_le_bytes());
    }
    Ok(DecodedTexture {
        width: width as u32,
        height: height as u32,
        format: TextureFormat::Rgba32F,
        pixels,
    })
}

fn read_exr_header(bytes: &[u8]) -> Result<ExrHeader, TextureImportError> {
    if bytes.len() < 8 {
        return Err(TextureImportError::Truncated);
    }
    if bytes[0..4] != EXR_MAGIC {
        return Err(TextureImportError::InvalidSignature);
    }
    let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    let flags = version & !0xff;
    if flags & FLAG_TILED != 0 {
        return Err(TextureImportError::UnsupportedFormat(
            "EXR tiled images are not supported".to_string(),
        ));
    }
    if flags & FLAG_MULTI_PART != 0 {
        return Err(TextureImportError::UnsupportedFormat(
            "EXR multi-part images are not supported".to_string(),
        ));
    }
    if flags & FLAG_DEEP != 0 {
        return Err(TextureImportError::UnsupportedFormat(
            "EXR deep images are not supported".to_string(),
        ));
    }
    if version & 0xff != 2 {
        return Err(TextureImportError::UnsupportedFormat(format!(
            "EXR version {}",
            version & 0xff
        )));
    }

    let mut cursor = 8usize;
    let mut channels: Vec<ExrChannel> = Vec::new();
    let mut compression = COMPRESSION_NONE;
    let mut window = (0i32, 0i32, -1i32, -1i32);
    let mut have_window = false;

    loop {
        let name = read_c_string(bytes, &mut cursor)?;
        if name.is_empty() {
            break;
        }
        let kind = read_c_string(bytes, &mut cursor)?;
        if cursor + 4 > bytes.len() {
            return Err(TextureImportError::Truncated);
        }
        let size = u32::from_le_bytes([
            bytes[cursor],
            bytes[cursor + 1],
            bytes[cursor + 2],
            bytes[cursor + 3],
        ]) as usize;
        cursor += 4;
        if cursor + size > bytes.len() {
            return Err(TextureImportError::Truncated);
        }
        let value = &bytes[cursor..cursor + size];
        cursor += size;
        match name.as_str() {
            "channels" => channels = parse_channels(value)?,
            "compression" => {
                if kind != "compression" || value.is_empty() {
                    return Err(TextureImportError::CorruptData(
                        "EXR compression attribute is malformed".to_string(),
                    ));
                }
                compression = value[0];
            }
            "dataWindow" => {
                window = parse_box2i(value)?;
                have_window = true;
            }
            _ => {}
        }
    }

    if channels.is_empty() {
        return Err(TextureImportError::CorruptData(
            "EXR image has no channels".to_string(),
        ));
    }
    if !have_window {
        return Err(TextureImportError::CorruptData(
            "EXR image has no data window".to_string(),
        ));
    }
    for channel in &channels {
        if channel.x_sampling != 1 || channel.y_sampling != 1 {
            return Err(TextureImportError::UnsupportedFormat(format!(
                "EXR channel {} is subsampled",
                channel.name
            )));
        }
        if !matches!(
            channel.pixel_type,
            PIXEL_TYPE_UINT | PIXEL_TYPE_HALF | PIXEL_TYPE_FLOAT
        ) {
            return Err(TextureImportError::UnsupportedFormat(format!(
                "EXR channel {} has an unsupported pixel type",
                channel.name
            )));
        }
    }

    Ok(ExrHeader {
        channels,
        compression,
        window,
        cursor,
    })
}

fn read_exr_offset_table(
    bytes: &[u8],
    cursor: usize,
    block_count: usize,
) -> Result<Vec<usize>, TextureImportError> {
    let offset_table_start = cursor;
    if cursor + block_count * 8 > bytes.len() {
        return Err(TextureImportError::Truncated);
    }
    let mut offsets = Vec::with_capacity(block_count);
    for index in 0..block_count {
        let start = cursor + index * 8;
        let offset = u64::from_le_bytes([
            bytes[start],
            bytes[start + 1],
            bytes[start + 2],
            bytes[start + 3],
            bytes[start + 4],
            bytes[start + 5],
            bytes[start + 6],
            bytes[start + 7],
        ]);
        offsets.push(offset_table_start + offset as usize);
    }
    Ok(offsets)
}

fn decode_exr_blocks(
    bytes: &[u8],
    header: &ExrHeader,
    offsets: &[usize],
    frame: &ExrFrame,
    values: &mut [f32],
) -> Result<(), TextureImportError> {
    let channels = &header.channels;
    let ExrFrame {
        y_min,
        width,
        height,
        rows_per_block,
    } = *frame;
    for (block, offset) in offsets.iter().enumerate() {
        let start = *offset;
        if start + 8 > bytes.len() {
            return Err(TextureImportError::Truncated);
        }
        let block_y = i32::from_le_bytes([
            bytes[start],
            bytes[start + 1],
            bytes[start + 2],
            bytes[start + 3],
        ]);
        let data_size = u32::from_le_bytes([
            bytes[start + 4],
            bytes[start + 5],
            bytes[start + 6],
            bytes[start + 7],
        ]) as usize;
        let data_start = start + 8;
        if data_start + data_size > bytes.len() {
            return Err(TextureImportError::Truncated);
        }
        let payload = &bytes[data_start..data_start + data_size];
        let first_row = block * rows_per_block;
        let rows = rows_per_block.min(height - first_row);
        let raw_size: usize = channels
            .iter()
            .map(|channel| channel.bytes_per_sample() * width * rows)
            .sum();
        let raw = match header.compression {
            COMPRESSION_NONE => payload.to_vec(),
            _ => unzip(payload, raw_size)?,
        };
        if raw.len() != raw_size {
            return Err(TextureImportError::CorruptData(format!(
                "EXR scan line block holds {} bytes, expected {raw_size}",
                raw.len()
            )));
        }
        let mut raw_cursor = 0usize;
        for channel in channels {
            let samples = width * rows;
            let sample_bytes = channel.bytes_per_sample();
            for row in 0..rows {
                let y = block_y - y_min + row as i32;
                if y < 0 || y as usize >= height {
                    raw_cursor += samples * sample_bytes;
                    continue;
                }
                for x in 0..width {
                    let value = read_sample(channel, &raw[raw_cursor..])?;
                    raw_cursor += sample_bytes;
                    let target = (y as usize * width + x) * 4;
                    apply_channel(&mut values[target..target + 4], &channel.name, value);
                }
            }
        }
    }
    Ok(())
}

fn apply_channel(target: &mut [f32], channel: &str, value: f32) {
    match channel {
        "R" => target[0] = value,
        "G" => target[1] = value,
        "B" => target[2] = value,
        "A" => target[3] = value,
        "Y" => {
            target[0] = value;
            target[1] = value;
            target[2] = value;
        }
        _ => {}
    }
}

fn read_sample(channel: &ExrChannel, bytes: &[u8]) -> Result<f32, TextureImportError> {
    match channel.pixel_type {
        PIXEL_TYPE_HALF => {
            if bytes.len() < 2 {
                return Err(TextureImportError::Truncated);
            }
            Ok(half_to_f32(u16::from_le_bytes([bytes[0], bytes[1]])))
        }
        PIXEL_TYPE_FLOAT => {
            if bytes.len() < 4 {
                return Err(TextureImportError::Truncated);
            }
            Ok(f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
        }
        _ => {
            if bytes.len() < 4 {
                return Err(TextureImportError::Truncated);
            }
            Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f32)
        }
    }
}

fn half_to_f32(bits: u16) -> f32 {
    let sign = if bits & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exponent = i32::from((bits >> 10) & 0x1f);
    let mantissa = f32::from(bits & 0x03ff);
    let magnitude = match exponent {
        0 => mantissa * 2f32.powi(-24),
        31 => {
            if mantissa == 0.0 {
                f32::INFINITY
            } else {
                f32::NAN
            }
        }
        _ => (mantissa + 1024.0) * 2f32.powi(exponent - 25),
    };
    sign * magnitude
}

fn unzip(payload: &[u8], expected: usize) -> Result<Vec<u8>, TextureImportError> {
    let mut buffer = inflate_zlib(payload)
        .map_err(|error| TextureImportError::CorruptData(format!("EXR zip block: {error}")))?;
    if buffer.len() != expected {
        return Err(TextureImportError::CorruptData(format!(
            "EXR zip block holds {} bytes, expected {expected}",
            buffer.len()
        )));
    }
    for index in 1..buffer.len() {
        let previous = i32::from(buffer[index - 1]);
        let current = i32::from(buffer[index]);
        buffer[index] = ((previous + current - 128) & 0xff) as u8;
    }
    let half = buffer.len().div_ceil(2);
    let mut out = Vec::with_capacity(buffer.len());
    let mut first = 0usize;
    let mut second = half;
    while out.len() < buffer.len() {
        out.push(buffer[first]);
        first += 1;
        if out.len() >= buffer.len() {
            break;
        }
        out.push(buffer[second]);
        second += 1;
    }
    Ok(out)
}

fn parse_channels(value: &[u8]) -> Result<Vec<ExrChannel>, TextureImportError> {
    let mut cursor = 0usize;
    let mut channels = Vec::new();
    loop {
        let name = read_c_string(value, &mut cursor)?;
        if name.is_empty() {
            break;
        }
        if cursor + 16 > value.len() {
            return Err(TextureImportError::Truncated);
        }
        let pixel_type = i32::from_le_bytes([
            value[cursor],
            value[cursor + 1],
            value[cursor + 2],
            value[cursor + 3],
        ]);
        let x_sampling = i32::from_le_bytes([
            value[cursor + 8],
            value[cursor + 9],
            value[cursor + 10],
            value[cursor + 11],
        ]);
        let y_sampling = i32::from_le_bytes([
            value[cursor + 12],
            value[cursor + 13],
            value[cursor + 14],
            value[cursor + 15],
        ]);
        cursor += 16;
        channels.push(ExrChannel {
            name,
            pixel_type,
            x_sampling,
            y_sampling,
        });
    }
    Ok(channels)
}

fn parse_box2i(value: &[u8]) -> Result<(i32, i32, i32, i32), TextureImportError> {
    if value.len() < 16 {
        return Err(TextureImportError::Truncated);
    }
    let read = |index: usize| {
        i32::from_le_bytes([
            value[index],
            value[index + 1],
            value[index + 2],
            value[index + 3],
        ])
    };
    Ok((read(0), read(4), read(8), read(12)))
}

fn read_c_string(bytes: &[u8], cursor: &mut usize) -> Result<String, TextureImportError> {
    let start = *cursor;
    let mut end = start;
    while end < bytes.len() && bytes[end] != 0 {
        end += 1;
    }
    if end >= bytes.len() {
        return Err(TextureImportError::Truncated);
    }
    *cursor = end + 1;
    Ok(String::from_utf8_lossy(&bytes[start..end]).into_owned())
}
