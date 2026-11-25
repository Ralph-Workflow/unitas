use crate::texture::import::TextureFormat;
pub use crate::texture::import::{DecodedTexture, TextureImportError};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    ColourMapped,
    TrueColour,
    Greyscale,
}

pub fn decode_tga(bytes: &[u8]) -> Result<DecodedTexture, TextureImportError> {
    if bytes.len() < 18 {
        return Err(TextureImportError::Truncated);
    }
    let id_length = usize::from(bytes[0]);
    let colour_map_type = bytes[1];
    let image_type = bytes[2];
    let colour_map_first = usize::from(u16::from_le_bytes([bytes[3], bytes[4]]));
    let colour_map_length = usize::from(u16::from_le_bytes([bytes[5], bytes[6]]));
    let colour_map_entry_size = bytes[7];
    let width = u32::from(u16::from_le_bytes([bytes[12], bytes[13]]));
    let height = u32::from(u16::from_le_bytes([bytes[14], bytes[15]]));
    let pixel_depth = bytes[16];
    let descriptor = bytes[17];
    if width == 0 || height == 0 {
        return Err(TextureImportError::CorruptData(
            "zero image size".to_string(),
        ));
    }
    let (kind, run_length) = match image_type {
        1 => (Kind::ColourMapped, false),
        2 => (Kind::TrueColour, false),
        3 => (Kind::Greyscale, false),
        9 => (Kind::ColourMapped, true),
        10 => (Kind::TrueColour, true),
        11 => (Kind::Greyscale, true),
        other => return Err(TextureImportError::UnsupportedTgaImageType(other)),
    };
    let mapped = kind == Kind::ColourMapped;
    if mapped && colour_map_type != 1 {
        return Err(TextureImportError::CorruptData(
            "colour mapped image has no colour map".to_string(),
        ));
    }
    if !mapped && colour_map_type != 0 {
        return Err(TextureImportError::CorruptData(
            "image type does not use a colour map".to_string(),
        ));
    }
    let entry_bytes = match colour_map_entry_size {
        24 => 3usize,
        32 => 4usize,
        _ => {
            if mapped {
                return Err(TextureImportError::UnsupportedTgaColourMap);
            }
            0
        }
    };
    let sample_bytes = match (kind, pixel_depth) {
        (Kind::ColourMapped, 8) => 1usize,
        (Kind::ColourMapped, 16) => 2usize,
        (Kind::TrueColour, 16) => 2usize,
        (Kind::TrueColour, 24) => 3usize,
        (Kind::TrueColour, 32) => 4usize,
        (Kind::Greyscale, 8) => 1usize,
        _ => return Err(TextureImportError::UnsupportedTgaPixelDepth(pixel_depth)),
    };
    let mut offset = 18 + id_length;
    let mut palette: Vec<[u8; 4]> = Vec::new();
    if mapped {
        let span = colour_map_length * entry_bytes;
        if bytes.len() < offset + span {
            return Err(TextureImportError::Truncated);
        }
        for index in 0..colour_map_length {
            let start = offset + index * entry_bytes;
            let entry = &bytes[start..start + entry_bytes];
            let alpha = if entry_bytes == 4 { entry[3] } else { 255 };
            palette.push([entry[2], entry[1], entry[0], alpha]);
        }
        offset += span;
    }
    let pixel_count = width as usize * height as usize;
    let total = pixel_count * sample_bytes;
    let samples = if run_length {
        decode_rle(bytes, &mut offset, total, sample_bytes)?
    } else {
        if bytes.len() < offset + total {
            return Err(TextureImportError::Truncated);
        }
        bytes[offset..offset + total].to_vec()
    };
    let (format, mut pixels) = convert(
        kind,
        pixel_depth,
        &samples,
        &palette,
        colour_map_first,
        entry_bytes,
    )?;
    let channels = format.channels();
    let row_length = width as usize * channels;
    if descriptor & 0x20 == 0 {
        let mut flipped = Vec::with_capacity(pixels.len());
        for row in (0..height as usize).rev() {
            flipped.extend_from_slice(&pixels[row * row_length..(row + 1) * row_length]);
        }
        pixels = flipped;
    }
    if descriptor & 0x10 != 0 {
        let mut flipped = Vec::with_capacity(pixels.len());
        for row in 0..height as usize {
            for column in (0..width as usize).rev() {
                let start = row * row_length + column * channels;
                flipped.extend_from_slice(&pixels[start..start + channels]);
            }
        }
        pixels = flipped;
    }
    Ok(DecodedTexture {
        width,
        height,
        format,
        pixels,
    })
}

