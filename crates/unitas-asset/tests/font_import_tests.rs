use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use unitas_asset::database::identify_asset_family;
use unitas_asset::dependencies::AssetImportState;
use unitas_asset::font::{
    font_import_outcome, import_font, FontImportError, FontImportSettings, FontRenderingMode,
};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

const FONT_ASSET: &str = "Assets/Fonts/NotoSans-Regular.ttf";
const UNITS_PER_EM: u16 = 1000;
const ASCENT_UNITS: f32 = 1069.0;
const DESCENT_UNITS: f32 = 293.0;
const ADVANCE_A_UNITS: f32 = 639.0;
const GLYPH_A: u16 = 36;
const KERN_AV_UNITS: f32 = -40.0;

fn sample_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples/marble-rush")
}

fn sample_font_bytes() -> TestResult<Vec<u8>> {
    Ok(fs::read(sample_root().join(FONT_ASSET))?)
}

fn sample_meta_text() -> TestResult<String> {
    Ok(fs::read_to_string(
        sample_root().join(format!("{FONT_ASSET}.meta")),
    )?)
}

fn scale_for(settings: &FontImportSettings) -> f32 {
    settings.ppem() / f32::from(UNITS_PER_EM)
}

#[test]
fn imports_the_shipped_font_with_the_known_metrics() -> TestResult {
    let bytes = sample_font_bytes()?;
    let settings = FontImportSettings::default();
    let imported = import_font(&bytes, "NotoSans-Regular.ttf", &settings)?;
    let scale = scale_for(&settings);
    assert_eq!(imported.units_per_em, UNITS_PER_EM);
    assert_eq!(imported.name, "NotoSans-Regular");
    assert_eq!(imported.family, "Noto Sans");
    assert!((imported.ascent - ASCENT_UNITS * scale).abs() < 0.001);
    assert!((imported.descent - DESCENT_UNITS * scale).abs() < 0.001);
    assert!(imported.line_gap.abs() < 0.001);
    let glyph = imported.glyph('A').ok_or("the font must map A")?;
    assert_eq!(glyph.glyph_id, GLYPH_A);
    assert!((glyph.advance - ADVANCE_A_UNITS * scale).abs() < 0.001);
    assert!(glyph.width > 0);
    assert!(glyph.height > 0);
    assert!(imported.atlas.width > 0);
    assert!(imported.atlas.height > 0);
    assert!(imported.atlas.pixels.iter().any(|value| *value > 0));
    Ok(())
}

#[test]
fn imports_the_kerning_the_font_carries() -> TestResult {
    let bytes = sample_font_bytes()?;
    let settings = FontImportSettings::default();
    let imported = import_font(&bytes, "NotoSans-Regular.ttf", &settings)?;
    let adjustment = imported
        .kerning('A', 'V')
        .ok_or("the font kerns A against V")?;
    let expected = KERN_AV_UNITS * scale_for(&settings);
    assert!((adjustment - expected).abs() < 0.001);
    assert!(adjustment < 0.0);
    Ok(())
}

#[test]
fn reimports_the_font_to_the_same_cooked_data() -> TestResult {
    let bytes = sample_font_bytes()?;
    let settings = FontImportSettings::default();
    let first = import_font(&bytes, "NotoSans-Regular.ttf", &settings)?;
    let second = import_font(&bytes, "NotoSans-Regular.ttf", &settings)?;
    assert_eq!(first.name, second.name);
    assert_eq!(first.family, second.family);
    assert_eq!(first.units_per_em, second.units_per_em);
    assert_eq!(first.glyphs, second.glyphs);
    assert_eq!(first.kerning, second.kerning);
    assert_eq!(first.atlas.width, second.atlas.width);
    assert_eq!(first.atlas.height, second.atlas.height);
    assert_eq!(first.atlas.pixels, second.atlas.pixels);
    Ok(())
}

