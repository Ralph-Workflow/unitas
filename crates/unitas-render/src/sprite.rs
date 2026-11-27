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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Vec3;

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
}