fn decode_rle(
    bytes: &[u8],
    offset: &mut usize,
    total: usize,
    sample_bytes: usize,
) -> Result<Vec<u8>, TextureImportError> {
    let mut out: Vec<u8> = Vec::with_capacity(total);
    while out.len() < total {
        let header = match bytes.get(*offset) {
            Some(value) => *value,
            None => return Err(TextureImportError::Truncated),
        };
        *offset += 1;
        let count = usize::from(header & 0x7F) + 1;
        if header & 0x80 != 0 {
            let end = *offset + sample_bytes;
            if bytes.len() < end {
                return Err(TextureImportError::Truncated);
            }
            let pixel = &bytes[*offset..end];
            let take = ((total - out.len()) / sample_bytes).min(count);
            for _ in 0..take {
                out.extend_from_slice(pixel);
            }
            *offset = end;
        } else {
            let end = *offset + count * sample_bytes;
            if bytes.len() < end {
                return Err(TextureImportError::Truncated);
            }
            let take = (total - out.len()).min(count * sample_bytes);
            out.extend_from_slice(&bytes[*offset..*offset + take]);
            *offset = end;
        }
    }
    Ok(out)
}

fn convert(
    kind: Kind,
    pixel_depth: u8,
    samples: &[u8],
    palette: &[[u8; 4]],
    colour_map_first: usize,
    colour_map_entry_bytes: usize,
) -> Result<(TextureFormat, Vec<u8>), TextureImportError> {
    let pixel_count = samples.len()
        / match pixel_depth {
            8 => 1,
            16 => 2,
            24 => 3,
            _ => 4,
        };
    let mut pixels: Vec<u8> = Vec::with_capacity(pixel_count * 4);
    match kind {
        Kind::ColourMapped => {
            let has_alpha = colour_map_entry_bytes == 4;
            for index in 0..pixel_count {
                let value = if pixel_depth == 8 {
                    usize::from(samples[index])
                } else {
                    usize::from(u16::from_le_bytes([
                        samples[index * 2],
                        samples[index * 2 + 1],
                    ]))
                };
                let entry = match palette.get(value + colour_map_first) {
                    Some(entry) => entry,
                    None => {
                        return Err(TextureImportError::PaletteIndexOutOfRange(
                            value + colour_map_first,
                        ))
                    }
                };
                pixels.push(entry[0]);
                pixels.push(entry[1]);
                pixels.push(entry[2]);
                if has_alpha {
                    pixels.push(entry[3]);
                }
            }
            let format = if has_alpha {
                TextureFormat::Rgba8
            } else {
                TextureFormat::Rgb8
            };
            Ok((format, pixels))
        }
        Kind::TrueColour => match pixel_depth {
            16 => {
                for index in 0..pixel_count {
                    let value = u16::from_le_bytes([samples[index * 2], samples[index * 2 + 1]]);
                    let alpha = if value & 0x8000 == 0 { 0 } else { 255 };
                    pixels.push(expand_five(((value >> 10) & 0x1F) as u8));
                    pixels.push(expand_five(((value >> 5) & 0x1F) as u8));
                    pixels.push(expand_five((value & 0x1F) as u8));
                    pixels.push(alpha);
                }
                Ok((TextureFormat::Rgba8, pixels))
            }
            24 => {
                for index in 0..pixel_count {
                    pixels.push(samples[index * 3 + 2]);
                    pixels.push(samples[index * 3 + 1]);
                    pixels.push(samples[index * 3]);
                }
                Ok((TextureFormat::Rgb8, pixels))
            }
            _ => {
                for index in 0..pixel_count {
                    pixels.push(samples[index * 4 + 2]);
                    pixels.push(samples[index * 4 + 1]);
                    pixels.push(samples[index * 4]);
                    pixels.push(samples[index * 4 + 3]);
                }
                Ok((TextureFormat::Rgba8, pixels))
            }
        },
        Kind::Greyscale => {
            for sample in samples.iter().take(pixel_count) {
                pixels.push(*sample);
            }
            Ok((TextureFormat::R8, pixels))
        }
    }
}

fn expand_five(value: u8) -> u8 {
    ((u16::from(value) * 255 + 15) / 31) as u8
}
