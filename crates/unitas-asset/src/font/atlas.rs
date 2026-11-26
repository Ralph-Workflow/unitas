use super::FontGlyph;

pub const ATLAS_WIDTH: u32 = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontAtlas {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl FontAtlas {
    pub fn coverage(&self, x: u32, y: u32) -> Option<u8> {
        if x >= self.width || y >= self.height {
            return None;
        }
        self.pixels.get((y * self.width + x) as usize).copied()
    }
}

pub fn pack_atlas(glyphs: &mut [FontGlyph], bitmaps: &[Vec<u8>], padding: u32) -> FontAtlas {
    let widest = glyphs.iter().map(|glyph| glyph.width).max().unwrap_or(0);
    let width = ATLAS_WIDTH.max(widest + padding * 2);
    let mut order: Vec<usize> = (0..glyphs.len()).collect();
    order.sort_by(|left, right| {
        let left = &glyphs[*left];
        let right = &glyphs[*right];
        right
            .height
            .cmp(&left.height)
            .then(left.code_point.cmp(&right.code_point))
    });
    let mut placements = vec![(0u32, 0u32); glyphs.len()];
    let mut x = padding;
    let mut y = padding;
    let mut shelf = 0u32;
    let mut height = padding;
    for index in order {
        let glyph = &glyphs[index];
        if glyph.width == 0 || glyph.height == 0 {
            continue;
        }
        if x + glyph.width + padding > width {
            x = padding;
            y += shelf + padding;
            shelf = 0;
        }
        placements[index] = (x, y);
        x += glyph.width + padding;
        shelf = shelf.max(glyph.height);
        height = height.max(y + glyph.height + padding);
    }
    let mut pixels = vec![0u8; (width * height) as usize];
    for (index, glyph) in glyphs.iter_mut().enumerate() {
        let (left, top) = placements[index];
        glyph.atlas_x = left;
        glyph.atlas_y = top;
        let bitmap = match bitmaps.get(index) {
            Some(bitmap) => bitmap,
            None => continue,
        };
        for row in 0..glyph.height {
            for column in 0..glyph.width {
                let source = (row * glyph.width + column) as usize;
                let target = ((top + row) * width + left + column) as usize;
                if let Some(value) = bitmap.get(source) {
                    if let Some(slot) = pixels.get_mut(target) {
                        *slot = *value;
                    }
                }
            }
        }
    }
    FontAtlas {
        width,
        height,
        pixels,
    }
}
