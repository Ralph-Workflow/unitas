use super::render_tests::shared_renderer;
use unitas_render::camera::{look_at, Camera};
use unitas_render::gpu::{RenderError, Renderer};
use unitas_render::image::Image;
use unitas_render::math::{Mat4, Vec3};
use unitas_render::mesh::{cube, Mesh};
use unitas_render::model::{DirectionalLight, MeshInstance};
use unitas_render::shadow::{shadow_map_size, ShadowSettings};

fn floor_quad(half_extent: f32) -> Mesh {
    Mesh {
        positions: vec![
            Vec3::new(-half_extent, 0.0, -half_extent),
            Vec3::new(half_extent, 0.0, -half_extent),
            Vec3::new(half_extent, 0.0, half_extent),
            Vec3::new(-half_extent, 0.0, half_extent),
        ],
        normals: vec![
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ],
        indices: vec![0, 2, 1, 0, 3, 2],
    }
}

fn translated_scaled(scale: f32, translation: Vec3) -> Mat4 {
    Mat4::from_cols_array([
        scale,
        0.0,
        0.0,
        0.0,
        0.0,
        scale,
        0.0,
        0.0,
        0.0,
        0.0,
        scale,
        0.0,
        translation.x,
        translation.y,
        translation.z,
        1.0,
    ])
}

fn region_mean_luma(
    image: &Image,
    x_range: std::ops::Range<u32>,
    y_range: std::ops::Range<u32>,
) -> f32 {
    let mut sum = 0.0_f64;
    let mut count = 0_u64;
    for y in y_range {
        for x in x_range.clone() {
            let [r, g, b] = image.get_pixel(x, y);
            sum += (f64::from(r) + f64::from(g) + f64::from(b)) / 3.0;
            count += 1;
        }
    }
    if count == 0 {
        0.0
    } else {
        (sum / (count as f64)) as f32
    }
}

fn shadow_camera() -> Camera {
    Camera::orthographic(6.0, 1.0, 0.1, 100.0)
}

fn shadow_view() -> Mat4 {
    look_at(
        Vec3::new(0.0, 10.0, -0.001),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
    )
}

fn shadow_light() -> DirectionalLight {
    DirectionalLight {
        direction: Vec3::new(-1.0, 1.0, 0.0).normalize(),
        color: [1.0, 1.0, 1.0],
        intensity: 0.75,
    }
}

fn shadow_instances<'a>(floor: &'a Mesh, caster: &'a Mesh) -> [MeshInstance<'a>; 2] {
    [
        MeshInstance {
            mesh: floor,
            model: Mat4::identity(),
            color: [0.9, 0.9, 0.9],
        },
        MeshInstance {
            mesh: caster,
            model: translated_scaled(2.0, Vec3::new(0.0, 2.5, 0.0)),
            color: [0.8, 0.3, 0.2],
        },
    ]
}

fn enabled_shadow_settings(resolution_index: u32, distance: f32, soft: bool) -> ShadowSettings {
    ShadowSettings {
        enabled: true,
        soft,
        resolution_index,
        distance,
    }
}

fn render_shadow_quality(
    renderer: &Renderer,
    instances: &[MeshInstance],
    camera: &Camera,
    view: Mat4,
    light: &DirectionalLight,
    settings: ShadowSettings,
) -> Result<Image, RenderError> {
    renderer.set_shadow_settings(settings);
    renderer.render_offscreen(instances, camera, view, light, [0.0, 0.0, 0.0], (256, 256))
}

fn assert_shadow_contrast(
    image: &Image,
    label: &str,
    shadowed_x: &std::ops::Range<u32>,
    lit_x: &std::ops::Range<u32>,
    sample_y: &std::ops::Range<u32>,
) -> f32 {
    let shadowed = region_mean_luma(image, shadowed_x.clone(), sample_y.clone());
    let lit = region_mean_luma(image, lit_x.clone(), sample_y.clone());
    assert!(
        shadowed > 30.0,
        "{label} shadowed mean {shadowed} should preserve ambient light"
    );
    assert!(
        lit - shadowed > 40.0,
        "{label} lit {lit} vs shadowed {shadowed} delta must exceed 40.0"
    );
    lit
}

fn assert_unshadowed_matches_lit(
    image: &Image,
    shadowed_x: &std::ops::Range<u32>,
    lit_x: &std::ops::Range<u32>,
    sample_y: &std::ops::Range<u32>,
    reference_lit: f32,
) {
    let shadowed = region_mean_luma(image, shadowed_x.clone(), sample_y.clone());
    let lit = region_mean_luma(image, lit_x.clone(), sample_y.clone());
    assert!(
        (lit - shadowed).abs() <= 2.0,
        "disabled shadow region {shadowed} must match lit region {lit}"
    );
    assert!(
        (lit - reference_lit).abs() <= 2.0,
        "disabled lit {lit} must match unoccluded soft lit {reference_lit}"
    );
}

