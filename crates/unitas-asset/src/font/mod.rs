mod atlas;
mod settings;

pub use atlas::FontAtlas;
pub use settings::{FontImportSettings, FontRenderingMode};

use atlas::pack_atlas;
use std::fmt;
use std::path::Path;

use swash::scale::{Render, ScaleContext, Source};
use swash::shape::ShapeContext;
use swash::text::Script;

use crate::dependencies::{ImportDiagnostic, ImportOutcome};

const TRUETYPE_SIGNATURE: [u8; 4] = [0x00, 0x01, 0x00, 0x00];
const APPLE_TRUETYPE_SIGNATURE: [u8; 4] = *b"true";
const CFF_SIGNATURE: [u8; 4] = *b"OTTO";
const COLLECTION_SIGNATURE: [u8; 4] = *b"ttcf";
const WOFF_SIGNATURE: [u8; 4] = *b"wOFF";
const WOFF2_SIGNATURE: [u8; 4] = *b"wOF2";

pub const FONT_IMPORTER_VERSION: &str = "unitas-font-importer-1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FontImportError {
    UnsupportedContainer(&'static str),
    MalformedFont(String),
    InvalidMetaText(String),
}

impl fmt::Display for FontImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FontImportError::UnsupportedContainer(name) => {
                write!(formatter, "{name} font container is Unsupported")
            }
            FontImportError::MalformedFont(reason) => {
                write!(formatter, "malformed font: {reason}")
            }
            FontImportError::InvalidMetaText(message) => {
                write!(formatter, "invalid font meta text: {message}")
            }
        }
    }
}

