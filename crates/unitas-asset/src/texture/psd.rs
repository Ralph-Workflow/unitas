use std::cmp::Ordering;

use crate::inflate::inflate_zlib;
use crate::texture::import::TextureFormat;
pub use crate::texture::import::{DecodedTexture, TextureImportError};

const SIGNATURE: [u8; 4] = [0x38, 0x42, 0x50, 0x53];

const COLOR_MODE_RGB: u16 = 3;

const COMPRESSION_RAW: u16 = 0;
const COMPRESSION_RLE: u16 = 1;
const COMPRESSION_ZIP: u16 = 2;
const COMPRESSION_ZIP_PREDICTION: u16 = 3;

pub fn has_psd_signature(bytes: &[u8]) -> bool {
    bytes.len() >= SIGNATURE.len() && bytes[..SIGNATURE.len()] == SIGNATURE
}

pub fn decode_psd(bytes: &[u8]) -> Result<DecodedTexture, TextureImportError> {
    if !has_psd_signature(bytes) {
        return Err(TextureImportError::InvalidSignature);
    }
    if bytes.len() < 26 {
        return Err(TextureImportError::Truncated);
    }
    let mut cursor = 4usize;
    let version = read_u16(bytes, &mut cursor)?;
    if version != 1 {
        return Err(TextureImportError::UnsupportedFormat(format!(
            "PSD version {version}"
        )));
    }
    cursor += 6;
    let channels = usize::from(read_u16(bytes, &mut cursor)?);
    let height = read_u32(bytes, &mut cursor)? as usize;
    let width = read_u32(bytes, &mut cursor)? as usize;
    let depth = read_u16(bytes, &mut cursor)?;
    let color_mode = read_u16(bytes, &mut cursor)?;
    if color_mode != COLOR_MODE_RGB {
        return Err(TextureImportError::UnsupportedFormat(format!(
            "PSD colour mode {color_mode}"
        )));
    }
    if depth != 8 && depth != 16 {
        return Err(TextureImportError::UnsupportedFormat(format!(
            "PSD bit depth {depth}"
        )));
    }
    if width == 0 || height == 0 {
        return Err(TextureImportError::CorruptData(
            "PSD has a zero image size".to_string(),
        ));
    }
    if channels < 3 {
        return Err(TextureImportError::UnsupportedFormat(format!(
            "PSD has {channels} channels"
        )));
    }
    let sample_bytes = usize::from(depth / 8);
    let row_size = width
        .checked_mul(sample_bytes)
        .ok_or_else(|| TextureImportError::CorruptData("PSD row size overflows".to_string()))?;
    let plane_size = row_size
        .checked_mul(height)
        .ok_or_else(|| TextureImportError::CorruptData("PSD plane size overflows".to_string()))?;
    let total = plane_size
        .checked_mul(channels)
        .ok_or_else(|| TextureImportError::CorruptData("PSD image size overflows".to_string()))?;
    for _ in 0..3 {
        skip_section(bytes, &mut cursor)?;
    }
    let compression = read_u16(bytes, &mut cursor)?;
    let planes = match compression {
        COMPRESSION_RAW => raw_planes(bytes, &mut cursor, total)?,
        COMPRESSION_RLE => rle_planes(bytes, &mut cursor, row_size, height, channels)?,
        COMPRESSION_ZIP => zip_planes(bytes, &mut cursor, total)?,
        COMPRESSION_ZIP_PREDICTION => {
            let mut planes = zip_planes(bytes, &mut cursor, total)?;
            undo_prediction(&mut planes, width, height * channels, sample_bytes);
            planes
        }
        other => {
            return Err(TextureImportError::UnsupportedCompression(
                u8::try_from(other).unwrap_or(u8::MAX),
            ))
        }
    };
    let pixel_count = width * height;
    let out_channels = if channels >= 4 { 4 } else { 3 };
    let mut pixels = Vec::with_capacity(pixel_count * out_channels);
    for index in 0..pixel_count {
        for channel in 0..out_channels {
            let offset = (channel * pixel_count + index) * sample_bytes;
            pixels.push(planes[offset]);
        }
    }
    let format = if out_channels == 4 {
        TextureFormat::Rgba8
    } else {
        TextureFormat::Rgb8
    };
    Ok(DecodedTexture {
        width: width as u32,
        height: height as u32,
        format,
        pixels,
    })
}

fn raw_planes(
    bytes: &[u8],
    cursor: &mut usize,
    total: usize,
) -> Result<Vec<u8>, TextureImportError> {
    let end = cursor
        .checked_add(total)
        .ok_or(TextureImportError::Truncated)?;
    if end > bytes.len() {
        return Err(TextureImportError::Truncated);
    }
    let planes = bytes[*cursor..end].to_vec();
    *cursor = end;
    Ok(planes)
}

