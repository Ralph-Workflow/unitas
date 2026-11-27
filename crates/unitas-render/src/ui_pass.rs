use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

pub use crate::glyphs::{HorizontalAlign, Rect, TextLayoutBox, VerticalAlign};
use crate::image::Rgba8;

static UI_SPRITE_REGISTRY: OnceLock<Mutex<HashMap<String, Arc<Rgba8>>>> = OnceLock::new();

fn ui_sprite_registry() -> &'static Mutex<HashMap<String, Arc<Rgba8>>> {
    UI_SPRITE_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn register_ui_sprite_image(guid: impl Into<String>, image: Rgba8) {
    let mut guard = match ui_sprite_registry().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    guard.insert(guid.into(), Arc::new(image));
}

pub fn ui_sprite_image(guid: &str) -> Option<Rgba8> {
    let guard = match ui_sprite_registry().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    guard.get(guid).map(|image| (**image).clone())
}

/// The registered sprite texture, shared so a sliced image does not copy it per quad.
pub fn ui_sprite_texture(guid: &str) -> Option<Arc<Rgba8>> {
    let guard = match ui_sprite_registry().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    guard.get(guid).cloned()
}

/// The pixel size of a registered sprite, used to split its nine-slice border.
pub fn ui_sprite_size(guid: &str) -> Option<[f32; 2]> {
    let guard = match ui_sprite_registry().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    guard
        .get(guid)
        .map(|image| [image.width as f32, image.height as f32])
}

pub const FOCUS_INDICATOR_COLOR: [f32; 4] = [1.0, 0.9, 0.0, 1.0];

