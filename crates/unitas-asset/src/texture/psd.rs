//! Photoshop document parsing for the PSD (container version 1) and the PSB
//! (container version 2) containers.
//!
//! The containers are read by a hand-written parser because no maintained Rust
//! library parses the Photoshop large document container; the texture importer
//! carried that reader for PSD already, and this module extends it to PSB and
//! to the layer hierarchy.

use std::cmp::Ordering;

use crate::inflate::inflate_zlib;
use crate::texture::import::TextureFormat;
pub use crate::texture::import::{DecodedTexture, TextureImportError};

const SIGNATURE: [u8; 4] = [0x38, 0x42, 0x50, 0x53];

const VERSION_PSD: u16 = 1;
const VERSION_PSB: u16 = 2;

const COLOR_MODE_RGB: u16 = 3;

const COMPRESSION_RAW: u16 = 0;
const COMPRESSION_RLE: u16 = 1;
const COMPRESSION_ZIP: u16 = 2;
const COMPRESSION_ZIP_PREDICTION: u16 = 3;

const SECTION_DIVIDER_OPEN_FOLDER: u32 = 1;
const SECTION_DIVIDER_CLOSED_FOLDER: u32 = 2;
const SECTION_DIVIDER_BOUNDING: u32 = 3;

/// The rectangle a layer covers, in document pixels measured from the top left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PsdRect {
    pub top: i32,
    pub left: i32,
    pub bottom: i32,
    pub right: i32,
}

impl PsdRect {
    pub fn width(self) -> i32 {
        self.right.saturating_sub(self.left).max(0)
    }

    pub fn height(self) -> i32 {
        self.bottom.saturating_sub(self.top).max(0)
    }
}

/// A named layer of a Photoshop document; a group carries its children.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PsdLayer {
    pub name: String,
    pub rect: PsdRect,
    pub children: Vec<PsdLayer>,
}

/// The header and the layer hierarchy of a Photoshop document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PsdDocument {
    pub version: u16,
    pub width: u32,
    pub height: u32,
    pub layers: Vec<PsdLayer>,
}

pub fn has_psd_signature(bytes: &[u8]) -> bool {
    bytes.len() >= SIGNATURE.len() && bytes[..SIGNATURE.len()] == SIGNATURE
}

/// Decodes the composite image of a PSD or PSB document.
pub fn decode_psd(bytes: &[u8]) -> Result<DecodedTexture, TextureImportError> {
    let header = read_header(bytes)?;
    let mut reader = Reader::new(bytes, header.version);
    reader.position = header.size;
    reader.skip_section()?;
    reader.skip_section()?;
    reader.skip_section()?;
    let compression = reader.u16()?;
    let width = header.width as usize;
    let height = header.height as usize;
    let channels = usize::from(header.channels);
    let sample_bytes = usize::from(header.depth / 8);
    let row_size = width
        .checked_mul(sample_bytes)
        .ok_or_else(|| TextureImportError::CorruptData("PSD row size overflows".to_string()))?;
    let plane_size = row_size
        .checked_mul(height)
        .ok_or_else(|| TextureImportError::CorruptData("PSD plane size overflows".to_string()))?;
    let total = plane_size
        .checked_mul(channels)
        .ok_or_else(|| TextureImportError::CorruptData("PSD image size overflows".to_string()))?;
    let planes = match compression {
        COMPRESSION_RAW => raw_planes(&mut reader, total)?,
        COMPRESSION_RLE => rle_planes(&mut reader, row_size, height, channels)?,
        COMPRESSION_ZIP => zip_planes(&mut reader, total)?,
        COMPRESSION_ZIP_PREDICTION => {
            let mut planes = zip_planes(&mut reader, total)?;
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
        width: header.width,
        height: header.height,
        format,
        pixels,
    })
}

/// Parses the header and the layer hierarchy of a PSD or PSB document.
pub fn parse_psd_document(bytes: &[u8]) -> Result<PsdDocument, TextureImportError> {
    let header = read_header(bytes)?;
    let mut reader = Reader::new(bytes, header.version);
    reader.position = header.size;
    reader.skip_section()?;
    reader.skip_section()?;
    let layers = read_layer_section(&mut reader)?;
    Ok(PsdDocument {
        version: header.version,
        width: header.width,
        height: header.height,
        layers,
    })
}

struct PsdHeader {
    version: u16,
    channels: u16,
    width: u32,
    height: u32,
    depth: u16,
    size: usize,
}

