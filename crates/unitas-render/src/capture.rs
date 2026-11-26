use crate::camera::Camera;
use crate::gpu::{RenderError, Renderer};
use crate::image::Rgba8;
use crate::math::Mat4;
use crate::mesh::Mesh;
use crate::model::DirectionalLight;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScreenBounds {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureObject {
    pub id: u32,
    pub name: String,
    pub bounds: ScreenBounds,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureFrame {
    pub image: Rgba8,
    pub id_map: Vec<u32>,
    pub objects: Vec<CaptureObject>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaptureInstance<'a> {
    pub name: &'a str,
    pub mesh: &'a Mesh,
    pub model: Mat4,
    pub color: [f32; 3],
    pub lit: bool,
}

pub fn render_offscreen_with_ids(
    renderer: &Renderer,
    instances: &[CaptureInstance<'_>],
    camera: &Camera,
    view: Mat4,
    light: &DirectionalLight,
    background: [f32; 3],
    size: (u32, u32),
) -> Result<CaptureFrame, RenderError> {
    renderer.render_offscreen_with_ids(instances, camera, view, light, background, size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::look_at;
    use crate::math::Vec3;

    const WIDTH: u32 = 128;
    const HEIGHT: u32 = 128;
    const BACKGROUND: [f32; 3] = [0.05, 0.05, 0.08];

    fn camera() -> Camera {
        Camera::perspective(60.0, 1.0, 0.1, 100.0)
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

    fn identity() -> Mat4 {
        Mat4::identity()
    }

    fn renderer_or_skip() -> Option<Renderer> {
        match Renderer::new() {
            Ok(renderer) => Some(renderer),
            Err(error) => {
                eprintln!("skipping GPU capture test, no GPU device available: {error}");
                None
            }
        }
    }

    fn quad(x_min: f32, x_max: f32, y_min: f32, y_max: f32) -> Mesh {
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

    fn id_at(frame: &CaptureFrame, x: u32, y: u32) -> u32 {
        frame.id_map[(y as usize) * (frame.image.width as usize) + (x as usize)]
    }

    #[test]
    fn capture_reports_object_ids_bounds_and_background() -> Result<(), Box<dyn std::error::Error>>
    {
        let renderer = match renderer_or_skip() {
            Some(renderer) => renderer,
            None => return Ok(()),
        };
        let left = quad(-0.8, -0.4, -0.4, 0.4);
        let right = quad(0.4, 0.8, -0.4, 0.4);
        let instances = vec![
            CaptureInstance {
                name: "left",
                mesh: &left,
                model: identity(),
                color: [0.2, 0.4, 0.8],
                lit: true,
            },
            CaptureInstance {
                name: "right",
                mesh: &right,
                model: identity(),
                color: [0.6, 0.2, 0.8],
                lit: true,
            },
        ];
        let frame = renderer.render_offscreen_with_ids(
            &instances,
            &camera(),
            view(),
            &light(),
            BACKGROUND,
            (WIDTH, HEIGHT),
        )?;
        assert_eq!(frame.id_map.len(), (WIDTH * HEIGHT) as usize);
        assert_eq!(frame.image.as_bytes().len(), (WIDTH * HEIGHT * 4) as usize);
        assert_eq!(id_at(&frame, 50, 64), 1);
        assert_eq!(id_at(&frame, 77, 64), 2);
        assert_eq!(id_at(&frame, 10, 64), 0);
        assert_eq!(frame.objects.len(), 2);
        let left_object = &frame.objects[0];
        assert_eq!(left_object.id, 1);
        assert_eq!(left_object.name, "left");
        assert!(left_object.bounds.width > 0 && left_object.bounds.height > 0);
        assert!(left_object.bounds.x <= 50);
        assert!(left_object.bounds.x + left_object.bounds.width > 50);
        assert!(left_object.bounds.y <= 64);
        assert!(left_object.bounds.y + left_object.bounds.height > 64);
        let right_object = &frame.objects[1];
        assert_eq!(right_object.id, 2);
        assert_eq!(right_object.name, "right");
        assert!(right_object.bounds.width > 0 && right_object.bounds.height > 0);
        assert!(right_object.bounds.x <= 77);
        assert!(right_object.bounds.x + right_object.bounds.width > 77);
        assert!(right_object.bounds.y <= 64);
        assert!(right_object.bounds.y + right_object.bounds.height > 64);
        Ok(())
    }

    #[test]
    fn capture_object_table_matches_the_id_map() -> Result<(), Box<dyn std::error::Error>> {
        let renderer = match renderer_or_skip() {
            Some(renderer) => renderer,
            None => return Ok(()),
        };
        let left = quad(-0.8, -0.4, -0.4, 0.4);
        let right = quad(0.4, 0.8, -0.4, 0.4);
        let instances = vec![
            CaptureInstance {
                name: "left",
                mesh: &left,
                model: identity(),
                color: [0.2, 0.4, 0.8],
                lit: true,
            },
            CaptureInstance {
                name: "right",
                mesh: &right,
                model: identity(),
                color: [0.6, 0.2, 0.8],
                lit: true,
            },
        ];
        let frame = renderer.render_offscreen_with_ids(
            &instances,
            &camera(),
            view(),
            &light(),
            BACKGROUND,
            (WIDTH, HEIGHT),
        )?;
        let mut present: Vec<u32> = frame.id_map.iter().copied().filter(|id| *id != 0).collect();
        present.sort_unstable();
        present.dedup();
        let mut listed: Vec<u32> = frame.objects.iter().map(|object| object.id).collect();
        listed.sort_unstable();
        assert_eq!(present, listed);
        for object in &frame.objects {
            let covered = frame.id_map.iter().filter(|id| **id == object.id).count();
            assert!(covered > 0, "object {} has no covered pixels", object.id);
        }
        Ok(())
    }

    #[test]
    fn capture_colour_matches_the_plain_offscreen_render() -> Result<(), Box<dyn std::error::Error>>
    {
        let renderer = match renderer_or_skip() {
            Some(renderer) => renderer,
            None => return Ok(()),
        };
        let left = quad(-0.8, -0.4, -0.4, 0.4);
        let right = quad(0.4, 0.8, -0.4, 0.4);
        let capture_instances = vec![
            CaptureInstance {
                name: "left",
                mesh: &left,
                model: identity(),
                color: [0.2, 0.4, 0.8],
                lit: true,
            },
            CaptureInstance {
                name: "right",
                mesh: &right,
                model: identity(),
                color: [0.6, 0.2, 0.8],
                lit: true,
            },
        ];
        let plain_instances = vec![
            crate::model::MeshInstance {
                mesh: &left,
                model: identity(),
                color: [0.2, 0.4, 0.8],
            },
            crate::model::MeshInstance {
                mesh: &right,
                model: identity(),
                color: [0.6, 0.2, 0.8],
            },
        ];
        let frame = renderer.render_offscreen_with_ids(
            &capture_instances,
            &camera(),
            view(),
            &light(),
            BACKGROUND,
            (WIDTH, HEIGHT),
        )?;
        let plain = renderer.render_offscreen(
            &plain_instances,
            &camera(),
            view(),
            &light(),
            BACKGROUND,
            (WIDTH, HEIGHT),
        )?;
        for index in 0..(WIDTH * HEIGHT) as usize {
            let captured = frame
                .image
                .get_pixel(index as u32 % WIDTH, index as u32 / WIDTH);
            let expected = plain.pixels[index];
            assert_eq!(
                &captured[0..3],
                &expected[..],
                "pixel {index} differs between the capture and the plain offscreen render"
            );
            assert_eq!(captured[3], 255);
        }
        Ok(())
    }
}
