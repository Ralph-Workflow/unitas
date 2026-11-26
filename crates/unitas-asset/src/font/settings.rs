use super::FontImportError;

pub const DEFAULT_FONT_SIZE: u32 = 16;
pub const DEFAULT_CHARACTER_PADDING: u32 = 1;
pub const IMPORTER_SECTION: &str = "TrueTypeFontImporter:";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontRenderingMode {
    Smooth,
    Hinted,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FontImportSettings {
    pub font_size: u32,
    pub character_padding: u32,
    pub character_spacing: f32,
    pub include_font_data: bool,
    pub rendering_mode: FontRenderingMode,
    pub characters: Vec<char>,
}

impl Default for FontImportSettings {
    fn default() -> Self {
        FontImportSettings {
            font_size: DEFAULT_FONT_SIZE,
            character_padding: DEFAULT_CHARACTER_PADDING,
            character_spacing: 0.0,
            include_font_data: true,
            rendering_mode: FontRenderingMode::Smooth,
            characters: default_characters(),
        }
    }
}

impl FontImportSettings {
    pub fn ppem(&self) -> f32 {
        self.font_size as f32
    }

    pub fn from_meta_text(text: &str) -> Result<FontImportSettings, FontImportError> {
        let values = importer_values(text);
        let mut settings = FontImportSettings::default();
        if let Some(value) = values.iter().find(|(key, _)| key == "fontSize") {
            settings.font_size = parse_u32("fontSize", &value.1)?;
        }
        if let Some(value) = values.iter().find(|(key, _)| key == "characterPadding") {
            settings.character_padding = parse_u32("characterPadding", &value.1)?;
        }
        if let Some(value) = values.iter().find(|(key, _)| key == "characterSpacing") {
            settings.character_spacing = parse_f32("characterSpacing", &value.1)?;
        }
        if let Some(value) = values.iter().find(|(key, _)| key == "includeFontData") {
            settings.include_font_data = parse_bool("includeFontData", &value.1)?;
        }
        if let Some(value) = values.iter().find(|(key, _)| key == "fontRenderingMode") {
            let code = parse_u32("fontRenderingMode", &value.1)?;
            settings.rendering_mode = match code {
                1 => FontRenderingMode::Hinted,
                _ => FontRenderingMode::Smooth,
            };
        }
        Ok(settings)
    }
}

pub fn default_characters() -> Vec<char> {
    (0x20u32..0x7F).filter_map(char::from_u32).collect()
}

fn importer_values(text: &str) -> Vec<(String, String)> {
    let mut values = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let indented = line.len() > line.trim_start().len();
        if !indented {
            inside = trimmed == IMPORTER_SECTION;
            continue;
        }
        if !inside {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once(':') {
            values.push((key.trim().to_string(), value.trim().to_string()));
        }
    }
    values
}

fn parse_u32(key: &str, value: &str) -> Result<u32, FontImportError> {
    value
        .parse::<u32>()
        .map_err(|_| FontImportError::InvalidMetaText(format!("{key} is not a number")))
}

fn parse_f32(key: &str, value: &str) -> Result<f32, FontImportError> {
    value
        .parse::<f32>()
        .map_err(|_| FontImportError::InvalidMetaText(format!("{key} is not a number")))
}

fn parse_bool(key: &str, value: &str) -> Result<bool, FontImportError> {
    match value {
        "0" => Ok(false),
        "1" => Ok(true),
        other => Err(FontImportError::InvalidMetaText(format!(
            "{key} is {other}"
        ))),
    }
}
