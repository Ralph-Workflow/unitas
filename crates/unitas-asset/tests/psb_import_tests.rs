use std::collections::HashSet;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use unitas_asset::dependencies::{ImportCandidate, ImportOutcome, ImportSession};
use unitas_asset::import_cache::{ImportCache, LIBRARY_DIRECTORY};
use unitas_asset::texture::import::{
    import_texture, SpriteMode, SpriteSlice, TextureFormat, TextureImportSettings,
    TextureImporterType,
};
use unitas_asset::texture::psd::parse_psd_document;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE: &[u8] = include_bytes!("psb_wick_layers.psb");
const CANVAS: u32 = 32;
const CHANNELS: u16 = 4;

struct LayerSpec {
    name: &'static str,
    top: i32,
    left: i32,
    bottom: i32,
    right: i32,
    divider: Option<u32>,
}

const LAYERS: &[LayerSpec] = &[
    LayerSpec {
        name: "Body",
        top: 0,
        left: 0,
        bottom: 8,
        right: 8,
        divider: None,
    },
    LayerSpec {
        name: "Head",
        top: 0,
        left: 8,
        bottom: 8,
        right: 16,
        divider: None,
    },
    LayerSpec {
        name: "Arm_Left",
        top: 0,
        left: 16,
        bottom: 8,
        right: 24,
        divider: None,
    },
    LayerSpec {
        name: "Arm_Right",
        top: 0,
        left: 24,
        bottom: 8,
        right: 32,
        divider: None,
    },
    LayerSpec {
        name: "Leg_Left",
        top: 8,
        left: 0,
        bottom: 16,
        right: 8,
        divider: None,
    },
    LayerSpec {
        name: "Leg_Right",
        top: 8,
        left: 8,
        bottom: 16,
        right: 16,
        divider: None,
    },
    LayerSpec {
        name: "Hand_Left",
        top: 8,
        left: 16,
        bottom: 16,
        right: 24,
        divider: None,
    },
    LayerSpec {
        name: "Hand_Right",
        top: 8,
        left: 24,
        bottom: 16,
        right: 32,
        divider: None,
    },
    LayerSpec {
        name: "Foot_Left",
        top: 16,
        left: 0,
        bottom: 24,
        right: 8,
        divider: None,
    },
    LayerSpec {
        name: "Foot_Right",
        top: 16,
        left: 8,
        bottom: 24,
        right: 16,
        divider: None,
    },
    LayerSpec {
        name: "</Layer group>",
        top: 0,
        left: 0,
        bottom: 0,
        right: 0,
        divider: Some(3),
    },
    LayerSpec {
        name: "Cape",
        top: 24,
        left: 8,
        bottom: 32,
        right: 16,
        divider: None,
    },
    LayerSpec {
        name: "Hair",
        top: 24,
        left: 0,
        bottom: 32,
        right: 8,
        divider: None,
    },
    LayerSpec {
        name: "Eye_Right",
        top: 16,
        left: 24,
        bottom: 24,
        right: 32,
        divider: None,
    },
    LayerSpec {
        name: "Eye_Left",
        top: 16,
        left: 16,
        bottom: 24,
        right: 24,
        divider: None,
    },
    LayerSpec {
        name: "Character",
        top: 0,
        left: 0,
        bottom: 32,
        right: 32,
        divider: Some(1),
    },
];

const EXPECTED_SPRITES: &[(&str, i32, i32, i32, i32)] = &[
    ("Body", 0, 24, 8, 8),
    ("Head", 8, 24, 8, 8),
    ("Arm_Left", 16, 24, 8, 8),
    ("Arm_Right", 24, 24, 8, 8),
    ("Leg_Left", 0, 16, 8, 8),
    ("Leg_Right", 8, 16, 8, 8),
    ("Hand_Left", 16, 16, 8, 8),
    ("Hand_Right", 24, 16, 8, 8),
    ("Foot_Left", 0, 8, 8, 8),
    ("Foot_Right", 8, 8, 8, 8),
    ("Cape", 8, 0, 8, 8),
    ("Hair", 0, 0, 8, 8),
    ("Eye_Right", 24, 8, 8, 8),
    ("Eye_Left", 16, 8, 8, 8),
];