#[test]
fn directional_shadow_caster_darkens_receiver_when_enabled_and_matches_when_disabled(
) -> Result<(), Box<dyn std::error::Error>> {
    let renderer = shared_renderer()?;
    let (floor, caster) = (floor_quad(8.0), cube(1.0));
    let (camera, view, light) = (shadow_camera(), shadow_view(), shadow_light());
    let instances = shadow_instances(&floor, &caster);
    let (shadowed_x, lit_x, sample_y) = (165..185, 15..55, 110..146);

    let soft_img = render_shadow_quality(
        &renderer,
        &instances,
        &camera,
        view,
        &light,
        enabled_shadow_settings(2, 50.0, true),
    );
    let soft_map_size = renderer.shadow_map_size();
    let hard_img = render_shadow_quality(
        &renderer,
        &instances,
        &camera,
        view,
        &light,
        enabled_shadow_settings(2, 50.0, false),
    );
    renderer.set_shadow_settings(ShadowSettings::disabled());
    let disabled_img = renderer.render_offscreen(
        &instances,
        &camera,
        view,
        &light,
        [0.0, 0.0, 0.0],
        (256, 256),
    );
    let disabled_map_size = renderer.shadow_map_size();

    let (soft_img, hard_img, disabled_img) = (soft_img?, hard_img?, disabled_img?);
    assert_eq!(soft_map_size, 512);
    let soft_lit = assert_shadow_contrast(&soft_img, "soft", &shadowed_x, &lit_x, &sample_y);
    assert_shadow_contrast(&hard_img, "hard", &shadowed_x, &lit_x, &sample_y);
    assert_eq!(disabled_map_size, 0);
    assert_unshadowed_matches_lit(&disabled_img, &shadowed_x, &lit_x, &sample_y, soft_lit);
    Ok(())
}

fn observe_shadow_map_sizes(
    renderer: &Renderer,
    instances: &[MeshInstance],
    camera: &Camera,
    view: Mat4,
    light: &DirectionalLight,
) -> Result<[u32; 4], Box<dyn std::error::Error>> {
    let mut observed_sizes = [0_u32; 4];
    for (slot, index) in observed_sizes.iter_mut().zip(0_u32..=3_u32) {
        let render_result = render_shadow_quality(
            renderer,
            instances,
            camera,
            view,
            light,
            enabled_shadow_settings(index, 50.0, true),
        );
        if render_result.is_err() {
            renderer.set_shadow_settings(ShadowSettings::disabled());
        }
        let _ = render_result?;
        *slot = renderer.shadow_map_size();
    }
    Ok(observed_sizes)
}

fn assert_short_distance_leaves_receiver_unshadowed(
    image: &Image,
    shadowed_x: &std::ops::Range<u32>,
    lit_x: &std::ops::Range<u32>,
    sample_y: &std::ops::Range<u32>,
) {
    let shadowed = region_mean_luma(image, shadowed_x.clone(), sample_y.clone());
    let lit = region_mean_luma(image, lit_x.clone(), sample_y.clone());
    assert!(
        (lit - shadowed).abs() <= 2.0,
        "short shadow distance {shadowed} must leave receiver unshadowed ({lit})"
    );
}

fn assert_full_distance_shadows_receiver(
    image: &Image,
    shadowed_x: &std::ops::Range<u32>,
    lit_x: &std::ops::Range<u32>,
    sample_y: &std::ops::Range<u32>,
) {
    let shadowed = region_mean_luma(image, shadowed_x.clone(), sample_y.clone());
    let lit = region_mean_luma(image, lit_x.clone(), sample_y.clone());
    assert!(
        lit - shadowed > 40.0,
        "full shadow distance lit {lit} vs shadowed {shadowed} delta must exceed 40.0"
    );
}

#[test]
fn shadow_map_texel_size_and_distance_follow_quality_level_settings(
) -> Result<(), Box<dyn std::error::Error>> {
    let renderer = shared_renderer()?;
    let (floor, caster) = (floor_quad(8.0), cube(1.0));
    let (camera, view, light) = (shadow_camera(), shadow_view(), shadow_light());
    let instances = shadow_instances(&floor, &caster);

    let observed_sizes = observe_shadow_map_sizes(&renderer, &instances, &camera, view, &light)?;
    let short_distance_img = render_shadow_quality(
        &renderer,
        &instances,
        &camera,
        view,
        &light,
        enabled_shadow_settings(2, 2.0, true),
    );
    let full_distance_img = render_shadow_quality(
        &renderer,
        &instances,
        &camera,
        view,
        &light,
        enabled_shadow_settings(2, 50.0, true),
    );
    renderer.set_shadow_settings(ShadowSettings::disabled());

    let (short_distance_img, full_distance_img) = (short_distance_img?, full_distance_img?);
    assert_eq!(observed_sizes[0], shadow_map_size(256, 0, 2048));
    assert_eq!(observed_sizes[1], shadow_map_size(256, 1, 2048));
    assert_eq!(observed_sizes[2], shadow_map_size(256, 2, 2048));
    assert_eq!(observed_sizes[3], shadow_map_size(256, 3, 4096));
    assert_eq!(observed_sizes, [128, 256, 512, 1024]);

    let (shadowed_x, lit_x, sample_y) = (165..185, 15..55, 110..146);
    assert_short_distance_leaves_receiver_unshadowed(
        &short_distance_img,
        &shadowed_x,
        &lit_x,
        &sample_y,
    );
    assert_full_distance_shadows_receiver(&full_distance_img, &shadowed_x, &lit_x, &sample_y);
    Ok(())
}
