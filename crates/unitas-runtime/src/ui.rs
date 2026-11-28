/// Text produced by the component canvas for the render overlay path.
/// Mirrors `unitas_render::TextOverlayItem`, which is only available with the
/// device feature; the host maps these into the overlay verbatim.
#[derive(Clone, Debug, PartialEq)]
pub struct UiTextItem {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub size_px: f32,
    pub color: [f32; 4],
}

#[cfg(test)]
mod text_glyph_tests {
    use super::UiTextItem;
    use unitas_render::glyphs::{
        FontLibrary, GlyphAtlas, SwashCache, TextStyle, layout_text, shape_glyph_quads,
    };

    fn layout_at_scale(
        fonts: &mut FontLibrary,
        item: &UiTextItem,
        scale: f32,
    ) -> (f32, f32, usize) {
        let size_px = item.size_px * scale;
        let style = TextStyle::new(size_px, size_px * 1.25, item.color);
        let layout = layout_text(fonts, &item.text, None, &style);
        assert!(
            !layout.glyphs.is_empty(),
            "label text must shape into glyphs at scale {scale}"
        );
        let mut cache = SwashCache::new();
        let mut atlas = GlyphAtlas::new(1024, 1024);
        let quads = shape_glyph_quads(fonts, &mut cache, &mut atlas, &layout);
        assert!(
            !quads.is_empty(),
            "label text must pack into glyph quads at scale {scale}"
        );
        let max_quad_height = quads.iter().map(|quad| quad.height).fold(0.0f32, f32::max);
        (layout.width, max_quad_height, quads.len())
    }

    #[test]
    fn label_text_layout_produces_glyph_quads_at_multiple_resolutions() {
        let mut fonts = FontLibrary::new();
        if !fonts.load_system_fonts() {
            return;
        }
        let item = UiTextItem {
            text: "Score: 42".to_string(),
            x: 10.0,
            y: 10.0,
            size_px: 24.0,
            color: [1.0, 1.0, 1.0, 1.0],
        };
        let (width_lo, height_lo, quads_lo) = layout_at_scale(&mut fonts, &item, 1.0);
        let (width_hi, height_hi, quads_hi) = layout_at_scale(&mut fonts, &item, 2.0);
        assert!(width_lo > 0.0 && height_lo > 0.0);
        assert!(width_hi > width_lo, "higher resolution must reshape wider");
        assert!(
            (width_hi - width_lo * 2.0).abs() < width_lo * 0.2,
            "width should track the resolution scale, got {width_lo} vs {width_hi}"
        );
        assert!(
            height_hi > height_lo,
            "glyph quads must grow with the resolution scale"
        );
        assert!(quads_hi >= quads_lo);
    }
}
