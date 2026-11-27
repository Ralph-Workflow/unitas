use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use unitas_asset::scene::{LightComponent, MeshRendererComponent, RenderBinding, SpriteDrawMode};
use unitas_core::{EntityId, IDENTITY_ROTATION, SceneGraph, Transform, rotate_vector};
use unitas_render::image::{Image, Rgba8};
use unitas_render::ui_pass::{GlyphTextPainter, UiTarget, composite_ui};
use unitas_render::{
    Camera, DirectionalLight, Mat4, Mesh, MeshAttributes, Renderer, Vec3, cube,
    gpu::{GpuLight, GpuLightKind, MAX_GPU_LIGHTS},
    look_at,
};

use super::{LoadedScene, Runtime};
use crate::canvas::UiDrawItem;
use crate::error::RenderError;
use crate::presenter::{GpuCameraBinding, scene_gpu_meshes};

pub const DEFAULT_MATERIAL_COLOR: [f32; 4] = [0.5, 0.5, 0.5, 1.0];
const SPRITE_PIXELS_PER_UNIT: f32 = 100.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderCameraParams {
    pub orthographic: bool,
    pub orthographic_size: f32,
    pub field_of_view: f32,
    pub near_clip_plane: f32,
    pub far_clip_plane: f32,
    pub background_color: [f32; 3],
}

