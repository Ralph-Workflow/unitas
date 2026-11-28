use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use unitas_animation::CurveBindingTarget;
use unitas_animation::curve::AssetReference;
use unitas_asset::meta::GuidIndex;
use unitas_asset::scene::{
    CameraComponent, LightComponent, MeshRendererComponent, RenderBinding, SpriteDrawMode,
    SpriteRendererComponent,
};
use unitas_asset::texture::import::TextureImportSettings;
use unitas_core::{EntityId, IDENTITY_ROTATION, SceneGraph, Transform, rotate_vector};
use unitas_render::image::{Image, Rgba8};
use unitas_render::ui_pass::{GlyphTextPainter, UiTarget, composite_ui};
use unitas_render::{
    Camera, DirectionalLight, Light2D, Light2DKind, MAX_LIGHTS_2D, Mat4, Mesh, MeshAttributes,
    Renderer, Vec3, cube,
    gpu::{GpuLight, GpuLightKind, MAX_GPU_LIGHTS},
    look_at,
};

use super::{LoadedScene, Runtime};
use crate::animation::is_sprite_renderer_sprite_binding;
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderLight2D {
    pub entity: Option<EntityId>,
    pub kind: Light2DKind,
    pub color: [f32; 3],
    pub intensity: f32,
    pub position: [f32; 2],
    pub direction: [f32; 2],
    pub inner_radius: f32,
    pub outer_radius: f32,
    pub inner_angle_deg: f32,
    pub outer_angle_deg: f32,
    pub enabled: bool,
    pub position_override: bool,
    pub direction_override: bool,
}

impl RenderLight2D {
    pub fn new(entity: Option<EntityId>) -> Self {
        Self {
            entity,
            kind: Light2DKind::Point,
            color: [1.0, 1.0, 1.0],
            intensity: 1.0,
            position: [0.0, 0.0],
            direction: [0.0, 1.0],
            inner_radius: 0.0,
            outer_radius: 1.0,
            inner_angle_deg: 180.0,
            outer_angle_deg: 180.0,
            enabled: true,
            position_override: false,
            direction_override: false,
        }
    }
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
    pub draw_mode: SpriteDrawMode,
    pub border: [f32; 4],
    slices: Option<SpriteSlices>,
}

#[derive(Clone, Debug)]
struct SpriteSlices {
    source: Arc<Image>,
    mode: SpriteDrawMode,
    parts: Vec<Arc<Image>>,
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
    lights_2d: HashMap<u64, RenderLight2D>,
    ambient: [f32; 3],
    sprites: Vec<RenderSprite>,
    sprite_asset_paths: HashMap<String, PathBuf>,
    sprite_texture_cache: HashMap<PathBuf, Arc<Image>>,
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
            lights_2d: HashMap::new(),
            ambient: [0.0; 3],
            sprites: Vec::new(),
            sprite_asset_paths: HashMap::new(),
            sprite_texture_cache: HashMap::new(),
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

    pub fn create_render_light_2d(&mut self, entity: Option<EntityId>) -> u64 {
        let handle = self.render.allocate();
        self.render
            .lights_2d
            .insert(handle, RenderLight2D::new(entity));
        handle
    }

    pub fn destroy_render_light_2d(&mut self, handle: u64) -> bool {
        self.render.lights_2d.remove(&handle).is_some()
    }

    pub fn render_light_2d(&self, handle: u64) -> Option<RenderLight2D> {
        self.render.lights_2d.get(&handle).copied()
    }

    pub fn set_render_light_2d_kind(&mut self, handle: u64, kind: Light2DKind) -> bool {
        match self.render.lights_2d.get_mut(&handle) {
            Some(light) => {
                light.kind = kind;
                true
            }
            None => false,
        }
    }

    pub fn set_render_light_2d_color(&mut self, handle: u64, color: [f32; 3]) -> bool {
        match self.render.lights_2d.get_mut(&handle) {
            Some(light) => {
                light.color = color;
                true
            }
            None => false,
        }
    }

    pub fn set_render_light_2d_intensity(&mut self, handle: u64, intensity: f32) -> bool {
        match self.render.lights_2d.get_mut(&handle) {
            Some(light) => {
                light.intensity = intensity;
                true
            }
            None => false,
        }
    }

    pub fn set_render_light_2d_radii(&mut self, handle: u64, inner: f32, outer: f32) -> bool {
        match self.render.lights_2d.get_mut(&handle) {
            Some(light) => {
                light.inner_radius = inner;
                light.outer_radius = outer;
                true
            }
            None => false,
        }
    }

    pub fn set_render_light_2d_angles(
        &mut self,
        handle: u64,
        inner_deg: f32,
        outer_deg: f32,
    ) -> bool {
        match self.render.lights_2d.get_mut(&handle) {
            Some(light) => {
                light.inner_angle_deg = inner_deg;
                light.outer_angle_deg = outer_deg;
                true
            }
            None => false,
        }
    }

    pub fn set_render_light_2d_position(&mut self, handle: u64, position: [f32; 2]) -> bool {
        match self.render.lights_2d.get_mut(&handle) {
            Some(light) => {
                light.position = position;
                light.position_override = true;
                true
            }
            None => false,
        }
    }