pub fn focus_border_rects(rect: [f32; 4], thickness: f32) -> [[f32; 4]; 4] {
    let clamped = thickness.min(rect[2] * 0.5).min(rect[3] * 0.5).max(1.0);
    [
        [rect[0], rect[1], rect[2], clamped],
        [rect[0], rect[1] + rect[3] - clamped, rect[2], clamped],
        [rect[0], rect[1], clamped, rect[3]],
        [rect[0] + rect[2] - clamped, rect[1], clamped, rect[3]],
    ]
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpriteRef {
    pub texture_guid: String,
    pub uv_rect: [f32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiText<'a> {
    pub text: &'a str,
    pub x: f32,
    pub y: f32,
    pub size_px: f32,
    pub color: [f32; 4],
    pub horizontal_align: HorizontalAlign,
    pub vertical_align: VerticalAlign,
    pub wrap: bool,
    pub max_width: Option<f32>,
    pub line_spacing: f32,
    pub clip_rect: Option<[f32; 4]>,
}

impl<'a> UiText<'a> {
    pub const fn new(text: &'a str, x: f32, y: f32, size_px: f32, color: [f32; 4]) -> Self {
        Self {
            text,
            x,
            y,
            size_px,
            color,
            horizontal_align: HorizontalAlign::Left,
            vertical_align: VerticalAlign::Top,
            wrap: false,
            max_width: None,
            line_spacing: 1.0,
            clip_rect: None,
        }
    }
}

pub trait UiElement {
    fn rect(&self) -> [f32; 4];
    fn color(&self) -> [f32; 4];
    fn sprite(&self) -> Option<&SpriteRef>;
    fn text(&self) -> Option<UiText<'_>>;
    fn clip_rect(&self) -> Option<[f32; 4]> {
        None
    }
}

pub trait UiTextPainter {
    fn paint(
        &mut self,
        target: &mut UiTarget<'_>,
        text: &str,
        x: f32,
        y: f32,
        size_px: f32,
        color: [f32; 4],
    );

    fn paint_text(&mut self, target: &mut UiTarget<'_>, rect: [f32; 4], text: UiText<'_>) {
        let _ = rect;
        self.paint(target, text.text, text.x, text.y, text.size_px, text.color);
    }
}

pub struct UiTarget<'a> {
    pub image: &'a mut Rgba8,
    pub ids: Option<&'a mut Vec<u32>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiCoverage {
    pub id: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

pub fn composite_ui<T: UiElement>(
    target: &mut UiTarget<'_>,
    items: &[T],
    first_id: u32,
    painter: Option<&mut dyn UiTextPainter>,
) -> Vec<UiCoverage> {
    let mut painter = painter;
    let mut coverages = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let id = first_id + u32::try_from(index).unwrap_or(u32::MAX);
        let bounds = match pixel_bounds(item.rect(), target.image.width, target.image.height) {
            Some(bounds) => bounds,
            None => continue,
        };
        match item.text() {
            Some(text) => {
                if let Some(painter) = painter.as_deref_mut() {
                    painter.paint_text(target, item.rect(), text);
                }
            }
            None => {
                let draw_bounds = match item.clip_rect() {
                    Some(clip) => match intersect_bounds(
                        bounds,
                        pixel_bounds(clip, target.image.width, target.image.height),
                    ) {
                        Some(clipped) => clipped,
                        None => continue,
                    },
                    None => bounds,
                };
                if let Some((sprite, image)) = item.sprite().and_then(|s| {
                    ui_sprite_texture(&s.texture_guid)
                        .filter(|img| img.width > 0 && img.height > 0)
                        .map(|img| (s, img))
                }) {
                    let quad = crate::glyphs::TexturedQuad {
                        x: bounds.0 as f32,
                        y: bounds.1 as f32,
                        width: (bounds.2 - bounds.0) as f32,
                        height: (bounds.3 - bounds.1) as f32,
                        uv: sprite.uv_rect,
                        color: item.color(),
                    };
                    crate::glyphs::draw_image_quads(
                        &mut *target.image,
                        &image,
                        &[quad],
                        Rect {
                            x: draw_bounds.0 as f32,
                            y: draw_bounds.1 as f32,
                            width: (draw_bounds.2 - draw_bounds.0) as f32,
                            height: (draw_bounds.3 - draw_bounds.1) as f32,
                        },
                    );
                } else {
                    fill_rect(target, draw_bounds, item.color());
                }
            }
        }
        write_ids(target, bounds, id);
        coverages.push(UiCoverage {
            id,
            x: bounds.0,
            y: bounds.1,
            width: bounds.2 - bounds.0,
            height: bounds.3 - bounds.1,
        });
    }
    coverages
}

fn pixel_bounds(rect: [f32; 4], width: u32, height: u32) -> Option<(u32, u32, u32, u32)> {
    let x0 = rect[0].floor().max(0.0);
    let y0 = rect[1].floor().max(0.0);
    let x1 = (rect[0] + rect[2]).ceil().max(0.0).min(width as f32);
    let y1 = (rect[1] + rect[3]).ceil().max(0.0).min(height as f32);
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    Some((x0 as u32, y0 as u32, x1 as u32, y1 as u32))
}

fn intersect_bounds(
    a: (u32, u32, u32, u32),
    b: Option<(u32, u32, u32, u32)>,
) -> Option<(u32, u32, u32, u32)> {
    let b = b?;
    let x0 = a.0.max(b.0);
    let y0 = a.1.max(b.1);
    let x1 = a.2.min(b.2);
    let y1 = a.3.min(b.3);
    (x1 > x0 && y1 > y0).then_some((x0, y0, x1, y1))
}

fn fill_rect(target: &mut UiTarget<'_>, bounds: (u32, u32, u32, u32), color: [f32; 4]) {
    let (x0, y0, x1, y1) = bounds;
    let alpha = color[3].clamp(0.0, 1.0);
    if alpha <= 0.0 {
        return;
    }
    let source = [color[0] * alpha, color[1] * alpha, color[2] * alpha];
    let keep = 1.0 - alpha;
    let width = target.image.width as usize;
    let row_bytes = (x1 - x0) as usize * 4;
    for y in y0..y1 {
        let row_start = (y as usize * width + x0 as usize) * 4;
        let row = &mut target.image.pixels[row_start..row_start + row_bytes];
        for pixel in row.chunks_exact_mut(4) {
            blend_pixel(pixel, source, alpha, keep);
        }
    }
}

fn write_ids(target: &mut UiTarget<'_>, bounds: (u32, u32, u32, u32), id: u32) {
    let (x0, y0, x1, y1) = bounds;
    let width = target.image.width as usize;
    let ids = match target.ids.as_deref_mut() {
        Some(ids) => ids,
        None => return,
    };
    for y in y0..y1 {
        for x in x0..x1 {
            let index = y as usize * width + x as usize;
            if index < ids.len() {
                ids[index] = id;
            }
        }
    }
}

fn blend_pixel(pixel: &mut [u8], source: [f32; 3], alpha: f32, keep: f32) {
    for channel in 0..3 {
        let destination = f32::from(pixel[channel]) / 255.0;
        let blended = (source[channel] + destination * keep).clamp(0.0, 1.0);
        pixel[channel] = (blended * 255.0).round() as u8;
    }
    let destination_alpha = f32::from(pixel[3]) / 255.0;
    let blended_alpha = (alpha + destination_alpha * keep).clamp(0.0, 1.0);
    pixel[3] = (blended_alpha * 255.0).round() as u8;
}

pub struct GlyphTextPainter {
    pub fonts: crate::glyphs::FontLibrary,
    pub cache: crate::glyphs::SwashCache,
    pub atlas: crate::glyphs::GlyphAtlas,
    shaped: HashMap<ShapedText, Vec<crate::glyphs::TexturedQuad>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ShapedText {
    text: String,
    size_bits: u32,
    color_bits: [u32; 4],
    rect_w_bits: u32,
    rect_h_bits: u32,
    horizontal_align: HorizontalAlign,
    vertical_align: VerticalAlign,
    wrap: bool,
    max_width_bits: Option<u32>,
    line_spacing_bits: u32,
}

const SHAPED_TEXT_LIMIT: usize = 256;

impl GlyphTextPainter {
    pub fn new() -> Self {
        let mut fonts = crate::glyphs::FontLibrary::new();
        fonts.load_bundled_font();
        fonts.load_system_fonts();
        Self {
            fonts,
            cache: crate::glyphs::SwashCache::new(),
            atlas: crate::glyphs::GlyphAtlas::new(1024, 1024),
            shaped: HashMap::new(),
        }
    }
}

impl Default for GlyphTextPainter {
    fn default() -> Self {
        Self::new()
    }
}

impl UiTextPainter for GlyphTextPainter {
    fn paint(
        &mut self,
        target: &mut UiTarget<'_>,
        text: &str,
        x: f32,
        y: f32,
        size_px: f32,
        color: [f32; 4],
    ) {
        self.paint_text(
            target,
            [x, y, 0.0, 0.0],
            UiText::new(text, x, y, size_px, color),
        );
    }

    fn paint_text(&mut self, target: &mut UiTarget<'_>, rect: [f32; 4], text: UiText<'_>) {
        if text.text.is_empty() || text.size_px <= 0.0 {
            return;
        }
        let key = shaped_text_key(rect, &text);
        if !self.shaped.contains_key(&key) {
            if self.shaped.len() >= SHAPED_TEXT_LIMIT {
                self.shaped.clear();
            }
            let style = crate::glyphs::TextStyle::new(text.size_px, text.size_px, text.color);
            let layout_box = TextLayoutBox {
                rect: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: rect[2],
                    height: rect[3],
                },
                horizontal_align: text.horizontal_align,
                vertical_align: text.vertical_align,
                wrap: text.wrap,
                max_width: text.max_width,
                line_spacing: text.line_spacing,
            };
            let layout =
                crate::glyphs::layout_text_in_box(&mut self.fonts, text.text, &layout_box, &style);
            let quads = crate::glyphs::shape_glyph_quads(
                &mut self.fonts,
                &mut self.cache,
                &mut self.atlas,
                &layout,
            );
            self.shaped.insert(key.clone(), quads);
        }
        let mut quads = self.shaped.get(&key).cloned().unwrap_or_default();
        for quad in &mut quads {
            quad.x += text.x;
            quad.y += text.y;
        }
        let clip = resolve_clip_rect(target.image.width, target.image.height, text.clip_rect);
        crate::glyphs::draw_text(&mut *target.image, &self.atlas, &quads, clip);
    }
}

fn shaped_text_key(rect: [f32; 4], text: &UiText<'_>) -> ShapedText {
    ShapedText {
        text: text.text.to_string(),
        size_bits: text.size_px.to_bits(),
        color_bits: [
            text.color[0].to_bits(),
            text.color[1].to_bits(),
            text.color[2].to_bits(),
            text.color[3].to_bits(),
        ],
        rect_w_bits: rect[2].to_bits(),
        rect_h_bits: rect[3].to_bits(),
        horizontal_align: text.horizontal_align,
        vertical_align: text.vertical_align,
        wrap: text.wrap,
        max_width_bits: text.max_width.map(f32::to_bits),
        line_spacing_bits: text.line_spacing.to_bits(),
    }
}

fn resolve_clip_rect(width: u32, height: u32, clip_rect: Option<[f32; 4]>) -> Rect {
    let screen = Rect {
        x: 0.0,
        y: 0.0,
        width: width as f32,
        height: height as f32,
    };
    let Some(clip) = clip_rect else {
        return screen;
    };
    let x0 = clip[0].max(0.0);
    let y0 = clip[1].max(0.0);
    let x1 = (clip[0] + clip[2]).min(screen.width).max(x0);
    let y1 = (clip[1] + clip[3]).min(screen.height).max(y0);
    Rect {
        x: x0,
        y: y0,
        width: x1 - x0,
        height: y1 - y0,
    }
}

pub fn composite_ui_onto_capture<T: UiElement>(
    frame: &mut crate::capture::CaptureFrame,
    items: &[T],
    painter: Option<&mut dyn UiTextPainter>,
) -> Vec<crate::capture::CaptureObject> {
    let first_id = frame
        .objects
        .iter()
        .map(|object| object.id)
        .max()
        .unwrap_or(0)
        + 1;
    let mut target = UiTarget {
        image: &mut frame.image,
        ids: Some(&mut frame.id_map),
    };
    let coverages = composite_ui(&mut target, items, first_id, painter);
    let mut objects = Vec::with_capacity(coverages.len());
    for (index, coverage) in coverages.iter().enumerate() {
        objects.push(crate::capture::CaptureObject {
            id: coverage.id,
            name: format!("ui-{index}"),
            bounds: crate::capture::ScreenBounds {
                x: coverage.x,
                y: coverage.y,
                width: coverage.width,
                height: coverage.height,
            },
        });
    }
    frame.objects.extend(objects.iter().cloned());
    objects
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FlatItem {
        rect: [f32; 4],
        color: [f32; 4],
    }

    impl UiElement for FlatItem {
        fn rect(&self) -> [f32; 4] {
            self.rect
        }

        fn color(&self) -> [f32; 4] {
            self.color
        }

        fn sprite(&self) -> Option<&SpriteRef> {
            None
        }

        fn text(&self) -> Option<UiText<'_>> {
            None
        }
    }

    #[test]
    fn focus_border_rects_composite_visible_outline_around_button_without_covering_interior() {
        let button_rect = [10.0, 20.0, 40.0, 16.0];
        let edges = focus_border_rects(button_rect, 2.0);
        let mut items = vec![FlatItem {
            rect: button_rect,
            color: [0.2, 0.5, 0.3, 1.0],
        }];
        for edge in edges {
            items.push(FlatItem {
                rect: edge,
                color: FOCUS_INDICATOR_COLOR,
            });
        }
        let mut image = Rgba8::new(64, 64);
        let mut target = UiTarget {
            image: &mut image,
            ids: None,
        };
        let coverages = composite_ui(&mut target, &items, 1, None);
        assert_eq!(coverages.len(), 5);

        let top_edge_px = ((20 * 64) + 25) * 4;
        assert_eq!(
            &image.pixels[top_edge_px..top_edge_px + 4],
            &[255, 230, 0, 255]
        );
        let bottom_edge_px = ((35 * 64) + 25) * 4;
        assert_eq!(
            &image.pixels[bottom_edge_px..bottom_edge_px + 4],
            &[255, 230, 0, 255]
        );
        let interior_px = ((28 * 64) + 25) * 4;
        assert_eq!(
            &image.pixels[interior_px..interior_px + 4],
            &[51, 128, 77, 255]
        );
    }

    struct TextTestItem<'a> {
        rect: [f32; 4],
        text: UiText<'a>,
    }

    impl UiElement for TextTestItem<'_> {
        fn rect(&self) -> [f32; 4] {
            self.rect
        }

        fn color(&self) -> [f32; 4] {
            self.text.color
        }

        fn sprite(&self) -> Option<&SpriteRef> {
            None
        }

        fn text(&self) -> Option<UiText<'_>> {
            Some(self.text)
        }
    }

    #[test]
    fn default_ui_text_produces_pixel_identical_output_to_legacy_paint() {
        let mut painter_a = GlyphTextPainter::new();
        let mut image_a = Rgba8::new(160, 48);
        let mut target_a = UiTarget {
            image: &mut image_a,
            ids: None,
        };
        painter_a.paint(
            &mut target_a,
            "Score 0",
            16.0,
            12.0,
            18.0,
            [1.0, 1.0, 1.0, 1.0],
        );

        let mut painter_b = GlyphTextPainter::new();
        let mut image_b = Rgba8::new(160, 48);
        let mut target_b = UiTarget {
            image: &mut image_b,
            ids: None,
        };
        let items = [TextTestItem {
            rect: [16.0, 12.0, 320.0, 28.0],
            text: UiText::new("Score 0", 16.0, 12.0, 18.0, [1.0, 1.0, 1.0, 1.0]),
        }];
        composite_ui(&mut target_b, &items, 1, Some(&mut painter_b));

        assert_eq!(image_a.pixels, image_b.pixels);
    }

    #[test]
    fn ui_text_clip_rect_truncates_overflowing_glyphs() {
        let mut painter = GlyphTextPainter::new();
        let mut unclipped = Rgba8::new(160, 80);
        let mut target_unclipped = UiTarget {
            image: &mut unclipped,
            ids: None,
        };
        let base_text = UiText {
            text: "Line 1\nLine 2\nLine 3",
            x: 10.0,
            y: 10.0,
            size_px: 18.0,
            color: [1.0, 1.0, 1.0, 1.0],
            horizontal_align: HorizontalAlign::Left,
            vertical_align: VerticalAlign::Top,
            wrap: true,
            max_width: Some(120.0),
            line_spacing: 1.0,
            clip_rect: None,
        };
        composite_ui(
            &mut target_unclipped,
            &[TextTestItem {
                rect: [10.0, 10.0, 120.0, 60.0],
                text: base_text,
            }],
            1,
            Some(&mut painter),
        );

        let mut clipped = Rgba8::new(160, 80);
        let mut target_clipped = UiTarget {
            image: &mut clipped,
            ids: None,
        };
        let mut clipped_text = base_text;
        clipped_text.clip_rect = Some([10.0, 10.0, 120.0, 22.0]);
        composite_ui(
            &mut target_clipped,
            &[TextTestItem {
                rect: [10.0, 10.0, 120.0, 22.0],
                text: clipped_text,
            }],
            1,
            Some(&mut painter),
        );

        let has_ink_below_32_unclipped =
            (33..80).any(|y| (0..160).any(|x| unclipped.get_pixel(x, y)[3] > 0));
        let has_ink_below_32_clipped =
            (33..80).any(|y| (0..160).any(|x| clipped.get_pixel(x, y)[3] > 0));
        assert!(has_ink_below_32_unclipped);
        assert!(!has_ink_below_32_clipped);
    }
}
