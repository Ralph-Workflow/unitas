use unitas_render::camera::Camera;
use unitas_render::gpu::{GpuInstance, Renderer};
use unitas_render::math::{Mat4, Vec3};
use unitas_render::mesh::cube;
use unitas_render::model::DirectionalLight;
use unitas_render::{Image, PostSettings};

use crate::render_tests::shared_renderer;

const SIZE: (u32, u32) = (64, 64);
const BACKGROUND: [f32; 3] = [0.05, 0.05, 0.08];

fn view() -> Mat4 {
    let mut values = [0.0f32; 16];
    values[0] = 1.0;
    values[5] = 1.0;
    values[10] = 1.0;
    values[14] = -3.0;
    values[15] = 1.0;
    Mat4::from_cols_array(values)
}

fn light() -> DirectionalLight {
    DirectionalLight {
        direction: Vec3::new(0.4, 0.8, 0.5),
        color: [1.0, 1.0, 1.0],
        intensity: 0.85,
    }
}

fn white_cube(renderer: &Renderer) -> Option<Image> {
    let mesh = cube(1.0);
    let camera = Camera::perspective(60.0, 1.0, 0.1, 100.0);
    let instances = vec![GpuInstance {
        mesh: &mesh,
        model: Mat4::from_cols_array([
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]),
        color: [1.0, 1.0, 1.0],
        lit: false,
    }];
    renderer
        .render_offscreen_with(&instances, &camera, view(), &light(), BACKGROUND, SIZE)
        .ok()
}

#[test]
fn the_post_chain_blooms_the_bright_pixels_and_tonemaps_the_frame() {
    let Ok(renderer) = shared_renderer() else {
        return;
    };
    let Some(without) = white_cube(&renderer) else {
        return;
    };
    renderer.set_post_settings(PostSettings {
        enabled: true,
        ..PostSettings::default()
    });
    let Some(with) = white_cube(&renderer) else {
        return;
    };
    renderer.set_post_settings(PostSettings::default());

    let mut brighter = 0usize;
    let mut darker = 0usize;
    let mut corner_brighter = 0usize;
    for y in 0..SIZE.1 {
        for x in 0..SIZE.0 {
            let plain = without.get_pixel(x, y);
            let posted = with.get_pixel(x, y);
            let lighter = posted[0] > plain[0] || posted[1] > plain[1] || posted[2] > plain[2];
            let heavier = posted[0] < plain[0] || posted[1] < plain[1] || posted[2] < plain[2];
            if lighter {
                brighter += 1;
            }
            if heavier {
                darker += 1;
            }
            if lighter && x < 8 && y < 8 {
                corner_brighter += 1;
            }
        }
    }
    assert!(
        brighter > 0,
        "the bloom added no light around the bright cube"
    );
    assert!(darker > 0, "the tone map shifted no pixel of the frame");
    assert_eq!(
        corner_brighter, 0,
        "the bloom reached the dark corner of the frame, which carries no bright pixels"
    );
}
