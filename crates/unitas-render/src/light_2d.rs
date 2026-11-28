use crate::model::{Light2D, Light2DKind, MAX_LIGHTS_2D};

const LIGHT_FLOATS: usize = 16;
const HEADER_FLOATS: usize = 4;
pub(crate) const LIGHTS_2D_BYTES: u64 = ((HEADER_FLOATS + MAX_LIGHTS_2D * LIGHT_FLOATS) * 4) as u64;

pub(crate) fn create_lights_2d_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("lights-2d"),
        size: LIGHTS_2D_BYTES,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

pub(crate) fn pack_lights_2d(lights: &[Light2D]) -> Vec<f32> {
    let mut data = vec![0.0f32; HEADER_FLOATS + MAX_LIGHTS_2D * LIGHT_FLOATS];
    let count = lights.len().min(MAX_LIGHTS_2D);
    data[0] = count as f32;
    for (index, light) in lights.iter().take(count).enumerate() {
        let base = HEADER_FLOATS + index * LIGHT_FLOATS;
        let outer_angle = light.outer_angle_deg.clamp(0.0, 180.0);
        data[base] = light.color[0];
        data[base + 1] = light.color[1];
        data[base + 2] = light.color[2];
        data[base + 3] = light.intensity.max(0.0);
        data[base + 4] = light.position[0];
        data[base + 5] = light.position[1];
        data[base + 6] = light.inner_radius.max(0.0);
        data[base + 7] = light.outer_radius.max(0.0);
        let direction = normalized_direction(light.direction);
        data[base + 8] = direction[0];
        data[base + 9] = direction[1];
        data[base + 10] = light.inner_angle_deg.clamp(0.0, outer_angle);
        data[base + 11] = outer_angle;
        data[base + 12] = match light.kind {
            Light2DKind::Global => 0.0,
            Light2DKind::Point => 1.0,
            Light2DKind::Spot => 2.0,
        };
    }
    data
}

