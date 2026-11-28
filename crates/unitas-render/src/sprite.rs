use std::ops::Range;
use std::sync::Arc;

use crate::image::Image;
use crate::math::Mat4;
use crate::mesh::{Mesh, MeshAttributes};

#[derive(Debug, Clone)]
pub struct Sprite {
    pub texture: Arc<Image>,
    pub model: Mat4,
    pub color: [f32; 4],
    pub sorting_layer: i32,
    pub sorting_order: i32,
}

impl Sprite {
    pub fn sort_key(&self) -> (i32, i32) {
        (self.sorting_layer, self.sorting_order)
    }
}

pub fn sprite_quad() -> (Mesh, MeshAttributes) {
    let positions = [
        [-0.5, -0.5, 0.0],
        [-0.5, 0.5, 0.0],
        [0.5, 0.5, 0.0],
        [0.5, -0.5, 0.0],
    ];
    let normals = [[0.0, 0.0, -1.0]; 4];
    let indices = [0u32, 1, 2, 0, 2, 3];
    let mesh = Mesh::from_buffers(&positions, &normals, &indices);
    let uvs = vec![[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]];
    let attributes = mesh.attributes_with_uvs(uvs);
    (mesh, attributes)
}

pub fn sort_sprites(sprites: &mut [Sprite]) {
    sprites.sort_by_key(Sprite::sort_key);
}

