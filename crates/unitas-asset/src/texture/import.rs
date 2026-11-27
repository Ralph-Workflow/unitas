use std::collections::HashSet;
use std::fmt;

use crate::dependencies::{ImportDiagnostic, ImportOutcome};
use crate::texture::exr::decode_exr;
use crate::texture::hdr::decode_hdr;
use crate::texture::jpeg::{decode_jpeg, has_jpeg_signature};
use crate::texture::png::decode_png;
use crate::texture::psd::{
    decode_psd, has_psd_signature, parse_psd_document, PsdDocument, PsdLayer,
};
use crate::texture::tga::decode_tga;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureFormat {
    R8,
    Rg8,
    Rgb8,
    Rgba8,
    Rgba32F,
}

impl TextureFormat {
    pub fn channels(self) -> usize {
        match self {
            TextureFormat::R8 => 1,
            TextureFormat::Rg8 => 2,
            TextureFormat::Rgb8 => 3,
            TextureFormat::Rgba8 => 4,
            TextureFormat::Rgba32F => 4,
        }
    }

    pub fn bytes_per_channel(self) -> usize {
        match self {
            TextureFormat::Rgba32F => 4,
            _ => 1,
        }
    }

    pub fn bytes_per_pixel(self) -> usize {
        self.channels() * self.bytes_per_channel()
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
        let bytes = self.format.bytes_per_pixel();
        let start = ((y as usize) * (self.width as usize) + (x as usize)) * bytes;
        self.pixels.get(start..start + bytes)
    }

