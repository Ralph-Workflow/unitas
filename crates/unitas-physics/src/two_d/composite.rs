use std::collections::BTreeMap;

use crate::two_d::collider::Shape2D;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CompositeGeometryType2D {
    #[default]
    Outlines,
    Polygons,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CompositeGenerationType2D {
    #[default]
    Synchronous,
    Manual,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompositeCollider2D {
    pub geometry_type: CompositeGeometryType2D,
    pub generation_type: CompositeGenerationType2D,
    pub vertex_distance: f32,
    pub offset_distance: f32,
    pub edge_radius: f32,
}

impl Default for CompositeCollider2D {
    fn default() -> Self {
        Self {
            geometry_type: CompositeGeometryType2D::Outlines,
            generation_type: CompositeGenerationType2D::Synchronous,
            vertex_distance: 0.000_05,
            offset_distance: 0.000_005,
            edge_radius: 0.0,
        }
    }
}

impl CompositeCollider2D {
    pub fn new(
        geometry_type: CompositeGeometryType2D,
        generation_type: CompositeGenerationType2D,
    ) -> Self {
        Self {
            geometry_type,
            generation_type,
            ..Self::default()
        }
    }

    pub fn with_geometry_type(mut self, geometry_type: CompositeGeometryType2D) -> Self {
        self.geometry_type = geometry_type;
        self
    }

    pub fn with_generation_type(mut self, generation_type: CompositeGenerationType2D) -> Self {
        self.generation_type = generation_type;
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TileColliderType2D {
    None,
    Grid,
    Sprite { vertices: Vec<[f32; 2]> },
}

#[derive(Clone, Debug, PartialEq)]
pub struct TilemapCollider2D {
    pub cell_size: [f32; 2],
    pub used_by_composite: bool,
    tiles: BTreeMap<[i32; 2], TileColliderType2D>,
}

impl Default for TilemapCollider2D {
    fn default() -> Self {
        Self::new([1.0, 1.0])
    }
}

impl TilemapCollider2D {
    pub fn new(cell_size: [f32; 2]) -> Self {
        Self {
            cell_size,
            used_by_composite: false,
            tiles: BTreeMap::new(),
        }
    }

    pub fn with_used_by_composite(mut self, used_by_composite: bool) -> Self {
        self.used_by_composite = used_by_composite;
        self
    }

    pub fn set_tile(&mut self, cell: [i32; 2], collider_type: TileColliderType2D) {
        if matches!(collider_type, TileColliderType2D::None) {
            self.tiles.remove(&cell);
        } else {
            self.tiles.insert(cell, collider_type);
        }
    }

    pub fn remove_tile(&mut self, cell: [i32; 2]) -> bool {
        self.tiles.remove(&cell).is_some()
    }

    pub fn tile(&self, cell: [i32; 2]) -> Option<&TileColliderType2D> {
        self.tiles.get(&cell)
    }

    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    pub fn build_shapes(&self) -> Vec<Shape2D> {
        let cw = self.cell_size[0].max(0.0);
        let ch = self.cell_size[1].max(0.0);
        if cw <= 0.0 || ch <= 0.0 {
            return Vec::new();
        }
        let mut grid_rows: BTreeMap<i32, Vec<i32>> = BTreeMap::new();
        let mut shapes = Vec::new();
        for (cell, kind) in &self.tiles {
            match kind {
                TileColliderType2D::None => {}
                TileColliderType2D::Grid => {
                    grid_rows.entry(cell[1]).or_default().push(cell[0]);
                }
                TileColliderType2D::Sprite { vertices } => {
                    if vertices.len() >= 3 {
                        let base_x = (cell[0] as f32) * cw;
                        let base_y = (cell[1] as f32) * ch;
                        let world_verts = vertices
                            .iter()
                            .map(|v| [base_x + v[0] * cw, base_y + v[1] * ch])
                            .collect();
                        shapes.push(Shape2D::Polygon {
                            vertices: world_verts,
                        });
                    }
                }
            }
        }
        for (y, mut xs) in grid_rows {
            xs.sort_unstable();
            xs.dedup();
            let mut idx = 0usize;
            while idx < xs.len() {
                let start_x = xs[idx];
                let mut end_x = start_x;
                while idx + 1 < xs.len() && xs[idx + 1] == end_x + 1 {
                    idx += 1;
                    end_x = xs[idx];
                }
                let min_x = (start_x as f32) * cw;
                let max_x = ((end_x + 1) as f32) * cw;
                let min_y = (y as f32) * ch;
                let max_y = ((y + 1) as f32) * ch;
                shapes.push(Shape2D::Polygon {
                    vertices: vec![
                        [min_x, min_y],
                        [max_x, min_y],
                        [max_x, max_y],
                        [min_x, max_y],
                    ],
                });
                idx += 1;
            }
        }
        shapes
    }
}
