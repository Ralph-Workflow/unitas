use std::sync::{Mutex, MutexGuard, OnceLock};
use unitas_render::camera::{look_at, Camera};
use unitas_render::gpu::{GpuMaterialInstance, Renderer};
use unitas_render::math::{Mat4, Vec3};
use unitas_render::mesh::{cube, Mesh};
use unitas_render::model::{
    effective_alpha, BlendMode, DirectionalLight, Material, MaterialInstance, MeshInstance,
};
use unitas_render::render_graph::{RenderGraph, RenderPassKind};

fn shared_renderer() -> Result<MutexGuard<'static, Renderer>, String> {
    static RENDERER: OnceLock<Result<Mutex<Renderer>, String>> = OnceLock::new();
    let slot = RENDERER.get_or_init(|| {
        Renderer::new()
            .map(Mutex::new)
            .map_err(|error| error.to_string())
    });
    let mutex = match slot {
        Ok(mutex) => mutex,
        Err(message) => return Err(String::clone(message)),
    };
    match mutex.lock() {
        Ok(guard) => Ok(guard),
        Err(poisoned) => Ok(poisoned.into_inner()),
    }
}

fn camera() -> Camera {
    Camera::perspective(60.0, 1.0, 0.1, 100.0)
}

fn view() -> Mat4 {
    look_at(
        Vec3::new(0.0, 0.0, -5.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    )
}

fn unlit_light() -> DirectionalLight {
    DirectionalLight {
        direction: Vec3::new(0.0, 0.0, -1.0),
        color: [1.0, 1.0, 1.0],
        intensity: 0.0,
    }
}

fn translated(x: f32, y: f32, z: f32) -> Mat4 {
    let mut model = Mat4::identity();
    model.set(3, 0, x);
    model.set(3, 1, y);
    model.set(3, 2, z);
    model
}

fn quad(extent: f32) -> Mesh {
    let positions = [
        [-extent, -extent, 0.0],
        [-extent, extent, 0.0],
        [extent, extent, 0.0],
        [extent, -extent, 0.0],
    ];
    let normals = [[0.0, 0.0, -1.0]; 4];
    let indices = [0u32, 1, 2, 0, 2, 3];
    Mesh::from_buffers(&positions, &normals, &indices)
}

#[test]
fn opaque_cube_renders_shaded_center_and_background_deterministically(
) -> Result<(), Box<dyn std::error::Error>> {
    let renderer = shared_renderer()?;
    let mesh = cube(1.0);
    let instances = [MeshInstance {
        mesh: &mesh,
        model: translated(0.0, 0.0, -1.0)
            * Mat4::from_cols_array([
                2.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ]),
        color: [0.8, 0.4, 0.2],
    }];
    let light = DirectionalLight {
        direction: Vec3::new(0.0, 0.0, -1.0),
        color: [0.9, 0.8, 0.7],
        intensity: 0.5,
    };
    let first = renderer.render_offscreen(
        &instances,
        &camera(),
        view(),
        &light,
        [0.2, 0.4, 0.6],
        (64, 64),
    )?;
    let second = renderer.render_offscreen(
        &instances,
        &camera(),
        view(),
        &light,
        [0.2, 0.4, 0.6],
        (64, 64),
    )?;
    assert_eq!(first.pixels, second.pixels);
    assert_eq!(first.get_pixel(32, 32), [194, 92, 43]);
    assert_eq!(first.get_pixel(0, 0), [51, 102, 153]);
    Ok(())
}

#[test]
fn translucent_quad_blends_over_opaque_background() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = shared_renderer()?;
    let mesh = quad(2.0);
    let opaque = Material::plain();
    let translucent = Material {
        alpha: 0.5,
        blend: BlendMode::Blend,
        ..Material::plain()
    };
    let instances = [
        GpuMaterialInstance::from(MaterialInstance {
            mesh: &mesh,
            attributes: None,
            model: translated(0.0, 0.0, 1.0),
            color: [0.0, 0.0, 1.0],
            material: &opaque,
        }),
        GpuMaterialInstance::from(MaterialInstance {
            mesh: &mesh,
            attributes: None,
            model: translated(0.0, 0.0, -0.5),
            color: [1.0, 0.0, 0.0],
            material: &translucent,
        }),
    ];
    let image = renderer.render_offscreen_materials(
        &instances,
        &camera(),
        view(),
        &unlit_light(),
        [0.0, 0.5, 0.0],
        (32, 32),
    )?;
    assert_eq!(image.get_pixel(16, 16), [128, 0, 128]);
    assert_eq!(image.get_pixel(0, 0), [0, 128, 0]);
    Ok(())
}