impl std::error::Error for FontImportError {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontGlyph {
    pub code_point: u32,
    pub glyph_id: u16,
    pub advance: f32,
    pub left_side_bearing: f32,
    pub bearing_x: f32,
    pub bearing_y: f32,
    pub atlas_x: u32,
    pub atlas_y: u32,
    pub width: u32,
    pub height: u32,
}

impl FontGlyph {
    pub fn character(&self) -> Option<char> {
        char::from_u32(self.code_point)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KernPair {
    pub left: u32,
    pub right: u32,
    pub adjustment: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FontImportResult {
    pub name: String,
    pub family: String,
    pub units_per_em: u16,
    pub ascent: f32,
    pub descent: f32,
    pub line_gap: f32,
    pub glyphs: Vec<FontGlyph>,
    pub kerning: Vec<KernPair>,
    pub atlas: FontAtlas,
    pub source: Option<Vec<u8>>,
}

impl FontImportResult {
    pub fn glyph(&self, character: char) -> Option<&FontGlyph> {
        let code_point = u32::from(character);
        self.glyphs
            .iter()
            .find(|glyph| glyph.code_point == code_point)
    }

    pub fn kerning(&self, left: char, right: char) -> Option<f32> {
        let pair = (u32::from(left), u32::from(right));
        self.kerning
            .iter()
            .find(|kern| (kern.left, kern.right) == pair)
            .map(|kern| kern.adjustment)
    }
}

pub fn import_font(
    bytes: &[u8],
    file_name: &str,
    settings: &FontImportSettings,
) -> Result<FontImportResult, FontImportError> {
    detect_container(bytes)?;
    let font = swash::FontRef::from_index(bytes, 0).ok_or_else(|| {
        FontImportError::MalformedFont("the font table directory is unreadable".to_string())
    })?;
    let metrics = font.metrics(&[]);
    if metrics.units_per_em == 0 {
        return Err(FontImportError::MalformedFont(
            "the head table declares no units per em".to_string(),
        ));
    }
    let ppem = settings.ppem();
    let scaled = metrics.scale(ppem);
    let glyph_metrics = font.glyph_metrics(&[]).scale(ppem);
    let mut context = ShapeContext::new();
    let mut scale_context = ScaleContext::new();
    let mut scaler = scale_context
        .builder(font)
        .size(ppem)
        .hint(settings.rendering_mode == FontRenderingMode::Hinted)
        .build();
    let sources = [Source::Outline];
    let render = Render::new(&sources);
    let mut glyphs: Vec<FontGlyph> = Vec::new();
    let mut bitmaps: Vec<Vec<u8>> = Vec::new();
    for &character in &settings.characters {
        let shaped = shape_text(&mut context, &font, ppem, &[character], &[]);
        let glyph_id = match shaped.first() {
            Some(glyph) => glyph.id,
            None => continue,
        };
        if glyph_id >= metrics.glyph_count {
            continue;
        }
        let (width, height, bearing_x, bearing_y, bitmap) =
            match render.render(&mut scaler, glyph_id) {
                Some(image) => (
                    image.placement.width,
                    image.placement.height,
                    image.placement.left as f32,
                    image.placement.height as f32 - image.placement.top as f32,
                    image.data,
                ),
                None => (0, 0, 0.0, 0.0, Vec::new()),
            };
        let width = if height == 0 { 0 } else { width };
        glyphs.push(FontGlyph {
            code_point: u32::from(character),
            glyph_id,
            advance: glyph_metrics.advance_width(glyph_id),
            left_side_bearing: glyph_metrics.lsb(glyph_id),
            bearing_x,
            bearing_y,
            atlas_x: 0,
            atlas_y: 0,
            width,
            height,
        });
        bitmaps.push(bitmap);
    }
    if glyphs.is_empty() {
        return Err(FontImportError::MalformedFont(
            "the font maps none of the requested characters".to_string(),
        ));
    }
    let kerning = collect_kerning(&mut context, &font, ppem, &glyphs);
    for glyph in &mut glyphs {
        glyph.advance += settings.character_spacing;
    }
    let atlas = pack_atlas(&mut glyphs, &bitmaps, settings.character_padding);
    Ok(FontImportResult {
        name: asset_name(file_name),
        family: family_name(&font),
        units_per_em: metrics.units_per_em,
        ascent: scaled.ascent,
        descent: scaled.descent,
        line_gap: scaled.leading,
        glyphs,
        kerning,
        atlas,
        source: if settings.include_font_data {
            Some(bytes.to_vec())
        } else {
            None
        },
    })
}

pub fn font_import_outcome(
    path: &str,
    result: Result<&FontImportResult, &FontImportError>,
) -> ImportOutcome {
    match result {
        Ok(_) => ImportOutcome::ready(),
        Err(error) => ImportOutcome::failed(ImportDiagnostic::new(
            path,
            error.to_string(),
            "the font cannot be imported; the asset stays failed and no glyph data is available",
        )),
    }
}

fn detect_container(bytes: &[u8]) -> Result<(), FontImportError> {
    let signature = bytes.get(..4).ok_or_else(|| {
        FontImportError::MalformedFont("the file is too short to hold a font signature".to_string())
    })?;
    if signature == TRUETYPE_SIGNATURE.as_slice()
        || signature == APPLE_TRUETYPE_SIGNATURE.as_slice()
        || signature == CFF_SIGNATURE.as_slice()
    {
        return Ok(());
    }
    if signature == COLLECTION_SIGNATURE.as_slice() {
        return Err(FontImportError::UnsupportedContainer("TrueType Collection"));
    }
    if signature == WOFF_SIGNATURE.as_slice() {
        return Err(FontImportError::UnsupportedContainer("WOFF"));
    }
    if signature == WOFF2_SIGNATURE.as_slice() {
        return Err(FontImportError::UnsupportedContainer("WOFF2"));
    }
    Err(FontImportError::MalformedFont(
        "the file does not start with a TrueType or OpenType signature".to_string(),
    ))
}

fn asset_name(file_name: &str) -> String {
    Path::new(file_name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(file_name)
        .to_string()
}

fn family_name(font: &swash::FontRef<'_>) -> String {
    let mut fallback = String::new();
    for entry in font.localized_strings() {
        if entry.id() != swash::StringId::Family {
            continue;
        }
        let name = entry.to_string();
        if entry.language() == "en" {
            return name;
        }
        if fallback.is_empty() {
            fallback = name;
        }
    }
    fallback
}

struct ShapedGlyph {
    id: u16,
    advance: f32,
}

fn shape_text(
    context: &mut ShapeContext,
    font: &swash::FontRef<'_>,
    ppem: f32,
    text: &[char],
    features: &[(&str, u16)],
) -> Vec<ShapedGlyph> {
    let mut source = String::with_capacity(text.len());
    for character in text {
        source.push(*character);
    }
    let mut shaper = context
        .builder(*font)
        .script(Script::Latin)
        .size(ppem)
        .features(features.iter().copied())
        .build();
    shaper.add_str(&source);
    let mut shaped = Vec::new();
    shaper.shape_with(|cluster| {
        for glyph in cluster.glyphs {
            shaped.push(ShapedGlyph {
                id: glyph.id,
                advance: glyph.advance,
            });
        }
    });
    shaped
}

fn collect_kerning(
    context: &mut ShapeContext,
    font: &swash::FontRef<'_>,
    ppem: f32,
    glyphs: &[FontGlyph],
) -> Vec<KernPair> {
    const DISABLED: [(&str, u16); 4] = [("liga", 0), ("clig", 0), ("dlig", 0), ("rlig", 0)];
    let mut pairs = Vec::new();
    for left in glyphs {
        let left_character = match char::from_u32(left.code_point) {
            Some(character) => character,
            None => continue,
        };
        for right in glyphs {
            let right_character = match char::from_u32(right.code_point) {
                Some(character) => character,
                None => continue,
            };
            let shaped = shape_text(
                context,
                font,
                ppem,
                &[left_character, right_character],
                &DISABLED,
            );
            if shaped.len() != 2 {
                continue;
            }
            if shaped[0].id != left.glyph_id || shaped[1].id != right.glyph_id {
                continue;
            }
            // shortcut: kerning is read from the shaped advance delta, so a pair the
            // font expresses only as a placement offset is missed; upgrade when the
            // importer reads the GPOS pair table directly.
            let adjustment =
                (shaped[0].advance + shaped[1].advance) - (left.advance + right.advance);
            if adjustment.abs() > 0.001 {
                pairs.push(KernPair {
                    left: left.code_point,
                    right: right.code_point,
                    adjustment,
                });
            }
        }
    }
    pairs
}