fn section_length(version: u16, length: usize) -> Vec<u8> {
    if version == 2 {
        (length as u64).to_be_bytes().to_vec()
    } else {
        (length as u32).to_be_bytes().to_vec()
    }
}

fn layer_area(layer: &LayerSpec) -> usize {
    let width = (layer.right - layer.left).max(0) as usize;
    let height = (layer.bottom - layer.top).max(0) as usize;
    width * height
}

fn build_layer_record(version: u16, layer: &LayerSpec) -> Vec<u8> {
    let mut record = Vec::new();
    record.extend_from_slice(&layer.top.to_be_bytes());
    record.extend_from_slice(&layer.left.to_be_bytes());
    record.extend_from_slice(&layer.bottom.to_be_bytes());
    record.extend_from_slice(&layer.right.to_be_bytes());
    record.extend_from_slice(&CHANNELS.to_be_bytes());
    for channel in 0..4i16 {
        record.extend_from_slice(&(channel - 1).to_be_bytes());
        let length = layer_area(layer);
        if version == 2 {
            record.extend_from_slice(&(length as u64).to_be_bytes());
        } else {
            record.extend_from_slice(&(length as u32).to_be_bytes());
        }
    }
    record.extend_from_slice(b"8BIM");
    record.extend_from_slice(b"norm");
    record.extend_from_slice(&[255u8, 0, 0, 0]);
    let mut extra = Vec::new();
    extra.extend_from_slice(&0u32.to_be_bytes());
    extra.extend_from_slice(&0u32.to_be_bytes());
    let name = layer.name.as_bytes();
    extra.push(name.len() as u8);
    extra.extend_from_slice(name);
    let padding = (4 - name.len() % 4) % 4;
    extra.extend(std::iter::repeat_n(0u8, padding));
    if let Some(kind) = layer.divider {
        extra.extend_from_slice(b"8BIM");
        extra.extend_from_slice(b"lsct");
        extra.extend_from_slice(&4u32.to_be_bytes());
        extra.extend_from_slice(&kind.to_be_bytes());
    }
    extra.extend_from_slice(b"8BIM");
    extra.extend_from_slice(b"luni");
    let units: Vec<u16> = layer.name.encode_utf16().collect();
    extra.extend_from_slice(&((units.len() * 2) as u32).to_be_bytes());
    for unit in units {
        extra.extend_from_slice(&unit.to_be_bytes());
    }
    record.extend_from_slice(&(extra.len() as u32).to_be_bytes());
    record.extend_from_slice(&extra);
    record
}

fn packbits(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for chunk in data.chunks(128) {
        out.push((chunk.len() - 1) as u8);
        out.extend_from_slice(chunk);
    }
    out
}