#[test]
fn reports_a_named_diagnostic_for_a_malformed_font() -> TestResult {
    let bytes = b"this is not a font at all";
    let error = match import_font(bytes, "Broken.ttf", &FontImportSettings::default()) {
        Ok(_) => return Err("a malformed font must not import".into()),
        Err(error) => error,
    };
    assert_eq!(
        error,
        FontImportError::MalformedFont(
            "the file does not start with a TrueType or OpenType signature".to_string()
        )
    );
    let outcome = font_import_outcome("Assets/Fonts/Broken.ttf", Err(&error));
    assert_eq!(outcome.state, AssetImportState::Failed);
    let diagnostic = outcome
        .diagnostics
        .first()
        .ok_or("a failed font import carries a diagnostic")?;
    assert!(diagnostic.issue.contains("TrueType"));
    assert!(!diagnostic.consequence.is_empty());
    Ok(())
}

#[test]
fn reports_a_named_diagnostic_for_an_unsupported_container() -> TestResult {
    let mut bytes = b"wOF2".to_vec();
    bytes.extend_from_slice(&[0; 32]);
    let error = match import_font(&bytes, "Web.woff2", &FontImportSettings::default()) {
        Ok(_) => return Err("an unsupported container must not import".into()),
        Err(error) => error,
    };
    assert_eq!(error, FontImportError::UnsupportedContainer("WOFF2"));
    let outcome = font_import_outcome("Assets/Fonts/Web.woff2", Err(&error));
    assert_eq!(outcome.state, AssetImportState::Failed);
    let diagnostic = outcome
        .diagnostics
        .first()
        .ok_or("a failed font import carries a diagnostic")?;
    assert!(diagnostic.issue.contains("WOFF2"));
    Ok(())
}

#[test]
fn honors_the_import_settings_of_the_meta_file() -> TestResult {
    let meta = sample_meta_text()?;
    let settings = FontImportSettings::from_meta_text(&meta)?;
    assert_eq!(settings.font_size, 16);
    assert_eq!(settings.character_padding, 2);
    assert!(settings.include_font_data);
    assert_eq!(settings.rendering_mode, FontRenderingMode::Smooth);
    let bytes = sample_font_bytes()?;
    let imported = import_font(&bytes, "NotoSans-Regular.ttf", &settings)?;
    assert!(imported.source.is_some());
    let glyph = imported.glyph('A').ok_or("the font must map A")?;
    assert!(glyph.width > 0);
    let leftmost = imported
        .glyphs
        .iter()
        .filter(|glyph| glyph.width > 0)
        .map(|glyph| glyph.atlas_x)
        .min()
        .unwrap_or(0);
    assert_eq!(leftmost, 2);
    let doubled = FontImportSettings {
        font_size: 32,
        ..FontImportSettings::default()
    };
    let scaled = import_font(&bytes, "NotoSans-Regular.ttf", &doubled)?;
    let glyph = scaled.glyph('A').ok_or("the font must map A")?;
    assert!((glyph.advance - ADVANCE_A_UNITS * scale_for(&doubled)).abs() < 0.001);
    Ok(())
}

#[test]
fn the_dispatch_imports_fonts_and_names_the_failures() -> TestResult {
    let bytes = sample_font_bytes()?;
    let meta = sample_meta_text()?;
    let identified = identify_asset_family(Path::new(FONT_ASSET), &bytes, Some(&meta))
        .ok_or("a font asset is recognized")?;
    assert_eq!(identified.family.class_name(), "Font");
    assert_eq!(identified.outcome.state, AssetImportState::Ready);
    let mut broken = b"wOF2".to_vec();
    broken.extend_from_slice(&[0; 32]);
    let identified = identify_asset_family(Path::new("Assets/Fonts/Broken.ttf"), &broken, None)
        .ok_or("a broken font asset is recognized")?;
    assert_eq!(identified.outcome.state, AssetImportState::Failed);
    let diagnostic = identified
        .outcome
        .diagnostics
        .first()
        .ok_or("a failed font import carries a diagnostic")?;
    assert!(diagnostic.issue.contains("WOFF2"));
    Ok(())
}
