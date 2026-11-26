use super::render_tests::shared_renderer;
use unitas_render::camera::{look_at, Camera};
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

#[test]
fn directional_shadow_caster_darkens_receiver_when_enabled_and_matches_when_disabled(
) -> Result<(), Box<dyn std::error::Error>> {
    let renderer = shared_renderer()?;
    let floor = floor_quad(8.0);
    let caster = cube(1.0);
    let camera = Camera::orthographic(6.0, 1.0, 0.1, 100.0);
    let view = look_at(
        Vec3::new(0.0, 10.0, -0.001),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
    );
    let light = DirectionalLight {
        direction: Vec3::new(-1.0, 1.0, 0.0).normalize(),
        color: [1.0, 1.0, 1.0],
        intensity: 0.75,
    };
    let instances = [
        MeshInstance {
            mesh: &floor,
            model: Mat4::identity(),
            color: [0.9, 0.9, 0.9],
        },
        MeshInstance {
            mesh: &caster,
            model: translated_scaled(2.0, Vec3::new(0.0, 2.5, 0.0)),
            color: [0.8, 0.3, 0.2],
        },
    ];
    let shadowed_x = 165..185;
    let lit_x = 15..55;
    let sample_y = 110..146;

    renderer.set_shadow_settings(ShadowSettings {
        enabled: true,
        soft: true,
        resolution_index: 2,
        distance: 50.0,
    });
    let soft_img = renderer.render_offscreen(
        &instances,
        &camera,
        view,
        &light,
        [0.0, 0.0, 0.0],
        (256, 256),
    );
    let soft_map_size = renderer.shadow_map_size();

    renderer.set_shadow_settings(ShadowSettings {
        enabled: true,
        soft: false,
        resolution_index: 2,
        distance: 50.0,
    });
    let hard_img = renderer.render_offscreen(
        &instances,
        &camera,
        view,
        &light,
        [0.0, 0.0, 0.0],
        (256, 256),
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

    let soft_img = soft_img?;
    let hard_img = hard_img?;
    let disabled_img = disabled_img?;

    assert_eq!(soft_map_size, 512);
    let soft_shadowed = region_mean_luma(&soft_img, shadowed_x.clone(), sample_y.clone());
    let soft_lit = region_mean_luma(&soft_img, lit_x.clone(), sample_y.clone());
    assert!(
        soft_shadowed > 30.0,
        "soft shadowed mean {soft_shadowed} should preserve ambient light"
    );
    assert!(
        soft_lit - soft_shadowed > 40.0,
        "soft lit {soft_lit} vs shadowed {soft_shadowed} delta must exceed 40.0"
    );

    let hard_shadowed = region_mean_luma(&hard_img, shadowed_x.clone(), sample_y.clone());
    let hard_lit = region_mean_luma(&hard_img, lit_x.clone(), sample_y.clone());
    assert!(
        hard_shadowed > 30.0,
        "hard shadowed mean {hard_shadowed} should preserve ambient light"
    );
    assert!(
        hard_lit - hard_shadowed > 40.0,
        "hard lit {hard_lit} vs shadowed {hard_shadowed} delta must exceed 40.0"
    );

    assert_eq!(disabled_map_size, 0);
    let disabled_shadowed = region_mean_luma(&disabled_img, shadowed_x, sample_y.clone());
    let disabled_lit = region_mean_luma(&disabled_img, lit_x, sample_y);
    assert!(
        (disabled_lit - disabled_shadowed).abs() <= 2.0,
        "disabled shadow region {disabled_shadowed} must match lit region {disabled_lit}"
    );
    assert!(
        (disabled_lit - soft_lit).abs() <= 2.0,
        "disabled lit {disabled_lit} must match unoccluded soft lit {soft_lit}"
    );

    Ok(())
}

#[test]
fn shadow_map_texel_size_and_distance_follow_quality_level_settings(
) -> Result<(), Box<dyn std::error::Error>> {
    let renderer = shared_renderer()?;
    let floor = floor_quad(8.0);
    let caster = cube(1.0);
    let camera = Camera::orthographic(6.0, 1.0, 0.1, 100.0);
    let view = look_at(
        Vec3::new(0.0, 10.0, -0.001),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
    );
    let light = DirectionalLight {
        direction: Vec3::new(-1.0, 1.0, 0.0).normalize(),
        color: [1.0, 1.0, 1.0],
        intensity: 0.75,
    };
    let instances = [
        MeshInstance {
            mesh: &floor,
            model: Mat4::identity(),
            color: [0.9, 0.9, 0.9],
        },
        MeshInstance {
            mesh: &caster,
            model: translated_scaled(2.0, Vec3::new(0.0, 2.5, 0.0)),
            color: [0.8, 0.3, 0.2],
        },
    ];

    let mut observed_sizes = [0_u32; 4];
    for (slot, index) in observed_sizes.iter_mut().zip(0_u32..=3_u32) {
        renderer.set_shadow_settings(ShadowSettings {
            enabled: true,
            soft: true,
            resolution_index: index,
            distance: 50.0,
        });
        let render_result = renderer.render_offscreen(
            &instances,
            &camera,
            view,
            &light,
            [0.0, 0.0, 0.0],
            (256, 256),
        );
        if render_result.is_err() {
            renderer.set_shadow_settings(ShadowSettings::disabled());
        }
        let _ = render_result?;
        *slot = renderer.shadow_map_size();
    }

    renderer.set_shadow_settings(ShadowSettings {
        enabled: true,
        soft: true,
        resolution_index: 2,
        distance: 2.0,
    });
    let short_distance_img = renderer.render_offscreen(
        &instances,
        &camera,
        view,
        &light,
        [0.0, 0.0, 0.0],
        (256, 256),
    );

    renderer.set_shadow_settings(ShadowSettings {
        enabled: true,
        soft: true,
        resolution_index: 2,
        distance: 50.0,
    });
    let full_distance_img = renderer.render_offscreen(
        &instances,
        &camera,
        view,
        &light,
        [0.0, 0.0, 0.0],
        (256, 256),
    );

    renderer.set_shadow_settings(ShadowSettings::disabled());

    let short_distance_img = short_distance_img?;
    let full_distance_img = full_distance_img?;

    assert_eq!(observed_sizes[0], shadow_map_size(256, 0, 2048));
    assert_eq!(observed_sizes[1], shadow_map_size(256, 1, 2048));
    assert_eq!(observed_sizes[2], shadow_map_size(256, 2, 2048));
    assert_eq!(observed_sizes[3], shadow_map_size(256, 3, 4096));
    assert_eq!(observed_sizes, [128, 256, 512, 1024]);

    let shadowed_x = 165..185;
    let lit_x = 15..55;
    let sample_y = 110..146;

    let short_shadowed =
        region_mean_luma(&short_distance_img, shadowed_x.clone(), sample_y.clone());
    let short_lit = region_mean_luma(&short_distance_img, lit_x.clone(), sample_y.clone());
    assert!(
        (short_lit - short_shadowed).abs() <= 2.0,
        "short shadow distance {short_shadowed} must leave receiver unshadowed ({short_lit})"
    );

    let full_shadowed = region_mean_luma(&full_distance_img, shadowed_x, sample_y.clone());
    let full_lit = region_mean_luma(&full_distance_img, lit_x, sample_y);
    assert!(
        full_lit - full_shadowed > 40.0,
        "full shadow distance lit {full_lit} vs shadowed {full_shadowed} delta must exceed 40.0"
    );

    Ok(())
}
