//! Sprite import details the scene parser needs: the nine-slice border of a
//! sprite and the draw mode of a uGUI image.

use super::SpriteDrawMode;
use crate::meta::GuidIndex;
use crate::unity_yaml::Node;

/// The `Image` type: Simple, Sliced or Tiled. Filled is not drawn yet and draws simple.
pub fn ui_image_draw_mode(node: Option<&Node>) -> SpriteDrawMode {
    match node.and_then(Node::as_i64).unwrap_or(0) {
        1 => SpriteDrawMode::Sliced,
        2 => SpriteDrawMode::Tiled,
        _ => SpriteDrawMode::Simple,
    }
}

/// The nine-slice border of a sprite from its texture import meta, in sprite
/// pixels: left, bottom, right, top.
pub fn sprite_border(index: Option<&GuidIndex>, guid: Option<&str>) -> [f32; 4] {
    let Some(guid) = guid else {
        return [0.0; 4];
    };
    let Some(path) = index.and_then(|index| index.get(guid)) else {
        return [0.0; 4];
    };
    let meta_path = format!("{}.meta", path.display());
    let Ok(text) = std::fs::read_to_string(meta_path) else {
        return [0.0; 4];
    };
    if let Ok(settings) = crate::texture::import::TextureImportSettings::from_meta_text(&text) {
        if settings.sprite_slices.len() == 1 {
            let slice = &settings.sprite_slices[0];
            return [
                slice.border_left,
                slice.border_bottom,
                slice.border_right,
                slice.border_top,
            ];
        }
    }
    meta_sprite_border(&text)
}

/// Unity's single sprite border from a texture import meta.
fn meta_sprite_border(text: &str) -> [f32; 4] {
    let mut border = [0.0; 4];
    let mut indent = 0usize;
    let mut inside = false;
    for line in text.lines() {
        let trimmed = line.trim();
        let line_indent = line.len() - line.trim_start().len();
        if !inside {
            if trimmed == "spriteBorder:" {
                inside = true;
                indent = line_indent;
            }
            continue;
        }
        if line_indent <= indent {
            break;
        }
        let Some((key, value)) = trimmed.split_once(':') else {
            continue;
        };
        let Ok(number) = value.trim().parse::<f32>() else {
            continue;
        };
        match key {
            "x" => border[0] = number,
            "y" => border[1] = number,
            "z" => border[2] = number,
            "w" => border[3] = number,
            _ => {}
        }
    }
    border
}