    pub fn pixel_f32(&self, x: u32, y: u32) -> Option<[f32; 4]> {
        let bytes = self.pixel(x, y)?;
        if self.format != TextureFormat::Rgba32F {
            return None;
        }
        let mut values = [0.0f32; 4];
        for (index, value) in values.iter_mut().enumerate() {
            let start = index * 4;
            let mut raw = [0u8; 4];
            raw.copy_from_slice(bytes.get(start..start + 4)?);
            *value = f32::from_le_bytes(raw);
        }
        Some(values)
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
pub enum TextureImporterType {
    Default,
    NormalMap,
    Gui,
    Sprite,
    Cursor,
    Cookie,
    Lightmap,
    DirectionalLightmap,
    Shadowmask,
    SingleChannel,
}

impl TextureImporterType {
    pub fn code(self) -> u8 {
        match self {
            TextureImporterType::Default => 0,
            TextureImporterType::NormalMap => 1,
            TextureImporterType::Gui => 2,
            TextureImporterType::Cookie => 4,
            TextureImporterType::Lightmap => 6,
            TextureImporterType::Cursor => 7,
            TextureImporterType::Sprite => 8,
            TextureImporterType::SingleChannel => 10,
            TextureImporterType::Shadowmask => 11,
            TextureImporterType::DirectionalLightmap => 12,
        }
    }

    pub fn from_code(code: u8) -> Option<TextureImporterType> {
        match code {
            0 => Some(TextureImporterType::Default),
            1 => Some(TextureImporterType::NormalMap),
            2 => Some(TextureImporterType::Gui),
            4 => Some(TextureImporterType::Cookie),
            6 => Some(TextureImporterType::Lightmap),
            7 => Some(TextureImporterType::Cursor),
            8 => Some(TextureImporterType::Sprite),
            10 => Some(TextureImporterType::SingleChannel),
            11 => Some(TextureImporterType::Shadowmask),
            12 => Some(TextureImporterType::DirectionalLightmap),
            _ => None,
        }
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureCompressionQuality {
    Uncompressed,
    Normal,
    HighQuality,
    LowQuality,
}

impl TextureCompressionQuality {
    pub fn code(self) -> u8 {
        match self {
            TextureCompressionQuality::Uncompressed => 0,
            TextureCompressionQuality::Normal => 1,
            TextureCompressionQuality::HighQuality => 2,
            TextureCompressionQuality::LowQuality => 3,
        }
    }

    pub fn from_code(code: u8) -> Option<TextureCompressionQuality> {
        match code {
            0 => Some(TextureCompressionQuality::Uncompressed),
            1 => Some(TextureCompressionQuality::Normal),
            2 => Some(TextureCompressionQuality::HighQuality),
            3 => Some(TextureCompressionQuality::LowQuality),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TexturePlatformFormat {
    Automatic,
    Uncompressed,
    Rgba32,
    Rgb24,
    Dxt1,
    Dxt5,
    Bc7,
    Etc2Rgb4,
    Etc2Rgba8,
    Astc4x4,
    Astc6x6,
    Astc8x8,
}

impl TexturePlatformFormat {
    pub fn code(self) -> i32 {
        match self {
            TexturePlatformFormat::Automatic => -1,
            TexturePlatformFormat::Uncompressed => 0,
            TexturePlatformFormat::Rgb24 => 3,
            TexturePlatformFormat::Rgba32 => 4,
            TexturePlatformFormat::Dxt1 => 10,
            TexturePlatformFormat::Dxt5 => 12,
            TexturePlatformFormat::Bc7 => 25,
            TexturePlatformFormat::Etc2Rgb4 => 45,
            TexturePlatformFormat::Etc2Rgba8 => 47,
            TexturePlatformFormat::Astc4x4 => 48,
            TexturePlatformFormat::Astc6x6 => 50,
            TexturePlatformFormat::Astc8x8 => 51,
        }
    }

    pub fn from_code(code: i32) -> Option<TexturePlatformFormat> {
        match code {
            -1 => Some(TexturePlatformFormat::Automatic),
            0 => Some(TexturePlatformFormat::Uncompressed),
            3 => Some(TexturePlatformFormat::Rgb24),
            4 => Some(TexturePlatformFormat::Rgba32),
            10 => Some(TexturePlatformFormat::Dxt1),
            12 => Some(TexturePlatformFormat::Dxt5),
            25 => Some(TexturePlatformFormat::Bc7),
            45 => Some(TexturePlatformFormat::Etc2Rgb4),
            47 => Some(TexturePlatformFormat::Etc2Rgba8),
            48 => Some(TexturePlatformFormat::Astc4x4),
            50 => Some(TexturePlatformFormat::Astc6x6),
            51 => Some(TexturePlatformFormat::Astc8x8),
            _ => None,
        }
    }

    pub fn is_compressed(self) -> bool {
        matches!(
            self,
            TexturePlatformFormat::Dxt1
                | TexturePlatformFormat::Dxt5
                | TexturePlatformFormat::Bc7
                | TexturePlatformFormat::Etc2Rgb4
                | TexturePlatformFormat::Etc2Rgba8
                | TexturePlatformFormat::Astc4x4
                | TexturePlatformFormat::Astc6x6
                | TexturePlatformFormat::Astc8x8
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TexturePlatformSettings {
    pub build_target: String,
    pub overridden: bool,
    pub max_texture_size: u32,
    pub texture_compression: TextureCompressionQuality,
    pub format: TexturePlatformFormat,
    pub compression_quality: u8,
    pub crunched_compression: bool,
}

impl Default for TexturePlatformSettings {
    fn default() -> Self {
        TexturePlatformSettings {
            build_target: "DefaultTexturePlatform".to_string(),
            overridden: false,
            max_texture_size: 2048,
            texture_compression: TextureCompressionQuality::Normal,
            format: TexturePlatformFormat::Automatic,
            compression_quality: 50,
            crunched_compression: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpriteSlice {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub pivot_x: f32,
    pub pivot_y: f32,
    pub border_left: f32,
    pub border_bottom: f32,
    pub border_right: f32,
    pub border_top: f32,
}

impl SpriteSlice {
    pub fn new(name: impl Into<String>, x: i32, y: i32, width: i32, height: i32) -> Self {
        SpriteSlice {
            name: name.into(),
            x,
            y,
            width,
            height,
            pivot_x: 0.5,
            pivot_y: 0.5,
            border_left: 0.0,
            border_bottom: 0.0,
            border_right: 0.0,
            border_top: 0.0,
        }
    }

    pub fn sub_asset_id(&self) -> &str {
        &self.name
    }

    pub fn unity_local_file_id(&self) -> i64 {
        let mut h: u32 = 2_166_136_261;
        for byte in self.name.as_bytes() {
            h ^= u32::from(*byte);
            h = h.wrapping_mul(16_777_619);
        }
        213_000_000 + i64::from(h % 999_999)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextureImportSettings {
    pub texture_type: TextureImporterType,
    pub srgb: bool,
    pub max_size: u32,
    pub mipmaps: bool,
    pub sprite_mode: SpriteMode,
    pub alpha_is_transparency: bool,
    pub wrap_mode: WrapMode,
    pub filter_mode: FilterMode,
    pub platform_settings: Vec<TexturePlatformSettings>,
    pub sprite_slices: Vec<SpriteSlice>,
}

impl Default for TextureImportSettings {
    fn default() -> Self {
        TextureImportSettings {
            texture_type: TextureImporterType::Default,
            srgb: true,
            max_size: 2048,
            mipmaps: true,
            sprite_mode: SpriteMode::None,
            alpha_is_transparency: true,
            wrap_mode: WrapMode::Repeat,
            filter_mode: FilterMode::Bilinear,
            platform_settings: Vec::new(),
            sprite_slices: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextureImportResult {
    pub source: DecodedTexture,
    pub texture: DecodedTexture,
    pub mips: Vec<DecodedTexture>,
    pub settings: TextureImportSettings,
    pub frame_count: usize,
    pub excluded_frames: usize,
    pub applied_target: Option<String>,
    pub applied_format: Option<TexturePlatformFormat>,
    pub applied_compression: Option<TextureCompressionQuality>,
    pub encoded_format: TexturePlatformFormat,
    pub compressed_data: Option<Vec<u8>>,
    pub compressed_mip_data: Vec<Vec<u8>>,
    pub sprites: Vec<SpriteSlice>,
    pub notes: Vec<String>,
}

impl TextureImportSettings {
    pub fn is_linear(&self) -> bool {
        !self.srgb
            || matches!(
                self.texture_type,
                TextureImporterType::NormalMap
                    | TextureImporterType::Lightmap
                    | TextureImporterType::DirectionalLightmap
                    | TextureImporterType::Shadowmask
                    | TextureImporterType::SingleChannel
            )
    }

    pub fn is_normal_map(&self) -> bool {
        self.texture_type == TextureImporterType::NormalMap
    }

    pub fn to_linear(&self, value: u8) -> f32 {
        srgb_to_linear(value)
    }

    pub fn set_platform_override(&mut self, setting: TexturePlatformSettings) {
        if let Some(existing) = self
            .platform_settings
            .iter_mut()
            .find(|item| item.build_target == setting.build_target)
        {
            *existing = setting;
        } else {
            self.platform_settings.push(setting);
        }
    }

    pub fn platform_override(&self, build_target: &str) -> Option<&TexturePlatformSettings> {
        self.platform_settings
            .iter()
            .find(|item| item.build_target == build_target && item.overridden)
    }

    pub fn effective_max_size(&self, build_target: Option<&str>) -> u32 {
        match build_target.and_then(|target| self.platform_override(target)) {
            Some(override_setting) => override_setting.max_texture_size,
            None => self.max_size,
        }
    }

    pub fn to_meta_text(&self) -> String {
        let mut text = String::from("TextureImporter:\n");
        text.push_str(&format!("  textureType: {}\n", self.texture_type.code()));
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
        if !self.sprite_slices.is_empty() {
            text.push_str("  spriteSheet:\n");
            text.push_str("    serializedVersion: 2\n");
            text.push_str("    sprites:\n");
            for slice in &self.sprite_slices {
                text.push_str("    - serializedVersion: 2\n");
                text.push_str(&format!("      name: {}\n", slice.name));
                text.push_str("      rect:\n");
                text.push_str("        serializedVersion: 2\n");
                text.push_str(&format!("        x: {}\n", slice.x));
                text.push_str(&format!("        y: {}\n", slice.y));
                text.push_str(&format!("        width: {}\n", slice.width));
                text.push_str(&format!("        height: {}\n", slice.height));
                text.push_str(&format!(
                    "      pivot: {{x: {}, y: {}}}\n",
                    slice.pivot_x, slice.pivot_y
                ));
                text.push_str(&format!(
                    "      border: {{x: {}, y: {}, z: {}, w: {}}}\n",
                    slice.border_left, slice.border_bottom, slice.border_right, slice.border_top
                ));
            }
        }
        if !self.platform_settings.is_empty() {
            text.push_str("  platformSettings:\n");
            for platform in &self.platform_settings {
                text.push_str(&format!("  - buildTarget: {}\n", platform.build_target));
                text.push_str(&format!(
                    "    maxTextureSize: {}\n",
                    platform.max_texture_size
                ));
                text.push_str(&format!("    format: {}\n", platform.format.code()));
                text.push_str(&format!(
                    "    textureCompression: {}\n",
                    platform.texture_compression.code()
                ));
                text.push_str(&format!(
                    "    compressionQuality: {}\n",
                    platform.compression_quality
                ));
                text.push_str(&format!(
                    "    crunchedCompression: {}\n",
                    u8::from(platform.crunched_compression)
                ));
                text.push_str(&format!(
                    "    overridden: {}\n",
                    u8::from(platform.overridden)
                ));
            }
        }
        text
    }

    pub fn from_meta_text(text: &str) -> Result<TextureImportSettings, TextureImportError> {
        let texture_type = match meta_value(text, "textureType") {
            Some(raw) => {
                let code = raw
                    .parse::<u8>()
                    .map_err(|_| TextureImportError::InvalidMetaText("textureType".to_string()))?;
                TextureImporterType::from_code(code).ok_or_else(|| {
                    TextureImportError::InvalidMetaText(format!("textureType {code}"))
                })?
            }
            None => TextureImporterType::Default,
        };
        let srgb = meta_bool(text, "sRGBTexture")?;
        let max_size = meta_u32(text, "maxTextureSize")?;
        let mipmaps = meta_bool(text, "mipmapEnabled")?;
        let sprite_code = meta_u32(text, "spriteMode")?;
        let sprite_mode = match SpriteMode::from_code(u8::try_from(sprite_code).unwrap_or(u8::MAX))
        {
            Some(mode) => mode,
            None => {
                return Err(TextureImportError::InvalidMetaText(
                    "spriteMode".to_string(),
                ))
            }
        };
        let alpha_is_transparency = meta_bool(text, "alphaIsTransparency")?;
        let wrap_code = meta_u32(text, "wrapMode")?;
        let wrap_mode = match WrapMode::from_code(u8::try_from(wrap_code).unwrap_or(u8::MAX)) {
            Some(mode) => mode,
            None => return Err(TextureImportError::InvalidMetaText("wrapMode".to_string())),
        };
        let filter_code = meta_u32(text, "filterMode")?;
        let filter_mode = match FilterMode::from_code(u8::try_from(filter_code).unwrap_or(u8::MAX))
        {
            Some(mode) => mode,
            None => {
                return Err(TextureImportError::InvalidMetaText(
                    "filterMode".to_string(),
                ))
            }
        };
        let platform_settings = parse_platform_settings(text)?;
        let sprite_slices = parse_sprite_slices(text)?;
        Ok(TextureImportSettings {
            texture_type,
            srgb,
            max_size,
            mipmaps,
            sprite_mode,
            alpha_is_transparency,
            wrap_mode,
            filter_mode,
            platform_settings,
            sprite_slices,
        })
    }

    pub fn round_trip(&self) -> Result<TextureImportSettings, TextureImportError> {
        TextureImportSettings::from_meta_text(&self.to_meta_text())
    }

    pub fn build_mip_chain(&self, texture: &DecodedTexture) -> Vec<DecodedTexture> {
        self.build_mip_chain_for_target(texture, None)
    }

    pub fn build_mip_chain_for_target(
        &self,
        texture: &DecodedTexture,
        build_target: Option<&str>,
    ) -> Vec<DecodedTexture> {
        let limited = apply_max_size(texture, self.effective_max_size(build_target));
        if !self.mipmaps {
            return vec![limited];
        }
        generate_mip_chain(&limited)
    }
}

fn parse_platform_settings(text: &str) -> Result<Vec<TexturePlatformSettings>, TextureImportError> {
    let mut settings = Vec::new();
    let mut in_section = false;
    let mut current: Option<TexturePlatformSettings> = None;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == "platformSettings:" {
            in_section = true;
            continue;
        }
        if !in_section {
            continue;
        }
        if !line.starts_with("  ") && !trimmed.is_empty() {
            break;
        }
        if let Some(rest) = trimmed.strip_prefix("- ") {
            if let Some(finished) = current.take() {
                settings.push(finished);
            }
            let mut item = TexturePlatformSettings::default();
            apply_platform_kv(&mut item, rest)?;
            current = Some(item);
            continue;
        }
        if let Some(item) = current.as_mut() {
            if !trimmed.is_empty() {
                apply_platform_kv(item, trimmed)?;
            }
        }
    }
    if let Some(finished) = current {
        settings.push(finished);
    }
    Ok(settings)
}

fn apply_platform_kv(
    item: &mut TexturePlatformSettings,
    kv_line: &str,
) -> Result<(), TextureImportError> {
    let (key, raw_val) = match kv_line.split_once(':') {
        Some((k, v)) => (k.trim(), v.trim()),
        None => return Ok(()),
    };
    match key {
        "buildTarget" => {
            if raw_val.is_empty() {
                return Err(TextureImportError::InvalidMetaText(
                    "buildTarget".to_string(),
                ));
            }
            item.build_target = raw_val.to_string();
        }
        "maxTextureSize" => {
            item.max_texture_size = raw_val.parse::<u32>().map_err(|_| {
                TextureImportError::InvalidMetaText("platform maxTextureSize".to_string())
            })?;
        }
        "format" => {
            let code = raw_val
                .parse::<i32>()
                .map_err(|_| TextureImportError::InvalidMetaText("platform format".to_string()))?;
            item.format = TexturePlatformFormat::from_code(code).ok_or_else(|| {
                TextureImportError::InvalidMetaText(format!("platform format {code}"))
            })?;
        }
        "textureCompression" => {
            let code = raw_val.parse::<u8>().map_err(|_| {
                TextureImportError::InvalidMetaText("platform textureCompression".to_string())
            })?;
            item.texture_compression =
                TextureCompressionQuality::from_code(code).ok_or_else(|| {
                    TextureImportError::InvalidMetaText(format!(
                        "platform textureCompression {code}"
                    ))
                })?;
        }
        "compressionQuality" => {
            item.compression_quality = raw_val.parse::<u8>().map_err(|_| {
                TextureImportError::InvalidMetaText("platform compressionQuality".to_string())
            })?;
        }
        "crunchedCompression" => {
            item.crunched_compression = match raw_val {
                "0" => false,
                "1" => true,
                _ => {
                    return Err(TextureImportError::InvalidMetaText(
                        "platform crunchedCompression".to_string(),
                    ))
                }
            };
        }
        "overridden" => {
            item.overridden = match raw_val {
                "0" => false,
                "1" => true,
                _ => {
                    return Err(TextureImportError::InvalidMetaText(
                        "platform overridden".to_string(),
                    ))
                }
            };
        }
        _ => {}
    }
    Ok(())
}

fn parse_sprite_slices(text: &str) -> Result<Vec<SpriteSlice>, TextureImportError> {
    let mut slices: Vec<SpriteSlice> = Vec::new();
    let mut sprites_indent: Option<usize> = None;
    let mut current: Option<SpriteSlice> = None;
    let mut in_rect = false;

    for line in text.lines() {
        let trimmed = line.trim();
        let indent = line.len() - line.trim_start().len();
        match sprites_indent {
            None => {
                if trimmed == "sprites:" {
                    sprites_indent = Some(indent);
                }
                continue;
            }
            Some(list_indent) => {
                if trimmed.is_empty() {
                    continue;
                }
                if trimmed.starts_with('-') {
                    if indent != list_indent {
                        continue;
                    }
                    if let Some(slice) = current.take() {
                        slices.push(slice);
                    }
                    current = Some(SpriteSlice::new(String::new(), 0, 0, 0, 0));
                    in_rect = false;
                    continue;
                }
                if indent <= list_indent {
                    break;
                }
            }
        }
        let slice = match current.as_mut() {
            Some(slice) => slice,
            None => continue,
        };
        if trimmed == "rect:" {
            in_rect = true;
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("name:") {
            slice.name = value.trim().to_string();
            in_rect = false;
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("pivot:") {
            let (pivot_x, pivot_y) = parse_meta_pair(value)?;
            slice.pivot_x = pivot_x;
            slice.pivot_y = pivot_y;
            in_rect = false;
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("border:") {
            let (left, bottom, right, top) = parse_meta_quad(value)?;
            slice.border_left = left;
            slice.border_bottom = bottom;
            slice.border_right = right;
            slice.border_top = top;
            in_rect = false;
            continue;
        }
        if in_rect {
            if let Some(value) = trimmed.strip_prefix("x:") {
                slice.x = parse_meta_i32(value)?;
                continue;
            }
            if let Some(value) = trimmed.strip_prefix("y:") {
                slice.y = parse_meta_i32(value)?;
                continue;
            }
            if let Some(value) = trimmed.strip_prefix("width:") {
                slice.width = parse_meta_i32(value)?;
                continue;
            }
            if let Some(value) = trimmed.strip_prefix("height:") {
                slice.height = parse_meta_i32(value)?;
            }
        }
    }
    if let Some(slice) = current.take() {
        slices.push(slice);
    }
    Ok(slices)
}

fn parse_meta_pair(value: &str) -> Result<(f32, f32), TextureImportError> {
    let inner = value.trim().trim_start_matches('{').trim_end_matches('}');
    let mut x = None;
    let mut y = None;
    for part in inner.split(',') {
        let (key, raw) = match part.split_once(':') {
            Some(pair) => pair,
            None => continue,
        };
        let number = raw
            .trim()
            .parse::<f32>()
            .map_err(|_| TextureImportError::InvalidMetaText(format!("sprite {}", key.trim())))?;
        match key.trim() {
            "x" => x = Some(number),
            "y" => y = Some(number),
            _ => {}
        }
    }
    match (x, y) {
        (Some(x), Some(y)) => Ok((x, y)),
        _ => Err(TextureImportError::InvalidMetaText(
            "sprite pivot is incomplete".to_string(),
        )),
    }
}

fn parse_meta_quad(value: &str) -> Result<(f32, f32, f32, f32), TextureImportError> {
    let inner = value.trim().trim_start_matches('{').trim_end_matches('}');
    let mut values: Vec<(String, f32)> = Vec::new();
    for part in inner.split(',') {
        let (key, raw) = match part.split_once(':') {
            Some(pair) => pair,
            None => continue,
        };
        let number = raw
            .trim()
            .parse::<f32>()
            .map_err(|_| TextureImportError::InvalidMetaText(format!("sprite {}", key.trim())))?;
        values.push((key.trim().to_string(), number));
    }
    if values.len() != 4 {
        return Err(TextureImportError::InvalidMetaText(
            "sprite border is incomplete".to_string(),
        ));
    }
    Ok((values[0].1, values[1].1, values[2].1, values[3].1))
}

fn parse_meta_i32(value: &str) -> Result<i32, TextureImportError> {
    value
        .trim()
        .parse::<i32>()
        .map_err(|_| TextureImportError::InvalidMetaText("sprite rect".to_string()))
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
    if texture.format == TextureFormat::Rgba32F {
        return downscale_float(texture);
    }
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
                    for (channel, sum) in sums.iter_mut().enumerate().take(channels) {
                        *sum += u32::from(
                            texture
                                .pixels
                                .get(base + channel)
                                .copied()
                                .unwrap_or_default(),
                        );
                    }
                    count += 1;
                }
            }
            let target = (y * width as usize + x) * channels;
            for (channel, sum) in sums.iter().enumerate().take(channels) {
                pixels[target + channel] = ((*sum + count / 2) / count) as u8;
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

fn downscale_float(texture: &DecodedTexture) -> DecodedTexture {
    let width = (texture.width / 2).max(1);
    let height = (texture.height / 2).max(1);
    let mut pixels = vec![0u8; width as usize * height as usize * 16];
    for y in 0..height as usize {
        for x in 0..width as usize {
            let mut sums = [0.0f32; 4];
            let mut count = 0.0f32;
            for offset_y in 0..2 {
                for offset_x in 0..2 {
                    let source_x = (x * 2 + offset_x).min(texture.width as usize - 1);
                    let source_y = (y * 2 + offset_y).min(texture.height as usize - 1);
                    let base = (source_y * texture.width as usize + source_x) * 16;
                    for (channel, sum) in sums.iter_mut().enumerate() {
                        let start = base + channel * 4;
                        let mut raw = [0u8; 4];
                        if let Some(slice) = texture.pixels.get(start..start + 4) {
                            raw.copy_from_slice(slice);
                        }
                        *sum += f32::from_le_bytes(raw);
                    }
                    count += 1.0;
                }
            }
            let target = (y * width as usize + x) * 16;
            for (channel, sum) in sums.iter().enumerate() {
                let value = sum / count;
                let start = target + channel * 4;
                if let Some(slice) = pixels.get_mut(start..start + 4) {
                    slice.copy_from_slice(&value.to_le_bytes());
                }
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
    if is_pict_file(file_name) {
        return Err(TextureImportError::UnsupportedFormat(format!(
            "PICT ({file_name})"
        )));
    }
    if has_gif_signature(bytes) {
        return Ok(decode_gif_first_frame(bytes)?.0);
    }
    if has_jpeg_signature(bytes) {
        return decode_jpeg(bytes);
    }
    let lower = file_name.to_ascii_lowercase();
    if lower.ends_with(".gif") {
        return Ok(decode_gif_first_frame(bytes)?.0);
    }
    if lower.ends_with(".png") {
        return decode_png(bytes);
    }
    if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        return decode_jpeg(bytes);
    }
    if lower.ends_with(".tga") || lower.ends_with(".targa") {
        return decode_tga(bytes);
    }
    if lower.ends_with(".exr") {
        return decode_exr(bytes);
    }
    if has_psd_signature(bytes) || lower.ends_with(".psd") || lower.ends_with(".psb") {
        return decode_psd(bytes);
    }
    if lower.ends_with(".hdr") {
        return decode_hdr(bytes);
    }
    Err(TextureImportError::UnsupportedFormat(file_name.to_string()))
}

pub fn import_texture(
    bytes: &[u8],
    file_name: &str,
    settings: &TextureImportSettings,
) -> Result<TextureImportResult, TextureImportError> {
    import_texture_for_target(bytes, file_name, settings, None)
}

pub fn import_texture_for_target(
    bytes: &[u8],
    file_name: &str,
    settings: &TextureImportSettings,
    build_target: Option<&str>,
) -> Result<TextureImportResult, TextureImportError> {
    if is_pict_file(file_name) {
        return Err(TextureImportError::UnsupportedFormat(format!(
            "PICT ({file_name})"
        )));
    }
    let lower = file_name.to_ascii_lowercase();
    let (decoded, frame_count) = if has_gif_signature(bytes) || lower.ends_with(".gif") {
        decode_gif_first_frame(bytes)?
    } else {
        (decode_texture(bytes, file_name)?, 1)
    };
    let excluded_frames = frame_count.saturating_sub(1);
    let mut notes = Vec::new();
    if has_gif_signature(bytes) || lower.ends_with(".gif") {
        if excluded_frames > 0 {
            notes.push(format!(
                "{file_name}: GIF is Partially supported; imported first frame only ({excluded_frames} later frame(s) excluded)"
            ));
        } else {
            notes.push(format!(
                "{file_name}: GIF is Partially supported; imported first frame (later frames excluded)"
            ));
        }
    }
    let override_setting = build_target.and_then(|target| settings.platform_override(target));
    let (applied_target, applied_format, applied_compression) = match override_setting {
        Some(platform) => (
            Some(platform.build_target.clone()),
            Some(platform.format),
            Some(platform.texture_compression),
        ),
        None => (None, None, None),
    };
    let mips = settings.build_mip_chain_for_target(&decoded, build_target);
    let texture = mips
        .first()
        .cloned()
        .unwrap_or_else(|| apply_max_size(&decoded, settings.effective_max_size(build_target)));
    let sprites = match settings.sprite_mode {
        SpriteMode::Multiple => {
            if settings.sprite_slices.is_empty() {
                document_sprite_slices(bytes, &texture)?
            } else {
                settings.sprite_slices.clone()
            }
        }
        _ => Vec::new(),
    };
    let has_alpha = texture_has_alpha(&texture, settings);
    let encoded_format =
        resolve_encoded_format(applied_format, applied_compression, build_target, has_alpha);
    let compressed_mip_data = if encoded_format.is_compressed() {
        let mut encoded_mips = Vec::with_capacity(mips.len());
        for mip in &mips {
            let rgba = texture_to_rgba8(mip);
            if let Some(encoded) =
                encode_texture_blocks(&rgba, mip.width, mip.height, encoded_format)
            {
                encoded_mips.push(encoded);
            }
        }
        encoded_mips
    } else {
        Vec::new()
    };
    let compressed_data = compressed_mip_data.first().cloned();
    Ok(TextureImportResult {
        source: decoded,
        texture,
        mips,
        settings: settings.clone(),
        frame_count,
        excluded_frames,
        applied_target,
        applied_format,
        applied_compression,
        encoded_format,
        compressed_data,
        compressed_mip_data,
        sprites,
        notes,
    })
}

fn document_sprite_slices(
    bytes: &[u8],
    texture: &DecodedTexture,
) -> Result<Vec<SpriteSlice>, TextureImportError> {
    if !has_psd_signature(bytes) {
        return Ok(Vec::new());
    }
    let document = parse_psd_document(bytes)?;
    Ok(layer_sprite_slices(&document, texture))
}

fn layer_sprite_slices(document: &PsdDocument, texture: &DecodedTexture) -> Vec<SpriteSlice> {
    let scale_x = f64::from(texture.width) / f64::from(document.width.max(1));
    let scale_y = f64::from(texture.height) / f64::from(document.height.max(1));
    let mut slices = Vec::new();
    let mut names = HashSet::new();
    collect_layer_slices(
        &document.layers,
        document.height,
        scale_x,
        scale_y,
        &mut names,
        &mut slices,
    );
    slices
}

fn collect_layer_slices(
    layers: &[PsdLayer],
    document_height: u32,
    scale_x: f64,
    scale_y: f64,
    names: &mut HashSet<String>,
    slices: &mut Vec<SpriteSlice>,
) {
    for layer in layers {
        if !layer.children.is_empty() {
            collect_layer_slices(
                &layer.children,
                document_height,
                scale_x,
                scale_y,
                names,
                slices,
            );
            continue;
        }
        let width = layer.rect.width();
        let height = layer.rect.height();
        if width == 0 || height == 0 {
            continue;
        }
        let name = unique_sprite_name(&layer.name, names);
        let x = (f64::from(layer.rect.left) * scale_x).round() as i32;
        let y =
            ((f64::from(document_height) - f64::from(layer.rect.bottom)) * scale_y).round() as i32;
        let sprite_width = (f64::from(width) * scale_x).round() as i32;
        let sprite_height = (f64::from(height) * scale_y).round() as i32;
        slices.push(SpriteSlice::new(name, x, y, sprite_width, sprite_height));
    }
}

fn unique_sprite_name(name: &str, names: &mut HashSet<String>) -> String {
    if names.insert(name.to_string()) {
        return name.to_string();
    }
    let mut suffix = 2;
    loop {
        let candidate = format!("{name}#{suffix}");
        if names.insert(candidate.clone()) {
            return candidate;
        }
        suffix += 1;
    }
}

pub fn texture_import_outcome(
    path: &str,
    result: Result<&TextureImportResult, &TextureImportError>,
) -> ImportOutcome {
    match result {
        Ok(imported) => match imported.notes.first() {
            None => ImportOutcome::ready(),
            Some(note) => ImportOutcome::limited(
                ImportDiagnostic::new(
                    path,
                    note.clone(),
                    "the asset imports with the stated limitation; the excluded content stays unavailable",
                )
                .partially_usable(),
            ),
        },
        Err(error) => ImportOutcome::failed(ImportDiagnostic::new(
            path,
            error.to_string(),
            "the asset cannot be imported; the asset stays failed and its content is unavailable",
        )),
    }
}

fn is_pict_file(file_name: &str) -> bool {
    let lower = file_name.to_ascii_lowercase();
    lower.ends_with(".pict") || lower.ends_with(".pct") || lower.ends_with(".pic")
}

fn has_gif_signature(bytes: &[u8]) -> bool {
    bytes.len() >= 6 && (&bytes[0..6] == b"GIF87a" || &bytes[0..6] == b"GIF89a")
}

pub fn decode_gif_first_frame(bytes: &[u8]) -> Result<(DecodedTexture, usize), TextureImportError> {
    if bytes.len() < 6 {
        return Err(TextureImportError::Truncated);
    }
    if !has_gif_signature(bytes) {
        return Err(TextureImportError::InvalidSignature);
    }
    if bytes.len() < 13 {
        return Err(TextureImportError::Truncated);
    }
    let screen_width = u16::from_le_bytes([bytes[6], bytes[7]]);
    let screen_height = u16::from_le_bytes([bytes[8], bytes[9]]);
    if screen_width == 0 || screen_height == 0 {
        return Err(TextureImportError::CorruptData(
            "GIF screen dimensions cannot be zero".to_string(),
        ));
    }
    let packed = bytes[10];
    let has_gct = (packed & 0x80) != 0;
    let gct_size = 1usize << ((packed & 0x07) + 1);
    let mut cursor = 13usize;
    let global_palette = if has_gct {
        let byte_len = gct_size * 3;
        if cursor + byte_len > bytes.len() {
            return Err(TextureImportError::Truncated);
        }
        let slice = &bytes[cursor..cursor + byte_len];
        cursor += byte_len;
        Some(slice)
    } else {
        None
    };

    let mut transparent_index: Option<u8> = None;
    let mut first_texture: Option<DecodedTexture> = None;
    let mut frame_count = 0usize;

    while cursor < bytes.len() {
        let introducer = bytes[cursor];
        cursor += 1;
        match introducer {
            0x3B => break,
            0x21 => {
                if cursor >= bytes.len() {
                    return Err(TextureImportError::Truncated);
                }
                let label = bytes[cursor];
                cursor += 1;
                if label == 0xF9 {
                    let block = read_gif_sub_blocks(bytes, &mut cursor)?;
                    if block.len() >= 4 && (block[0] & 0x01) != 0 {
                        transparent_index = Some(block[3]);
                    }
                } else {
                    let _ = read_gif_sub_blocks(bytes, &mut cursor)?;
                }
            }
            0x2C => {
                if cursor + 9 > bytes.len() {
                    return Err(TextureImportError::Truncated);
                }
                let left = u16::from_le_bytes([bytes[cursor], bytes[cursor + 1]]);
                let top = u16::from_le_bytes([bytes[cursor + 2], bytes[cursor + 3]]);
                let width = u16::from_le_bytes([bytes[cursor + 4], bytes[cursor + 5]]);
                let height = u16::from_le_bytes([bytes[cursor + 6], bytes[cursor + 7]]);
                let frame_packed = bytes[cursor + 8];
                cursor += 9;

                if width == 0 || height == 0 {
                    return Err(TextureImportError::CorruptData(
                        "GIF frame dimensions cannot be zero".to_string(),
                    ));
                }

                let has_lct = (frame_packed & 0x80) != 0;
                let interlaced = (frame_packed & 0x40) != 0;
                let lct_size = 1usize << ((frame_packed & 0x07) + 1);
                let local_palette = if has_lct {
                    let byte_len = lct_size * 3;
                    if cursor + byte_len > bytes.len() {
                        return Err(TextureImportError::Truncated);
                    }
                    let slice = &bytes[cursor..cursor + byte_len];
                    cursor += byte_len;
                    Some(slice)
                } else {
                    None
                };

                if cursor >= bytes.len() {
                    return Err(TextureImportError::Truncated);
                }
                let min_code_size = bytes[cursor];
                cursor += 1;
                let compressed = read_gif_sub_blocks(bytes, &mut cursor)?;
                frame_count += 1;

                if first_texture.is_none() {
                    let palette = local_palette
                        .or(global_palette)
                        .ok_or(TextureImportError::MissingPalette)?;
                    let pixel_count = usize::from(width) * usize::from(height);
                    let indices = lzw_decode_gif(min_code_size, &compressed, pixel_count)?;
                    let ordered = if interlaced {
                        deinterlace_gif_indices(&indices, usize::from(width), usize::from(height))
                    } else {
                        indices
                    };
                    let canvas_w = usize::from(screen_width.max(left.saturating_add(width)));
                    let canvas_h = usize::from(screen_height.max(top.saturating_add(height)));
                    let mut rgba = vec![0u8; canvas_w * canvas_h * 4];
                    let palette_entries = palette.len() / 3;
                    for y in 0..usize::from(height) {
                        for x in 0..usize::from(width) {
                            let idx = ordered[y * usize::from(width) + x] as usize;
                            if idx >= palette_entries {
                                return Err(TextureImportError::PaletteIndexOutOfRange(idx));
                            }
                            let dst_x = usize::from(left) + x;
                            let dst_y = usize::from(top) + y;
                            let dst_offset = (dst_y * canvas_w + dst_x) * 4;
                            if transparent_index == Some(idx as u8) {
                                rgba[dst_offset] = palette[idx * 3];
                                rgba[dst_offset + 1] = palette[idx * 3 + 1];
                                rgba[dst_offset + 2] = palette[idx * 3 + 2];
                                rgba[dst_offset + 3] = 0;
                            } else {
                                rgba[dst_offset] = palette[idx * 3];
                                rgba[dst_offset + 1] = palette[idx * 3 + 1];
                                rgba[dst_offset + 2] = palette[idx * 3 + 2];
                                rgba[dst_offset + 3] = 255;
                            }
                        }
                    }
                    first_texture = Some(DecodedTexture {
                        width: canvas_w as u32,
                        height: canvas_h as u32,
                        format: TextureFormat::Rgba8,
                        pixels: rgba,
                    });
                }
                transparent_index = None;
            }
            _ => {
                return Err(TextureImportError::CorruptData(format!(
                    "unexpected GIF block introducer 0x{introducer:02X}"
                )));
            }
        }
    }

    match first_texture {
        Some(texture) => Ok((texture, frame_count)),
        None => Err(TextureImportError::CorruptData(
            "GIF contains no image frames".to_string(),
        )),
    }
}

fn read_gif_sub_blocks(bytes: &[u8], cursor: &mut usize) -> Result<Vec<u8>, TextureImportError> {
    let mut out = Vec::new();
    loop {
        if *cursor >= bytes.len() {
            return Err(TextureImportError::Truncated);
        }
        let block_len = usize::from(bytes[*cursor]);
        *cursor += 1;
        if block_len == 0 {
            break;
        }
        if *cursor + block_len > bytes.len() {
            return Err(TextureImportError::Truncated);
        }
        out.extend_from_slice(&bytes[*cursor..*cursor + block_len]);
        *cursor += block_len;
    }
    Ok(out)
}

fn deinterlace_gif_indices(indices: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut out = vec![0u8; width * height];
    let passes = [(0usize, 8usize), (4, 8), (2, 4), (1, 2)];
    let mut src_row = 0usize;
    for (start, step) in passes {
        let mut dst_row = start;
        while dst_row < height {
            if src_row < height {
                let src_start = src_row * width;
                let dst_start = dst_row * width;
                out[dst_start..dst_start + width]
                    .copy_from_slice(&indices[src_start..src_start + width]);
                src_row += 1;
            }
            dst_row += step;
        }
    }
    out
}

fn lzw_decode_gif(
    min_code_size: u8,
    compressed: &[u8],
    expected_pixels: usize,
) -> Result<Vec<u8>, TextureImportError> {
    if !(2..=8).contains(&min_code_size) {
        return Err(TextureImportError::CorruptData(format!(
            "invalid GIF LZW minimum code size {min_code_size}"
        )));
    }
    let clear_code = 1u16 << min_code_size;
    let end_code = clear_code + 1;
    let mut code_size = u32::from(min_code_size) + 1;
    let mut table: Vec<Vec<u8>> = Vec::with_capacity(4096);
    reset_lzw_table(&mut table, min_code_size);

    let mut bit_pos = 0usize;
    let total_bits = compressed.len() * 8;
    let mut prev_code: Option<u16> = None;
    let mut output = Vec::with_capacity(expected_pixels);

    while output.len() < expected_pixels {
        let code_bits = code_size as usize;
        if bit_pos + code_bits > total_bits {
            return Err(TextureImportError::Truncated);
        }
        let mut code = 0u16;
        for bit_offset in 0..code_bits {
            let abs_bit = bit_pos + bit_offset;
            let byte = compressed[abs_bit / 8];
            let bit = (byte >> (abs_bit % 8)) & 1;
            code |= u16::from(bit) << bit_offset;
        }
        bit_pos += code_bits;

        if code == clear_code {
            reset_lzw_table(&mut table, min_code_size);
            code_size = u32::from(min_code_size) + 1;
            prev_code = None;
            continue;
        }
        if code == end_code {
            break;
        }

        let entry = if (code as usize) < table.len() {
            table[code as usize].clone()
        } else if let Some(prev) = prev_code {
            if (code as usize) == table.len() && (prev as usize) < table.len() {
                let mut seq = table[prev as usize].clone();
                if let Some(&first_byte) = seq.first() {
                    seq.push(first_byte);
                }
                seq
            } else {
                return Err(TextureImportError::CorruptData(format!(
                    "out-of-range GIF LZW code {code}"
                )));
            }
        } else {
            return Err(TextureImportError::CorruptData(format!(
                "invalid initial GIF LZW code {code}"
            )));
        };

        output.extend_from_slice(&entry);

        if let Some(prev) = prev_code {
            if table.len() < 4096 && (prev as usize) < table.len() {
                if let Some(&first_byte) = entry.first() {
                    let mut new_seq = table[prev as usize].clone();
                    new_seq.push(first_byte);
                    table.push(new_seq);
                    if table.len() == (1usize << code_size) && code_size < 12 {
                        code_size += 1;
                    }
                }
            }
        }
        prev_code = Some(code);
    }

    if output.len() < expected_pixels {
        return Err(TextureImportError::Truncated);
    }
    output.truncate(expected_pixels);
    Ok(output)
}

fn reset_lzw_table(table: &mut Vec<Vec<u8>>, min_code_size: u8) {
    table.clear();
    let clear_code = 1u16 << min_code_size;
    for index in 0..clear_code {
        table.push(vec![index as u8]);
    }
    table.push(Vec::new());
    table.push(Vec::new());
}

fn texture_has_alpha(texture: &DecodedTexture, settings: &TextureImportSettings) -> bool {
    if !settings.alpha_is_transparency {
        return false;
    }
    match texture.format {
        TextureFormat::Rgba8 => texture
            .pixels
            .chunks_exact(4)
            .any(|px| px.get(3).copied().unwrap_or(255) < 255),
        TextureFormat::Rgba32F => true,
        _ => false,
    }
}

fn resolve_encoded_format(
    applied_format: Option<TexturePlatformFormat>,
    applied_compression: Option<TextureCompressionQuality>,
    build_target: Option<&str>,
    has_alpha: bool,
) -> TexturePlatformFormat {
    match applied_format {
        Some(TexturePlatformFormat::Automatic) => {
            let quality = applied_compression.unwrap_or(TextureCompressionQuality::Normal);
            if quality == TextureCompressionQuality::Uncompressed {
                return TexturePlatformFormat::Uncompressed;
            }
            resolve_automatic_platform_format(build_target, quality, has_alpha)
        }
        Some(format) => format,
        None => TexturePlatformFormat::Automatic,
    }
}

fn resolve_automatic_platform_format(
    build_target: Option<&str>,
    quality: TextureCompressionQuality,
    has_alpha: bool,
) -> TexturePlatformFormat {
    let Some(target) = build_target else {
        return TexturePlatformFormat::Automatic;
    };
    match target {
        "Standalone"
        | "StandaloneOSX"
        | "StandaloneWindows"
        | "StandaloneWindows64"
        | "StandaloneLinux64"
        | "WebGL" => match quality {
            TextureCompressionQuality::Uncompressed => TexturePlatformFormat::Uncompressed,
            TextureCompressionQuality::HighQuality => TexturePlatformFormat::Bc7,
            TextureCompressionQuality::Normal | TextureCompressionQuality::LowQuality => {
                if has_alpha {
                    TexturePlatformFormat::Dxt5
                } else {
                    TexturePlatformFormat::Dxt1
                }
            }
        },
        "iPhone" | "iOS" | "Android" | "tvOS" => match quality {
            TextureCompressionQuality::Uncompressed => TexturePlatformFormat::Uncompressed,
            TextureCompressionQuality::HighQuality => TexturePlatformFormat::Astc4x4,
            TextureCompressionQuality::Normal => TexturePlatformFormat::Astc6x6,
            TextureCompressionQuality::LowQuality => TexturePlatformFormat::Astc8x8,
        },
        _ => TexturePlatformFormat::Automatic,
    }
}

pub fn texture_to_rgba8(texture: &DecodedTexture) -> Vec<u8> {
    let width = texture.width as usize;
    let height = texture.height as usize;
    let total = width * height;
    let mut out = vec![0u8; total * 4];
    match texture.format {
        TextureFormat::Rgba8 => {
            let copy_len = out.len().min(texture.pixels.len());
            out[..copy_len].copy_from_slice(&texture.pixels[..copy_len]);
        }
        TextureFormat::Rgb8 => {
            for idx in 0..total {
                let src = idx * 3;
                let dst = idx * 4;
                out[dst] = texture.pixels.get(src).copied().unwrap_or(0);
                out[dst + 1] = texture.pixels.get(src + 1).copied().unwrap_or(0);
                out[dst + 2] = texture.pixels.get(src + 2).copied().unwrap_or(0);
                out[dst + 3] = 255;
            }
        }
        TextureFormat::Rg8 => {
            for idx in 0..total {
                let src = idx * 2;
                let dst = idx * 4;
                out[dst] = texture.pixels.get(src).copied().unwrap_or(0);
                out[dst + 1] = texture.pixels.get(src + 1).copied().unwrap_or(0);
                out[dst + 2] = 0;
                out[dst + 3] = 255;
            }
        }
        TextureFormat::R8 => {
            for idx in 0..total {
                let v = texture.pixels.get(idx).copied().unwrap_or(0);
                let dst = idx * 4;
                out[dst] = v;
                out[dst + 1] = v;
                out[dst + 2] = v;
                out[dst + 3] = 255;
            }
        }
        TextureFormat::Rgba32F => {
            for y in 0..texture.height {
                for x in 0..texture.width {
                    let f = texture.pixel_f32(x, y).unwrap_or([0.0, 0.0, 0.0, 1.0]);
                    let dst = ((y as usize) * width + (x as usize)) * 4;
                    out[dst] = (f[0].clamp(0.0, 1.0) * 255.0).round() as u8;
                    out[dst + 1] = (f[1].clamp(0.0, 1.0) * 255.0).round() as u8;
                    out[dst + 2] = (f[2].clamp(0.0, 1.0) * 255.0).round() as u8;
                    out[dst + 3] = (f[3].clamp(0.0, 1.0) * 255.0).round() as u8;
                }
            }
        }
    }
    out
}

pub fn encode_texture_blocks(
    rgba: &[u8],
    width: u32,
    height: u32,
    format: TexturePlatformFormat,
) -> Option<Vec<u8>> {
    if width == 0 || height == 0 {
        return None;
    }
    let expected_len = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;
    if rgba.len() < expected_len {
        return None;
    }
    match format {
        TexturePlatformFormat::Dxt1 => {
            Some(encode_blocks_4x4(rgba, width, height, 8, |block, out| {
                let encoded = encode_bc1_block(block);
                out.copy_from_slice(&encoded);
            }))
        }
        TexturePlatformFormat::Dxt5 => {
            Some(encode_blocks_4x4(rgba, width, height, 16, |block, out| {
                let alpha = encode_bc3_alpha_block(block);
                let color = encode_bc1_block(block);
                out[0..8].copy_from_slice(&alpha);
                out[8..16].copy_from_slice(&color);
            }))
        }
        TexturePlatformFormat::Bc7 => {
            Some(encode_blocks_4x4(rgba, width, height, 16, |block, out| {
                let encoded = encode_bc7_mode6_block(block);
                out.copy_from_slice(&encoded);
            }))
        }
        TexturePlatformFormat::Etc2Rgb4 => {
            Some(encode_blocks_4x4(rgba, width, height, 8, |block, out| {
                let encoded = encode_etc2_rgb_block(block);
                out.copy_from_slice(&encoded);
            }))
        }
        TexturePlatformFormat::Etc2Rgba8 => {
            Some(encode_blocks_4x4(rgba, width, height, 16, |block, out| {
                let alpha = encode_eac_alpha_block(block);
                let color = encode_etc2_rgb_block(block);
                out[0..8].copy_from_slice(&alpha);
                out[8..16].copy_from_slice(&color);
            }))
        }
        TexturePlatformFormat::Astc4x4 => Some(encode_astc_image(rgba, width, height, 4, 4)),
        TexturePlatformFormat::Astc6x6 => Some(encode_astc_image(rgba, width, height, 6, 6)),
        TexturePlatformFormat::Astc8x8 => Some(encode_astc_image(rgba, width, height, 8, 8)),
        TexturePlatformFormat::Automatic
        | TexturePlatformFormat::Uncompressed
        | TexturePlatformFormat::Rgba32
        | TexturePlatformFormat::Rgb24 => None,
    }
}

pub fn decode_texture_blocks(
    encoded: &[u8],
    width: u32,
    height: u32,
    format: TexturePlatformFormat,
) -> Option<Vec<u8>> {
    if width == 0 || height == 0 {
        return None;
    }
    match format {
        TexturePlatformFormat::Dxt1 => decode_blocks_4x4(encoded, width, height, 8, |slice| {
            let mut raw = [0u8; 8];
            raw.copy_from_slice(slice);
            decode_bc1_block(&raw)
        }),
        TexturePlatformFormat::Dxt5 => decode_blocks_4x4(encoded, width, height, 16, |slice| {
            let mut alpha_raw = [0u8; 8];
            let mut color_raw = [0u8; 8];
            alpha_raw.copy_from_slice(&slice[0..8]);
            color_raw.copy_from_slice(&slice[8..16]);
            let alphas = decode_bc3_alpha_block(&alpha_raw);
            let mut colors = decode_bc1_block(&color_raw);
            for i in 0..16 {
                colors[i][3] = alphas[i];
            }
            colors
        }),
        TexturePlatformFormat::Bc7 => decode_blocks_4x4(encoded, width, height, 16, |slice| {
            let mut raw = [0u8; 16];
            raw.copy_from_slice(slice);
            decode_bc7_mode6_block(&raw)
        }),
        TexturePlatformFormat::Etc2Rgb4 => decode_blocks_4x4(encoded, width, height, 8, |slice| {
            let mut raw = [0u8; 8];
            raw.copy_from_slice(slice);
            decode_etc2_rgb_block(&raw)
        }),
        TexturePlatformFormat::Etc2Rgba8 => {
            decode_blocks_4x4(encoded, width, height, 16, |slice| {
                let mut alpha_raw = [0u8; 8];
                let mut color_raw = [0u8; 8];
                alpha_raw.copy_from_slice(&slice[0..8]);
                color_raw.copy_from_slice(&slice[8..16]);
                let alphas = decode_eac_alpha_block(&alpha_raw);
                let mut colors = decode_etc2_rgb_block(&color_raw);
                for i in 0..16 {
                    colors[i][3] = alphas[i];
                }
                colors
            })
        }
        TexturePlatformFormat::Astc4x4 => decode_astc_image(encoded, width, height, 4, 4),
        TexturePlatformFormat::Astc6x6 => decode_astc_image(encoded, width, height, 6, 6),
        TexturePlatformFormat::Astc8x8 => decode_astc_image(encoded, width, height, 8, 8),
        TexturePlatformFormat::Automatic
        | TexturePlatformFormat::Uncompressed
        | TexturePlatformFormat::Rgba32
        | TexturePlatformFormat::Rgb24 => None,
    }
}

fn gather_block_4x4(rgba: &[u8], width: u32, height: u32, bx: u32, by: u32) -> [[u8; 4]; 16] {
    let mut block = [[0u8; 4]; 16];
    let w = width as usize;
    let h = height as usize;
    for py in 0..4usize {
        let sy = ((by as usize) * 4 + py).min(h - 1);
        for px in 0..4usize {
            let sx = ((bx as usize) * 4 + px).min(w - 1);
            let offset = (sy * w + sx) * 4;
            block[py * 4 + px] = [
                rgba[offset],
                rgba[offset + 1],
                rgba[offset + 2],
                rgba[offset + 3],
            ];
        }
    }
    block
}

fn encode_blocks_4x4<F>(
    rgba: &[u8],
    width: u32,
    height: u32,
    block_bytes: usize,
    mut encode_fn: F,
) -> Vec<u8>
where
    F: FnMut(&[[u8; 4]; 16], &mut [u8]),
{
    let blocks_x = width.div_ceil(4);
    let blocks_y = height.div_ceil(4);
    let total_blocks = (blocks_x as usize) * (blocks_y as usize);
    let mut out = vec![0u8; total_blocks * block_bytes];
    for by in 0..blocks_y {
        for bx in 0..blocks_x {
            let block = gather_block_4x4(rgba, width, height, bx, by);
            let block_index = ((by * blocks_x + bx) as usize) * block_bytes;
            encode_fn(&block, &mut out[block_index..block_index + block_bytes]);
        }
    }
    out
}

fn decode_blocks_4x4<F>(
    encoded: &[u8],
    width: u32,
    height: u32,
    block_bytes: usize,
    mut decode_fn: F,
) -> Option<Vec<u8>>
where
    F: FnMut(&[u8]) -> [[u8; 4]; 16],
{
    let blocks_x = width.div_ceil(4);
    let blocks_y = height.div_ceil(4);
    let total_blocks = (blocks_x as usize).checked_mul(blocks_y as usize)?;
    let expected_bytes = total_blocks.checked_mul(block_bytes)?;
    if encoded.len() != expected_bytes {
        return None;
    }
    let w = width as usize;
    let h = height as usize;
    let mut rgba = vec![0u8; w * h * 4];
    for by in 0..blocks_y {
        for bx in 0..blocks_x {
            let block_index = ((by * blocks_x + bx) as usize) * block_bytes;
            let decoded = decode_fn(&encoded[block_index..block_index + block_bytes]);
            for py in 0..4usize {
                let dy = (by as usize) * 4 + py;
                if dy >= h {
                    continue;
                }
                for px in 0..4usize {
                    let dx = (bx as usize) * 4 + px;
                    if dx >= w {
                        continue;
                    }
                    let dst = (dy * w + dx) * 4;
                    rgba[dst..dst + 4].copy_from_slice(&decoded[py * 4 + px]);
                }
            }
        }
    }
    Some(rgba)
}

fn pack_rgb565(r: u8, g: u8, b: u8) -> u16 {
    let r5 = (u16::from(r) * 31 + 127) / 255;
    let g6 = (u16::from(g) * 63 + 127) / 255;
    let b5 = (u16::from(b) * 31 + 127) / 255;
    (r5 << 11) | (g6 << 5) | b5
}

fn unpack_rgb565(color: u16) -> [u8; 3] {
    let r5 = ((color >> 11) & 0x1F) as u8;
    let g6 = ((color >> 5) & 0x3F) as u8;
    let b5 = (color & 0x1F) as u8;
    [
        (r5 << 3) | (r5 >> 2),
        (g6 << 2) | (g6 >> 4),
        (b5 << 3) | (b5 >> 2),
    ]
}

fn bc1_palette(c0: u16, c1: u16) -> [[u8; 4]; 4] {
    let rgb0 = unpack_rgb565(c0);
    let rgb1 = unpack_rgb565(c1);
    if c0 > c1 {
        let rgb2 = [
            ((2 * u16::from(rgb0[0]) + u16::from(rgb1[0]) + 1) / 3) as u8,
            ((2 * u16::from(rgb0[1]) + u16::from(rgb1[1]) + 1) / 3) as u8,
            ((2 * u16::from(rgb0[2]) + u16::from(rgb1[2]) + 1) / 3) as u8,
        ];
        let rgb3 = [
            ((u16::from(rgb0[0]) + 2 * u16::from(rgb1[0]) + 1) / 3) as u8,
            ((u16::from(rgb0[1]) + 2 * u16::from(rgb1[1]) + 1) / 3) as u8,
            ((u16::from(rgb0[2]) + 2 * u16::from(rgb1[2]) + 1) / 3) as u8,
        ];
        [
            [rgb0[0], rgb0[1], rgb0[2], 255],
            [rgb1[0], rgb1[1], rgb1[2], 255],
            [rgb2[0], rgb2[1], rgb2[2], 255],
            [rgb3[0], rgb3[1], rgb3[2], 255],
        ]
    } else {
        let rgb2 = [
            ((u16::from(rgb0[0]) + u16::from(rgb1[0])) / 2) as u8,
            ((u16::from(rgb0[1]) + u16::from(rgb1[1])) / 2) as u8,
            ((u16::from(rgb0[2]) + u16::from(rgb1[2])) / 2) as u8,
        ];
        [
            [rgb0[0], rgb0[1], rgb0[2], 255],
            [rgb1[0], rgb1[1], rgb1[2], 255],
            [rgb2[0], rgb2[1], rgb2[2], 255],
            [0, 0, 0, 255],
        ]
    }
}

fn encode_bc1_block(block: &[[u8; 4]; 16]) -> [u8; 8] {
    let mut min_rgb = [255u8; 3];
    let mut max_rgb = [0u8; 3];
    for px in block {
        for c in 0..3 {
            min_rgb[c] = min_rgb[c].min(px[c]);
            max_rgb[c] = max_rgb[c].max(px[c]);
        }
    }
    let mut c0 = pack_rgb565(max_rgb[0], max_rgb[1], max_rgb[2]);
    let mut c1 = pack_rgb565(min_rgb[0], min_rgb[1], min_rgb[2]);
    if c0 < c1 {
        std::mem::swap(&mut c0, &mut c1);
    }
    let palette = bc1_palette(c0, c1);
    let max_idx = if c0 == c1 { 1 } else { 4 };
    let mut indices = 0u32;
    for (i, px) in block.iter().enumerate() {
        let mut best_idx = 0u32;
        let mut best_dist = u32::MAX;
        for (p_idx, pal) in palette.iter().enumerate().take(max_idx) {
            let dr = i32::from(px[0]) - i32::from(pal[0]);
            let dg = i32::from(px[1]) - i32::from(pal[1]);
            let db = i32::from(px[2]) - i32::from(pal[2]);
            let dist = (dr * dr + dg * dg + db * db) as u32;
            if dist < best_dist {
                best_dist = dist;
                best_idx = p_idx as u32;
            }
        }
        indices |= (best_idx & 0x03) << (i * 2);
    }
    let mut out = [0u8; 8];
    out[0..2].copy_from_slice(&c0.to_le_bytes());
    out[2..4].copy_from_slice(&c1.to_le_bytes());
    out[4..8].copy_from_slice(&indices.to_le_bytes());
    out
}

fn decode_bc1_block(raw: &[u8; 8]) -> [[u8; 4]; 16] {
    let c0 = u16::from_le_bytes([raw[0], raw[1]]);
    let c1 = u16::from_le_bytes([raw[2], raw[3]]);
    let indices = u32::from_le_bytes([raw[4], raw[5], raw[6], raw[7]]);
    let palette = bc1_palette(c0, c1);
    let mut out = [[0u8; 4]; 16];
    for (i, slot) in out.iter_mut().enumerate() {
        let idx = ((indices >> (i * 2)) & 0x03) as usize;
        *slot = palette[idx];
    }
    out
}

fn bc3_alpha_palette(a0: u8, a1: u8) -> [u8; 8] {
    let mut pal = [0u8; 8];
    pal[0] = a0;
    pal[1] = a1;
    if a0 > a1 {
        for i in 1..7u16 {
            pal[(i + 1) as usize] = (((7 - i) * u16::from(a0) + i * u16::from(a1) + 3) / 7) as u8;
        }
    } else {
        for i in 1..5u16 {
            pal[(i + 1) as usize] = (((5 - i) * u16::from(a0) + i * u16::from(a1) + 2) / 5) as u8;
        }
        pal[6] = 0;
        pal[7] = 255;
    }
    pal
}

fn encode_bc3_alpha_block(block: &[[u8; 4]; 16]) -> [u8; 8] {
    let mut min_a = 255u8;
    let mut max_a = 0u8;
    for px in block {
        min_a = min_a.min(px[3]);
        max_a = max_a.max(px[3]);
    }
    let a0 = max_a;
    let a1 = min_a;
    let palette = bc3_alpha_palette(a0, a1);
    let max_idx = if a0 == a1 { 1 } else { 8 };
    let mut bits = 0u64;
    for (i, px) in block.iter().enumerate() {
        let mut best_idx = 0u64;
        let mut best_dist = u8::MAX;
        for (p_idx, &pal_a) in palette.iter().enumerate().take(max_idx) {
            let dist = px[3].abs_diff(pal_a);
            if dist < best_dist {
                best_dist = dist;
                best_idx = p_idx as u64;
            }
        }
        bits |= (best_idx & 0x07) << (i * 3);
    }
    let mut out = [0u8; 8];
    out[0] = a0;
    out[1] = a1;
    let bit_bytes = bits.to_le_bytes();
    out[2..8].copy_from_slice(&bit_bytes[0..6]);
    out
}

fn decode_bc3_alpha_block(raw: &[u8; 8]) -> [u8; 16] {
    let a0 = raw[0];
    let a1 = raw[1];
    let palette = bc3_alpha_palette(a0, a1);
    let mut bit_bytes = [0u8; 8];
    bit_bytes[0..6].copy_from_slice(&raw[2..8]);
    let bits = u64::from_le_bytes(bit_bytes);
    let mut out = [0u8; 16];
    for (i, slot) in out.iter_mut().enumerate() {
        let idx = ((bits >> (i * 3)) & 0x07) as usize;
        *slot = palette[idx];
    }
    out
}

const BC7_WEIGHTS_4: [u16; 16] = [0, 4, 9, 13, 17, 21, 26, 30, 34, 38, 43, 47, 51, 55, 60, 64];

fn write_bits_u128(word: &mut u128, bit_pos: &mut u32, value: u32, num_bits: u32) {
    let mask = if num_bits == 32 {
        u128::from(u32::MAX)
    } else {
        (1u128 << num_bits) - 1
    };
    *word |= (u128::from(value) & mask) << *bit_pos;
    *bit_pos += num_bits;
}

fn read_bits_u128(word: u128, bit_pos: &mut u32, num_bits: u32) -> u32 {
    let mask = if num_bits == 32 {
        u128::from(u32::MAX)
    } else {
        (1u128 << num_bits) - 1
    };
    let val = ((word >> *bit_pos) & mask) as u32;
    *bit_pos += num_bits;
    val
}

fn bc7_interpolate(e0: u8, e1: u8, weight_idx: usize) -> u8 {
    let w = BC7_WEIGHTS_4[weight_idx & 0x0F];
    (((64 - w) * u16::from(e0) + w * u16::from(e1) + 32) >> 6) as u8
}

fn encode_bc7_mode6_block(block: &[[u8; 4]; 16]) -> [u8; 16] {
    let mut min_c = [255u8; 4];
    let mut max_c = [0u8; 4];
    for px in block {
        for c in 0..4 {
            min_c[c] = min_c[c].min(px[c]);
            max_c[c] = max_c[c].max(px[c]);
        }
    }
    let mut e0 = min_c;
    let mut e1 = max_c;
    let mut weights = [0u8; 16];
    for (i, px) in block.iter().enumerate() {
        let mut best_w = 0u8;
        let mut best_err = u32::MAX;
        for w_idx in 0..16 {
            let mut err = 0u32;
            for c in 0..4 {
                let interp = bc7_interpolate(e0[c], e1[c], w_idx);
                let diff = i32::from(px[c]) - i32::from(interp);
                err += (diff * diff) as u32;
            }
            if err < best_err {
                best_err = err;
                best_w = w_idx as u8;
            }
        }
        weights[i] = best_w;
    }
    if (weights[0] & 0x08) != 0 {
        std::mem::swap(&mut e0, &mut e1);
        for w in &mut weights {
            *w = 15 - *w;
        }
    }
    let p0 = e0[0] & 1;
    let p1 = e1[0] & 1;
    let mut word = 0u128;
    let mut bit_pos = 0u32;
    write_bits_u128(&mut word, &mut bit_pos, 0b0100_0000, 7);
    for c in 0..4 {
        write_bits_u128(&mut word, &mut bit_pos, u32::from(e0[c] >> 1), 7);
        write_bits_u128(&mut word, &mut bit_pos, u32::from(e1[c] >> 1), 7);
    }
    write_bits_u128(&mut word, &mut bit_pos, u32::from(p0), 1);
    write_bits_u128(&mut word, &mut bit_pos, u32::from(p1), 1);
    write_bits_u128(&mut word, &mut bit_pos, u32::from(weights[0] & 0x07), 3);
    for &w in weights.iter().skip(1) {
        write_bits_u128(&mut word, &mut bit_pos, u32::from(w & 0x0F), 4);
    }
    word.to_le_bytes()
}

fn decode_bc7_mode6_block(raw: &[u8; 16]) -> [[u8; 4]; 16] {
    let word = u128::from_le_bytes(*raw);
    let mut bit_pos = 7u32;
    let mut e0_7 = [0u8; 4];
    let mut e1_7 = [0u8; 4];
    for c in 0..4 {
        e0_7[c] = read_bits_u128(word, &mut bit_pos, 7) as u8;
        e1_7[c] = read_bits_u128(word, &mut bit_pos, 7) as u8;
    }
    let p0 = read_bits_u128(word, &mut bit_pos, 1) as u8;
    let p1 = read_bits_u128(word, &mut bit_pos, 1) as u8;
    let mut e0 = [0u8; 4];
    let mut e1 = [0u8; 4];
    for c in 0..4 {
        e0[c] = (e0_7[c] << 1) | p0;
        e1[c] = (e1_7[c] << 1) | p1;
    }
    let mut weights = [0usize; 16];
    weights[0] = read_bits_u128(word, &mut bit_pos, 3) as usize;
    for slot in weights.iter_mut().skip(1) {
        *slot = read_bits_u128(word, &mut bit_pos, 4) as usize;
    }
    let mut out = [[0u8; 4]; 16];
    for i in 0..16 {
        for c in 0..4 {
            out[i][c] = bc7_interpolate(e0[c], e1[c], weights[i]);
        }
    }
    out
}

const ETC2_MODIFIER_TABLES: [[i16; 4]; 8] = [
    [2, 8, -2, -8],
    [5, 17, -5, -17],
    [9, 29, -9, -29],
    [13, 42, -13, -42],
    [18, 60, -18, -60],
    [24, 80, -24, -80],
    [33, 106, -33, -106],
    [47, 183, -47, -183],
];

fn expand_4to8(v4: u8) -> u8 {
    let nib = v4 & 0x0F;
    (nib << 4) | nib
}

fn quantize_8to4(v8: u8) -> u8 {
    ((u16::from(v8) * 15 + 135) >> 8) as u8
}

fn encode_etc2_subblock(pixels: &[[u8; 4]; 8]) -> ([u8; 3], u8, [u8; 8]) {
    let mut sum = [0u32; 3];
    for px in pixels {
        sum[0] += u32::from(px[0]);
        sum[1] += u32::from(px[1]);
        sum[2] += u32::from(px[2]);
    }
    let base4 = [
        quantize_8to4(((sum[0] + 4) / 8) as u8),
        quantize_8to4(((sum[1] + 4) / 8) as u8),
        quantize_8to4(((sum[2] + 4) / 8) as u8),
    ];
    let base8 = [
        expand_4to8(base4[0]),
        expand_4to8(base4[1]),
        expand_4to8(base4[2]),
    ];
    let mut best_table = 0u8;
    let mut best_codes = [0u8; 8];
    let mut best_total_err = u32::MAX;
    for (t_idx, table) in ETC2_MODIFIER_TABLES.iter().enumerate() {
        let mut codes = [0u8; 8];
        let mut total_err = 0u32;
        for (p_idx, px) in pixels.iter().enumerate() {
            let mut best_code = 0u8;
            let mut best_px_err = u32::MAX;
            for (code, &delta) in table.iter().enumerate() {
                let r = (i16::from(base8[0]) + delta).clamp(0, 255) as i32;
                let g = (i16::from(base8[1]) + delta).clamp(0, 255) as i32;
                let b = (i16::from(base8[2]) + delta).clamp(0, 255) as i32;
                let dr = i32::from(px[0]) - r;
                let dg = i32::from(px[1]) - g;
                let db = i32::from(px[2]) - b;
                let err = (dr * dr + dg * dg + db * db) as u32;
                if err < best_px_err {
                    best_px_err = err;
                    best_code = code as u8;
                }
            }
            codes[p_idx] = best_code;
            total_err += best_px_err;
        }
        if total_err < best_total_err {
            best_total_err = total_err;
            best_table = t_idx as u8;
            best_codes = codes;
        }
    }
    (base4, best_table, best_codes)
}

fn encode_etc2_rgb_block(block: &[[u8; 4]; 16]) -> [u8; 8] {
    let mut sub0 = [[0u8; 4]; 8];
    let mut sub1 = [[0u8; 4]; 8];
    for x in 0..2usize {
        for y in 0..4usize {
            sub0[x * 4 + y] = block[y * 4 + x];
            sub1[x * 4 + y] = block[y * 4 + (x + 2)];
        }
    }
    let (base0, table0, codes0) = encode_etc2_subblock(&sub0);
    let (base1, table1, codes1) = encode_etc2_subblock(&sub1);
    let mut msb = 0u16;
    let mut lsb = 0u16;
    for x in 0..4usize {
        for y in 0..4usize {
            let bit_idx = x * 4 + y;
            let code = if x < 2 {
                codes0[x * 4 + y]
            } else {
                codes1[(x - 2) * 4 + y]
            };
            msb |= u16::from((code >> 1) & 1) << bit_idx;
            lsb |= u16::from(code & 1) << bit_idx;
        }
    }
    let mut out = [0u8; 8];
    out[0] = (base0[0] << 4) | (base1[0] & 0x0F);
    out[1] = (base0[1] << 4) | (base1[1] & 0x0F);
    out[2] = (base0[2] << 4) | (base1[2] & 0x0F);
    out[3] = ((table0 & 0x07) << 5) | ((table1 & 0x07) << 2);
    out[4..6].copy_from_slice(&msb.to_be_bytes());
    out[6..8].copy_from_slice(&lsb.to_be_bytes());
    out
}

fn decode_etc2_rgb_block(raw: &[u8; 8]) -> [[u8; 4]; 16] {
    let diff = (raw[3] & 0x02) != 0;
    let flip = (raw[3] & 0x01) != 0;
    let table0 = ((raw[3] >> 5) & 0x07) as usize;
    let table1 = ((raw[3] >> 2) & 0x07) as usize;
    let (base0, base1) = if !diff {
        (
            [
                expand_4to8(raw[0] >> 4),
                expand_4to8(raw[1] >> 4),
                expand_4to8(raw[2] >> 4),
            ],
            [
                expand_4to8(raw[0] & 0x0F),
                expand_4to8(raw[1] & 0x0F),
                expand_4to8(raw[2] & 0x0F),
            ],
        )
    } else {
        let mut b0 = [0u8; 3];
        let mut b1 = [0u8; 3];
        for c in 0..3 {
            let v5 = i16::from((raw[c] >> 3) & 0x1F);
            let d_raw = i16::from(raw[c] & 0x07);
            let d3 = if d_raw >= 4 { d_raw - 8 } else { d_raw };
            let v5_1 = (v5 + d3).clamp(0, 31) as u8;
            let v5_0 = v5 as u8;
            b0[c] = (v5_0 << 3) | (v5_0 >> 2);
            b1[c] = (v5_1 << 3) | (v5_1 >> 2);
        }
        (b0, b1)
    };
    let msb = u16::from_be_bytes([raw[4], raw[5]]);
    let lsb = u16::from_be_bytes([raw[6], raw[7]]);
    let mut out = [[0u8; 4]; 16];
    for x in 0..4usize {
        for y in 0..4usize {
            let bit_idx = x * 4 + y;
            let code = (((msb >> bit_idx) & 1) << 1) | ((lsb >> bit_idx) & 1);
            let second_sub = if flip { y >= 2 } else { x >= 2 };
            let (base, table_idx) = if second_sub {
                (base1, table1)
            } else {
                (base0, table0)
            };
            let delta = ETC2_MODIFIER_TABLES[table_idx][code as usize];
            out[y * 4 + x] = [
                (i16::from(base[0]) + delta).clamp(0, 255) as u8,
                (i16::from(base[1]) + delta).clamp(0, 255) as u8,
                (i16::from(base[2]) + delta).clamp(0, 255) as u8,
                255,
            ];
        }
    }
    out
}

const EAC_ALPHA_TABLES: [[i16; 8]; 16] = [
    [-3, -6, -9, -15, 2, 5, 8, 14],
    [-3, -7, -10, -13, 2, 6, 9, 12],
    [-2, -5, -8, -13, 1, 4, 7, 12],
    [-2, -4, -6, -13, 1, 3, 5, 12],
    [-3, -6, -8, -12, 2, 5, 7, 11],
    [-3, -7, -9, -11, 2, 6, 8, 10],
    [-4, -7, -8, -11, 3, 6, 7, 10],
    [-3, -5, -8, -11, 2, 4, 7, 10],
    [-2, -6, -8, -10, 1, 5, 7, 9],
    [-2, -5, -8, -10, 1, 4, 7, 9],
    [-2, -4, -8, -10, 1, 3, 7, 9],
    [-2, -5, -7, -10, 1, 4, 6, 9],
    [-3, -4, -7, -10, 2, 3, 6, 9],
    [-1, -2, -3, -10, 0, 1, 2, 9],
    [-4, -6, -8, -9, 3, 5, 7, 8],
    [-3, -5, -7, -9, 2, 4, 6, 8],
];

fn encode_eac_alpha_block(block: &[[u8; 4]; 16]) -> [u8; 8] {
    let mut min_a = 255u8;
    let mut max_a = 0u8;
    let mut sum_a = 0u32;
    for px in block {
        min_a = min_a.min(px[3]);
        max_a = max_a.max(px[3]);
        sum_a += u32::from(px[3]);
    }
    let base_codeword = ((sum_a + 8) / 16) as u8;
    let span = i16::from(max_a) - i16::from(min_a);
    let approx_mul = ((span + 15) / 18).clamp(1, 15) as u8;
    let mut best_mul = approx_mul;
    let mut best_table = 0u8;
    let mut best_indices = [0u8; 16];
    let mut best_err = u32::MAX;
    for mul_candidate in [1u8, approx_mul, (approx_mul + 1).min(15)] {
        for (t_idx, table) in EAC_ALPHA_TABLES.iter().enumerate() {
            let mut indices = [0u8; 16];
            let mut total_err = 0u32;
            for x in 0..4usize {
                for y in 0..4usize {
                    let target = block[y * 4 + x][3];
                    let mut best_idx = 0u8;
                    let mut best_px_err = u8::MAX;
                    for (idx, &m) in table.iter().enumerate() {
                        let val = (i16::from(base_codeword) + m * i16::from(mul_candidate))
                            .clamp(0, 255) as u8;
                        let err = target.abs_diff(val);
                        if err < best_px_err {
                            best_px_err = err;
                            best_idx = idx as u8;
                        }
                    }
                    indices[x * 4 + y] = best_idx;
                    total_err += u32::from(best_px_err) * u32::from(best_px_err);
                }
            }
            if total_err < best_err {
                best_err = total_err;
                best_mul = mul_candidate;
                best_table = t_idx as u8;
                best_indices = indices;
            }
        }
    }
    let mut index_bits = 0u64;
    for &idx in &best_indices {
        index_bits = (index_bits << 3) | u64::from(idx & 0x07);
    }
    let mut out = [0u8; 8];
    out[0] = base_codeword;
    out[1] = ((best_mul & 0x0F) << 4) | (best_table & 0x0F);
    let be_bytes = index_bits.to_be_bytes();
    out[2..8].copy_from_slice(&be_bytes[2..8]);
    out
}

fn decode_eac_alpha_block(raw: &[u8; 8]) -> [u8; 16] {
    let base_codeword = i16::from(raw[0]);
    let multiplier = i16::from((raw[1] >> 4) & 0x0F);
    let table_idx = (raw[1] & 0x0F) as usize;
    let table = &EAC_ALPHA_TABLES[table_idx];
    let mut be_bytes = [0u8; 8];
    be_bytes[2..8].copy_from_slice(&raw[2..8]);
    let index_bits = u64::from_be_bytes(be_bytes);
    let mut out = [0u8; 16];
    for x in 0..4usize {
        for y in 0..4usize {
            let order = x * 4 + y;
            let shift = (15 - order) * 3;
            let idx = ((index_bits >> shift) & 0x07) as usize;
            let delta = if multiplier == 0 {
                table[idx]
            } else {
                table[idx] * multiplier
            };
            out[y * 4 + x] = (base_codeword + delta).clamp(0, 255) as u8;
        }
    }
    out
}

const ASTC_BLOCK_MODE_VOID_EXTENT: u16 = 0x1FC;
const ASTC_BLOCK_MODE_LDR_RGBA_3X3_3BIT: u16 = 0x15A;
const ASTC_WEIGHTS_3BIT: [u16; 8] = [0, 9, 18, 27, 37, 46, 55, 64];

fn astc_interpolate(e0: u8, e1: u8, weight: u16) -> u8 {
    (((64 - weight) * u16::from(e0) + weight * u16::from(e1) + 32) >> 6) as u8
}

fn astc_grid_weight_3x3(grid: &[u16; 9], px: usize, py: usize, bx: usize, by: usize) -> u16 {
    let gx_fp = if bx > 1 { (px * 32) / (bx - 1) } else { 0 };
    let gy_fp = if by > 1 { (py * 32) / (by - 1) } else { 0 };
    let ix = (gx_fp / 16).min(1);
    let iy = (gy_fp / 16).min(1);
    let fx = (gx_fp - ix * 16) as u16;
    let fy = (gy_fp - iy * 16) as u16;
    let w00 = grid[iy * 3 + ix];
    let w10 = grid[iy * 3 + ix + 1];
    let w01 = grid[(iy + 1) * 3 + ix];
    let w11 = grid[(iy + 1) * 3 + ix + 1];
    let top = (16 - fx) * w00 + fx * w10;
    let bot = (16 - fx) * w01 + fx * w11;
    ((16 - fy) * top + fy * bot + 128) >> 8
}

fn encode_astc_block(texels: &[[u8; 4]], bx: usize, by: usize) -> [u8; 16] {
    let first = texels.first().copied().unwrap_or([0, 0, 0, 255]);
    let is_uniform = texels.iter().all(|&px| px == first);
    if is_uniform {
        let mut word = 0u128;
        let mut pos = 0u32;
        write_bits_u128(
            &mut word,
            &mut pos,
            u32::from(ASTC_BLOCK_MODE_VOID_EXTENT),
            12,
        );
        write_bits_u128(&mut word, &mut pos, 0xFFFF_FFFF, 32);
        write_bits_u128(&mut word, &mut pos, 0x000F_FFFF, 20);
        for &c in &first {
            let c16 = (u32::from(c) << 8) | u32::from(c);
            write_bits_u128(&mut word, &mut pos, c16, 16);
        }
        return word.to_le_bytes();
    }

    let mut min_c = [255u8; 4];
    let mut max_c = [0u8; 4];
    for px in texels {
        for c in 0..4 {
            min_c[c] = min_c[c].min(px[c]);
            max_c[c] = max_c[c].max(px[c]);
        }
    }
    let mut weight_indices = [0u8; 9];
    for gy in 0..3usize {
        let py = if by > 1 { (gy * (by - 1)) / 2 } else { 0 };
        for gx in 0..3usize {
            let px = if bx > 1 { (gx * (bx - 1)) / 2 } else { 0 };
            let sample = texels[py * bx + px];
            let mut best_idx = 0u8;
            let mut best_err = u32::MAX;
            for (idx, &w) in ASTC_WEIGHTS_3BIT.iter().enumerate() {
                let mut err = 0u32;
                for c in 0..4 {
                    let interp = astc_interpolate(min_c[c], max_c[c], w);
                    let diff = i32::from(sample[c]) - i32::from(interp);
                    err += (diff * diff) as u32;
                }
                if err < best_err {
                    best_err = err;
                    best_idx = idx as u8;
                }
            }
            weight_indices[gy * 3 + gx] = best_idx;
        }
    }

    let mut word = 0u128;
    let mut pos = 0u32;
    write_bits_u128(
        &mut word,
        &mut pos,
        u32::from(ASTC_BLOCK_MODE_LDR_RGBA_3X3_3BIT),
        11,
    );
    write_bits_u128(&mut word, &mut pos, 0, 2);
    write_bits_u128(&mut word, &mut pos, 12, 4);
    for c in 0..4 {
        write_bits_u128(&mut word, &mut pos, u32::from(min_c[c]), 8);
        write_bits_u128(&mut word, &mut pos, u32::from(max_c[c]), 8);
    }
    let mut weight_pos = 128u32;
    for &w_idx in &weight_indices {
        let val = u128::from(w_idx & 0x07);
        for bit in 0..3u32 {
            weight_pos -= 1;
            let b = (val >> bit) & 1;
            word |= b << weight_pos;
        }
    }
    word.to_le_bytes()
}

fn decode_astc_block(raw: &[u8; 16], bx: usize, by: usize) -> Vec<[u8; 4]> {
    let word = u128::from_le_bytes(*raw);
    let mode_12 = (word & 0xFFF) as u16;
    let total = bx * by;
    if (mode_12 & 0x1FC) == ASTC_BLOCK_MODE_VOID_EXTENT {
        let mut pos = 64u32;
        let r16 = read_bits_u128(word, &mut pos, 16);
        let g16 = read_bits_u128(word, &mut pos, 16);
        let b16 = read_bits_u128(word, &mut pos, 16);
        let a16 = read_bits_u128(word, &mut pos, 16);
        let rgba = [
            (r16 >> 8) as u8,
            (g16 >> 8) as u8,
            (b16 >> 8) as u8,
            (a16 >> 8) as u8,
        ];
        return vec![rgba; total];
    }
    let mut pos = 17u32;
    let mut e0 = [0u8; 4];
    let mut e1 = [0u8; 4];
    for c in 0..4 {
        e0[c] = read_bits_u128(word, &mut pos, 8) as u8;
        e1[c] = read_bits_u128(word, &mut pos, 8) as u8;
    }
    let mut weight_pos = 128u32;
    let mut grid = [0u16; 9];
    for slot in &mut grid {
        let mut idx = 0u32;
        for bit in 0..3u32 {
            weight_pos -= 1;
            let b = ((word >> weight_pos) & 1) as u32;
            idx |= b << bit;
        }
        *slot = ASTC_WEIGHTS_3BIT[(idx & 0x07) as usize];
    }
    let mut out = vec![[0u8; 4]; total];
    for py in 0..by {
        for px in 0..bx {
            let w = astc_grid_weight_3x3(&grid, px, py, bx, by);
            out[py * bx + px] = [
                astc_interpolate(e0[0], e1[0], w),
                astc_interpolate(e0[1], e1[1], w),
                astc_interpolate(e0[2], e1[2], w),
                astc_interpolate(e0[3], e1[3], w),
            ];
        }
    }
    out
}

fn encode_astc_image(rgba: &[u8], width: u32, height: u32, bx: usize, by: usize) -> Vec<u8> {
    let w = width as usize;
    let h = height as usize;
    let blocks_x = w.div_ceil(bx);
    let blocks_y = h.div_ceil(by);
    let mut out = vec![0u8; blocks_x * blocks_y * 16];
    let mut texels = vec![[0u8; 4]; bx * by];
    for block_y in 0..blocks_y {
        for block_x in 0..blocks_x {
            for py in 0..by {
                let sy = (block_y * by + py).min(h - 1);
                for px in 0..bx {
                    let sx = (block_x * bx + px).min(w - 1);
                    let src = (sy * w + sx) * 4;
                    texels[py * bx + px] = [rgba[src], rgba[src + 1], rgba[src + 2], rgba[src + 3]];
                }
            }
            let encoded = encode_astc_block(&texels, bx, by);
            let dst = (block_y * blocks_x + block_x) * 16;
            out[dst..dst + 16].copy_from_slice(&encoded);
        }
    }
    out
}

fn decode_astc_image(
    encoded: &[u8],
    width: u32,
    height: u32,
    bx: usize,
    by: usize,
) -> Option<Vec<u8>> {
    let w = width as usize;
    let h = height as usize;
    let blocks_x = w.div_ceil(bx);
    let blocks_y = h.div_ceil(by);
    let expected_bytes = blocks_x.checked_mul(blocks_y)?.checked_mul(16)?;
    if encoded.len() != expected_bytes {
        return None;
    }
    let mut rgba = vec![0u8; w * h * 4];
    for block_y in 0..blocks_y {
        for block_x in 0..blocks_x {
            let src = (block_y * blocks_x + block_x) * 16;
            let mut raw = [0u8; 16];
            raw.copy_from_slice(&encoded[src..src + 16]);
            let decoded = decode_astc_block(&raw, bx, by);
            for py in 0..by {
                let dy = block_y * by + py;
                if dy >= h {
                    continue;
                }
                for px in 0..bx {
                    let dx = block_x * bx + px;
                    if dx >= w {
                        continue;
                    }
                    let dst = (dy * w + dx) * 4;
                    rgba[dst..dst + 4].copy_from_slice(&decoded[py * bx + px]);
                }
            }
        }
    }
    Some(rgba)
}