fn build_photoshop_document(
    version: u16,
    width: u32,
    height: u32,
    layers: &[LayerSpec],
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"8BPS");
    out.extend_from_slice(&version.to_be_bytes());
    out.extend_from_slice(&[0u8; 6]);
    out.extend_from_slice(&CHANNELS.to_be_bytes());
    if version == 2 {
        out.extend_from_slice(&u64::from(height).to_be_bytes());
        out.extend_from_slice(&u64::from(width).to_be_bytes());
    } else {
        out.extend_from_slice(&height.to_be_bytes());
        out.extend_from_slice(&width.to_be_bytes());
    }
    out.extend_from_slice(&8u16.to_be_bytes());
    out.extend_from_slice(&3u16.to_be_bytes());
    out.extend_from_slice(&section_length(version, 0));
    out.extend_from_slice(&section_length(version, 0));

    let mut info = Vec::new();
    info.extend_from_slice(&(layers.len() as i16).to_be_bytes());
    for layer in layers {
        info.extend_from_slice(&build_layer_record(version, layer));
    }
    for layer in layers {
        for _ in 0..4 {
            info.extend_from_slice(&0u16.to_be_bytes());
            info.extend(std::iter::repeat_n(0u8, layer_area(layer)));
        }
    }
    let mut layer_and_mask = Vec::new();
    layer_and_mask.extend_from_slice(&section_length(version, info.len()));
    layer_and_mask.extend_from_slice(&info);
    layer_and_mask.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&section_length(version, layer_and_mask.len()));
    out.extend_from_slice(&layer_and_mask);

    out.extend_from_slice(&1u16.to_be_bytes());
    let mut rows: Vec<Vec<u8>> = Vec::new();
    for channel in 0..4u32 {
        for y in 0..height {
            let mut row = Vec::with_capacity(width as usize);
            for x in 0..width {
                let value = if channel == 3 {
                    255u8
                } else {
                    ((x + y * 2 + channel * 16) % 256) as u8
                };
                row.push(value);
            }
            rows.push(packbits(&row));
        }
    }
    for row in &rows {
        if version == 2 {
            out.extend_from_slice(&(row.len() as u32).to_be_bytes());
        } else {
            out.extend_from_slice(&(row.len() as u16).to_be_bytes());
        }
    }
    for row in &rows {
        out.extend_from_slice(row);
    }
    out
}

fn sprite_settings() -> TextureImportSettings {
    TextureImportSettings {
        texture_type: TextureImporterType::Sprite,
        sprite_mode: SpriteMode::Multiple,
        mipmaps: false,
        ..TextureImportSettings::default()
    }
}

fn temp_dir(name: &str) -> std::io::Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!("unitas-{name}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&path)?;
    Ok(path)
}

#[test]
fn psb_fixture_file_matches_the_generator() {
    let generated = build_photoshop_document(2, CANVAS, CANVAS, LAYERS);
    assert_eq!(
        FIXTURE,
        generated.as_slice(),
        "the committed fixture is the generated PSB container"
    );
}

#[test]
fn psb_import_decodes_the_composite_and_names_every_layer_as_a_sprite() -> TestResult {
    let imported = import_texture(FIXTURE, "Assets/Characters/Wick.psb", &sprite_settings())?;
    assert_eq!(imported.texture.format, TextureFormat::Rgba8);
    assert_eq!(
        (imported.texture.width, imported.texture.height),
        (CANVAS, CANVAS)
    );
    assert_eq!(imported.texture.pixel(0, 0), Some(&[0u8, 16, 32, 255][..]));
    assert_eq!(
        imported.texture.pixel(31, 31),
        Some(&[93u8, 109, 125, 255][..])
    );
    assert_eq!(
        imported.sprites.len(),
        14,
        "every named layer of the document becomes a sprite"
    );
    for (sprite, expected) in imported.sprites.iter().zip(EXPECTED_SPRITES) {
        assert_eq!(sprite.name, expected.0);
        assert_eq!(
            (sprite.x, sprite.y, sprite.width, sprite.height),
            (expected.1, expected.2, expected.3, expected.4),
            "the sprite rect of {} follows the layer rect",
            expected.0
        );
    }
    Ok(())
}

#[test]
fn psb_layers_keep_their_group_hierarchy() -> TestResult {
    let document = parse_psd_document(FIXTURE)?;
    assert_eq!(document.version, 2);
    assert_eq!((document.width, document.height), (CANVAS, CANVAS));
    assert_eq!(document.layers.len(), 11);
    let group = document
        .layers
        .last()
        .ok_or("the document carries no group layer")?;
    assert_eq!(group.name, "Character");
    assert_eq!(group.children.len(), 4);
    let child_names: Vec<&str> = group
        .children
        .iter()
        .map(|child| child.name.as_str())
        .collect();
    assert_eq!(child_names, ["Cape", "Hair", "Eye_Right", "Eye_Left"]);
    let cape = group.children.first().ok_or("the group carries no child")?;
    assert_eq!((cape.rect.left, cape.rect.top), (8, 24));
    assert_eq!((cape.rect.right, cape.rect.bottom), (16, 32));
    Ok(())
}

