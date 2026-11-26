use unitas_render::camera::{look_at, Camera};
use unitas_render::image::Image;
use unitas_render::math::{Mat4, Vec3};
use unitas_render::mesh::{cube, Mesh};
use unitas_render::raster::{
    render_scene, render_scene_materials, BlendMode, DirectionalLight, Material, MaterialInstance,
    MeshInstance,
};

/// FNV-1a 64 over the pixel bytes of the opaque fixture, captured from the
/// scene of `perspective_camera_renders_cube_center_and_background`
/// (crates/unitas-render/tests/render/render_tests.rs:266) before the
/// translucency work; the opaque path must keep producing these bytes.
const OPAQUE_CUBE_FIXTURE_HASH: u64 = 0xfa8e_17c5_2e3b_83b5;

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn pixel_bytes(image: &Image) -> Vec<u8> {
    image
        .pixels
        .iter()
        .flat_map(|pixel| pixel.iter().copied())
        .collect()
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

fn opaque_cube_fixture() -> Image {
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
    render_scene(
        &instances,
        &camera(),
        view(),
        &light,
        [0.2, 0.4, 0.6],
        64,
        64,
    )
}

#[test]
fn opaque_cube_render_bytes_are_unchanged() {
    let image = opaque_cube_fixture();
    assert_eq!(fnv1a(&pixel_bytes(&image)), OPAQUE_CUBE_FIXTURE_HASH);
    assert_eq!(image.get_pixel(32, 32), [194, 92, 43]);
    assert_eq!(image.get_pixel(0, 0), [51, 102, 153]);
}

#[test]
fn translucent_quad_blends_over_opaque_background() {
    let mesh = quad(2.0);
    let opaque = Material::plain();
    let translucent = Material {
        alpha: 0.5,
        blend: BlendMode::Blend,
        ..Material::plain()
    };
    let instances = [
        MaterialInstance {
            mesh: &mesh,
            attributes: None,
            model: translated(0.0, 0.0, 1.0),
            color: [0.0, 0.0, 1.0],
            material: &opaque,
        },
        MaterialInstance {
            mesh: &mesh,
            attributes: None,
            model: translated(0.0, 0.0, -0.5),
            color: [1.0, 0.0, 0.0],
            material: &translucent,
        },
    ];
    let image = render_scene_materials(
        &instances,
        &camera(),
        view(),
        &unlit_light(),
        [0.0, 0.5, 0.0],
        32,
        32,
    );
    // The translucent red quad is drawn after the opaque blue quad, so the
    // centre pixel is 0.5 * red + 0.5 * blue.
    assert_eq!(image.get_pixel(16, 16), [128, 0, 128]);
    assert_eq!(image.get_pixel(0, 0), [0, 128, 0]);
}

#[test]
fn translucent_quads_are_drawn_back_to_front() {
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
    let instances = [
        MaterialInstance {
            mesh: &mesh,
            attributes: None,
            model: translated(0.0, 0.0, -0.5),
            color: [1.0, 0.0, 0.0],
            material: &red,
        },
        MaterialInstance {
            mesh: &mesh,
            attributes: None,
            model: translated(0.0, 0.0, 0.5),
            color: [0.0, 0.0, 1.0],
            material: &blue,
        },
    ];
    let image = render_scene_materials(
        &instances,
        &camera(),
        view(),
        &unlit_light(),
        [0.0, 0.0, 0.0],
        32,
        32,
    );
    // The far blue quad is drawn first even though the near red quad is
    // listed first, so the centre pixel is 0.5 * red + (0.5 * blue) * 0.5.
    assert_eq!(image.get_pixel(16, 16), [128, 0, 64]);
}

#[cfg(feature = "device")]
#[test]
fn gpu_blends_translucent_quad_over_opaque_background_like_the_cpu_raster(
) -> Result<(), Box<dyn std::error::Error>> {
    use unitas_render::gpu::{GpuMaterialInstance, Renderer, GPU_CPU_MEAN_ERROR_TOLERANCE};

    let renderer = match Renderer::new() {
        Ok(renderer) => renderer,
        Err(error) => {
            eprintln!("skipping GPU test, no GPU device available: {error}");
            return Ok(());
        }
    };
    let mesh = quad(2.0);
    let opaque = Material::plain();
    let translucent = Material {
        alpha: 0.5,
        blend: BlendMode::Blend,
        ..Material::plain()
    };
    let gpu_instances = [
        GpuMaterialInstance {
            mesh: &mesh,
            attributes: None,
            model: translated(0.0, 0.0, 1.0),
            color: [0.0, 0.0, 1.0],
            material: &opaque,
        },
        GpuMaterialInstance {
            mesh: &mesh,
            attributes: None,
            model: translated(0.0, 0.0, -0.5),
            color: [1.0, 0.0, 0.0],
            material: &translucent,
        },
    ];
    let cpu_instances = [
        MaterialInstance {
            mesh: &mesh,
            attributes: None,
            model: translated(0.0, 0.0, 1.0),
            color: [0.0, 0.0, 1.0],
            material: &opaque,
        },
        MaterialInstance {
            mesh: &mesh,
            attributes: None,
            model: translated(0.0, 0.0, -0.5),
            color: [1.0, 0.0, 0.0],
            material: &translucent,
        },
    ];
    let gpu = renderer.render_offscreen_materials(
        &gpu_instances,
        &camera(),
        view(),
        &unlit_light(),
        [0.0, 0.5, 0.0],
        (32, 32),
    )?;
    let cpu = render_scene_materials(
        &cpu_instances,
        &camera(),
        view(),
        &unlit_light(),
        [0.0, 0.5, 0.0],
        32,
        32,
    );
    assert_eq!(gpu.get_pixel(16, 16), [128, 0, 128]);
    assert_eq!(gpu.get_pixel(0, 0), [0, 128, 0]);
    let mut total = 0.0f32;
    let mut samples = 0usize;
    for (gpu_pixel, cpu_pixel) in gpu.pixels.iter().zip(cpu.pixels.iter()) {
        for channel in 0..3 {
            total += (f32::from(gpu_pixel[channel]) - f32::from(cpu_pixel[channel])).abs();
            samples += 1;
        }
    }
    let mean_error = total / samples as f32;
    assert!(
        mean_error <= GPU_CPU_MEAN_ERROR_TOLERANCE,
        "gpu/cpu mean channel error {mean_error} exceeds the declared tolerance {GPU_CPU_MEAN_ERROR_TOLERANCE}"
    );
    Ok(())
}

#[cfg(feature = "device")]
#[test]
fn gpu_draws_translucent_quads_back_to_front() -> Result<(), Box<dyn std::error::Error>> {
    use unitas_render::gpu::{GpuMaterialInstance, Renderer};

    let renderer = match Renderer::new() {
        Ok(renderer) => renderer,
        Err(error) => {
            eprintln!("skipping GPU test, no GPU device available: {error}");
            return Ok(());
        }
    };
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
    let instances = [
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
    let gpu = renderer.render_offscreen_materials(
        &instances,
        &camera(),
        view(),
        &unlit_light(),
        [0.0, 0.0, 0.0],
        (32, 32),
    )?;
    assert_eq!(gpu.get_pixel(16, 16), [128, 0, 64]);
    Ok(())
}
