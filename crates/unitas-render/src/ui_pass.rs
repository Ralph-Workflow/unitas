use crate::image::Rgba8;

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
}

pub trait UiElement {
    fn rect(&self) -> [f32; 4];
    fn color(&self) -> [f32; 4];
    fn sprite(&self) -> Option<&SpriteRef>;
    fn text(&self) -> Option<UiText<'_>>;
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
                    painter.paint(target, text.text, text.x, text.y, text.size_px, text.color);
                }
            }
            None => fill_rect(target, bounds, item.color()),
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

fn fill_rect(target: &mut UiTarget<'_>, bounds: (u32, u32, u32, u32), color: [f32; 4]) {
    let (x0, y0, x1, y1) = bounds;
    let width = target.image.width as usize;
    for y in y0..y1 {
        for x in x0..x1 {
            let offset = (y as usize * width + x as usize) * 4;
            blend_pixel(&mut target.image.pixels, offset, color);
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

fn blend_pixel(pixels: &mut [u8], offset: usize, color: [f32; 4]) {
    let alpha = color[3].clamp(0.0, 1.0);
    if alpha <= 0.0 {
        return;
    }
    for channel in 0..3 {
        let source = color[channel] * alpha;
        let destination = f32::from(pixels[offset + channel]) / 255.0;
        let blended = (source + destination * (1.0 - alpha)).clamp(0.0, 1.0);
        pixels[offset + channel] = (blended * 255.0).round() as u8;
    }
    let destination_alpha = f32::from(pixels[offset + 3]) / 255.0;
    let alpha = (alpha + destination_alpha * (1.0 - alpha)).clamp(0.0, 1.0);
    pixels[offset + 3] = (alpha * 255.0).round() as u8;
}

#[cfg(feature = "device")]
pub struct GlyphTextPainter {
    pub fonts: crate::glyphs::FontLibrary,
    pub cache: crate::glyphs::SwashCache,
    pub atlas: crate::glyphs::GlyphAtlas,
}

#[cfg(feature = "device")]
impl GlyphTextPainter {
    pub fn new() -> Self {
        let mut fonts = crate::glyphs::FontLibrary::new();
        fonts.load_system_fonts();
        Self {
            fonts,
            cache: crate::glyphs::SwashCache::new(),
            atlas: crate::glyphs::GlyphAtlas::new(1024, 1024),
        }
    }
}

#[cfg(feature = "device")]
impl Default for GlyphTextPainter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "device")]
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
        if text.is_empty() || size_px <= 0.0 {
            return;
        }
        let style = crate::glyphs::TextStyle::new(size_px, size_px, color);
        let layout = crate::glyphs::layout_text(&mut self.fonts, text, None, &style);
        let mut quads =
            crate::glyphs::rasterize(&mut self.fonts, &mut self.cache, &mut self.atlas, &layout);
        for quad in &mut quads {
            quad.x += x;
            quad.y += y;
        }
        let clip = crate::glyphs::Rect {
            x: 0.0,
            y: 0.0,
            width: target.image.width as f32,
            height: target.image.height as f32,
        };
        crate::glyphs::draw_text(&mut *target.image, &self.atlas, &quads, clip);
    }
}

#[cfg(feature = "device")]
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