fn rle_planes(
    bytes: &[u8],
    cursor: &mut usize,
    row_size: usize,
    height: usize,
    channels: usize,
) -> Result<Vec<u8>, TextureImportError> {
    let rows = height * channels;
    let mut counts = Vec::with_capacity(rows);
    for _ in 0..rows {
        counts.push(usize::from(read_u16(bytes, cursor)?));
    }
    let mut planes = Vec::with_capacity(rows * row_size);
    for count in counts {
        let end = cursor
            .checked_add(count)
            .ok_or(TextureImportError::Truncated)?;
        if end > bytes.len() {
            return Err(TextureImportError::Truncated);
        }
        let row = &bytes[*cursor..end];
        *cursor = end;
        planes.extend_from_slice(&decode_packbits(row, row_size)?);
    }
    Ok(planes)
}

fn zip_planes(
    bytes: &[u8],
    cursor: &mut usize,
    total: usize,
) -> Result<Vec<u8>, TextureImportError> {
    let inflated = inflate_zlib(&bytes[*cursor..])
        .map_err(|error| TextureImportError::InflateFailed(format!("{error}")))?;
    if inflated.len() != total {
        return Err(TextureImportError::CorruptData(format!(
            "PSD image data holds {} bytes, expected {total}",
            inflated.len()
        )));
    }
    *cursor = bytes.len();
    Ok(inflated)
}

fn decode_packbits(data: &[u8], size: usize) -> Result<Vec<u8>, TextureImportError> {
    let mut out = Vec::with_capacity(size);
    let mut cursor = 0usize;
    while out.len() < size {
        let header = *data.get(cursor).ok_or(TextureImportError::Truncated)?;
        cursor += 1;
        match header.cmp(&128) {
            Ordering::Greater => {
                let count = 257 - usize::from(header);
                let value = *data.get(cursor).ok_or(TextureImportError::Truncated)?;
                cursor += 1;
                let take_count = count.min(size - out.len());
                out.extend(std::iter::repeat_n(value, take_count));
            }
            Ordering::Equal => {}
            Ordering::Less => {
                let count = usize::from(header) + 1;
                let end = cursor
                    .checked_add(count)
                    .ok_or(TextureImportError::Truncated)?;
                if end > data.len() {
                    return Err(TextureImportError::Truncated);
                }
                let take_count = count.min(size - out.len());
                out.extend_from_slice(&data[cursor..cursor + take_count]);
                cursor = end;
            }
        }
    }
    Ok(out)
}

fn undo_prediction(data: &mut [u8], width: usize, rows: usize, sample_bytes: usize) {
    if sample_bytes == 1 {
        for row in data.chunks_exact_mut(width).take(rows) {
            for index in 1..row.len() {
                row[index] = row[index].wrapping_add(row[index - 1]);
            }
        }
        return;
    }
    for row in data.chunks_exact_mut(width * 2).take(rows) {
        for index in 1..width {
            let previous = u16::from_be_bytes([row[(index - 1) * 2], row[(index - 1) * 2 + 1]]);
            let current = u16::from_be_bytes([row[index * 2], row[index * 2 + 1]]);
            let sum = current.wrapping_add(previous);
            row[index * 2..index * 2 + 2].copy_from_slice(&sum.to_be_bytes());
        }
    }
}

fn skip_section(bytes: &[u8], cursor: &mut usize) -> Result<(), TextureImportError> {
    let length = read_u32(bytes, cursor)? as usize;
    let end = cursor
        .checked_add(length)
        .ok_or(TextureImportError::Truncated)?;
    if end > bytes.len() {
        return Err(TextureImportError::Truncated);
    }
    *cursor = end;
    Ok(())
}

fn read_u16(bytes: &[u8], cursor: &mut usize) -> Result<u16, TextureImportError> {
    let end = cursor.checked_add(2).ok_or(TextureImportError::Truncated)?;
    if end > bytes.len() {
        return Err(TextureImportError::Truncated);
    }
    let value = u16::from_be_bytes([bytes[*cursor], bytes[*cursor + 1]]);
    *cursor = end;
    Ok(value)
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> Result<u32, TextureImportError> {
    let end = cursor.checked_add(4).ok_or(TextureImportError::Truncated)?;
    if end > bytes.len() {
        return Err(TextureImportError::Truncated);
    }
    let value = u32::from_be_bytes([
        bytes[*cursor],
        bytes[*cursor + 1],
        bytes[*cursor + 2],
        bytes[*cursor + 3],
    ]);
    *cursor = end;
    Ok(value)
}
