use unitas_render::camera::look_at;
use unitas_render::capture::{CaptureFrame, CaptureInstance, CaptureObject, ScreenBounds};
use unitas_render::math::{Mat4, Vec3};
use unitas_render::mesh::Mesh;
use unitas_render::model::DirectionalLight;
use unitas_render::render_graph::{RenderGraph, RenderPassKind};
use unitas_render::ui_pass::{
    composite_ui, composite_ui_onto_capture, SpriteRef, UiElement, UiTarget, UiText,
};
use unitas_render::Rgba8;

use super::render_tests::shared_renderer;

const WIDTH: u32 = 64;
const HEIGHT: u32 = 64;
const BACKGROUND: [f32; 3] = [0.05, 0.05, 0.08];

struct TestItem {
    rect: [f32; 4],
    color: [f32; 4],
    sprite: Option<SpriteRef>,
    text: Option<UiText<'static>>,
}

impl UiElement for TestItem {
    fn rect(&self) -> [f32; 4] {
        self.rect
    }

    fn color(&self) -> [f32; 4] {
        self.color
    }

    fn sprite(&self) -> Option<&SpriteRef> {
        self.sprite.as_ref()
    }

    fn text(&self) -> Option<UiText<'_>> {
        self.text
    }
}

fn quad(rect: [f32; 4], color: [f32; 4]) -> TestItem {
    TestItem {
        rect,
        color,
        sprite: None,
        text: None,
    }
}

fn camera() -> unitas_render::Camera {
    unitas_render::Camera::perspective(60.0, 1.0, 0.1, 100.0)
}

fn light() -> DirectionalLight {
    DirectionalLight {
        direction: Vec3::new(0.0, 0.0, -1.0),
        color: [1.0, 1.0, 1.0],
        intensity: 0.8,
    }
}

fn view() -> Mat4 {
    look_at(
        Vec3::new(0.0, 0.0, -5.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    )
}

fn quad_mesh(x_min: f32, x_max: f32, y_min: f32, y_max: f32) -> Mesh {
    let positions = [
        [x_min, y_min, 0.0],
        [x_min, y_max, 0.0],
        [x_max, y_max, 0.0],
        [x_max, y_min, 0.0],
    ];
    let normals = [[0.0, 0.0, -1.0]; 4];
    let indices = [0, 1, 2, 0, 2, 3];
    Mesh::from_buffers(&positions, &normals, &indices)
}

#[test]
fn ui_pass_blends_a_quad_and_writes_its_object_id() {
    let mut image = Rgba8::new(WIDTH, HEIGHT);
    let mut ids = vec![0u32; (WIDTH * HEIGHT) as usize];
    let items = vec![quad([8.0, 8.0, 16.0, 16.0], [0.0, 1.0, 0.0, 0.5])];
    {
        let mut target = UiTarget {
            image: &mut image,
            ids: Some(&mut ids),
        };
        let coverages = composite_ui(&mut target, &items, 7, None);
        assert_eq!(coverages.len(), 1);
        assert_eq!(coverages[0].id, 7);
        assert_eq!(coverages[0].x, 8);
        assert_eq!(coverages[0].y, 8);
        assert_eq!(coverages[0].width, 16);
        assert_eq!(coverages[0].height, 16);
    }
    let offset = (12 * WIDTH as usize + 12) * 4;
    assert_eq!(&image.pixels[offset..offset + 3], &[0, 128, 0]);
    assert_eq!(ids[12 * WIDTH as usize + 12], 7);
    assert_eq!(ids[0], 0);
}

#[test]
fn ui_pass_skips_items_outside_the_target() {
    let mut image = Rgba8::new(WIDTH, HEIGHT);
    let mut ids = vec![0u32; (WIDTH * HEIGHT) as usize];
    let items = vec![quad([100.0, 100.0, 10.0, 10.0], [1.0, 1.0, 1.0, 1.0])];
    let mut target = UiTarget {
        image: &mut image,
        ids: Some(&mut ids),
    };
    let coverages = composite_ui(&mut target, &items, 1, None);
    assert!(coverages.is_empty());
    assert!(ids.iter().all(|id| *id == 0));
}

#[test]
fn ui_capture_frame_composite_extends_the_object_table() {
    let mut frame = CaptureFrame {
        image: Rgba8::new(WIDTH, HEIGHT),
        id_map: vec![0u32; (WIDTH * HEIGHT) as usize],
        objects: vec![CaptureObject {
            id: 1,
            name: "world".to_string(),
            bounds: ScreenBounds {
                x: 0,
                y: 0,
                width: 4,
                height: 4,
            },
        }],
    };
    let items = vec![quad([4.0, 4.0, 20.0, 12.0], [1.0, 0.0, 0.0, 1.0])];
    let objects = composite_ui_onto_capture(&mut frame, &items, None);
    assert_eq!(objects.len(), 1);
    assert_eq!(objects[0].id, 2);
    assert_eq!(objects[0].name, "ui-0");
    assert_eq!(objects[0].bounds.width, 20);
    assert_eq!(objects[0].bounds.height, 12);
    let pixel = frame.image.get_pixel(10, 8);
    assert_eq!(&pixel[0..3], &[255, 0, 0]);
    assert_eq!(frame.id_map[8 * WIDTH as usize + 10], 2);
    assert_eq!(frame.objects.len(), 2);
    assert_eq!(frame.objects[1].id, 2);
}

#[test]
fn ui_capture_frame_reports_ui_pixels_and_object_ids() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = shared_renderer()?;
    let mesh = quad_mesh(-0.8, -0.4, -0.4, 0.4);
    let instances = vec![CaptureInstance {
        name: "world",
        mesh: &mesh,
        model: Mat4::identity(),
        color: [0.2, 0.4, 0.8],
        lit: true,
    }];
    let mut frame = renderer.render_offscreen_with_ids(
        &instances,
        &camera(),
        view(),
        &light(),
        BACKGROUND,
        (WIDTH, HEIGHT),
    )?;
    assert_eq!(
        RenderGraph::capture(false).pass_order(),
        vec![
            RenderPassKind::Opaque,
            RenderPassKind::Transparent,
            RenderPassKind::CaptureIds
        ]
    );
    assert_eq!(
        RenderGraph::offscreen(true, false).pass_order(),
        vec![
            RenderPassKind::Opaque,
            RenderPassKind::Transparent,
            RenderPassKind::UiOverlay
        ]
    );
    let items = vec![quad([4.0, 4.0, 20.0, 12.0], [1.0, 0.0, 0.0, 1.0])];
    let objects = composite_ui_onto_capture(&mut frame, &items, None);
    assert_eq!(objects.len(), 1);
    assert_eq!(objects[0].id, 2);
    assert_eq!(objects[0].bounds.x, 4);
    assert_eq!(objects[0].bounds.y, 4);
    assert_eq!(objects[0].bounds.width, 20);
    assert_eq!(objects[0].bounds.height, 12);
    let pixel = frame.image.get_pixel(10, 8);
    assert_eq!(&pixel[0..3], &[255, 0, 0]);
    assert_eq!(frame.id_map[8 * WIDTH as usize + 10], 2);
    assert!(frame.objects.iter().any(|object| object.id == 2));
    let mut listed: Vec<u32> = frame.objects.iter().map(|object| object.id).collect();
    listed.sort_unstable();
    let mut present: Vec<u32> = frame.id_map.iter().copied().filter(|id| *id != 0).collect();
    present.sort_unstable();
    present.dedup();
    assert_eq!(present, listed);
    Ok(())
}
