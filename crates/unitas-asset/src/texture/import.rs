use std::fmt;

use crate::dependencies::{ImportDiagnostic, ImportOutcome};
use crate::texture::exr::decode_exr;
use crate::texture::jpeg::{decode_jpeg, has_jpeg_signature};
use crate::texture::png::decode_png;
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
        SpriteMode::Multiple => settings.sprite_slices.clone(),
        _ => Vec::new(),
    };
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
        sprites,
        notes,
    })
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
