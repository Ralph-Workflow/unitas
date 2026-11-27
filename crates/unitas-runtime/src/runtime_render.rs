use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use unitas_asset::scene::{MeshRendererComponent, RenderBinding};
use unitas_core::{EntityId, IDENTITY_ROTATION, SceneGraph, rotate_vector};
use unitas_render::image::{Image, Rgba8};
use unitas_render::ui_pass::{GlyphTextPainter, UiTarget, composite_ui};
use unitas_render::{
    Camera, DirectionalLight, Mat4, Mesh, MeshAttributes, Renderer, Vec3, cube, look_at,
};

use super::{LoadedScene, Runtime};
use crate::canvas::UiDrawItem;
use crate::error::RenderError;
use crate::presenter::{GpuCameraBinding, scene_gpu_meshes};

pub const DEFAULT_MATERIAL_COLOR: [f32; 4] = [0.5, 0.5, 0.5, 1.0];

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

/// One scene renderer a presenter can draw: the mesh it draws, the material
/// it draws it with and the entity it belongs to.
#[derive(Clone, Copy, Debug)]
pub struct SceneRenderable<'a> {
    pub entity: EntityId,
    pub mesh: &'a Mesh,
    pub attributes: &'a MeshAttributes,
    pub material: &'a RenderMaterial,
    pub color: [f32; 3],
}

/// A render mesh with the texture coordinates and tangents the shader reads
/// beside its positions.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderMesh {
    pub mesh: Mesh,
    pub attributes: MeshAttributes,
}

/// The scene camera a presenter can draw with: the entity it follows and the
/// projection and background parameters.
#[derive(Clone, Copy, Debug)]
pub struct SceneCamera {
    pub entity: Option<EntityId>,
    pub params: RenderCameraParams,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderLight {
    pub entity: Option<EntityId>,
    pub direction: [f32; 3],
    pub color: [f32; 3],
    pub intensity: f32,
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
                direction: [0.0, 1.0, 0.0],
                color: [1.0, 1.0, 1.0],
                intensity: 1.0,
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
            } else if let Some(guid) = &renderer.material_guid {
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
        for light in &binding.lights {
            let entity = EntityId::new(entity_base + light.entity.as_u64());
            if !light.directional {
                if let Some(diagnostic) = &light.unsupported_diagnostic {
                    diagnostics.push((entity, diagnostic.clone()));
                }
                continue;
            }
            let rotation = graph
                .world_transform(entity)
                .map(|transform| transform.rotation)
                .unwrap_or(IDENTITY_ROTATION);
            let direction = toward_light_direction(rotation, light.direction);
            self.render.light_defaults.insert(
                entity,
                RenderLight {
                    entity: Some(entity),
                    direction,
                    color: light.color,
                    intensity: light.intensity,
                },
            );
            let handle = self.create_render_light(Some(entity));
            self.update_render_light(handle, direction, light.color, light.intensity);
        }
        diagnostics
    }