#[test]
fn psb_sprite_identities_survive_reimport() -> TestResult {
    let first = import_texture(FIXTURE, "Assets/Characters/Wick.psb", &sprite_settings())?;
    let second = import_texture(FIXTURE, "Assets/Characters/Wick.psb", &sprite_settings())?;
    let first_ids: Vec<i64> = first
        .sprites
        .iter()
        .map(|sprite| sprite.unity_local_file_id())
        .collect();
    let second_ids: Vec<i64> = second
        .sprites
        .iter()
        .map(|sprite| sprite.unity_local_file_id())
        .collect();
    assert_eq!(first_ids, second_ids, "a reimport keeps every identity");
    let unique: HashSet<i64> = first_ids.iter().copied().collect();
    assert_eq!(unique.len(), 14, "every layer owns its own identity");
    for (sprite, expected) in first.sprites.iter().zip(EXPECTED_SPRITES) {
        let named = SpriteSlice::new(expected.0, 0, 0, 0, 0);
        assert_eq!(
            sprite.unity_local_file_id(),
            named.unity_local_file_id(),
            "the identity of {} is keyed by its layer name",
            expected.0
        );
    }
    Ok(())
}

#[test]
fn psb_reimport_with_unchanged_inputs_reuses_the_cached_result() -> TestResult {
    let root = temp_dir("psb-reimport")?;
    let library = root.join("project").join(LIBRARY_DIRECTORY);
    let candidate = ImportCandidate {
        path: PathBuf::from("Assets/Characters/Wick.psb"),
        guid: "3f1c9a4d-8b2e-47c6-a5d0-e91f2b7c8d34".to_string(),
        source: FIXTURE.to_vec(),
        metadata: String::new(),
        settings: "TextureImporter".to_string(),
        importer_version: "1".to_string(),
        dependencies: Vec::new(),
    };
    let mut importer = |_: &ImportCandidate| ImportOutcome::ready();

    let mut first = ImportSession::with_cache(ImportCache::open(&library));
    let report = first.run(std::slice::from_ref(&candidate), &mut importer);
    assert_eq!(report.imported, 1);
    assert_eq!(report.reused, 0);
    let first_fingerprint = candidate.fingerprint();

    let mut second = ImportSession::with_cache(ImportCache::open(&library));
    let report = second.run(std::slice::from_ref(&candidate), &mut importer);
    assert_eq!(
        report.imported, 0,
        "an unchanged source is not imported again"
    );
    assert_eq!(report.reused, 1, "the unchanged source keeps its result");
    let record = report
        .records
        .first()
        .ok_or("the session recorded no result")?;
    assert_eq!(record.fingerprint, first_fingerprint);
    assert!(record.reused);
    Ok(())
}

#[test]
fn psd_version_one_layers_import_as_the_same_sprites() -> TestResult {
    let document = build_photoshop_document(1, CANVAS, CANVAS, LAYERS);
    let imported = import_texture(&document, "Assets/Characters/Wick.psd", &sprite_settings())?;
    assert_eq!(imported.texture.format, TextureFormat::Rgba8);
    assert_eq!(
        (imported.texture.width, imported.texture.height),
        (CANVAS, CANVAS)
    );
    assert_eq!(imported.texture.pixel(0, 0), Some(&[0u8, 16, 32, 255][..]));
    assert_eq!(imported.sprites.len(), 14);
    for (sprite, expected) in imported.sprites.iter().zip(EXPECTED_SPRITES) {
        assert_eq!(sprite.name, expected.0);
        assert_eq!(
            (sprite.x, sprite.y, sprite.width, sprite.height),
            (expected.1, expected.2, expected.3, expected.4)
        );
    }
    Ok(())
}