pub fn texture_runs(sprites: &[Sprite]) -> Vec<Range<usize>> {
    let mut runs = Vec::new();
    let mut start = 0;
    while start < sprites.len() {
        let mut end = start + 1;
        while end < sprites.len() && Arc::ptr_eq(&sprites[end].texture, &sprites[start].texture) {
            end += 1;
        }
        runs.push(start..end);
        start = end;
    }
    runs
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpritePart {
    pub source: [u32; 4],
    pub target: [f32; 4],
}

fn nine_slice_border(border: [f32; 4], source_size: [f32; 2], target_size: [f32; 2]) -> [f32; 4] {
    let [source_width, source_height] = source_size;
    let [target_width, target_height] = target_size;
    let [mut left, mut bottom, mut right, mut top] = border;
    left = left.clamp(0.0, source_width * 0.5);
    right = right.clamp(0.0, source_width * 0.5);
    bottom = bottom.clamp(0.0, source_height * 0.5);
    top = top.clamp(0.0, source_height * 0.5);
    if left + right > target_width {
        let shrink = target_width / (left + right);
        left *= shrink;
        right *= shrink;
    }
    if bottom + top > target_height {
        let shrink = target_height / (bottom + top);
        bottom *= shrink;
        top *= shrink;
    }
    [left, bottom, right, top]
}

fn nine_slice_grid_parts(
    source_x: [u32; 4],
    source_y: [u32; 4],
    target_x: [f32; 4],
    target_y: [f32; 4],
) -> Vec<SpritePart> {
    let mut parts = Vec::with_capacity(9);
    for row in 0..3 {
        for column in 0..3 {
            let width = target_x[column + 1] - target_x[column];
            let height = target_y[row + 1] - target_y[row];
            if width <= 0.0 || height <= 0.0 {
                continue;
            }
            parts.push(SpritePart {
                source: [
                    source_x[column],
                    source_y[row],
                    source_x[column + 1] - source_x[column],
                    source_y[row + 1] - source_y[row],
                ],
                target: [target_x[column], target_y[row], width, height],
            });
        }
    }
    parts
}

pub fn nine_slice_parts(
    border: [f32; 4],
    source_size: [f32; 2],
    target_size: [f32; 2],
) -> Vec<SpritePart> {
    let [source_width, source_height] = source_size;
    let [target_width, target_height] = target_size;
    if source_width <= 0.0 || source_height <= 0.0 || target_width <= 0.0 || target_height <= 0.0 {
        return Vec::new();
    }
    let [left, bottom, right, top] = nine_slice_border(border, source_size, target_size);
    let source_x = [
        0u32,
        left.round() as u32,
        (source_width - right).round() as u32,
        source_width.round() as u32,
    ];
    let source_y = [
        0u32,
        bottom.round() as u32,
        (source_height - top).round() as u32,
        source_height.round() as u32,
    ];
    let target_x = [0.0, left, target_width - right, target_width];
    let target_y = [0.0, bottom, target_height - top, target_height];
    nine_slice_grid_parts(source_x, source_y, target_x, target_y)
}

pub fn tiled_parts(source_size: [f32; 2], target_size: [f32; 2]) -> Vec<SpritePart> {
    let [source_width, source_height] = source_size;
    let [target_width, target_height] = target_size;
    if source_width <= 0.0 || source_height <= 0.0 || target_width <= 0.0 || target_height <= 0.0 {
        return Vec::new();
    }
    let columns = (target_width / source_width).ceil().max(1.0) as usize;
    let rows = (target_height / source_height).ceil().max(1.0) as usize;
    let source = [
        0u32,
        0u32,
        source_width.round() as u32,
        source_height.round() as u32,
    ];
    let mut parts = Vec::with_capacity(columns * rows);
    for row in 0..rows {
        for column in 0..columns {
            let x = column as f32 * source_width;
            let y = row as f32 * source_height;
            let width = (target_width - x).min(source_width);
            let height = (target_height - y).min(source_height);
            parts.push(SpritePart {
                source,
                target: [x, y, width, height],
            });
        }
    }
    parts
}

pub fn crop_texture(image: &Image, rect: [u32; 4]) -> Image {
    let [x, y, width, height] = rect;
    let mut cropped = Image::new(width, height);
    let first_row = image.height.saturating_sub(y.saturating_add(height));
    for row in 0..height {
        let source_row = first_row
            .saturating_add(row)
            .min(image.height.saturating_sub(1));
        for column in 0..width {
            let source_column = x.saturating_add(column).min(image.width.saturating_sub(1));
            cropped.set_pixel(column, row, image.get_pixel(source_column, source_row));
        }
    }
    cropped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Vec3;

    #[test]
    fn texture_runs_group_consecutive_sprites_sharing_one_texture() {
        let red = Arc::new(Image::new(1, 1));
        let blue = Arc::new(Image::new(1, 1));
        let make = |texture: &Arc<Image>| Sprite {
            texture: Arc::clone(texture),
            model: Mat4::identity(),
            color: [1.0, 1.0, 1.0, 1.0],
            sorting_layer: 0,
            sorting_order: 0,
        };
        let batched = vec![make(&red), make(&red), make(&red)];
        assert_eq!(texture_runs(&batched), vec![0..3]);
        let split = vec![make(&red), make(&red), make(&blue), make(&red)];
        assert_eq!(texture_runs(&split), vec![0..2, 2..3, 3..4]);
        assert!(texture_runs(&[]).is_empty());
    }

    #[test]
    fn sort_sprites_orders_by_layer_then_order_and_preserves_tie_order() {
        let texture = Arc::new(Image::new(1, 1));
        let make = |layer: i32, order: i32, tag: f32| Sprite {
            texture: Arc::clone(&texture),
            model: Mat4::identity(),
            color: [tag, 0.0, 0.0, 1.0],
            sorting_layer: layer,
            sorting_order: order,
        };
        let mut sprites = vec![
            make(2, 0, 1.0),
            make(1, 5, 2.0),
            make(1, 2, 3.0),
            make(1, 5, 4.0),
            make(-1, 10, 5.0),
        ];
        sort_sprites(&mut sprites);
        let ordered: Vec<(i32, i32, f32)> = sprites
            .iter()
            .map(|sprite| (sprite.sorting_layer, sprite.sorting_order, sprite.color[0]))
            .collect();
        assert_eq!(
            ordered,
            vec![
                (-1, 10, 5.0),
                (1, 2, 3.0),
                (1, 5, 2.0),
                (1, 5, 4.0),
                (2, 0, 1.0),
            ]
        );
    }

    #[test]
    fn sprite_quad_unit_geometry_uvs_and_flip_transform() {
        let (mesh, attributes) = sprite_quad();
        assert_eq!(
            mesh.positions,
            vec![
                Vec3::new(-0.5, -0.5, 0.0),
                Vec3::new(-0.5, 0.5, 0.0),
                Vec3::new(0.5, 0.5, 0.0),
                Vec3::new(0.5, -0.5, 0.0),
            ]
        );
        assert_eq!(mesh.normals, vec![Vec3::new(0.0, 0.0, -1.0); 4]);
        assert_eq!(mesh.indices, vec![0, 1, 2, 0, 2, 3]);
        assert_eq!(
            attributes.uvs,
            vec![[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]
        );

        let flip_xy = Mat4::from_cols_array([
            -2.0, 0.0, 0.0, 0.0, 0.0, -3.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]);
        let bottom_left = flip_xy.transform_point(mesh.positions[0]);
        let top_right = flip_xy.transform_point(mesh.positions[2]);
        assert_eq!(
            [bottom_left.x, bottom_left.y, bottom_left.z],
            [1.0, 1.5, 0.0]
        );
        assert_eq!([top_right.x, top_right.y, top_right.z], [-1.0, -1.5, 0.0]);
    }

    #[test]
    fn nine_slice_parts_splits_the_sprite_into_nine_regions() {
        let parts = nine_slice_parts([20.0, 20.0, 20.0, 20.0], [100.0, 100.0], [100.0, 100.0]);
        assert_eq!(parts.len(), 9);
        let expected = [
            ([0, 0, 20, 20], [0.0, 0.0, 20.0, 20.0]),
            ([20, 0, 60, 20], [20.0, 0.0, 60.0, 20.0]),
            ([80, 0, 20, 20], [80.0, 0.0, 20.0, 20.0]),
            ([0, 20, 20, 60], [0.0, 20.0, 20.0, 60.0]),
            ([20, 20, 60, 60], [20.0, 20.0, 60.0, 60.0]),
            ([80, 20, 20, 60], [80.0, 20.0, 20.0, 60.0]),
            ([0, 80, 20, 20], [0.0, 80.0, 20.0, 20.0]),
            ([20, 80, 60, 20], [20.0, 80.0, 60.0, 20.0]),
            ([80, 80, 20, 20], [80.0, 80.0, 20.0, 20.0]),
        ];
        for (index, (source, target)) in expected.iter().enumerate() {
            assert_eq!(parts[index].source, *source, "part {index} source");
            assert_eq!(parts[index].target, *target, "part {index} target");
        }
    }

    #[test]
    fn nine_slice_parts_keep_the_border_size_and_stretch_the_centre() {
        let parts = nine_slice_parts([20.0, 20.0, 20.0, 20.0], [100.0, 100.0], [200.0, 200.0]);
        assert_eq!(parts.len(), 9);
        assert_eq!(parts[0].target, [0.0, 0.0, 20.0, 20.0]);
        assert_eq!(parts[4].source, [20, 20, 60, 60]);
        assert_eq!(parts[4].target, [20.0, 20.0, 160.0, 160.0]);
        assert_eq!(parts[8].target, [180.0, 180.0, 20.0, 20.0]);
    }

    #[test]
    fn nine_slice_parts_without_a_border_draw_the_whole_sprite() {
        let parts = nine_slice_parts([0.0, 0.0, 0.0, 0.0], [100.0, 50.0], [100.0, 50.0]);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].source, [0, 0, 100, 50]);
        assert_eq!(parts[0].target, [0.0, 0.0, 100.0, 50.0]);
    }

    #[test]
    fn tiled_parts_repeat_the_sprite_rect_and_clip_the_last_row() {
        let parts = tiled_parts([100.0, 50.0], [250.0, 50.0]);
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0].target, [0.0, 0.0, 100.0, 50.0]);
        assert_eq!(parts[1].target, [100.0, 0.0, 100.0, 50.0]);
        assert_eq!(parts[2].target, [200.0, 0.0, 50.0, 50.0]);
        for part in &parts {
            assert_eq!(part.source, [0, 0, 100, 50]);
        }
    }

    #[test]
    fn crop_texture_cuts_the_region_measured_from_the_bottom() {
        let mut image = Image::new(2, 2);
        image.set_pixel(0, 0, [1, 1, 1]);
        image.set_pixel(1, 0, [2, 2, 2]);
        image.set_pixel(0, 1, [3, 3, 3]);
        image.set_pixel(1, 1, [4, 4, 4]);
        let cropped = crop_texture(&image, [1, 0, 1, 1]);
        assert_eq!(cropped.width, 1);
        assert_eq!(cropped.height, 1);
        assert_eq!(cropped.get_pixel(0, 0), [4, 4, 4]);
        let top = crop_texture(&image, [0, 1, 2, 1]);
        assert_eq!(top.width, 2);
        assert_eq!(top.get_pixel(0, 0), [1, 1, 1]);
        assert_eq!(top.get_pixel(1, 0), [2, 2, 2]);
    }
}