fn normalized_direction(direction: [f32; 2]) -> [f32; 2] {
    let length = (direction[0] * direction[0] + direction[1] * direction[1]).sqrt();
    if length > 1e-8 {
        [direction[0] / length, direction[1] / length]
    } else {
        [0.0, 1.0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use crate::camera::{look_at, Camera};
    use crate::gpu::Renderer;
    use crate::image::Image;
    use crate::math::{Mat4, Vec3};
    use crate::model::DirectionalLight;
    use crate::sprite::Sprite;

    const WIDTH: u32 = 64;
    const HEIGHT: u32 = 64;
    const TINT: [f32; 4] = [0.4, 0.6, 0.8, 1.0];
    const UNLIT_PIXEL: [u8; 3] = [102, 153, 204];

    fn renderer_or_skip() -> Option<Renderer> {
        match Renderer::new() {
            Ok(renderer) => Some(renderer),
            Err(error) => {
                eprintln!("skipping GPU 2D light test, no GPU device available: {error}");
                None
            }
        }
    }

    fn camera() -> Camera {
        Camera::orthographic(2.0, 1.0, 0.1, 100.0)
    }

    fn view() -> Mat4 {
        look_at(
            Vec3::new(0.0, 0.0, -5.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        )
    }

    fn light() -> DirectionalLight {
        DirectionalLight {
            direction: Vec3::new(0.0, 0.0, 1.0),
            color: [1.0, 1.0, 1.0],
            intensity: 0.0,
        }
    }

    fn white_texture() -> Arc<Image> {
        let mut image = Image::new(1, 1);
        image.pixels[0] = [255, 255, 255];
        Arc::new(image)
    }

    fn sprite_at(x: f32, y: f32) -> Sprite {
        Sprite {
            texture: white_texture(),
            model: Mat4::from_cols_array([
                0.4, 0.0, 0.0, 0.0, 0.0, 0.4, 0.0, 0.0, 0.0, 0.0, 0.4, 0.0, x, y, 0.0, 1.0,
            ]),
            color: TINT,
            sorting_layer: 0,
            sorting_order: 0,
        }
    }

    fn render(renderer: &Renderer, sprites: Vec<Sprite>) -> Result<Image, crate::gpu::RenderError> {
        renderer.set_sprites(sprites);
        renderer.render_offscreen(
            &[],
            &camera(),
            view(),
            &light(),
            [0.0, 0.0, 0.0],
            (WIDTH, HEIGHT),
        )
    }

    fn brightness(pixel: [u8; 3]) -> u32 {
        u32::from(pixel[0]) + u32::from(pixel[1]) + u32::from(pixel[2])
    }

    #[test]
    fn point_light_brightens_the_sprite_next_to_it() -> Result<(), Box<dyn std::error::Error>> {
        let Some(renderer) = renderer_or_skip() else {
            return Ok(());
        };
        renderer.set_lights_2d(&[Light2D {
            kind: Light2DKind::Point,
            color: [1.0, 1.0, 1.0],
            intensity: 0.5,
            position: [0.0, 0.0],
            direction: [0.0, 1.0],
            inner_radius: 0.1,
            outer_radius: 0.4,
            inner_angle_deg: 0.0,
            outer_angle_deg: 0.0,
        }]);
        let image = render(&renderer, vec![sprite_at(0.0, 0.0), sprite_at(0.8, 0.0)])?;
        let near = image.get_pixel(32, 32);
        let far = image.get_pixel(44, 32);
        assert!(
            brightness(near) > brightness(far),
            "expected the sprite next to the point light to be brighter, got {near:?} and {far:?}"
        );
        assert_eq!(
            far, UNLIT_PIXEL,
            "expected the sprite outside the point light radius to stay unlit"
        );
        Ok(())
    }

    #[test]
    fn global_light_tints_the_sprite() -> Result<(), Box<dyn std::error::Error>> {
        let Some(renderer) = renderer_or_skip() else {
            return Ok(());
        };
        renderer.set_lights_2d(&[Light2D {
            kind: Light2DKind::Global,
            color: [1.0, 0.0, 0.0],
            intensity: 0.5,
            position: [0.0, 0.0],
            direction: [0.0, 1.0],
            inner_radius: 0.0,
            outer_radius: 0.0,
            inner_angle_deg: 0.0,
            outer_angle_deg: 0.0,
        }]);
        let image = render(&renderer, vec![sprite_at(0.0, 0.0)])?;
        let tinted = image.get_pixel(32, 32);
        assert_eq!(
            tinted,
            [153, 153, 204],
            "expected the global light to raise the red channel only"
        );
        Ok(())
    }

    #[test]
    fn spot_light_lights_the_sprite_inside_its_cone() -> Result<(), Box<dyn std::error::Error>> {
        let Some(renderer) = renderer_or_skip() else {
            return Ok(());
        };
        renderer.set_lights_2d(&[Light2D {
            kind: Light2DKind::Spot,
            color: [1.0, 1.0, 1.0],
            intensity: 0.5,
            position: [0.0, 0.0],
            direction: [1.0, 0.0],
            inner_radius: 0.0,
            outer_radius: 0.6,
            inner_angle_deg: 10.0,
            outer_angle_deg: 30.0,
        }]);
        let image = render(&renderer, vec![sprite_at(0.4, 0.0), sprite_at(0.0, 0.4)])?;
        let inside = image.get_pixel(38, 32);
        let outside = image.get_pixel(32, 25);
        assert!(
            brightness(inside) > brightness(outside),
            "expected the sprite inside the spot cone to be brighter, got {inside:?} and {outside:?}"
        );
        assert_eq!(
            outside, UNLIT_PIXEL,
            "expected the sprite outside the spot cone to stay unlit"
        );
        Ok(())
    }

    #[test]
    fn no_2d_lights_leave_the_sprite_pass_byte_identical() -> Result<(), Box<dyn std::error::Error>>
    {
        let Some(renderer) = renderer_or_skip() else {
            return Ok(());
        };
        renderer.set_lights_2d(&[]);
        let image = render(&renderer, vec![sprite_at(0.0, 0.0)])?;
        assert_eq!(
            image.get_pixel(32, 32),
            UNLIT_PIXEL,
            "expected the unlit sprite to keep its albedo times tint value"
        );
        for pixel in &image.pixels {
            assert!(
                *pixel == [0, 0, 0] || *pixel == UNLIT_PIXEL,
                "expected only background and unlit sprite pixels, got {pixel:?}"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod packing_tests {
    use super::*;

    #[test]
    fn packing_caps_the_light_count_and_normalizes_the_direction() {
        let lights = vec![
            Light2D {
                kind: Light2DKind::Spot,
                color: [0.1, 0.2, 0.3],
                intensity: 2.0,
                position: [1.0, -2.0],
                direction: [3.0, 4.0],
                inner_radius: 1.0,
                outer_radius: 4.0,
                inner_angle_deg: 90.0,
                outer_angle_deg: 30.0,
            };
            MAX_LIGHTS_2D + 4
        ];
        let packed = pack_lights_2d(&lights);
        assert_eq!(packed.len(), HEADER_FLOATS + MAX_LIGHTS_2D * LIGHT_FLOATS);
        assert_eq!(packed[0], MAX_LIGHTS_2D as f32);
        let base = HEADER_FLOATS;
        assert_eq!(packed[base + 3], 2.0);
        assert_eq!(packed[base + 4], 1.0);
        assert_eq!(packed[base + 5], -2.0);
        assert_eq!(packed[base + 8], 0.6);
        assert_eq!(packed[base + 9], 0.8);
        assert_eq!(packed[base + 10], 30.0);
        assert_eq!(packed[base + 11], 30.0);
        assert_eq!(packed[base + 12], 2.0);
    }

    #[test]
    fn an_empty_light_list_packs_a_zero_count() {
        let packed = pack_lights_2d(&[]);
        assert_eq!(packed[0], 0.0);
        assert!(packed[1..].iter().all(|value| *value == 0.0));
    }
}
