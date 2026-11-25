use crate::inflate::inflate_zlib;

pub use crate::texture::import::{DecodedTexture, TextureImportError};
use crate::texture::import::TextureFormat;

const SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

struct Header {
    width: u32,
    height: u32,
    bit_depth: u8,
    color_type: u8,
    compression: u8,
    filter_method: u8,
    interlace: u8,
}

enum Transparency {
    Gray(u8),
    Rgb([u8; 3]),
    Palette(Vec<u8>),
}

pub fn decode_png(bytes: &[u8]) -> Result<DecodedTexture, TextureImportError> {
    if bytes.len() < SIGNATURE.len() || bytes[..SIGNATURE.len()] != SIGNATURE {
        return Err(TextureImportError::InvalidSignature);
    }
    let mut offset = SIGNATURE.len();
    let mut header: Option<Header> = None;
    let mut palette: Vec<[u8; 3]> = Vec::new();
    let mut transparency: Option<Vec<u8>> = None;
    let mut idat: Vec<u8> = Vec::new();
    let mut ended = false;
    while bytes.len() >= offset + 8 {
        let length = u32::from_be_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        let kind = [
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ];
        let start = offset + 8;
        let end = match start.checked_add(length) {
            Some(end) => end,
            None => return Err(TextureImportError::Truncated),
        };
        if bytes.len() < end + 4 {
            return Err(TextureImportError::Truncated);
        }
        let data = &bytes[start..end];
        match &kind {
            b"IHDR" => header = Some(parse_header(data)?),
            b"PLTE" => palette = parse_palette(data)?,
            b"tRNS" => transparency = Some(data.to_vec()),
            b"IDAT" => idat.extend_from_slice(data),
            b"IEND" => {
                ended = true;
                break;
            }
            _ => {}
        }
        offset = end + 4;
    }
    let header = match header {
        Some(header) => header,
        None => return Err(TextureImportError::MissingChunk("IHDR")),
    };
    if !ended {
        return Err(TextureImportError::Truncated);
    }
    if header.interlace != 0 {
        return Err(TextureImportError::InterlacedNotSupported);
    }
    if header.compression != 0 {
        return Err(TextureImportError::UnsupportedCompression(header.compression));
    }
    if header.filter_method != 0 {
        return Err(TextureImportError::UnsupportedFilterMethod(header.filter_method));
    }
    if header.width == 0 || header.height == 0 {
        return Err(TextureImportError::CorruptData("zero image size".to_string()));
    }
    let channels = match header.color_type {
        0 => 1,
        2 => 3,
        3 => 1,
        4 => 2,
        6 => 4,
        other => return Err(TextureImportError::UnsupportedColorType(other)),
    };
    if header.bit_depth != 8 && header.bit_depth != 16 {
        return Err(TextureImportError::UnsupportedBitDepth(header.bit_depth));
    }
    let sample_bytes = usize::from(header.bit_depth / 8);
    let transparency = match transparency {
        Some(data) => Some(parse_transparency(&data, header.color_type)?),
        None => None,
    };
    let raw = inflate_zlib(&idat)
        .map_err(|error| TextureImportError::InflateFailed(format!("{error:?}")))?;
    let stride = header.width as usize * channels * sample_bytes;
    let expected = header.height as usize * (stride + 1);
    if raw.len() < expected {
        return Err(TextureImportError::Truncated);
    }
    let samples = unfilter_rows(&raw, &header, stride, channels, sample_bytes)?;
    assemble(&header, channels, samples, palette, transparency)
}

fn parse_header(data: &[u8]) -> Result<Header, TextureImportError> {
    if data.len() != 13 {
        return Err(TextureImportError::InvalidChunkLength);
    }
    Ok(Header {
        width: u32::from_be_bytes([data[0], data[1], data[2], data[3]]),
        height: u32::from_be_bytes([data[4], data[5], data[6], data[7]]),
        bit_depth: data[8],
        color_type: data[9],
        compression: data[10],
        filter_method: data[11],
        interlace: data[12],
    })
}

fn parse_palette(data: &[u8]) -> Result<Vec<[u8; 3]>, TextureImportError> {
    if data.len() % 3 != 0 {
        return Err(TextureImportError::CorruptData(
            "palette length is not a multiple of three".to_string(),
        ));
    }
    Ok(data
        .chunks_exact(3)
        .map(|entry| [entry[0], entry[1], entry[2]])
        .collect())
}

fn parse_transparency(
    data: &[u8],
    color_type: u8,
) -> Result<Transparency, TextureImportError> {
    match color_type {
        0 => {
            if data.len() < 2 {
                return Err(TextureImportError::CorruptData(
                    "greyscale tRNS is too short".to_string(),
                ));
            }
            Ok(Transparency::Gray(data[0]))
        }
        2 => {
            if data.len() < 6 {
                return Err(TextureImportError::CorruptData(
                    "true colour tRNS is too short".to_string(),
                ));
            }
            Ok(Transparency::Rgb([data[0], data[1], data[2]]))
        }
        3 => Ok(Transparency::Palette(data.to_vec())),
        other => Err(TextureImportError::CorruptData(format!(
            "tRNS is not valid for colour type {other}"
        ))),
    }
}