    // Resolve a scene-declared MeshFilter reference into a render mesh:
    // Unity built-in primitives by (fileID, guid), otherwise an imported
    // model file. Returns None when the mesh cannot be resolved.
    fn import_scene_mesh(&mut self, renderer: &MeshRendererComponent) -> Option<u64> {
        let mesh_data = if let (Some(file_id), Some(guid)) =
            (renderer.mesh_file_id, renderer.mesh_guid.as_deref())
        {
            let primitive = unitas_asset::primitive::builtin_primitive_by_identity(file_id, guid)?;
            let mut data = unitas_asset::primitive::primitive_mesh(primitive);
            if primitive == unitas_asset::primitive::BuiltinPrimitive::Sphere {
                // Unity's built-in sphere is radius 0.5; unitas generates radius 1.
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

    /// The scene's directional light, or a default one when the scene has
    /// none.
    pub fn directional_light(&self) -> DirectionalLight {
        let mut selected: Option<(u64, RenderLight)> = None;
        for (handle, light) in &self.render.lights {
            let replace = match selected {
                Some((best, _)) => *handle > best,
                None => true,
            };
            if replace {
                selected = Some((*handle, *light));
            }
        }
        match selected {
            Some((_, light)) => DirectionalLight {
                direction: Vec3::new(light.direction[0], light.direction[1], light.direction[2]),
                color: light.color,
                intensity: light.intensity,
            },
            None => DirectionalLight {
                direction: Vec3::new(0.0, 1.0, 0.0),
                color: [1.0, 1.0, 1.0],
                intensity: 1.0,
            },
        }
    }

    /// The loaded scene's enabled renderers with the mesh and material colour
    /// each one draws, in the deterministic order the renderer draws them.
    pub fn scene_renderables(&self) -> Vec<SceneRenderable<'_>> {
        // Renderer handles are sorted so the instance order, and therefore the
        // rendered image, is the same in every process: a HashMap's iteration
        // order is seeded per process, so consecutive captures of one scene
        // otherwise disagree on which coincident surface wins the depth test.
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

    /// The scene camera with the highest handle, so the choice does not depend
    /// on map iteration order.
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
        let mut presenter = crate::presenter::GpuFramePresenter::new(renderer);
        presenter.bind_scene(self);
        crate::capture::capture_offscreen(&mut presenter, self, width, height)
            .map_err(|error| RenderError::DeviceFailed(error.to_string()))
    }

    /// Draws the canvas content of the loaded scenes onto a rendered frame, so
    /// the interface is part of the picture instead of only its state.
    pub(crate) fn composite_canvas_image(&self, image: Image, width: u32, height: u32) -> Image {
        let items = self.ui_draw_items_for([width as f32, height as f32]);
        if items.is_empty() {
            return image;
        }
        let mut rgba = image.to_rgba8();
        self.composite_canvas_into(&mut rgba, &items);
        Image::from_rgba8(rgba)
    }

    /// The same composition as `composite_canvas_image` for a frame that is
    /// already in RGBA, so the windowed path does not convert the frame
    /// between colour formats.
    pub(crate) fn composite_canvas_rgba(&self, mut rgba: Rgba8, width: u32, height: u32) -> Rgba8 {
        let items = self.ui_draw_items_for([width as f32, height as f32]);
        if items.is_empty() {
            return rgba;
        }
        self.composite_canvas_into(&mut rgba, &items);
        rgba
    }

    /// The one canvas composition both the capture path and the windowed path
    /// draw the interface with.
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

    /// The renderer every frame of this runtime is rendered by.
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

    /// The mesh the particle and trail effects are drawn with.
    pub(crate) fn effect_mesh(&self) -> &Mesh {
        &self.render.effect_mesh
    }

    /// The name of an entity of the loaded scenes.
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

/// The material a scene renderer draws with: its colour, the maps its
/// material declares and the surface floats the shader reads.
fn render_material_for(renderer: &MeshRendererComponent) -> RenderMaterial {
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

/// The texture at a path as the shader reads it, decoded with the asset
/// import path and linearised when the importer says it holds colour.
fn load_texture_image(path: Option<&Path>) -> Option<Image> {
    let path = path?;
    let bytes = std::fs::read(path).ok()?;
    let name = path.file_name().and_then(|name| name.to_str())?;
    let settings = texture_import_settings(path);
    let imported = unitas_asset::texture::import::import_texture(&bytes, name, &settings).ok()?;
    let mut image = image_from_decoded(&imported.texture);
    if !settings.is_linear() {
        linearize_image(&mut image);
    }
    Some(image)
}

/// The importer settings of the texture beside its path, which say whether it
/// holds colour or a normal map.
fn texture_import_settings(path: &Path) -> unitas_asset::texture::import::TextureImportSettings {
    let mut meta = path.as_os_str().to_os_string();
    meta.push(".meta");
    let text = std::fs::read_to_string(PathBuf::from(meta)).ok();
    match text {
        Some(text) => unitas_asset::texture::import::TextureImportSettings::from_meta_text(&text)
            .unwrap_or_default(),
        None => unitas_asset::texture::import::TextureImportSettings::default(),
    }
}

fn image_from_decoded(texture: &unitas_asset::texture::import::DecodedTexture) -> Image {
    let rgba = unitas_asset::texture::import::texture_to_rgba8(texture);
    let mut image = Image::new(texture.width, texture.height);
    for (index, pixel) in image.pixels.iter_mut().enumerate() {
        let offset = index * 4;
        pixel[0] = rgba.get(offset).copied().unwrap_or(0);
        pixel[1] = rgba.get(offset + 1).copied().unwrap_or(0);
        pixel[2] = rgba.get(offset + 2).copied().unwrap_or(0);
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