impl Default for RenderCameraParams {
    fn default() -> Self {
        Self {
            orthographic: false,
            orthographic_size: 5.0,
            field_of_view: 60.0,
            near_clip_plane: 0.3,
            far_clip_plane: 1000.0,
            background_color: [0.0; 3],
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RenderMaterial {
    pub color: [f32; 4],
    pub albedo: Option<Image>,
    pub normal: Option<Image>,
    pub emissive: Option<Image>,
    pub metallic: f32,
    pub smoothness: f32,
    pub emission_color: Option<[f32; 4]>,
}

impl RenderMaterial {
    pub fn plain() -> Self {
        Self {
            color: DEFAULT_MATERIAL_COLOR,
            albedo: None,
            normal: None,
            emissive: None,
            metallic: 0.0,
            smoothness: 0.0,
            emission_color: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderCamera {
    pub entity: Option<EntityId>,
    pub params: RenderCameraParams,
    pub viewport: Option<(u32, u32)>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderRenderer {
    pub entity: Option<EntityId>,
    pub mesh: Option<u64>,
    pub material: Option<u64>,
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct SceneRenderable<'a> {
    pub entity: EntityId,
    pub mesh: &'a Mesh,
    pub attributes: &'a MeshAttributes,
    pub material: &'a RenderMaterial,
    pub color: [f32; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct RenderMesh {
    pub mesh: Mesh,
    pub attributes: MeshAttributes,
}

#[derive(Clone, Copy, Debug)]
pub struct SceneCamera {
    pub entity: Option<EntityId>,
    pub params: RenderCameraParams,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderLight {
    pub entity: Option<EntityId>,
    pub kind: GpuLightKind,
    pub position: [f32; 3],
    pub direction: [f32; 3],
    pub color: [f32; 3],
    pub intensity: f32,
    pub range: f32,
    pub inner_spot_angle_deg: f32,
    pub spot_angle_deg: f32,
}

#[derive(Clone, Debug)]
pub struct RenderSprite {
    pub entity: EntityId,
    pub texture: Option<Arc<Image>>,
    pub color: [f32; 4],
    pub sorting_layer: i32,
    pub sorting_order: i32,
    pub size: [f32; 2],
    pub flip_x: bool,
    pub flip_y: bool,
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug)]
struct CameraBasis {
    eye: Vec3,
    right: Vec3,
    up: Vec3,
    forward: Vec3,
    view: Mat4,
}

#[derive(Debug)]
pub struct RenderWorld {
    meshes: HashMap<u64, RenderMesh>,
    materials: HashMap<u64, RenderMaterial>,
    plain_material: RenderMaterial,
    cameras: HashMap<u64, RenderCamera>,
    renderers: HashMap<u64, RenderRenderer>,
    lights: HashMap<u64, RenderLight>,
    ambient: [f32; 3],
    sprites: Vec<RenderSprite>,
    camera_defaults: HashMap<EntityId, RenderCameraParams>,
    light_defaults: HashMap<EntityId, RenderLight>,
    effect_mesh: Mesh,
    next_handle: u64,
}

impl Default for RenderWorld {
    fn default() -> Self {
        Self {
            meshes: HashMap::new(),
            materials: HashMap::new(),
            plain_material: RenderMaterial::plain(),
            cameras: HashMap::new(),
            renderers: HashMap::new(),
            lights: HashMap::new(),
            ambient: [0.0; 3],
            sprites: Vec::new(),
            camera_defaults: HashMap::new(),
            light_defaults: HashMap::new(),
            effect_mesh: cube(1.0),
            next_handle: 0,
        }
    }
}

impl RenderWorld {
    fn allocate(&mut self) -> u64 {
        self.next_handle = self.next_handle.saturating_add(1);
        self.next_handle
    }
}

fn toward_light_direction(rotation: [f32; 4], travel: [f32; 3]) -> [f32; 3] {
    let rotated = rotate_vector(rotation, travel);
    [-rotated[0], -rotated[1], -rotated[2]]
}

pub(crate) fn projection_for(params: &RenderCameraParams, aspect: f32) -> Camera {
    let near = params.near_clip_plane.max(0.001);
    let far = params.far_clip_plane.max(near + 0.001);
    if params.orthographic {
        Camera::orthographic(params.orthographic_size.max(0.001), aspect, near, far)
    } else {
        Camera::perspective(params.field_of_view.max(1.0), aspect, near, far)
    }
}

impl Runtime {
    pub(super) fn render_viewport(&self) -> Option<(u32, u32)> {
        self.render
            .cameras
            .values()
            .find_map(|camera| camera.viewport)
    }

    pub(crate) fn create_render_mesh(
        &mut self,
        positions: Vec<Vec3>,
        normals: Vec<Vec3>,
        indices: Vec<u32>,
    ) -> u64 {
        self.create_render_mesh_with_uvs(positions, normals, indices, Vec::new())
    }

    pub(crate) fn create_render_mesh_with_uvs(
        &mut self,
        positions: Vec<Vec3>,
        normals: Vec<Vec3>,
        indices: Vec<u32>,
        uvs: Vec<[f32; 2]>,
    ) -> u64 {
        let handle = self.render.allocate();
        let mesh = Mesh {
            positions,
            normals,
            indices,
        };
        let attributes = MeshAttributes::compute(&mesh, uvs);
        self.render
            .meshes
            .insert(handle, RenderMesh { mesh, attributes });
        handle
    }

    pub(crate) fn destroy_render_mesh(&mut self, handle: u64) -> bool {
        self.render.meshes.remove(&handle).is_some()
    }

    pub(crate) fn create_render_material(&mut self) -> u64 {
        let handle = self.render.allocate();
        self.render
            .materials
            .insert(handle, RenderMaterial::plain());
        handle
    }

    pub(crate) fn set_render_material(&mut self, handle: u64, material: RenderMaterial) -> bool {
        match self.render.materials.get_mut(&handle) {
            Some(slot) => {
                *slot = material;
                true
            }
            None => false,
        }
    }

    pub(crate) fn set_render_material_color(&mut self, handle: u64, color: [f32; 4]) -> bool {
        match self.render.materials.get_mut(&handle) {
            Some(material) => {
                material.color = color;
                true
            }
            None => false,
        }
    }

    pub(crate) fn create_render_camera(&mut self, entity: Option<EntityId>) -> u64 {
        let handle = self.render.allocate();
        let params = entity
            .and_then(|entity| self.render.camera_defaults.get(&entity).copied())
            .unwrap_or_default();
        self.render.cameras.insert(
            handle,
            RenderCamera {
                entity,
                params,
                viewport: None,
            },
        );
        handle
    }

    pub(crate) fn update_render_camera(&mut self, handle: u64, params: RenderCameraParams) -> bool {
        match self.render.cameras.get_mut(&handle) {
            Some(camera) => {
                camera.params = params;
                true
            }
            None => false,
        }
    }

    pub(crate) fn create_render_renderer(
        &mut self,
        entity: Option<EntityId>,
        mesh: Option<u64>,
        material: Option<u64>,
    ) -> u64 {
        let handle = self.render.allocate();
        self.render.renderers.insert(
            handle,
            RenderRenderer {
                entity,
                mesh,
                material,
                enabled: true,
            },
        );
        handle
    }

    pub(crate) fn set_render_renderer_enabled(&mut self, handle: u64, enabled: bool) -> bool {
        match self.render.renderers.get_mut(&handle) {
            Some(renderer) => {
                renderer.enabled = enabled;
                true
            }
            None => false,
        }
    }

    pub(crate) fn create_render_light(&mut self, entity: Option<EntityId>) -> u64 {
        let handle = self.render.allocate();
        let mut light = entity
            .and_then(|entity| self.render.light_defaults.get(&entity).copied())
            .unwrap_or(RenderLight {
                entity,
                kind: GpuLightKind::Directional,
                position: [0.0, 0.0, 0.0],
                direction: [0.0, 1.0, 0.0],
                color: [1.0, 1.0, 1.0],
                intensity: 1.0,
                range: 10.0,
                inner_spot_angle_deg: 21.80208,
                spot_angle_deg: 30.0,
            });
        light.entity = entity;
        self.render.lights.insert(handle, light);
        handle
    }

    pub(crate) fn update_render_light(
        &mut self,
        handle: u64,
        direction: [f32; 3],
        color: [f32; 3],
        intensity: f32,
    ) -> bool {
        match self.render.lights.get_mut(&handle) {
            Some(light) => {
                light.direction = direction;
                light.color = color;
                light.intensity = intensity;
                true
            }
            None => false,
        }
    }

    pub(crate) fn update_render_light_full(&mut self, handle: u64, updated: RenderLight) -> bool {
        match self.render.lights.get_mut(&handle) {
            Some(light) => {
                let entity = light.entity;
                *light = updated;
                light.entity = entity;
                true
            }
            None => false,
        }
    }

    pub fn set_ambient_light(&mut self, ambient: [f32; 3]) {
        self.render.ambient = [
            ambient[0].max(0.0),
            ambient[1].max(0.0),
            ambient[2].max(0.0),
        ];
    }

    pub fn ambient(&self) -> [f32; 3] {
        self.render.ambient
    }

    pub(super) fn populate_render_world(
        &mut self,
        binding: &RenderBinding,
        entity_base: u64,
        graph: &SceneGraph,
    ) -> Vec<(EntityId, String)> {
        let mut diagnostics = Vec::new();
        for camera in &binding.cameras {
            let entity = EntityId::new(entity_base + camera.entity.as_u64());
            let params = RenderCameraParams {
                orthographic: camera.orthographic,
                orthographic_size: camera.orthographic_size,
                field_of_view: camera.field_of_view,
                near_clip_plane: camera.near_clip_plane,
                far_clip_plane: camera.far_clip_plane,
                background_color: camera.background_color,
            };
            self.render.camera_defaults.insert(entity, params);
            let handle = self.create_render_camera(Some(entity));
            self.update_render_camera(handle, params);
        }
        for renderer in &binding.renderers {
            let entity = EntityId::new(entity_base + renderer.entity.as_u64());
            let material = self.create_render_material();
            self.set_render_material(material, render_material_for(renderer));
            let mesh = self.import_scene_mesh(renderer);
            let handle = self.create_render_renderer(Some(entity), mesh, Some(material));
            self.set_render_renderer_enabled(handle, renderer.enabled);
            if mesh.is_none() {
                if let Some(diagnostic) = &renderer.mesh_diagnostic {
                    diagnostics.push((entity, diagnostic.clone()));
                } else if let Some(guid) = &renderer.mesh_guid {
                    diagnostics.push((
                        entity,
                        format!(
                            "MeshRenderer on entity {} references mesh guid {} that could not be imported",
                            entity.as_u64(),
                            guid
                        ),
                    ));
                }
            }
            if let Some(diagnostic) = &renderer.material_diagnostic {
                diagnostics.push((entity, diagnostic.clone()));
            } else if renderer.material_path.is_none()
                && let Some(guid) = &renderer.material_guid
                && !is_builtin_material_reference(guid, None)
            {
                diagnostics.push((
                    entity,
                    format!(
                        "MeshRenderer on entity {} references material guid {} that could not be imported",
                        entity.as_u64(),
                        guid
                    ),
                ));
            }
        }
        for sprite in &binding.sprites {
            let entity = EntityId::new(entity_base + sprite.entity.as_u64());
            let texture = load_texture_image(sprite.sprite_path.as_deref()).map(Arc::new);
            if let Some(diagnostic) = &sprite.diagnostic {
                diagnostics.push((entity, diagnostic.clone()));
            }
            if texture.is_none() {
                diagnostics.push((
                    entity,
                    format!(
                        "SpriteRenderer on entity {} references sprite guid {} that could not be imported",
                        entity.as_u64(),
                        sprite.sprite_guid.as_deref().unwrap_or("none")
                    ),
                ));
            }
            let size = match texture.as_ref() {
                Some(image) => [
                    image.width as f32 / SPRITE_PIXELS_PER_UNIT,
                    image.height as f32 / SPRITE_PIXELS_PER_UNIT,
                ],
                None => [0.0, 0.0],
            };
            self.render.sprites.push(RenderSprite {
                entity,
                texture,
                color: sprite.color,
                sorting_layer: sprite.sorting_layer,
                sorting_order: sprite.sorting_order,
                size,
                flip_x: sprite.flip_x,
                flip_y: sprite.flip_y,
                enabled: sprite.enabled,
            });
        }
        for light in &binding.lights {
            let entity = EntityId::new(entity_base + light.entity.as_u64());
            let Some(kind) = scene_light_kind(light) else {
                if let Some(diagnostic) = &light.unsupported_diagnostic {
                    diagnostics.push((entity, diagnostic.clone()));
                }
                continue;
            };
            let (position, rotation) = graph
                .world_transform(entity)
                .map(|transform| (transform.translation, transform.rotation))
                .unwrap_or(([0.0, 0.0, 0.0], IDENTITY_ROTATION));
            let travel = match kind {
                GpuLightKind::Directional => light.direction,
                GpuLightKind::Point | GpuLightKind::Spot => [0.0, 0.0, 1.0],
            };
            let direction = toward_light_direction(rotation, travel);
            let render_light = RenderLight {
                entity: Some(entity),
                kind,
                position,
                direction,
                color: light.color,
                intensity: light.intensity,
                range: 10.0,
                inner_spot_angle_deg: 21.80208,
                spot_angle_deg: 30.0,
            };
            self.render.light_defaults.insert(entity, render_light);
            let handle = self.create_render_light(Some(entity));
            self.update_render_light_full(handle, render_light);
        }
        diagnostics
    }

    fn import_scene_mesh(&mut self, renderer: &MeshRendererComponent) -> Option<u64> {
        let mesh_data = if let (Some(file_id), Some(guid)) =
            (renderer.mesh_file_id, renderer.mesh_guid.as_deref())
        {
            let primitive = unitas_asset::primitive::builtin_primitive_by_identity(file_id, guid)?;
            let mut data = unitas_asset::primitive::primitive_mesh(primitive);
            if primitive == unitas_asset::primitive::BuiltinPrimitive::Sphere {
                for position in &mut data.positions {
                    *position = [position[0] * 0.5, position[1] * 0.5, position[2] * 0.5];
                }
            }
            Some(data)
        } else if let Some(path) = &renderer.mesh_path {
            let bytes = std::fs::read(path).ok()?;
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            unitas_asset::model::import_model(&bytes, name).ok()
        } else {
            None
        }?;
        let positions = mesh_data
            .positions
            .iter()
            .map(|p| Vec3::new(p[0], p[1], p[2]))
            .collect();
        let normals = mesh_data
            .normals
            .iter()
            .map(|n| Vec3::new(n[0], n[1], n[2]))
            .collect();
        Some(self.create_render_mesh_with_uvs(
            positions,
            normals,
            mesh_data.indices.clone(),
            mesh_data.uvs.clone(),
        ))
    }

    pub(super) fn remove_scene_render_handles(&mut self, scene: &LoadedScene) {
        let base = scene.entity_base;
        let count = scene.entity_count;
        let in_scene = |entity: Option<EntityId>| match entity {
            Some(entity) => {
                let value = entity.as_u64();
                value >= base && value < base + count
            }
            None => false,
        };
        self.render
            .cameras
            .retain(|_, camera| !in_scene(camera.entity));
        self.render
            .renderers
            .retain(|_, renderer| !in_scene(renderer.entity));
        self.render
            .lights
            .retain(|_, light| !in_scene(light.entity));
        self.render
            .sprites
            .retain(|sprite| !in_scene(Some(sprite.entity)));
        self.render
            .camera_defaults
            .retain(|entity, _| !in_scene(Some(*entity)));
        self.render
            .light_defaults
            .retain(|entity, _| !in_scene(Some(*entity)));
    }

    fn camera_basis(&self, entity: Option<EntityId>) -> Option<CameraBasis> {
        let entity = entity?;
        let transform = self.entity_transform(entity)?;
        let eye = Vec3::new(
            transform.translation[0],
            transform.translation[1],
            transform.translation[2],
        );
        let rotated = rotate_vector(transform.rotation, [0.0, 0.0, 1.0]);
        let mut forward = Vec3::new(rotated[0], rotated[1], rotated[2]);
        if forward.length_squared() < 1e-8 {
            forward = Vec3::new(0.0, 0.0, 1.0);
        }
        let forward = forward.normalize();
        let mut up = Vec3::new(0.0, 1.0, 0.0);
        if up.cross(forward).length_squared() < 1e-8 {
            up = Vec3::new(0.0, 0.0, 1.0);
        }
        let view = look_at(eye, eye + forward, up);
        let right = Vec3::new(view.get(0, 0), view.get(1, 0), view.get(2, 0));
        let up_axis = Vec3::new(view.get(0, 1), view.get(1, 1), view.get(2, 1));
        Some(CameraBasis {
            eye,
            right,
            up: up_axis,
            forward,
            view,
        })
    }

    fn primary_directional_handle(&self) -> Option<(u64, RenderLight)> {
        let mut selected: Option<(u64, RenderLight)> = None;
        for (handle, light) in &self.render.lights {
            if light.kind != GpuLightKind::Directional {
                continue;
            }
            let replace = match selected {
                Some((best, _)) => *handle > best,
                None => true,
            };
            if replace {
                selected = Some((*handle, *light));
            }
        }
        selected
    }

    pub fn directional_light(&self) -> DirectionalLight {
        match self.primary_directional_handle() {
            Some((_, light)) => DirectionalLight {
                direction: Vec3::new(light.direction[0], light.direction[1], light.direction[2]),
                color: light.color,
                intensity: light.intensity,
            },
            None if !self.render.lights.is_empty() => DirectionalLight {
                direction: Vec3::new(0.0, 1.0, 0.0),
                color: [0.0, 0.0, 0.0],
                intensity: 1.0,
            },
            None => DirectionalLight {
                direction: Vec3::new(0.0, 1.0, 0.0),
                color: [1.0, 1.0, 1.0],
                intensity: 1.0,
            },
        }
    }

    pub fn lights(&self) -> Vec<GpuLight> {
        let primary_handle = self.primary_directional_handle().map(|(handle, _)| handle);
        let mut handles: Vec<u64> = self.render.lights.keys().copied().collect();
        handles.sort_unstable();
        let mut collected = Vec::new();
        for handle in handles {
            if Some(handle) == primary_handle {
                continue;
            }
            let Some(light) = self.render.lights.get(&handle) else {
                continue;
            };
            let (position, direction) = match light.entity.and_then(|e| self.world_transform_of(e))
            {
                Some(transform) => {
                    let pos = Vec3::new(
                        transform.translation[0],
                        transform.translation[1],
                        transform.translation[2],
                    );
                    let dir = Vec3::new(light.direction[0], light.direction[1], light.direction[2]);
                    (pos, dir)
                }
                None => (
                    Vec3::new(light.position[0], light.position[1], light.position[2]),
                    Vec3::new(light.direction[0], light.direction[1], light.direction[2]),
                ),
            };
            collected.push(GpuLight {
                kind: light.kind,
                position,
                direction,
                color: light.color,
                intensity: light.intensity,
                range: light.range,
                inner_spot_angle_deg: light.inner_spot_angle_deg,
                spot_angle_deg: light.spot_angle_deg,
            });
            if collected.len() >= MAX_GPU_LIGHTS {
                break;
            }
        }
        collected
    }

    pub fn scene_renderables(&self) -> Vec<SceneRenderable<'_>> {
        let mut handles: Vec<u64> = self.render.renderers.keys().copied().collect();
        handles.sort_unstable();
        handles
            .iter()
            .filter_map(|handle| {
                let renderer = self.render.renderers.get(handle)?;
                if !renderer.enabled {
                    return None;
                }
                let entity = renderer.entity?;
                let render_mesh = renderer
                    .mesh
                    .and_then(|mesh| self.render.meshes.get(&mesh))?;
                let material = renderer
                    .material
                    .and_then(|material| self.render.materials.get(&material))
                    .unwrap_or(&self.render.plain_material);
                let color = [material.color[0], material.color[1], material.color[2]];
                Some(SceneRenderable {
                    entity,
                    mesh: &render_mesh.mesh,
                    attributes: &render_mesh.attributes,
                    material,
                    color,
                })
            })
            .collect()
    }

    pub fn scene_camera(&self) -> Option<SceneCamera> {
        let mut selected: Option<(u64, RenderCamera)> = None;
        for (handle, camera) in &self.render.cameras {
            let replace = match selected {
                Some((best, _)) => *handle > best,
                None => true,
            };
            if replace {
                selected = Some((*handle, *camera));
            }
        }
        selected.map(|(_, camera)| SceneCamera {
            entity: camera.entity,
            params: camera.params,
        })
    }

    pub fn sprite_instances(&self) -> Vec<unitas_render::Sprite> {
        let mut sprites: Vec<unitas_render::Sprite> = self
            .render
            .sprites
            .iter()
            .filter(|sprite| sprite.enabled)
            .filter_map(|sprite| {
                let texture = sprite.texture.clone()?;
                let transform = self.entity_transform(sprite.entity)?;
                Some(unitas_render::Sprite {
                    texture,
                    model: sprite_model(&transform, sprite),
                    color: sprite.color,
                    sorting_layer: sprite.sorting_layer,
                    sorting_order: sprite.sorting_order,
                })
            })
            .collect();
        unitas_render::sort_sprites(&mut sprites);
        sprites
    }

    fn runtime_sprite_index(&self, entity: EntityId) -> Option<usize> {
        self.render
            .sprites
            .iter()
            .position(|sprite| sprite.entity == entity)
    }

    pub fn create_runtime_sprite(&mut self, entity: EntityId, texture_path: Option<&Path>) -> bool {
        if entity.as_u64() == 0 {
            return false;
        }
        let texture = load_texture_image(texture_path).map(Arc::new);
        if texture.is_none()
            && let Some(path) = texture_path
        {
            self.push_runtime_diagnostic(
                entity,
                format!(
                    "SpriteRenderer on entity {} references sprite {} that could not be imported",
                    entity.as_u64(),
                    path.display()
                ),
            );
        }
        let size = match texture.as_ref() {
            Some(image) => [
                image.width as f32 / SPRITE_PIXELS_PER_UNIT,
                image.height as f32 / SPRITE_PIXELS_PER_UNIT,
            ],
            None => [0.0, 0.0],
        };
        match self.runtime_sprite_index(entity) {
            Some(index) => {
                let sprite = &mut self.render.sprites[index];
                sprite.texture = texture;
                sprite.size = size;
            }
            None => self.render.sprites.push(RenderSprite {
                entity,
                texture,
                color: [1.0, 1.0, 1.0, 1.0],
                sorting_layer: 0,
                sorting_order: 0,
                size,
                flip_x: false,
                flip_y: false,
                enabled: true,
            }),
        }
        true
    }

    pub fn set_runtime_sprite_color(&mut self, entity: EntityId, color: [f32; 4]) -> bool {
        let Some(index) = self.runtime_sprite_index(entity) else {
            return false;
        };
        self.render.sprites[index].color = color;
        true
    }

    pub fn set_runtime_sprite_sorting(
        &mut self,
        entity: EntityId,
        sorting_layer: i32,
        sorting_order: i32,
    ) -> bool {
        let Some(index) = self.runtime_sprite_index(entity) else {
            return false;
        };
        let sprite = &mut self.render.sprites[index];
        sprite.sorting_layer = sorting_layer;
        sprite.sorting_order = sorting_order;
        true
    }

    pub fn set_runtime_sprite_flip(
        &mut self,
        entity: EntityId,
        flip_x: bool,
        flip_y: bool,
    ) -> bool {
        let Some(index) = self.runtime_sprite_index(entity) else {
            return false;
        };
        let sprite = &mut self.render.sprites[index];
        sprite.flip_x = flip_x;
        sprite.flip_y = flip_y;
        true
    }

    pub fn set_runtime_sprite_enabled(&mut self, entity: EntityId, enabled: bool) -> bool {
        let Some(index) = self.runtime_sprite_index(entity) else {
            return false;
        };
        self.render.sprites[index].enabled = enabled;
        true
    }

    pub fn set_runtime_sprite_draw_mode(&mut self, entity: EntityId, mode: SpriteDrawMode) -> bool {
        if entity.as_u64() == 0 {
            return false;
        }
        self.clear_runtime_diagnostic(
            entity,
            &draw_mode_diagnostic(entity, SpriteDrawMode::Sliced),
        );
        self.clear_runtime_diagnostic(entity, &draw_mode_diagnostic(entity, SpriteDrawMode::Tiled));
        if mode != SpriteDrawMode::Simple {
            self.push_runtime_diagnostic(entity, draw_mode_diagnostic(entity, mode));
        }
        true
    }

    pub fn runtime_sprite_state(&self, entity: EntityId) -> Option<[f32; 12]> {
        let sprite = self
            .render
            .sprites
            .iter()
            .find(|sprite| sprite.entity == entity)?;
        Some([
            sprite.color[0],
            sprite.color[1],
            sprite.color[2],
            sprite.color[3],
            sprite.sorting_layer as f32,
            sprite.sorting_order as f32,
            f32::from(u8::from(sprite.flip_x)),
            f32::from(u8::from(sprite.flip_y)),
            f32::from(u8::from(sprite.enabled)),
            f32::from(u8::from(sprite.texture.is_some())),
            sprite.size[0] * SPRITE_PIXELS_PER_UNIT,
            sprite.size[1] * SPRITE_PIXELS_PER_UNIT,
        ])
    }

    fn push_runtime_diagnostic(&mut self, entity: EntityId, message: String) {
        let Some(index) = self.active_scene else {
            return;
        };
        let Some(scene) = self.scenes.get_mut(index) else {
            return;
        };
        if scene
            .diagnostics
            .iter()
            .any(|(id, text)| *id == entity && *text == message)
        {
            return;
        }
        scene.diagnostics.push((entity, message));
    }

    fn clear_runtime_diagnostic(&mut self, entity: EntityId, message: &str) {
        let Some(index) = self.active_scene else {
            return;
        };
        let Some(scene) = self.scenes.get_mut(index) else {
            return;
        };
        scene
            .diagnostics
            .retain(|(id, text)| !(*id == entity && text == message));
    }

    pub(crate) fn render_frame(
        &mut self,
        camera: u64,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>, RenderError> {
        if width == 0 || height == 0 {
            return Err(RenderError::InvalidArgument(
                "frame size must be greater than zero".to_string(),
            ));
        }
        let state = match self.render.cameras.get(&camera) {
            Some(state) => *state,
            None => return Err(RenderError::UnknownHandle),
        };
        if let Some(entity) = state.entity
            && self.world_transform_of(entity).is_none()
        {
            return Err(RenderError::InvalidArgument(
                "camera entity is not in a loaded scene".to_string(),
            ));
        }
        let renderer = self.gpu_renderer()?;
        renderer.set_sprites(self.sprite_instances());
        renderer.set_scene_lighting(&self.lights(), self.ambient());
        let mut presenter = crate::presenter::GpuFramePresenter::new(renderer);
        presenter.set_meshes(scene_gpu_meshes(self));
        let default = GpuCameraBinding::default();
        presenter.set_camera_binding(GpuCameraBinding {
            entity: state.entity,
            camera: projection_for(&state.params, default.camera.aspect),
            view: default.view,
            background: state.params.background_color,
        });
        presenter.set_light(self.directional_light());
        crate::presenter::apply_pending_display_quality(self);
        presenter.set_shadow_settings(crate::presenter::shadow_settings_for_runtime(self));
        let image = presenter
            .render_offscreen(self, width, height)
            .map_err(|error| RenderError::DeviceFailed(error.to_string()))?;
        let bytes = image.to_bmp_bytes();
        if let Some(camera_state) = self.render.cameras.get_mut(&camera) {
            camera_state.viewport = Some((width, height));
        }
        Ok(bytes)
    }

    pub(crate) fn render_id_map(
        &mut self,
        camera: u64,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>, RenderError> {
        if width == 0 || height == 0 {
            return Err(RenderError::InvalidArgument(
                "frame size must be greater than zero".to_string(),
            ));
        }
        let state = match self.render.cameras.get(&camera) {
            Some(state) => *state,
            None => return Err(RenderError::UnknownHandle),
        };
        if let Some(entity) = state.entity
            && self.world_transform_of(entity).is_none()
        {
            return Err(RenderError::InvalidArgument(
                "camera entity is not in a loaded scene".to_string(),
            ));
        }
        let renderer = self.gpu_renderer()?;
        renderer.set_sprites(self.sprite_instances());
        renderer.set_scene_lighting(&self.lights(), self.ambient());
        let mut presenter = crate::presenter::GpuFramePresenter::new(renderer);
        presenter.set_meshes(scene_gpu_meshes(self));
        let default = GpuCameraBinding::default();
        presenter.set_camera_binding(GpuCameraBinding {
            entity: state.entity,
            camera: projection_for(&state.params, default.camera.aspect),
            view: default.view,
            background: state.params.background_color,
        });
        presenter.set_light(self.directional_light());
        presenter.set_shadow_settings(crate::presenter::shadow_settings_for_runtime(self));
        let frame = crate::capture::capture_offscreen(&mut presenter, self, width, height)
            .map_err(|error| RenderError::DeviceFailed(error.to_string()))?;
        let mut bytes = Vec::with_capacity(frame.id_map.len() * 4);
        for id in frame.id_map {
            bytes.extend_from_slice(&id.to_le_bytes());
        }
        if let Some(camera_state) = self.render.cameras.get_mut(&camera) {
            camera_state.viewport = Some((width, height));
        }
        Ok(bytes)
    }

    pub fn capture_scene_frame(
        &mut self,
        width: u32,
        height: u32,
    ) -> Result<unitas_render::CaptureFrame, RenderError> {
        if width == 0 || height == 0 {
            return Err(RenderError::InvalidArgument(
                "frame size must be greater than zero".to_string(),
            ));
        }
        let renderer = self.gpu_renderer()?;
        renderer.set_sprites(self.sprite_instances());
        renderer.set_scene_lighting(&self.lights(), self.ambient());
        let mut presenter = crate::presenter::GpuFramePresenter::new(renderer);
        presenter.bind_scene(self);
        crate::capture::capture_offscreen(&mut presenter, self, width, height)
            .map_err(|error| RenderError::DeviceFailed(error.to_string()))
    }

    pub(crate) fn composite_canvas_image(&self, image: Image, width: u32, height: u32) -> Image {
        let items = self.ui_draw_items_for([width as f32, height as f32]);
        if items.is_empty() {
            return image;
        }
        let mut rgba = image.to_rgba8();
        self.composite_canvas_into(&mut rgba, &items);
        Image::from_rgba8(rgba)
    }

    pub(crate) fn composite_canvas_rgba(&self, mut rgba: Rgba8, width: u32, height: u32) -> Rgba8 {
        let items = self.ui_draw_items_for([width as f32, height as f32]);
        if items.is_empty() {
            return rgba;
        }
        self.composite_canvas_into(&mut rgba, &items);
        rgba
    }

    pub(crate) fn composite_canvas_into(&self, rgba: &mut Rgba8, items: &[UiDrawItem]) {
        let mut target = UiTarget {
            image: rgba,
            ids: None,
        };
        if items.iter().any(|item| item.text.is_some()) {
            let mut painter = self
                .ui_painter
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let painter = painter.get_or_insert_with(GlyphTextPainter::new);
            composite_ui(&mut target, items, 1, Some(painter));
        } else {
            composite_ui(&mut target, items, 1, None);
        }
    }

    pub(crate) fn gpu_renderer(&mut self) -> Result<Arc<Renderer>, RenderError> {
        if let Some(renderer) = &self.gpu_renderer {
            return Ok(Arc::clone(renderer));
        }
        let renderer = Renderer::new()
            .map(Arc::new)
            .map_err(|error| RenderError::DeviceFailed(error.to_string()))?;
        self.gpu_renderer = Some(Arc::clone(&renderer));
        Ok(renderer)
    }

    pub(crate) fn effect_mesh(&self) -> &Mesh {
        &self.render.effect_mesh
    }

    pub(crate) fn entity_name(&self, entity: EntityId) -> Option<&str> {
        self.scene_graph()
            .and_then(|graph| graph.entity(entity))
            .map(|entity| entity.name())
    }

    pub(crate) fn camera_world_to_screen_point(
        &self,
        camera: u64,
        point: [f32; 3],
    ) -> Result<[f32; 3], RenderError> {
        let state = match self.render.cameras.get(&camera) {
            Some(state) => *state,
            None => return Err(RenderError::UnknownHandle),
        };
        let (width, height) = match state.viewport {
            Some(viewport) => viewport,
            None => {
                return Err(RenderError::InvalidArgument(
                    "camera has no rendered frame".to_string(),
                ));
            }
        };
        let basis = match self.camera_basis(state.entity) {
            Some(basis) => basis,
            None => {
                return Err(RenderError::InvalidArgument(
                    "camera entity is not in a loaded scene".to_string(),
                ));
            }
        };
        let projection = projection_for(&state.params, width as f32 / height as f32);
        let clip = projection.projection_matrix() * basis.view;
        let world = Vec3::new(point[0], point[1], point[2]);
        let clip = clip.transform_point(world);
        if clip.w <= 1e-6 {
            return Err(RenderError::InvalidArgument(
                "point is behind the camera".to_string(),
            ));
        }
        let x_pixel = (clip.x / clip.w + 1.0) * 0.5 * width as f32;
        let y_pixel = (clip.y / clip.w + 1.0) * 0.5 * height as f32;
        let distance = (world - basis.eye).dot(basis.forward);
        Ok([x_pixel, y_pixel, distance])
    }

    pub(crate) fn camera_screen_point_to_ray(
        &self,
        camera: u64,
        point: [f32; 3],
    ) -> Result<[f32; 6], RenderError> {
        let state = match self.render.cameras.get(&camera) {
            Some(state) => *state,
            None => return Err(RenderError::UnknownHandle),
        };
        let (width, height) = match state.viewport {
            Some(viewport) => viewport,
            None => {
                return Err(RenderError::InvalidArgument(
                    "camera has no rendered frame".to_string(),
                ));
            }
        };
        let basis = match self.camera_basis(state.entity) {
            Some(basis) => basis,
            None => {
                return Err(RenderError::InvalidArgument(
                    "camera entity is not in a loaded scene".to_string(),
                ));
            }
        };
        let aspect = width as f32 / height as f32;
        let ndc_x = point[0] / (0.5 * width as f32) - 1.0;
        let ndc_y = point[1] / (0.5 * height as f32) - 1.0;
        let near = state.params.near_clip_plane.max(0.001);
        let (view_x, view_y) = if state.params.orthographic {
            (
                ndc_x * state.params.orthographic_size * aspect,
                ndc_y * state.params.orthographic_size,
            )
        } else {
            let half = (state.params.field_of_view.to_radians() * 0.5).tan();
            (ndc_x * near * half * aspect, ndc_y * near * half)
        };
        let direction =
            (basis.right * view_x + basis.up * view_y + basis.forward * near).normalize();
        Ok([
            basis.eye.x,
            basis.eye.y,
            basis.eye.z,
            direction.x,
            direction.y,
            direction.z,
        ])
    }
}

fn scene_light_kind(light: &LightComponent) -> Option<GpuLightKind> {
    if light.directional {
        return Some(GpuLightKind::Directional);
    }
    let diagnostic = light.unsupported_diagnostic.as_deref().unwrap_or("");
    if diagnostic.starts_with("point light") {
        Some(GpuLightKind::Point)
    } else if diagnostic.starts_with("spot light") {
        Some(GpuLightKind::Spot)
    } else {
        None
    }
}

fn is_builtin_material_reference(guid: &str, file_id: Option<i64>) -> bool {
    let _ = file_id;
    guid.eq_ignore_ascii_case(unitas_asset::primitive::BUILTIN_EXTRA_GUID)
}

fn render_material_for(renderer: &MeshRendererComponent) -> RenderMaterial {
    if let Some(path) = renderer.material_path.as_deref()
        && let Ok(material) = unitas_asset::material::parse_material(path)
    {
        return RenderMaterial {
            color: material.color,
            albedo: load_texture_image(renderer.albedo_tex_path.as_deref()),
            normal: load_texture_image(renderer.normal_tex_path.as_deref()),
            emissive: load_texture_image(renderer.emission_tex_path.as_deref()),
            metallic: material.metallic.or(renderer.metallic).unwrap_or(0.0),
            smoothness: material.glossiness.or(renderer.smoothness).unwrap_or(0.0),
            emission_color: material.emission_color.or(renderer.emission_color),
        };
    }
    RenderMaterial {
        color: [
            renderer.material_color[0],
            renderer.material_color[1],
            renderer.material_color[2],
            1.0,
        ],
        albedo: load_texture_image(renderer.albedo_tex_path.as_deref()),
        normal: load_texture_image(renderer.normal_tex_path.as_deref()),
        emissive: load_texture_image(renderer.emission_tex_path.as_deref()),
        metallic: renderer.metallic.unwrap_or(0.0),
        smoothness: renderer.smoothness.unwrap_or(0.0),
        emission_color: renderer.emission_color,
    }
}

fn draw_mode_diagnostic(entity: EntityId, mode: SpriteDrawMode) -> String {
    format!(
        "SpriteRenderer on entity {} draws in the {mode:?} mode, which the renderer does not draw yet",
        entity.as_u64()
    )
}

fn sprite_model(transform: &Transform, sprite: &RenderSprite) -> Mat4 {
    let flip_x = if sprite.flip_x { -1.0 } else { 1.0 };
    let flip_y = if sprite.flip_y { -1.0 } else { 1.0 };
    let scale = Mat4::from_cols_array([
        sprite.size[0] * flip_x,
        0.0,
        0.0,
        0.0,
        0.0,
        sprite.size[1] * flip_y,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
    ]);
    crate::presenter::transform_to_mat4(transform) * scale
}

fn load_texture_image(path: Option<&Path>) -> Option<Image> {
    let path = path?;
    let cooked = unitas_asset::texture::load_or_cook_texture(path)?;
    let mut image = image_from_cooked(&cooked);
    if cooked.srgb {
        linearize_image(&mut image);
    }
    Some(image)
}

fn image_from_cooked(cooked: &unitas_asset::texture::CookedTexture) -> Image {
    let mut image = Image::new(cooked.width, cooked.height);
    for (index, pixel) in image.pixels.iter_mut().enumerate() {
        let offset = index * 4;
        pixel[0] = cooked.rgba.get(offset).copied().unwrap_or(0);
        pixel[1] = cooked.rgba.get(offset + 1).copied().unwrap_or(0);
        pixel[2] = cooked.rgba.get(offset + 2).copied().unwrap_or(0);
    }
    image
}

fn linearize_image(image: &mut Image) {
    for pixel in &mut image.pixels {
        for channel in pixel.iter_mut() {
            let linear = unitas_asset::texture::import::srgb_to_linear(*channel);
            *channel = (linear * 255.0).round() as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_test_tga(path: &Path, width: u16, height: u16, rgb: [u8; 3]) -> std::io::Result<()> {
        let mut bytes = vec![0u8; 18 + (width as usize) * (height as usize) * 3];
        bytes[2] = 2;
        bytes[12..14].copy_from_slice(&width.to_le_bytes());
        bytes[14..16].copy_from_slice(&height.to_le_bytes());
        bytes[16] = 24;
        bytes[17] = 0x20;
        for chunk in bytes[18..].chunks_exact_mut(3) {
            chunk[0] = rgb[2];
            chunk[1] = rgb[1];
            chunk[2] = rgb[0];
        }
        std::fs::write(path, bytes)
    }

    #[test]
    fn sprite_renderer_scene_extracts_sorted_sprite_instances()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = std::env::temp_dir().join(format!(
            "unitas_runtime_sprite_extract_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir)?;

        let tex_path = dir.join("sprite.tga");
        write_test_tga(&tex_path, 100, 50, [255, 0, 0])?;
        let tex_meta_path = dir.join("sprite.tga.meta");
        std::fs::write(
            &tex_meta_path,
            "fileFormatVersion: 2\nguid: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n",
        )?;

        let scene_path = dir.join("SpriteScene.unity");
        let scene_yaml = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &1
GameObject:
  m_Name: FrontSprite
  m_Layer: 0
--- !u!4 &2
Transform:
  m_GameObject: {fileID: 1}
  m_LocalPosition: {x: 1, y: 2, z: 0}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalScale: {x: 2, y: 3, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!212 &3
SpriteRenderer:
  m_GameObject: {fileID: 1}
  m_Enabled: 1
  m_Sprite: {fileID: 21300000, guid: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, type: 3}
  m_Color: {r: 0.25, g: 0.5, b: 0.75, a: 1}
  m_FlipX: 1
  m_FlipY: 0
  m_DrawMode: 0
  m_SortingLayerID: 2
  m_SortingOrder: 1
--- !u!1 &4
GameObject:
  m_Name: BackSprite
  m_Layer: 0
--- !u!4 &5
Transform:
  m_GameObject: {fileID: 4}
  m_LocalPosition: {x: -1, y: 0, z: 0}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!212 &6
SpriteRenderer:
  m_GameObject: {fileID: 4}
  m_Enabled: 1
  m_Sprite: {fileID: 21300000, guid: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, type: 3}
  m_Color: {r: 1, g: 1, b: 1, a: 0.5}
  m_FlipX: 0
  m_FlipY: 1
  m_DrawMode: 0
  m_SortingLayerID: 1
  m_SortingOrder: 5
--- !u!1 &7
GameObject:
  m_Name: DisabledSprite
  m_Layer: 0
--- !u!4 &8
Transform:
  m_GameObject: {fileID: 7}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!212 &9
SpriteRenderer:
  m_GameObject: {fileID: 7}
  m_Enabled: 0
  m_Sprite: {fileID: 21300000, guid: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, type: 3}
  m_Color: {r: 1, g: 0, b: 0, a: 1}
  m_SortingLayerID: 0
  m_SortingOrder: 0
";
        std::fs::write(&scene_path, scene_yaml)?;

        let mut runtime = Runtime::new()?;
        runtime.load_scene(&scene_path)?;
        let _ = std::fs::remove_dir_all(&dir);

        let sprites = runtime.sprite_instances();
        assert_eq!(sprites.len(), 2);
        assert_eq!(sprites[0].sorting_layer, 1);
        assert_eq!(sprites[0].sorting_order, 5);
        assert_eq!(sprites[0].color, [1.0, 1.0, 1.0, 0.5]);
        assert_eq!(sprites[0].texture.width, 100);
        assert_eq!(sprites[0].texture.height, 50);
        assert!((sprites[0].model.get(0, 0) - 1.0).abs() < 1e-6);
        assert!((sprites[0].model.get(1, 1) - (-0.5)).abs() < 1e-6);

        assert_eq!(sprites[1].sorting_layer, 2);
        assert_eq!(sprites[1].sorting_order, 1);
        assert_eq!(sprites[1].color, [0.25, 0.5, 0.75, 1.0]);
        assert!((sprites[1].model.get(0, 0) - (-2.0)).abs() < 1e-6);
        assert!((sprites[1].model.get(1, 1) - 1.5).abs() < 1e-6);
        Ok(())
    }

    #[test]
    fn load_texture_image_reads_from_bundle_cooked_manifest_without_raw_source_file()
    -> Result<(), Box<dyn std::error::Error>> {
        use unitas_asset::texture::COOKED_DIR_NAME;
        use unitas_asset::texture::cook_project_textures;

        let dir = std::env::temp_dir().join(format!(
            "unitas_runtime_cooked_bundle_tex_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let project_dir = dir.join("project");
        let bundle_dir = dir.join("bundle");
        let cache_dir = dir.join("cache");
        std::fs::create_dir_all(project_dir.join("Assets/Textures"))?;
        std::fs::create_dir_all(bundle_dir.join("Assets/Textures"))?;

        let src_tex = project_dir.join("Assets/Textures/Tile.tga");
        write_test_tga(&src_tex, 4, 4, [255, 0, 0])?;
        let meta = "fileFormatVersion: 2\nguid: cccccccccccccccccccccccccccccccc\nTextureImporter:\n  sRGBTexture: 0\n  textureType: 0\n";
        std::fs::write(project_dir.join("Assets/Textures/Tile.tga.meta"), meta)?;
        std::fs::write(bundle_dir.join("Assets/Textures/Tile.tga.meta"), meta)?;
        std::fs::write(bundle_dir.join("bundle.manifest"), "format_version=1\n")?;

        let (manifest, report) =
            cook_project_textures(&project_dir, &cache_dir, &bundle_dir.join(COOKED_DIR_NAME))?;
        assert_eq!(report.imported, 1);
        assert_eq!(manifest.entries.len(), 1);

        let staged_virtual_tex_path = bundle_dir.join("Assets/Textures/Tile.tga");
        assert!(!staged_virtual_tex_path.exists());

        let image = load_texture_image(Some(&staged_virtual_tex_path))
            .ok_or("expected cooked texture to load from bundle Cooked/ manifest")?;
        assert_eq!(image.width, 4);
        assert_eq!(image.height, 4);
        assert_eq!(image.pixels[0], [255, 0, 0]);

        let unindexed_bundle_tex = bundle_dir.join("Assets/Textures/Missing.tga");
        assert!(load_texture_image(Some(&unindexed_bundle_tex)).is_none());

        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn point_and_spot_lights_are_collected_without_runtime_unsupported_diagnostics()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = std::env::temp_dir().join(format!(
            "unitas_runtime_multilight_extract_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir)?;
        let scene_path = dir.join("MultiLightScene.unity");
        let scene_yaml = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &10
GameObject:
  m_Name: Sun
  m_Layer: 0
--- !u!4 &11
Transform:
  m_GameObject: {fileID: 10}
  m_LocalPosition: {x: 0, y: 10, z: 0}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!108 &12
Light:
  m_GameObject: {fileID: 10}
  m_Enabled: 1
  m_Type: 1
  m_Color: {r: 1, g: 0.9, b: 0.8, a: 1}
  m_Intensity: 0.8
--- !u!1 &20
GameObject:
  m_Name: PointLamp
  m_Layer: 0
--- !u!4 &21
Transform:
  m_GameObject: {fileID: 20}
  m_LocalPosition: {x: 1, y: 2, z: 3}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!108 &22
Light:
  m_GameObject: {fileID: 20}
  m_Enabled: 1
  m_Type: 2
  m_Color: {r: 1, g: 0, b: 0, a: 1}
  m_Intensity: 2.5
--- !u!1 &30
GameObject:
  m_Name: SpotLamp
  m_Layer: 0
--- !u!4 &31
Transform:
  m_GameObject: {fileID: 30}
  m_LocalPosition: {x: 4, y: 5, z: 6}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!108 &32
Light:
  m_GameObject: {fileID: 30}
  m_Enabled: 1
  m_Type: 0
  m_Color: {r: 0, g: 1, b: 0, a: 1}
  m_Intensity: 3.0
";
        std::fs::write(&scene_path, scene_yaml)?;

        let mut runtime = Runtime::new()?;
        runtime.load_scene(&scene_path)?;
        let _ = std::fs::remove_dir_all(&dir);

        assert!(runtime.diagnostics().is_empty());
        assert_eq!(runtime.ambient(), [0.0, 0.0, 0.0]);
        runtime.set_ambient_light([0.1, 0.2, 0.3]);
        assert_eq!(runtime.ambient(), [0.1, 0.2, 0.3]);

        let dir_light = runtime.directional_light();
        assert!((dir_light.intensity - 0.8).abs() < 1e-6);
        assert_eq!(dir_light.color, [1.0, 0.9, 0.8]);

        let lights = runtime.lights();
        assert_eq!(lights.len(), 2);
        assert_eq!(lights[0].kind, GpuLightKind::Point);
        assert_eq!(lights[0].position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(lights[0].color, [1.0, 0.0, 0.0]);
        assert!((lights[0].intensity - 2.5).abs() < 1e-6);

        assert_eq!(lights[1].kind, GpuLightKind::Spot);
        assert_eq!(lights[1].position, Vec3::new(4.0, 5.0, 6.0));
        assert_eq!(lights[1].color, [0.0, 1.0, 0.0]);
        assert!((lights[1].intensity - 3.0).abs() < 1e-6);
        Ok(())
    }

    #[test]
    fn urp_lit_material_in_scene_binds_base_map_base_color_normal_emission_and_pbr_into_render_material()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = std::env::temp_dir().join(format!(
            "unitas_runtime_urp_material_bind_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let assets_dir = dir.join("Assets");
        std::fs::create_dir_all(&assets_dir)?;
        std::fs::create_dir_all(dir.join("ProjectSettings"))?;

        let base_tex = assets_dir.join("Base.tga");
        write_test_tga(&base_tex, 4, 4, [255, 0, 255])?;
        std::fs::write(
            assets_dir.join("Base.tga.meta"),
            "fileFormatVersion: 2\nguid: 11111111111111111111111111111111\nTextureImporter:\n  sRGBTexture: 0\n  textureType: 0\n",
        )?;

        let normal_tex = assets_dir.join("Normal.tga");
        write_test_tga(&normal_tex, 4, 4, [0, 0, 255])?;
        std::fs::write(
            assets_dir.join("Normal.tga.meta"),
            "fileFormatVersion: 2\nguid: 22222222222222222222222222222222\nTextureImporter:\n  sRGBTexture: 0\n  textureType: 1\n",
        )?;

        let emissive_tex = assets_dir.join("Emissive.tga");
        write_test_tga(&emissive_tex, 4, 4, [255, 255, 0])?;
        std::fs::write(
            assets_dir.join("Emissive.tga.meta"),
            "fileFormatVersion: 2\nguid: 33333333333333333333333333333333\nTextureImporter:\n  sRGBTexture: 0\n  textureType: 0\n",
        )?;

        let mat_yaml = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!21 &2100000
Material:
  m_Name: UrpLitMat
  m_Shader: {fileID: 4800000, guid: 933532a4fcc9baf4fa0491de14d08ed7, type: 3}
  m_SavedProperties:
    serializedVersion: 3
    m_TexEnvs:
    - _BaseMap:
        m_Texture: {fileID: 2800000, guid: 11111111111111111111111111111111, type: 3}
        m_Scale: {x: 1, y: 1}
        m_Offset: {x: 0, y: 0}
    - _MainTex:
        m_Texture: {fileID: 0}
        m_Scale: {x: 1, y: 1}
        m_Offset: {x: 0, y: 0}
    - _BumpMap:
        m_Texture: {fileID: 2800000, guid: 22222222222222222222222222222222, type: 3}
        m_Scale: {x: 1, y: 1}
        m_Offset: {x: 0, y: 0}
    - _EmissionMap:
        m_Texture: {fileID: 2800000, guid: 33333333333333333333333333333333, type: 3}
        m_Scale: {x: 1, y: 1}
        m_Offset: {x: 0, y: 0}
    m_Floats:
    - _Smoothness: 0.7
    - _Metallic: 0.35
    m_Colors:
    - _BaseColor: {r: 0.25, g: 0.5, b: 0.75, a: 0.8}
    - _EmissionColor: {r: 0.4, g: 0.2, b: 0.1, a: 1}
";
        std::fs::write(assets_dir.join("UrpLitMat.mat"), mat_yaml)?;
        std::fs::write(
            assets_dir.join("UrpLitMat.mat.meta"),
            "fileFormatVersion: 2\nguid: 44444444444444444444444444444444\n",
        )?;

        let scene_path = assets_dir.join("UrpScene.unity");
        let scene_yaml = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &10
GameObject:
  m_Name: UrpCube
  m_Layer: 0
--- !u!4 &11
Transform:
  m_GameObject: {fileID: 10}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!33 &12
MeshFilter:
  m_GameObject: {fileID: 10}
  m_Mesh: {fileID: 10202, guid: 0000000000000000e000000000000000, type: 0}
--- !u!23 &13
MeshRenderer:
  m_GameObject: {fileID: 10}
  m_Enabled: 1
  m_Materials:
  - {fileID: 2100000, guid: 44444444444444444444444444444444, type: 2}
";
        std::fs::write(&scene_path, scene_yaml)?;

        let mut runtime = Runtime::new()?;
        runtime.load_scene(&scene_path)?;
        let _ = std::fs::remove_dir_all(&dir);

        assert!(
            runtime.diagnostics().is_empty(),
            "expected no material diagnostics for valid project URP material, got {:?}",
            runtime.diagnostics()
        );
        let renderables = runtime.scene_renderables();
        assert_eq!(renderables.len(), 1);
        let renderable = &renderables[0];
        assert_eq!(renderable.color, [0.25, 0.5, 0.75]);
        assert_eq!(renderable.material.color, [0.25, 0.5, 0.75, 0.8]);
        assert!((renderable.material.metallic - 0.35).abs() < 1e-6);
        assert!((renderable.material.smoothness - 0.7).abs() < 1e-6);
        assert_eq!(
            renderable.material.emission_color,
            Some([0.4, 0.2, 0.1, 1.0])
        );
        let albedo = renderable
            .material
            .albedo
            .as_ref()
            .ok_or("expected URP _BaseMap albedo texture")?;
        assert_eq!((albedo.width, albedo.height), (4, 4));
        assert_eq!(albedo.pixels[0], [255, 0, 255]);
        let normal = renderable
            .material
            .normal
            .as_ref()
            .ok_or("expected URP _BumpMap normal texture")?;
        assert_eq!(normal.pixels[0], [0, 0, 255]);
        let emissive = renderable
            .material
            .emissive
            .as_ref()
            .ok_or("expected URP _EmissionMap emissive texture")?;
        assert_eq!(emissive.pixels[0], [255, 255, 0]);
        Ok(())
    }
}