fn unfilter_rows(
    raw: &[u8],
    header: &Header,
    stride: usize,
    channels: usize,
    sample_bytes: usize,
) -> Result<Vec<u8>, TextureImportError> {
    let filter_bpp = channels * sample_bytes;
    let mut previous = vec![0u8; stride];
    let mut row = vec![0u8; stride];
    let mut samples: Vec<u8> = Vec::with_capacity(
        header.width as usize * header.height as usize * channels,
    );
    let mut position = 0usize;
    for _ in 0..header.height {
        let filter = raw[position];
        position += 1;
        row.copy_from_slice(&raw[position..position + stride]);
        position += stride;
        unfilter(filter, &mut row, &previous, filter_bpp)?;
        previous.copy_from_slice(&row);
        for index in 0..header.width as usize * channels {
            samples.push(row[index * sample_bytes]);
        }
    }
    Ok(samples)
}

fn unfilter(
    filter: u8,
    row: &mut [u8],
    previous: &[u8],
    filter_bpp: usize,
) -> Result<(), TextureImportError> {
    match filter {
        0 => {}
        1 => {
            for index in filter_bpp..row.len() {
                row[index] = row[index].wrapping_add(row[index - filter_bpp]);
            }
        }
        2 => {
            for index in 0..row.len() {
                row[index] = row[index].wrapping_add(previous[index]);
            }
        }
        3 => {
            for index in 0..row.len() {
                let left = if index >= filter_bpp {
                    u16::from(row[index - filter_bpp])
                } else {
                    0
                };
                let up = u16::from(previous[index]);
                row[index] = row[index].wrapping_add(((left + up) / 2) as u8);
            }
        }
        4 => {
            for index in 0..row.len() {
                let left = if index >= filter_bpp {
                    row[index - filter_bpp]
                } else {
                    0
                };
                let up = previous[index];
                let upper_left = if index >= filter_bpp {
                    previous[index - filter_bpp]
                } else {
                    0
                };
                row[index] = row[index].wrapping_add(paeth(left, up, upper_left));
            }
        }
        other => return Err(TextureImportError::UnsupportedFilter(other)),
    }
    Ok(())
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

fn assemble(
    header: &Header,
    _channels: usize,
    samples: Vec<u8>,
    palette: Vec<[u8; 3]>,
    transparency: Option<Transparency>,
) -> Result<DecodedTexture, TextureImportError> {
    let pixel_count = header.width as usize * header.height as usize;
    let (format, pixels) = match header.color_type {
        0 => match &transparency {
            None => (TextureFormat::R8, samples),
            Some(Transparency::Gray(key)) => {
                let mut pixels = Vec::with_capacity(pixel_count * 4);
                for value in &samples {
                    let alpha = if *value == *key { 0 } else { 255 };
                    pixels.push(*value);
                    pixels.push(*value);
                    pixels.push(*value);
                    pixels.push(alpha);
                }
                (TextureFormat::Rgba8, pixels)
            }
            Some(_) => {
                return Err(TextureImportError::CorruptData(
                    "tRNS does not match the greyscale colour type".to_string(),
                ))
            }
        },
        2 => match &transparency {
            None => (TextureFormat::Rgb8, samples),
            Some(Transparency::Rgb(key)) => {
                let mut pixels = Vec::with_capacity(pixel_count * 4);
                for index in 0..pixel_count {
                    let base = index * 3;
                    let alpha = if samples[base] == key[0]
                        && samples[base + 1] == key[1]
                        && samples[base + 2] == key[2]
                    {
                        0
                    } else {
                        255
                    };
                    pixels.push(samples[base]);
                    pixels.push(samples[base + 1]);
                    pixels.push(samples[base + 2]);
                    pixels.push(alpha);
                }
                (TextureFormat::Rgba8, pixels)
            }
            Some(_) => {
                return Err(TextureImportError::CorruptData(
                    "tRNS does not match the true colour colour type".to_string(),
                ))
            }
        },
        3 => {
            if palette.is_empty() {
                return Err(TextureImportError::MissingPalette);
            }
            let alpha = match &transparency {
                Some(Transparency::Palette(values)) => Some(values),
                Some(_) => {
                    return Err(TextureImportError::CorruptData(
                        "tRNS does not match the indexed colour type".to_string(),
                    ))
                }
                None => None,
            };
            let with_alpha = alpha.is_some();
            let mut pixels = Vec::with_capacity(pixel_count * if with_alpha { 4 } else { 3 });
            for index in &samples {
                let entry = match palette.get(usize::from(*index)) {
                    Some(entry) => entry,
                    None => {
                        return Err(TextureImportError::PaletteIndexOutOfRange(
                            usize::from(*index),
                        ))
                    }
                };
                pixels.push(entry[0]);
                pixels.push(entry[1]);
                pixels.push(entry[2]);
                if with_alpha {
                    let value = match alpha {
                        Some(values) => values.get(usize::from(*index)).copied(),
                        None => None,
                    };
                    pixels.push(value.unwrap_or(255));
                }
            }
            let format = if with_alpha {
                TextureFormat::Rgba8
            } else {
                TextureFormat::Rgb8
            };
            (format, pixels)
        }
        4 => (TextureFormat::Rg8, samples),
        _ => (TextureFormat::Rgba8, samples),
    };
    Ok(DecodedTexture {
        width: header.width,
        height: header.height,
        format,
        pixels,
    })
}