fn read_header(bytes: &[u8]) -> Result<PsdHeader, TextureImportError> {
    if !has_psd_signature(bytes) {
        return Err(TextureImportError::InvalidSignature);
    }
    let mut reader = Reader::new(bytes, VERSION_PSD);
    reader.position = SIGNATURE.len();
    let version = reader.u16()?;
    if version != VERSION_PSD && version != VERSION_PSB {
        return Err(TextureImportError::UnsupportedFormat(format!(
            "PSD version {version}"
        )));
    }
    reader.version = version;
    reader.take(6)?;
    let channels = reader.u16()?;
    let (height, width) = if version == VERSION_PSB {
        (reader.u64()?, reader.u64()?)
    } else {
        (u64::from(reader.u32()?), u64::from(reader.u32()?))
    };
    let depth = reader.u16()?;
    let color_mode = reader.u16()?;
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
    let width = u32::try_from(width)
        .map_err(|_| TextureImportError::CorruptData("PSD width overflows".to_string()))?;
    let height = u32::try_from(height)
        .map_err(|_| TextureImportError::CorruptData("PSD height overflows".to_string()))?;
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
    Ok(PsdHeader {
        version,
        channels,
        width,
        height,
        depth,
        size: reader.position,
    })
}

fn read_layer_section(reader: &mut Reader) -> Result<Vec<PsdLayer>, TextureImportError> {
    let length = reader.section_length()?;
    if length == 0 {
        return Ok(Vec::new());
    }
    let body = reader.take(length)?;
    let mut body_reader = Reader::new(body, reader.version);
    let info_length = body_reader.section_length()?;
    if info_length == 0 {
        return Ok(Vec::new());
    }
    let info = body_reader.take(info_length)?;
    let mut info_reader = Reader::new(info, reader.version);
    let count = usize::from(info_reader.i16()?.unsigned_abs());
    let mut records = Vec::with_capacity(count);
    for _ in 0..count {
        records.push(read_layer_record(&mut info_reader)?);
    }
    Ok(build_layer_tree(records))
}

struct LayerRecord {
    name: String,
    rect: PsdRect,
    divider: Option<u32>,
}

fn read_layer_record(reader: &mut Reader) -> Result<LayerRecord, TextureImportError> {
    let top = reader.i32()?;
    let left = reader.i32()?;
    let bottom = reader.i32()?;
    let right = reader.i32()?;
    let channel_count = reader.u16()?;
    for _ in 0..channel_count {
        reader.i16()?;
        if reader.version == VERSION_PSB {
            reader.u64()?;
        } else {
            reader.u32()?;
        }
    }
    reader.take(4)?;
    reader.take(4)?;
    reader.u8()?;
    reader.u8()?;
    reader.u8()?;
    reader.u8()?;
    let extra_length = reader.u32()? as usize;
    let extra = reader.take(extra_length)?;
    let mut extra_reader = Reader::new(extra, reader.version);
    let mask_length = extra_reader.u32()? as usize;
    extra_reader.take(mask_length)?;
    let ranges_length = extra_reader.u32()? as usize;
    extra_reader.take(ranges_length)?;
    let name = read_pascal_string(&mut extra_reader)?;
    let (divider, unicode_name) = read_layer_tags(&mut extra_reader)?;
    Ok(LayerRecord {
        name: unicode_name.unwrap_or(name),
        rect: PsdRect {
            top,
            left,
            bottom,
            right,
        },
        divider,
    })
}

fn read_pascal_string(reader: &mut Reader) -> Result<String, TextureImportError> {
    let length = usize::from(reader.u8()?);
    let bytes = reader.take(length)?;
    let padding = (4 - length % 4) % 4;
    reader.take(padding)?;
    Ok(String::from_utf8_lossy(bytes).into_owned())
}

fn read_layer_tags(
    reader: &mut Reader,
) -> Result<(Option<u32>, Option<String>), TextureImportError> {
    let mut divider = None;
    let mut name = None;
    while reader.remaining() >= 8 {
        let signature = reader.take(4)?;
        if signature != b"8BIM" && signature != b"8B64" {
            break;
        }
        let key = reader.take(4)?;
        let length = reader.u32()? as usize;
        let data = reader.take(length)?;
        if key == b"lsct" {
            if data.len() >= 4 {
                divider = Some(u32::from_be_bytes([data[0], data[1], data[2], data[3]]));
            }
        } else if key == b"luni" {
            name = read_unicode_name(data);
        }
    }
    Ok((divider, name))
}

fn read_unicode_name(data: &[u8]) -> Option<String> {
    if data.len() < 4 {
        return None;
    }
    let count = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
    let byte_count = count.checked_mul(2)?;
    let bytes = data.get(4..4usize.checked_add(byte_count)?)?;
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
        .collect();
    String::from_utf16(&units).ok()
}