    pub fn set_render_light_2d_direction(&mut self, handle: u64, direction: [f32; 2]) -> bool {
        match self.render.lights_2d.get_mut(&handle) {
            Some(light) => {
                light.direction = direction;
                light.direction_override = true;
                true
            }
            None => false,
        }
    }

    pub fn set_render_light_2d_enabled(&mut self, handle: u64, enabled: bool) -> bool {
        match self.render.lights_2d.get_mut(&handle) {
            Some(light) => {
                light.enabled = enabled;
                true
            }
            None => false,
        }
    }

    pub fn update_render_light_2d(&mut self, handle: u64, updated: RenderLight2D) -> bool {
        match self.render.lights_2d.get_mut(&handle) {
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
        self.populate_scene_cameras(&binding.cameras, entity_base);
        self.populate_scene_renderers(&binding.renderers, entity_base, &mut diagnostics);
        self.populate_scene_sprites(&binding.sprites, entity_base, &mut diagnostics);
        self.populate_scene_lights(&binding.lights, entity_base, graph, &mut diagnostics);
        diagnostics
    }

    fn populate_scene_cameras(&mut self, cameras: &[CameraComponent], entity_base: u64) {
        for camera in cameras {
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
    }

    fn populate_scene_renderers(
        &mut self,
        renderers: &[MeshRendererComponent],
        entity_base: u64,
        diagnostics: &mut Vec<(EntityId, String)>,
    ) {
        for renderer in renderers {
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
    }

    fn populate_scene_sprites(
        &mut self,
        sprites: &[SpriteRendererComponent],
        entity_base: u64,
        diagnostics: &mut Vec<(EntityId, String)>,
    ) {
        for sprite in sprites {
            let entity = EntityId::new(entity_base + sprite.entity.as_u64());
            if let (Some(guid), Some(path)) = (&sprite.sprite_guid, &sprite.sprite_path) {
                self.register_sprite_asset_root(path);
                self.render
                    .sprite_asset_paths
                    .insert(guid.to_ascii_lowercase(), path.clone());
                self.animation.register_sprite_asset(guid, path);
            }
            let texture = self.load_cached_sprite_texture(sprite.sprite_path.as_deref());
            if let (Some(path), Some(tex)) = (&sprite.sprite_path, &texture) {
                self.render
                    .sprite_texture_cache
                    .insert(path.clone(), Arc::clone(tex));
                if let Some(guid) = &sprite.sprite_guid {
                    self.animation
                        .register_sprite_texture(guid, path, Arc::clone(tex));
                }
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
            let size = texture.as_ref().map_or([0.0, 0.0], |image| {
                [
                    image.width as f32 / SPRITE_PIXELS_PER_UNIT,
                    image.height as f32 / SPRITE_PIXELS_PER_UNIT,
                ]
            });
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
                draw_mode: sprite.draw_mode,
                border: sprite_border_from_meta(sprite.sprite_path.as_deref()),
                slices: None,
            });
        }
    }

    fn populate_scene_lights(
        &mut self,
        lights: &[LightComponent],
        entity_base: u64,
        graph: &SceneGraph,
        diagnostics: &mut Vec<(EntityId, String)>,
    ) {
        for light in lights {
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
    }

    fn import_scene_mesh(&mut self, renderer: &MeshRendererComponent) -> Option<u64> {
        let mesh_data = if let (Some(file_id), Some(guid)) =
            (renderer.mesh_file_id, renderer.mesh_guid.as_deref())
            && let Some(primitive) =
                unitas_asset::primitive::builtin_primitive_by_identity(file_id, guid)
        {
            let mut data = unitas_asset::primitive::primitive_mesh(primitive);
            if primitive == unitas_asset::primitive::BuiltinPrimitive::Sphere {
                for position in &mut data.positions {
                    *position = [position[0] * 0.5, position[1] * 0.5, position[2] * 0.5];
                }
            }
            Some(data)
        } else if let Some(path) = &renderer.mesh_path {
            unitas_asset::model::load_or_cook_mesh(path)
                .map(unitas_asset::model::CookedMesh::into_mesh)
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
            .lights_2d
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

    pub fn lights_2d(&self) -> Vec<Light2D> {
        let mut handles: Vec<u64> = self.render.lights_2d.keys().copied().collect();
        handles.sort_unstable();
        let mut collected = Vec::new();
        for handle in handles {
            let Some(light) = self.render.lights_2d.get(&handle) else {
                continue;
            };
            if !light.enabled {
                continue;
            }
            let transform = light.entity.and_then(|e| self.world_transform_of(e));
            let position = match (light.position_override, transform.as_ref()) {
                (false, Some(tf)) => [tf.translation[0], tf.translation[1]],
                _ => light.position,
            };
            let direction = match (light.direction_override, transform.as_ref()) {
                (false, Some(tf)) => {
                    let up = rotate_vector(tf.rotation, [0.0, 1.0, 0.0]);
                    let len = (up[0] * up[0] + up[1] * up[1]).sqrt();
                    if len > 1e-8 {
                        [up[0] / len, up[1] / len]
                    } else {
                        light.direction
                    }
                }
                _ => light.direction,
            };
            collected.push(Light2D {
                kind: light.kind,
                color: light.color,
                intensity: light.intensity,
                position,
                direction,
                inner_radius: light.inner_radius,
                outer_radius: light.outer_radius,
                inner_angle_deg: light.inner_angle_deg,
                outer_angle_deg: light.outer_angle_deg,
            });
            if collected.len() >= MAX_LIGHTS_2D {
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

    pub fn sprite_instances(&mut self) -> Vec<unitas_render::Sprite> {
        let mut sprites: Vec<unitas_render::Sprite> = Vec::new();
        for index in 0..self.render.sprites.len() {
            let mut adjusted = self.render.sprites[index].clone();
            if !adjusted.enabled {
                continue;
            }
            let Some(texture) = self.effective_sprite_texture(&adjusted) else {
                continue;
            };
            let Some(transform) = self.entity_transform(adjusted.entity) else {
                continue;
            };
            adjusted.size = [
                texture.width as f32 / SPRITE_PIXELS_PER_UNIT,
                texture.height as f32 / SPRITE_PIXELS_PER_UNIT,
            ];
            if adjusted.draw_mode == SpriteDrawMode::Simple {
                sprites.push(unitas_render::Sprite {
                    texture,
                    model: sprite_model(&transform, &adjusted),
                    color: adjusted.color,
                    sorting_layer: adjusted.sorting_layer,
                    sorting_order: adjusted.sorting_order,
                });
                continue;
            }
            self.append_sliced_sprite_instances(
                index,
                &texture,
                &transform,
                &adjusted,
                &mut sprites,
            );
        }
        unitas_render::sort_sprites(&mut sprites);
        sprites
    }

    fn append_sliced_sprite_instances(
        &mut self,
        index: usize,
        texture: &Arc<Image>,
        transform: &Transform,
        adjusted: &RenderSprite,
        sprites: &mut Vec<unitas_render::Sprite>,
    ) {
        let source_size = [texture.width as f32, texture.height as f32];
        let target_size = [
            source_size[0] * transform.scale[0].abs(),
            source_size[1] * transform.scale[1].abs(),
        ];
        let parts = match adjusted.draw_mode {
            SpriteDrawMode::Tiled => unitas_render::sprite::tiled_parts(source_size, target_size),
            _ => unitas_render::sprite::nine_slice_parts(adjusted.border, source_size, target_size),
        };
        let textures = self.sprite_part_textures(index, texture, adjusted.draw_mode, &parts);
        for (part, part_texture) in parts.iter().zip(textures.iter()) {
            sprites.push(unitas_render::Sprite {
                texture: Arc::clone(part_texture),
                model: sprite_part_model(transform, adjusted, target_size, part),
                color: adjusted.color,
                sorting_layer: adjusted.sorting_layer,
                sorting_order: adjusted.sorting_order,
            });
        }
    }

    fn sprite_part_textures(
        &mut self,
        index: usize,
        texture: &Arc<Image>,
        mode: SpriteDrawMode,
        parts: &[unitas_render::sprite::SpritePart],
    ) -> Vec<Arc<Image>> {
        if let Some(slices) = &self.render.sprites[index].slices
            && slices.mode == mode
            && slices.parts.len() == parts.len()
            && Arc::ptr_eq(&slices.source, texture)
        {
            return slices.parts.clone();
        }
        let textures: Vec<Arc<Image>> = parts
            .iter()
            .map(|part| {
                if part.source == [0, 0, texture.width, texture.height] {
                    Arc::clone(texture)
                } else {
                    Arc::new(unitas_render::sprite::crop_texture(texture, part.source))
                }
            })
            .collect();
        self.render.sprites[index].slices = Some(SpriteSlices {
            source: Arc::clone(texture),
            mode,
            parts: textures.clone(),
        });
        textures
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
        if let Some(path) = texture_path {
            self.register_sprite_asset_root(path);
        }
        let texture = self.load_cached_sprite_texture(texture_path);
        if let (Some(path), Some(tex)) = (texture_path, &texture) {
            self.render
                .sprite_texture_cache
                .insert(path.to_path_buf(), Arc::clone(tex));
        }
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
        let size = texture.as_ref().map_or([0.0, 0.0], |image| {
            [
                image.width as f32 / SPRITE_PIXELS_PER_UNIT,
                image.height as f32 / SPRITE_PIXELS_PER_UNIT,
            ]
        });
        let border = sprite_border_from_meta(texture_path);
        self.upsert_runtime_sprite_entry(entity, texture, size, border);
        true
    }

    fn upsert_runtime_sprite_entry(
        &mut self,
        entity: EntityId,
        texture: Option<Arc<Image>>,
        size: [f32; 2],
        border: [f32; 4],
    ) {
        match self.runtime_sprite_index(entity) {
            Some(index) => {
                let sprite = &mut self.render.sprites[index];
                sprite.texture = texture;
                sprite.size = size;
                sprite.border = border;
                sprite.slices = None;
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
                draw_mode: SpriteDrawMode::Simple,
                border,
                slices: None,
            }),
        }
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
        if let Some(index) = self.runtime_sprite_index(entity) {
            self.render.sprites[index].draw_mode = mode;
            self.render.sprites[index].slices = None;
        }
        true
    }

    pub fn runtime_sprite_state(&self, entity: EntityId) -> Option<[f32; 12]> {
        let sprite = self
            .render
            .sprites
            .iter()
            .find(|sprite| sprite.entity == entity)?;
        let texture = self.effective_sprite_texture(sprite);
        let size = match texture.as_ref() {
            Some(image) => [
                image.width as f32 / SPRITE_PIXELS_PER_UNIT,
                image.height as f32 / SPRITE_PIXELS_PER_UNIT,
            ],
            None => sprite.size,
        };
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
            f32::from(u8::from(texture.is_some())),
            size[0] * SPRITE_PIXELS_PER_UNIT,
            size[1] * SPRITE_PIXELS_PER_UNIT,
        ])
    }

    pub fn register_sprite_asset(&mut self, guid: &str, path: &Path) {
        self.register_sprite_asset_root(path);
        let lower = guid.trim().to_ascii_lowercase();
        self.render
            .sprite_asset_paths
            .insert(lower.clone(), path.to_path_buf());
        self.animation.register_sprite_asset(&lower, path);
        if let Some(image) = load_texture_image(Some(path)).map(Arc::new) {
            self.render
                .sprite_texture_cache
                .insert(path.to_path_buf(), Arc::clone(&image));
            self.animation.register_sprite_texture(&lower, path, image);
        }
    }

    fn register_sprite_asset_root(&mut self, asset_path: &Path) {
        let Some(parent) = asset_path.parent() else {
            return;
        };
        let mut index_root = parent.to_path_buf();
        for ancestor in parent.ancestors() {
            if ancestor.join("ProjectSettings").is_dir() {
                let assets = ancestor.join("Assets");
                index_root = if assets.is_dir() {
                    assets
                } else {
                    ancestor.to_path_buf()
                };
                break;
            }
            if ancestor.file_name().and_then(|n| n.to_str()) == Some("Assets") {
                index_root = ancestor.to_path_buf();
                break;
            }
        }
        if let Ok(index) = GuidIndex::build(&index_root) {
            for (guid, path) in index.iter() {
                let lower = guid.to_ascii_lowercase();
                self.render
                    .sprite_asset_paths
                    .entry(lower.clone())
                    .or_insert_with(|| path.to_path_buf());
                self.animation.register_sprite_asset(&lower, path);
            }
        }
    }

    fn load_cached_sprite_texture(&self, path: Option<&Path>) -> Option<Arc<Image>> {
        let path = path?;
        if let Some(cached) = self.render.sprite_texture_cache.get(path) {
            return Some(Arc::clone(cached));
        }
        load_texture_image(Some(path)).map(Arc::new)
    }

    fn resolve_sprite_reference_texture(&self, reference: AssetReference) -> Option<Arc<Image>> {
        if reference.is_null() {
            return None;
        }
        let guid = reference.guid_hex();
        let path = self.render.sprite_asset_paths.get(&guid)?;
        self.load_cached_sprite_texture(Some(path))
    }

    fn animated_sprite_reference_for(
        &self,
        sprite_entity: EntityId,
    ) -> Option<(EntityId, CurveBindingTarget, AssetReference)> {
        for anim_entity in self.animation.entities() {
            for applied in self.animation.applied_object_references(anim_entity) {
                if !is_sprite_renderer_sprite_binding(&applied.target) {
                    continue;
                }
                let target_entity = if applied.target.object_path.is_empty() {
                    Some(anim_entity)
                } else {
                    self.resolve_animated_child_entity(anim_entity, &applied.target.object_path)
                };
                if target_entity == Some(sprite_entity) {
                    return Some((anim_entity, applied.target.clone(), applied.reference));
                }
            }
        }
        None
    }

    fn resolve_animated_child_entity(&self, root: EntityId, object_path: &str) -> Option<EntityId> {
        for scene in &self.scenes {
            if !scene.graph.contains(root) {
                continue;
            }
            let mut current = root;
            for segment in object_path.split('/') {
                let trimmed = segment.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let mut next = None;
                for child_id in scene.graph.children(current) {
                    if let Some(entity) = scene.graph.entity(*child_id)
                        && entity.name == trimmed
                    {
                        next = Some(*child_id);
                        break;
                    }
                }
                current = next?;
            }
            return (current != root).then_some(current);
        }
        None
    }

    fn effective_sprite_texture(&self, sprite: &RenderSprite) -> Option<Arc<Image>> {
        if let Some((anim_entity, target, reference)) =
            self.animated_sprite_reference_for(sprite.entity)
        {
            if reference.is_null() {
                return None;
            }
            if let Some(Some(texture)) = self.animation.resolved_sprite(anim_entity, &target) {
                return Some(Arc::clone(texture));
            }
            if let Some(texture) = self.resolve_sprite_reference_texture(reference) {
                return Some(texture);
            }
        }
        sprite.texture.clone()
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
        renderer.set_lights_2d(&self.lights_2d());
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
        presenter.set_post_settings(crate::presenter::post_settings_for_runtime(self));
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
        renderer.set_lights_2d(&self.lights_2d());
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
        renderer.set_lights_2d(&self.lights_2d());
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

fn sprite_part_model(
    transform: &Transform,
    sprite: &RenderSprite,
    target_size: [f32; 2],
    part: &unitas_render::sprite::SpritePart,
) -> Mat4 {
    let width = part.target[2] / target_size[0];
    let height = part.target[3] / target_size[1];
    let center_x = part.target[0] / target_size[0] + width * 0.5 - 0.5;
    let center_y = part.target[1] / target_size[1] + height * 0.5 - 0.5;
    let offset = Mat4::from_cols_array([
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, center_x, center_y, 0.0, 1.0,
    ]);
    let scale = Mat4::from_cols_array([
        width, 0.0, 0.0, 0.0, 0.0, height, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]);
    sprite_model(transform, sprite) * offset * scale
}

fn sprite_border_from_meta(path: Option<&Path>) -> [f32; 4] {
    let Some(path) = path else {
        return [0.0; 4];
    };
    let mut meta = path.as_os_str().to_os_string();
    meta.push(".meta");
    let Ok(text) = std::fs::read_to_string(PathBuf::from(meta)) else {
        return [0.0; 4];
    };
    let Ok(settings) = TextureImportSettings::from_meta_text(&text) else {
        return [0.0; 4];
    };
    let Some(slice) = settings.sprite_slices.first() else {
        return [0.0; 4];
    };
    [
        slice.border_left,
        slice.border_bottom,
        slice.border_right,
        slice.border_top,
    ]
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

    fn write_sprite_extraction_scene(
        dir: &Path,
    ) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        let _ = std::fs::remove_dir_all(dir);
        std::fs::create_dir_all(dir)?;
        let tex_path = dir.join("sprite.tga");
        write_test_tga(&tex_path, 100, 50, [255, 0, 0])?;
        std::fs::write(
            dir.join("sprite.tga.meta"),
            "fileFormatVersion: 2\nguid: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n",
        )?;
        let scene_path = dir.join("SpriteScene.unity");
        let scene_yaml = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &1\nGameObject:\n  m_Name: FrontSprite\n  m_Layer: 0\n--- !u!4 &2\nTransform:\n  m_GameObject: {fileID: 1}\n  m_LocalPosition: {x: 1, y: 2, z: 0}\n  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n  m_LocalScale: {x: 2, y: 3, z: 1}\n  m_Children: []\n  m_Father: {fileID: 0}\n--- !u!212 &3\nSpriteRenderer:\n  m_GameObject: {fileID: 1}\n  m_Enabled: 1\n  m_Sprite: {fileID: 21300000, guid: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, type: 3}\n  m_Color: {r: 0.25, g: 0.5, b: 0.75, a: 1}\n  m_FlipX: 1\n  m_FlipY: 0\n  m_DrawMode: 0\n  m_SortingLayerID: 2\n  m_SortingOrder: 1\n--- !u!1 &4\nGameObject:\n  m_Name: BackSprite\n  m_Layer: 0\n--- !u!4 &5\nTransform:\n  m_GameObject: {fileID: 4}\n  m_LocalPosition: {x: -1, y: 0, z: 0}\n  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n  m_LocalScale: {x: 1, y: 1, z: 1}\n  m_Children: []\n  m_Father: {fileID: 0}\n--- !u!212 &6\nSpriteRenderer:\n  m_GameObject: {fileID: 4}\n  m_Enabled: 1\n  m_Sprite: {fileID: 21300000, guid: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, type: 3}\n  m_Color: {r: 1, g: 1, b: 1, a: 0.5}\n  m_FlipX: 0\n  m_FlipY: 1\n  m_DrawMode: 0\n  m_SortingLayerID: 1\n  m_SortingOrder: 5\n--- !u!1 &7\nGameObject:\n  m_Name: DisabledSprite\n  m_Layer: 0\n--- !u!4 &8\nTransform:\n  m_GameObject: {fileID: 7}\n  m_LocalPosition: {x: 0, y: 0, z: 0}\n  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n  m_LocalScale: {x: 1, y: 1, z: 1}\n  m_Children: []\n  m_Father: {fileID: 0}\n--- !u!212 &9\nSpriteRenderer:\n  m_GameObject: {fileID: 7}\n  m_Enabled: 0\n  m_Sprite: {fileID: 21300000, guid: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, type: 3}\n  m_Color: {r: 1, g: 0, b: 0, a: 1}\n  m_SortingLayerID: 0\n  m_SortingOrder: 0\n";
        std::fs::write(&scene_path, scene_yaml)?;
        Ok(scene_path)
    }

    #[test]
    fn sprite_renderer_scene_extracts_sorted_sprite_instances()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = std::env::temp_dir().join(format!(
            "unitas_runtime_sprite_extract_{}",
            std::process::id()
        ));
        let scene_path = write_sprite_extraction_scene(&dir)?;

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
    fn import_scene_mesh_reads_from_bundle_cooked_manifest_without_raw_source_file()
    -> Result<(), Box<dyn std::error::Error>> {
        use unitas_asset::model::{COOKED_MESH_DIR_NAME, cook_project_meshes};

        let dir = std::env::temp_dir().join(format!(
            "unitas_runtime_cooked_bundle_mesh_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let project_dir = dir.join("project");
        let bundle_dir = dir.join("bundle");
        let cache_dir = dir.join("cache");
        std::fs::create_dir_all(project_dir.join("Assets/Models"))?;
        std::fs::create_dir_all(bundle_dir.join("Assets/Models"))?;
        std::fs::create_dir_all(bundle_dir.join("ProjectSettings"))?;

        let obj = "o Hero\nv 0.0 0.0 0.0\nv 1.0 0.0 0.0\nv 0.0 1.0 0.0\nvn 0.0 0.0 1.0\nvt 0.0 0.0\nvt 1.0 0.0\nvt 0.0 1.0\nf 1/1/1 2/2/1 3/3/1\n";
        std::fs::write(project_dir.join("Assets/Models/Hero.obj"), obj)?;
        let meta = "fileFormatVersion: 2\nguid: dddddddddddddddddddddddddddddddd\n";
        std::fs::write(project_dir.join("Assets/Models/Hero.obj.meta"), meta)?;
        std::fs::write(bundle_dir.join("Assets/Models/Hero.obj.meta"), meta)?;
        std::fs::write(bundle_dir.join("bundle.manifest"), "format_version=1\n")?;

        let (manifest, report) = cook_project_meshes(
            &project_dir,
            &cache_dir,
            &bundle_dir.join(COOKED_MESH_DIR_NAME),
        )?;
        assert_eq!(report.imported, 1);
        assert_eq!(manifest.entries.len(), 1);

        let staged_virtual_mesh_path = bundle_dir.join("Assets/Models/Hero.obj");
        assert!(!staged_virtual_mesh_path.exists());

        let scene_path = bundle_dir.join("Assets/MeshScene.unity");
        let scene_yaml = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &10\nGameObject:\n  m_Name: HeroMesh\n  m_Layer: 0\n--- !u!4 &11\nTransform:\n  m_GameObject: {fileID: 10}\n  m_LocalPosition: {x: 0, y: 0, z: 0}\n  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n  m_LocalScale: {x: 1, y: 1, z: 1}\n  m_Children: []\n  m_Father: {fileID: 0}\n--- !u!33 &12\nMeshFilter:\n  m_GameObject: {fileID: 10}\n  m_Mesh: {fileID: 4300000, guid: dddddddddddddddddddddddddddddddd, type: 3}\n--- !u!23 &13\nMeshRenderer:\n  m_GameObject: {fileID: 10}\n  m_Enabled: 1\n  m_Materials: []\n";
        std::fs::write(&scene_path, scene_yaml)?;

        let mut runtime = Runtime::new()?;
        runtime.load_scene(&scene_path)?;
        assert!(runtime.diagnostics().is_empty());
        let renderables = runtime.scene_renderables();
        assert_eq!(renderables.len(), 1);
        assert_eq!(renderables[0].mesh.positions.len(), 3);
        assert_eq!(renderables[0].mesh.indices.len(), 3);

        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }

    fn write_multilight_scene(
        dir: &Path,
    ) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        let _ = std::fs::remove_dir_all(dir);
        std::fs::create_dir_all(dir)?;
        let scene_path = dir.join("MultiLightScene.unity");
        let scene_yaml = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &10\nGameObject:\n  m_Name: Sun\n  m_Layer: 0\n--- !u!4 &11\nTransform:\n  m_GameObject: {fileID: 10}\n  m_LocalPosition: {x: 0, y: 10, z: 0}\n  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n  m_LocalScale: {x: 1, y: 1, z: 1}\n  m_Children: []\n  m_Father: {fileID: 0}\n--- !u!108 &12\nLight:\n  m_GameObject: {fileID: 10}\n  m_Enabled: 1\n  m_Type: 1\n  m_Color: {r: 1, g: 0.9, b: 0.8, a: 1}\n  m_Intensity: 0.8\n--- !u!1 &20\nGameObject:\n  m_Name: PointLamp\n  m_Layer: 0\n--- !u!4 &21\nTransform:\n  m_GameObject: {fileID: 20}\n  m_LocalPosition: {x: 1, y: 2, z: 3}\n  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n  m_LocalScale: {x: 1, y: 1, z: 1}\n  m_Children: []\n  m_Father: {fileID: 0}\n--- !u!108 &22\nLight:\n  m_GameObject: {fileID: 20}\n  m_Enabled: 1\n  m_Type: 2\n  m_Color: {r: 1, g: 0, b: 0, a: 1}\n  m_Intensity: 2.5\n--- !u!1 &30\nGameObject:\n  m_Name: SpotLamp\n  m_Layer: 0\n--- !u!4 &31\nTransform:\n  m_GameObject: {fileID: 30}\n  m_LocalPosition: {x: 4, y: 5, z: 6}\n  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n  m_LocalScale: {x: 1, y: 1, z: 1}\n  m_Children: []\n  m_Father: {fileID: 0}\n--- !u!108 &32\nLight:\n  m_GameObject: {fileID: 30}\n  m_Enabled: 1\n  m_Type: 0\n  m_Color: {r: 0, g: 1, b: 0, a: 1}\n  m_Intensity: 3.0\n";
        std::fs::write(&scene_path, scene_yaml)?;
        Ok(scene_path)
    }

    #[test]
    fn point_and_spot_lights_are_collected_without_runtime_unsupported_diagnostics()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = std::env::temp_dir().join(format!(
            "unitas_runtime_multilight_extract_{}",
            std::process::id()
        ));
        let scene_path = write_multilight_scene(&dir)?;

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

    fn write_urp_lit_material_assets(
        dir: &Path,
    ) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        let _ = std::fs::remove_dir_all(dir);
        let assets_dir = dir.join("Assets");
        std::fs::create_dir_all(&assets_dir)?;
        std::fs::create_dir_all(dir.join("ProjectSettings"))?;
        for (name, guid, rgb, tex_type) in [
            (
                "Base.tga",
                "11111111111111111111111111111111",
                [255, 0, 255],
                0,
            ),
            (
                "Normal.tga",
                "22222222222222222222222222222222",
                [0, 0, 255],
                1,
            ),
            (
                "Emissive.tga",
                "33333333333333333333333333333333",
                [255, 255, 0],
                0,
            ),
        ] {
            write_test_tga(&assets_dir.join(name), 4, 4, rgb)?;
            std::fs::write(
                assets_dir.join(format!("{name}.meta")),
                format!(
                    "fileFormatVersion: 2\nguid: {guid}\nTextureImporter:\n  sRGBTexture: 0\n  textureType: {tex_type}\n"
                ),
            )?;
        }
        let mat_yaml = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!21 &2100000\nMaterial:\n  m_Name: UrpLitMat\n  m_Shader: {fileID: 4800000, guid: 933532a4fcc9baf4fa0491de14d08ed7, type: 3}\n  m_SavedProperties:\n    serializedVersion: 3\n    m_TexEnvs:\n    - _BaseMap:\n        m_Texture: {fileID: 2800000, guid: 11111111111111111111111111111111, type: 3}\n        m_Scale: {x: 1, y: 1}\n        m_Offset: {x: 0, y: 0}\n    - _MainTex:\n        m_Texture: {fileID: 0}\n        m_Scale: {x: 1, y: 1}\n        m_Offset: {x: 0, y: 0}\n    - _BumpMap:\n        m_Texture: {fileID: 2800000, guid: 22222222222222222222222222222222, type: 3}\n        m_Scale: {x: 1, y: 1}\n        m_Offset: {x: 0, y: 0}\n    - _EmissionMap:\n        m_Texture: {fileID: 2800000, guid: 33333333333333333333333333333333, type: 3}\n        m_Scale: {x: 1, y: 1}\n        m_Offset: {x: 0, y: 0}\n    m_Floats:\n    - _Smoothness: 0.7\n    - _Metallic: 0.35\n    m_Colors:\n    - _BaseColor: {r: 0.25, g: 0.5, b: 0.75, a: 0.8}\n    - _EmissionColor: {r: 0.4, g: 0.2, b: 0.1, a: 1}\n";
        std::fs::write(assets_dir.join("UrpLitMat.mat"), mat_yaml)?;
        std::fs::write(
            assets_dir.join("UrpLitMat.mat.meta"),
            "fileFormatVersion: 2\nguid: 44444444444444444444444444444444\n",
        )?;
        let scene_path = assets_dir.join("UrpScene.unity");
        let scene_yaml = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &10\nGameObject:\n  m_Name: UrpCube\n  m_Layer: 0\n--- !u!4 &11\nTransform:\n  m_GameObject: {fileID: 10}\n  m_LocalPosition: {x: 0, y: 0, z: 0}\n  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n  m_LocalScale: {x: 1, y: 1, z: 1}\n  m_Children: []\n  m_Father: {fileID: 0}\n--- !u!33 &12\nMeshFilter:\n  m_GameObject: {fileID: 10}\n  m_Mesh: {fileID: 10202, guid: 0000000000000000e000000000000000, type: 0}\n--- !u!23 &13\nMeshRenderer:\n  m_GameObject: {fileID: 10}\n  m_Enabled: 1\n  m_Materials:\n  - {fileID: 2100000, guid: 44444444444444444444444444444444, type: 2}\n";
        std::fs::write(&scene_path, scene_yaml)?;
        Ok(scene_path)
    }

    #[test]
    fn urp_lit_material_in_scene_binds_base_map_base_color_normal_emission_and_pbr_into_render_material()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = std::env::temp_dir().join(format!(
            "unitas_runtime_urp_material_bind_{}",
            std::process::id()
        ));
        let scene_path = write_urp_lit_material_assets(&dir)?;

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

    fn write_sprite_swap_scene(
        dir: &Path,
        guid_a: &str,
        guid_b: Option<&str>,
    ) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        let _ = std::fs::remove_dir_all(dir);
        let assets_dir = dir.join("Assets");
        std::fs::create_dir_all(&assets_dir)?;
        std::fs::create_dir_all(dir.join("ProjectSettings"))?;
        write_test_tga(&assets_dir.join("SpriteA.tga"), 100, 50, [255, 0, 0])?;
        std::fs::write(
            assets_dir.join("SpriteA.tga.meta"),
            format!("fileFormatVersion: 2\nguid: {guid_a}\nTextureImporter:\n  sRGBTexture: 0\n"),
        )?;
        if let Some(gb) = guid_b {
            write_test_tga(&assets_dir.join("SpriteB.tga"), 200, 80, [0, 255, 0])?;
            std::fs::write(
                assets_dir.join("SpriteB.tga.meta"),
                format!("fileFormatVersion: 2\nguid: {gb}\nTextureImporter:\n  sRGBTexture: 0\n"),
            )?;
        }
        let scene_path = assets_dir.join("SpriteSwapScene.unity");
        let scene_yaml = format!(
            "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &1\nGameObject:\n  m_Name: HeroSprite\n  m_Layer: 0\n--- !u!4 &2\nTransform:\n  m_GameObject: {{fileID: 1}}\n  m_LocalPosition: {{x: 0, y: 0, z: 0}}\n  m_LocalRotation: {{x: 0, y: 0, z: 0, w: 1}}\n  m_LocalScale: {{x: 1, y: 1, z: 1}}\n  m_Children: []\n  m_Father: {{fileID: 0}}\n--- !u!212 &3\nSpriteRenderer:\n  m_GameObject: {{fileID: 1}}\n  m_Enabled: 1\n  m_Sprite: {{fileID: 21300000, guid: {guid_a}, type: 3}}\n  m_Color: {{r: 1, g: 1, b: 1, a: 1}}\n  m_SortingLayerID: 0\n  m_SortingOrder: 0\n"
        );
        std::fs::write(&scene_path, scene_yaml)?;
        Ok(scene_path)
    }

    fn build_sprite_swap_controller(
        guid_a: &str,
        guid_b: &str,
        file_id_b: i64,
        wrap: unitas_animation::WrapMode,
    ) -> Result<unitas_animation::AnimatorController, Box<dyn std::error::Error>> {
        use unitas_animation::curve::{AssetReference, ObjectReferenceCurve, ObjectReferenceKey};
        use unitas_animation::{
            AnimationClip, AnimatorController, AnimatorState, CurveBinding, CurveBindingTarget,
            PropertyCurve, StateMotion,
        };
        let ref_a = AssetReference::from_hex(guid_a, 21300000).ok_or("invalid guid_a")?;
        let ref_b = AssetReference::from_hex(guid_b, file_id_b).ok_or("invalid guid_b")?;
        let clip = AnimationClip::new(
            "SwapSprites",
            2.0,
            60.0,
            wrap,
            vec![CurveBinding::new(
                CurveBindingTarget::new("", "SpriteRenderer", "m_Sprite"),
                PropertyCurve::ObjectReference(ObjectReferenceCurve::new(vec![
                    ObjectReferenceKey::new(0.0, ref_a),
                    ObjectReferenceKey::new(1.0, ref_b),
                ])),
            )],
            Vec::new(),
        );
        Ok(AnimatorController::new(
            "HeroController",
            Vec::new(),
            vec![AnimatorState::new("SwapSprites", StateMotion::Clip(clip))],
            0,
            Vec::new(),
        ))
    }

    #[test]
    fn ani_001_object_reference_curve_swaps_sprite_renderer_sprite_at_keyed_time()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = std::env::temp_dir().join(format!(
            "unitas_runtime_sprite_swap_anim_{}",
            std::process::id()
        ));
        let guid_a = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let guid_b = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let scene_path = write_sprite_swap_scene(&dir, guid_a, Some(guid_b))?;
        let controller = build_sprite_swap_controller(
            guid_a,
            guid_b,
            21300000,
            unitas_animation::WrapMode::Loop,
        )?;

        let mut runtime = Runtime::new()?;
        runtime.load_scene(&scene_path)?;
        let entity = EntityId::new(1);
        runtime.ensure_animator(entity.as_u64());
        let handle = runtime.animation_mut().register_controller(controller);
        runtime.set_animator_controller(entity, handle)?;

        let initial_sprites = runtime.sprite_instances();
        assert_eq!(initial_sprites.len(), 1);
        assert_eq!(initial_sprites[0].texture.width, 100);
        assert_eq!(initial_sprites[0].texture.height, 50);
        assert_eq!(initial_sprites[0].texture.pixels[0], [255, 0, 0]);

        runtime.tick(0.0)?;
        runtime.tick(1.0)?;

        let swapped_sprites = runtime.sprite_instances();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(swapped_sprites.len(), 1);
        assert_eq!(swapped_sprites[0].texture.width, 200);
        assert_eq!(swapped_sprites[0].texture.height, 80);
        assert_eq!(swapped_sprites[0].texture.pixels[0], [0, 255, 0]);
        assert!(
            runtime.diagnostics().is_empty(),
            "expected no diagnostics on valid sprite swap, got {:?}",
            runtime.diagnostics()
        );
        Ok(())
    }

    #[test]
    fn ani_001_unresolvable_sprite_reference_produces_diagnostic_naming_asset_and_member_without_crash()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = std::env::temp_dir().join(format!(
            "unitas_runtime_sprite_swap_missing_{}",
            std::process::id()
        ));
        let guid_a = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let missing_guid = "deadbeefdeadbeefdeadbeefdeadbeef";
        let scene_path = write_sprite_swap_scene(&dir, guid_a, None)?;
        let controller = build_sprite_swap_controller(
            guid_a,
            missing_guid,
            21300099,
            unitas_animation::WrapMode::Once,
        )?;

        let mut runtime = Runtime::new()?;
        runtime.load_scene(&scene_path)?;
        let entity = EntityId::new(1);
        runtime.ensure_animator(entity.as_u64());
        let handle = runtime.animation_mut().register_controller(controller);
        runtime.set_animator_controller(entity, handle)?;
        assert!(runtime.diagnostics().is_empty());

        runtime.tick(0.0)?;
        runtime.tick(1.0)?;
        let _ = std::fs::remove_dir_all(&dir);

        let diags = runtime.entity_diagnostics(entity);
        assert!(
            diags.iter().any(|msg| msg.contains(missing_guid)
                && msg.contains("SpriteRenderer")
                && msg.contains("m_Sprite")),
            "expected diagnostic naming asset guid and SpriteRenderer.m_Sprite, got {diags:?}"
        );
        Ok(())
    }
}
