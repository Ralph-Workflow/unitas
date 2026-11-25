use std::fmt;

use crate::texture::jpeg::{decode_jpeg, has_jpeg_signature};
use crate::texture::png::decode_png;
use crate::texture::tga::decode_tga;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureFormat {
    R8,
    Rg8,
    Rgb8,
    Rgba8,
}

impl TextureFormat {
    pub fn channels(self) -> usize {
        match self {
            TextureFormat::R8 => 1,
            TextureFormat::Rg8 => 2,
            TextureFormat::Rgb8 => 3,
            TextureFormat::Rgba8 => 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedTexture {
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub pixels: Vec<u8>,
}

impl DecodedTexture {
    pub fn channels(&self) -> usize {
        self.format.channels()
    }

    pub fn pixel(&self, x: u32, y: u32) -> Option<&[u8]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let channels = self.channels();
        let start = ((y as usize) * (self.width as usize) + (x as usize)) * channels;
        self.pixels.get(start..start + channels)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextureImportError {
    InvalidSignature,
    Truncated,
    InvalidChunkLength,
    MissingChunk(&'static str),
    UnsupportedColorType(u8),
    UnsupportedBitDepth(u8),
    UnsupportedCompression(u8),
    UnsupportedFilterMethod(u8),
    UnsupportedFilter(u8),
    InterlacedNotSupported,
    MissingPalette,
    PaletteIndexOutOfRange(usize),
    UnsupportedTgaImageType(u8),
    UnsupportedTgaPixelDepth(u8),
    UnsupportedTgaColourMap,
    InflateFailed(String),
    CorruptData(String),
    UnsupportedFormat(String),
    InvalidMetaText(String),
}

impl fmt::Display for TextureImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TextureImportError::InvalidSignature => {
                write!(formatter, "not a PNG or TGA image")
            }
            TextureImportError::Truncated => write!(formatter, "truncated image data"),
            TextureImportError::InvalidChunkLength => {
                write!(formatter, "invalid PNG chunk length")
            }
            TextureImportError::MissingChunk(name) => {
                write!(formatter, "PNG is missing the {name} chunk")
            }
            TextureImportError::UnsupportedColorType(value) => {
                write!(formatter, "unsupported PNG colour type {value}")
            }
            TextureImportError::UnsupportedBitDepth(value) => {
                write!(formatter, "unsupported PNG bit depth {value}")
            }
            TextureImportError::UnsupportedCompression(value) => {
                write!(formatter, "unsupported PNG compression method {value}")
            }
            TextureImportError::UnsupportedFilterMethod(value) => {
                write!(formatter, "unsupported PNG filter method {value}")
            }
            TextureImportError::UnsupportedFilter(value) => {
                write!(formatter, "unsupported PNG scanline filter {value}")
            }
            TextureImportError::InterlacedNotSupported => {
                write!(formatter, "interlaced PNG is not supported")
            }
            TextureImportError::MissingPalette => {
                write!(formatter, "indexed PNG has no palette")
            }
            TextureImportError::PaletteIndexOutOfRange(index) => {
                write!(formatter, "palette index {index} is out of range")
            }
            TextureImportError::UnsupportedTgaImageType(value) => {
                write!(formatter, "unsupported TGA image type {value}")
            }
            TextureImportError::UnsupportedTgaPixelDepth(value) => {
                write!(formatter, "unsupported TGA pixel depth {value}")
            }
            TextureImportError::UnsupportedTgaColourMap => {
                write!(formatter, "unsupported TGA colour map entry size")
            }
            TextureImportError::InflateFailed(message) => {
                write!(formatter, "zlib inflate failed: {message}")
            }
            TextureImportError::CorruptData(message) => {
                write!(formatter, "corrupt image data: {message}")
            }
            TextureImportError::UnsupportedFormat(name) => {
                write!(formatter, "unsupported texture format for {name}")
            }
            TextureImportError::InvalidMetaText(message) => {
                write!(formatter, "invalid texture meta text: {message}")
            }
        }
    }
}

impl std::error::Error for TextureImportError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpriteMode {
    None,
    Single,
    Multiple,
    Polygon,
}

impl SpriteMode {
    pub fn code(self) -> u8 {
        match self {
            SpriteMode::None => 0,
            SpriteMode::Single => 1,
            SpriteMode::Multiple => 2,
            SpriteMode::Polygon => 3,
        }
    }

    pub fn from_code(code: u8) -> Option<SpriteMode> {
        match code {
            0 => Some(SpriteMode::None),
            1 => Some(SpriteMode::Single),
            2 => Some(SpriteMode::Multiple),
            3 => Some(SpriteMode::Polygon),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WrapMode {
    Repeat,
    Clamp,
    Mirror,
}

impl WrapMode {
    pub fn code(self) -> u8 {
        match self {
            WrapMode::Repeat => 0,
            WrapMode::Clamp => 1,
            WrapMode::Mirror => 2,
        }
    }

    pub fn from_code(code: u8) -> Option<WrapMode> {
        match code {
            0 => Some(WrapMode::Repeat),
            1 => Some(WrapMode::Clamp),
            2 => Some(WrapMode::Mirror),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    Point,
    Bilinear,
    Trilinear,
}

impl FilterMode {
    pub fn code(self) -> u8 {
        match self {
            FilterMode::Point => 0,
            FilterMode::Bilinear => 1,
            FilterMode::Trilinear => 2,
        }
    }

    pub fn from_code(code: u8) -> Option<FilterMode> {
        match code {
            0 => Some(FilterMode::Point),
            1 => Some(FilterMode::Bilinear),
            2 => Some(FilterMode::Trilinear),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureImportSettings {
    pub srgb: bool,
    pub max_size: u32,
    pub mipmaps: bool,
    pub sprite_mode: SpriteMode,
    pub alpha_is_transparency: bool,
    pub wrap_mode: WrapMode,
    pub filter_mode: FilterMode,
}

impl Default for TextureImportSettings {
    fn default() -> Self {
        TextureImportSettings {
            srgb: true,
            max_size: 2048,
            mipmaps: true,
            sprite_mode: SpriteMode::None,
            alpha_is_transparency: true,
            wrap_mode: WrapMode::Repeat,
            filter_mode: FilterMode::Bilinear,
        }
    }
}

impl TextureImportSettings {
    pub fn is_linear(&self) -> bool {
        !self.srgb
    }

    pub fn to_linear(&self, value: u8) -> f32 {
        srgb_to_linear(value)
    }

    pub fn to_meta_text(&self) -> String {
        let mut text = String::from("TextureImporter:\n");
        text.push_str(&format!("  sRGBTexture: {}\n", u8::from(self.srgb)));
        text.push_str(&format!("  maxTextureSize: {}\n", self.max_size));
        text.push_str(&format!("  mipmapEnabled: {}\n", u8::from(self.mipmaps)));
        text.push_str(&format!("  spriteMode: {}\n", self.sprite_mode.code()));
        text.push_str(&format!(
            "  alphaIsTransparency: {}\n",
            u8::from(self.alpha_is_transparency)
        ));
        text.push_str(&format!("  wrapMode: {}\n", self.wrap_mode.code()));
        text.push_str(&format!("  filterMode: {}\n", self.filter_mode.code()));
        text
    }

    pub fn from_meta_text(text: &str) -> Result<TextureImportSettings, TextureImportError> {
        let srgb = meta_bool(text, "sRGBTexture")?;
        let max_size = meta_u32(text, "maxTextureSize")?;
        let mipmaps = meta_bool(text, "mipmapEnabled")?;
        let sprite_code = meta_u32(text, "spriteMode")?;
        let sprite_mode = match SpriteMode::from_code(u8::try_from(sprite_code).unwrap_or(u8::MAX)) {
            Some(mode) => mode,
            None => return Err(TextureImportError::InvalidMetaText("spriteMode".to_string())),
        };
        let alpha_is_transparency = meta_bool(text, "alphaIsTransparency")?;
        let wrap_code = meta_u32(text, "wrapMode")?;
        let wrap_mode = match WrapMode::from_code(u8::try_from(wrap_code).unwrap_or(u8::MAX)) {
            Some(mode) => mode,
            None => return Err(TextureImportError::InvalidMetaText("wrapMode".to_string())),
        };
        let filter_code = meta_u32(text, "filterMode")?;
        let filter_mode = match FilterMode::from_code(u8::try_from(filter_code).unwrap_or(u8::MAX)) {
            Some(mode) => mode,
            None => return Err(TextureImportError::InvalidMetaText("filterMode".to_string())),
        };
        Ok(TextureImportSettings {
            srgb,
            max_size,
            mipmaps,
            sprite_mode,
            alpha_is_transparency,
            wrap_mode,
            filter_mode,
        })
    }

    pub fn round_trip(&self) -> Result<TextureImportSettings, TextureImportError> {
        TextureImportSettings::from_meta_text(&self.to_meta_text())
    }

    pub fn build_mip_chain(&self, texture: &DecodedTexture) -> Vec<DecodedTexture> {
        let limited = apply_max_size(texture, self.max_size);
        if !self.mipmaps {
            return vec![limited];
        }
        generate_mip_chain(&limited)
    }
}

fn meta_value<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    for line in text.lines() {
        let trimmed = line.trim();
        let rest = match trimmed.strip_prefix(key) {
            Some(rest) => rest,
            None => continue,
        };
        let value = match rest.strip_prefix(':') {
            Some(value) => value.trim(),
            None => continue,
        };
        return Some(value);
    }
    None
}

fn meta_field<'a>(text: &'a str, key: &str) -> Result<&'a str, TextureImportError> {
    match meta_value(text, key) {
        Some(value) => Ok(value),
        None => Err(TextureImportError::InvalidMetaText(format!(
            "missing {key}"
        ))),
    }
}

fn meta_bool(text: &str, key: &str) -> Result<bool, TextureImportError> {
    match meta_field(text, key)? {
        "0" => Ok(false),
        "1" => Ok(true),
        other => Err(TextureImportError::InvalidMetaText(format!(
            "{key} is {other}"
        ))),
    }
}

fn meta_u32(text: &str, key: &str) -> Result<u32, TextureImportError> {
    match meta_field(text, key)?.parse::<u32>() {
        Ok(value) => Ok(value),
        Err(_) => Err(TextureImportError::InvalidMetaText(format!(
            "{key} is not a number"
        ))),
    }
}

pub fn srgb_to_linear(value: u8) -> f32 {
    let channel = f32::from(value) / 255.0;
    if channel <= 0.04045 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

pub fn generate_mip_chain(texture: &DecodedTexture) -> Vec<DecodedTexture> {
    let mut chain = Vec::new();
    let mut current = texture.clone();
    loop {
        chain.push(current.clone());
        if current.width <= 1 && current.height <= 1 {
            break;
        }
        current = downscale(&current);
    }
    chain
}

pub fn apply_max_size(texture: &DecodedTexture, max_size: u32) -> DecodedTexture {
    let mut current = texture.clone();
    while (current.width > max_size || current.height > max_size)
        && (current.width > 1 || current.height > 1)
    {
        current = downscale(&current);
    }
    current
}

fn downscale(texture: &DecodedTexture) -> DecodedTexture {
    let width = (texture.width / 2).max(1);
    let height = (texture.height / 2).max(1);
    let channels = texture.channels();
    let source_width = texture.width as usize;
    let source_height = texture.height as usize;
    let mut pixels = vec![0u8; width as usize * height as usize * channels];
    for y in 0..height as usize {
        for x in 0..width as usize {
            let mut sums = vec![0u32; channels];
            let mut count = 0u32;
            for offset_y in 0..2 {
                for offset_x in 0..2 {
                    let source_x = (x * 2 + offset_x).min(source_width - 1);
                    let source_y = (y * 2 + offset_y).min(source_height - 1);
                    let base = (source_y * source_width + source_x) * channels;
                    for channel in 0..channels {
                        sums[channel] += u32::from(
                            texture.pixels.get(base + channel).copied().unwrap_or_default(),
                        );
                    }
                    count += 1;
                }
            }
            let target = (y * width as usize + x) * channels;
            for channel in 0..channels {
                pixels[target + channel] = ((sums[channel] + count / 2) / count) as u8;
            }
        }
    }
    DecodedTexture {
        width,
        height,
        format: texture.format,
        pixels,
    }
}

pub fn decode_texture(bytes: &[u8], file_name: &str) -> Result<DecodedTexture, TextureImportError> {
    if has_jpeg_signature(bytes) {
        return decode_jpeg(bytes);
    }
    let lower = file_name.to_ascii_lowercase();
    if lower.ends_with(".png") {
        return decode_png(bytes);
    }
    if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        return decode_jpeg(bytes);
    }
    if lower.ends_with(".tga") || lower.ends_with(".targa") {
        return decode_tga(bytes);
    }
    Err(TextureImportError::UnsupportedFormat(file_name.to_string()))
}