struct GroupFrame {
    name: String,
    rect: PsdRect,
    children: Vec<PsdLayer>,
}

fn build_layer_tree(records: Vec<LayerRecord>) -> Vec<PsdLayer> {
    let mut roots: Vec<PsdLayer> = Vec::new();
    let mut stack: Vec<GroupFrame> = Vec::new();
    for record in records {
        match record.divider {
            Some(SECTION_DIVIDER_BOUNDING) => stack.push(GroupFrame {
                name: record.name,
                rect: record.rect,
                children: Vec::new(),
            }),
            Some(SECTION_DIVIDER_OPEN_FOLDER | SECTION_DIVIDER_CLOSED_FOLDER) => {
                let frame = stack.pop().unwrap_or(GroupFrame {
                    name: record.name.clone(),
                    rect: record.rect,
                    children: Vec::new(),
                });
                let group = PsdLayer {
                    name: record.name,
                    rect: record.rect,
                    children: frame.children,
                };
                push_layer(&mut stack, &mut roots, group);
            }
            _ => push_layer(
                &mut stack,
                &mut roots,
                PsdLayer {
                    name: record.name,
                    rect: record.rect,
                    children: Vec::new(),
                },
            ),
        }
    }
    while let Some(frame) = stack.pop() {
        let group = PsdLayer {
            name: frame.name,
            rect: frame.rect,
            children: frame.children,
        };
        push_layer(&mut stack, &mut roots, group);
    }
    roots
}

fn push_layer(stack: &mut [GroupFrame], roots: &mut Vec<PsdLayer>, layer: PsdLayer) {
    match stack.last_mut() {
        Some(frame) => frame.children.push(layer),
        None => roots.push(layer),
    }
}

fn raw_planes(reader: &mut Reader, total: usize) -> Result<Vec<u8>, TextureImportError> {
    Ok(reader.take(total)?.to_vec())
}

fn rle_planes(
    reader: &mut Reader,
    row_size: usize,
    height: usize,
    channels: usize,
) -> Result<Vec<u8>, TextureImportError> {
    let rows = height * channels;
    let mut counts = Vec::with_capacity(rows);
    for _ in 0..rows {
        let count = if reader.version == VERSION_PSB {
            reader.u32()? as usize
        } else {
            usize::from(reader.u16()?)
        };
        counts.push(count);
    }
    let mut planes = Vec::with_capacity(rows.saturating_mul(row_size));
    for count in counts {
        let row = reader.take(count)?;
        planes.extend_from_slice(&decode_packbits(row, row_size)?);
    }
    Ok(planes)
}

fn zip_planes(reader: &mut Reader, total: usize) -> Result<Vec<u8>, TextureImportError> {
    let inflated = inflate_zlib(&reader.bytes[reader.position..])
        .map_err(|error| TextureImportError::InflateFailed(format!("{error}")))?;
    if inflated.len() != total {
        return Err(TextureImportError::CorruptData(format!(
            "PSD image data holds {} bytes, expected {total}",
            inflated.len()
        )));
    }
    reader.position = reader.bytes.len();
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

struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
    version: u16,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8], version: u16) -> Self {
        Reader {
            bytes,
            position: 0,
            version,
        }
    }

    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.position)
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], TextureImportError> {
        let end = self
            .position
            .checked_add(count)
            .ok_or(TextureImportError::Truncated)?;
        if end > self.bytes.len() {
            return Err(TextureImportError::Truncated);
        }
        let slice = &self.bytes[self.position..end];
        self.position = end;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8, TextureImportError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, TextureImportError> {
        let bytes = self.take(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn i16(&mut self) -> Result<i16, TextureImportError> {
        let bytes = self.take(2)?;
        Ok(i16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn u32(&mut self) -> Result<u32, TextureImportError> {
        let bytes = self.take(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn i32(&mut self) -> Result<i32, TextureImportError> {
        let bytes = self.take(4)?;
        Ok(i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn u64(&mut self) -> Result<u64, TextureImportError> {
        let bytes = self.take(8)?;
        Ok(u64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn section_length(&mut self) -> Result<usize, TextureImportError> {
        let length = if self.version == VERSION_PSB {
            self.u64()?
        } else {
            u64::from(self.u32()?)
        };
        if length > usize::MAX as u64 {
            return Err(TextureImportError::CorruptData(
                "PSD section length does not fit the address space".to_string(),
            ));
        }
        Ok(length as usize)
    }

    fn skip_section(&mut self) -> Result<(), TextureImportError> {
        let length = self.section_length()?;
        self.take(length).map(|_| ())
    }
}