#[test]
fn translucent_quads_are_drawn_back_to_front() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = shared_renderer()?;
    let mesh = quad(2.0);
    let red = Material {
        alpha: 0.5,
        blend: BlendMode::Blend,
        ..Material::plain()
    };
    let blue = Material {
        alpha: 0.5,
        blend: BlendMode::Blend,
        ..Material::plain()
    };
    let near_first = [
        GpuMaterialInstance {
            mesh: &mesh,
            attributes: None,
            model: translated(0.0, 0.0, -0.5),
            color: [1.0, 0.0, 0.0],
            material: &red,
        },
        GpuMaterialInstance {
            mesh: &mesh,
            attributes: None,
            model: translated(0.0, 0.0, 0.5),
            color: [0.0, 0.0, 1.0],
            material: &blue,
        },
    ];
    let far_first = [near_first[1], near_first[0]];
    let near_first_image = renderer.render_offscreen_materials(
        &near_first,
        &camera(),
        view(),
        &unlit_light(),
        [0.0, 0.0, 0.0],
        (32, 32),
    )?;
    let far_first_image = renderer.render_offscreen_materials(
        &far_first,
        &camera(),
        view(),
        &unlit_light(),
        [0.0, 0.0, 0.0],
        (32, 32),
    )?;
    assert_eq!(near_first_image.get_pixel(16, 16), [128, 0, 64]);
    assert_eq!(far_first_image.get_pixel(16, 16), [128, 0, 64]);
    Ok(())
}

#[test]
fn nearer_opaque_occludes_farther_translucent_and_alpha_boundaries_hold(
) -> Result<(), Box<dyn std::error::Error>> {
    let renderer = shared_renderer()?;
    let mesh = quad(2.0);
    let opaque = Material::plain();
    let translucent = Material {
        alpha: 0.5,
        blend: BlendMode::Blend,
        ..Material::plain()
    };
    let invisible = Material {
        alpha: 0.0,
        blend: BlendMode::Blend,
        ..Material::plain()
    };
    let full_blend = Material {
        alpha: 1.0,
        blend: BlendMode::Blend,
        ..Material::plain()
    };
    assert!((effective_alpha(&opaque) - 1.0).abs() < 1e-6);
    assert!((effective_alpha(&translucent) - 0.5).abs() < 1e-6);
    assert!(effective_alpha(&invisible).abs() < 1e-6);
    assert!((effective_alpha(&full_blend) - 1.0).abs() < 1e-6);
    assert_eq!(
        RenderGraph::offscreen(false, false).pass_order(),
        vec![RenderPassKind::Opaque, RenderPassKind::Transparent]
    );

    let occluded_instances = [
        GpuMaterialInstance {
            mesh: &mesh,
            attributes: None,
            model: translated(0.0, 0.0, 1.0),
            color: [1.0, 0.0, 0.0],
            material: &translucent,
        },
        GpuMaterialInstance {
            mesh: &mesh,
            attributes: None,
            model: translated(0.0, 0.0, -0.5),
            color: [0.0, 0.0, 1.0],
            material: &opaque,
        },
    ];
    let occluded = renderer.render_offscreen_materials(
        &occluded_instances,
        &camera(),
        view(),
        &unlit_light(),
        [0.0, 0.0, 0.0],
        (32, 32),
    )?;
    assert_eq!(occluded.get_pixel(16, 16), [0, 0, 255]);

    let zero_alpha_instances = [GpuMaterialInstance {
        mesh: &mesh,
        attributes: None,
        model: translated(0.0, 0.0, -0.5),
        color: [1.0, 0.0, 0.0],
        material: &invisible,
    }];
    let zero_alpha_image = renderer.render_offscreen_materials(
        &zero_alpha_instances,
        &camera(),
        view(),
        &unlit_light(),
        [0.0, 1.0, 0.0],
        (32, 32),
    )?;
    assert_eq!(zero_alpha_image.get_pixel(16, 16), [0, 255, 0]);
    Ok(())
}
