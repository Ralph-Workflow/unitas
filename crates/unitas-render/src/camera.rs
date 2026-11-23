use crate::math::{Mat4, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub fov_y_degrees: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
    pub orthographic_size: Option<f32>,
}

impl Camera {
    pub const fn perspective(fov_y_degrees: f32, aspect: f32, near: f32, far: f32) -> Self {
        Self {
            fov_y_degrees,
            aspect,
            near,
            far,
            orthographic_size: None,
        }
    }

    pub const fn orthographic(size: f32, aspect: f32, near: f32, far: f32) -> Self {
        Self {
            fov_y_degrees: 60.0,
            aspect,
            near,
            far,
            orthographic_size: Some(size),
        }
    }

    pub fn projection_matrix(&self) -> Mat4 {
        let mut m = [0.0f32; 16];
        let nf = self.near - self.far;
        if let Some(size) = self.orthographic_size {
            m[0] = 1.0 / (size * self.aspect);
            m[5] = 1.0 / size;
            m[10] = 1.0 / nf;
            m[14] = self.near / nf;
            m[15] = 1.0;
        } else {
            let half_fov_tan = (self.fov_y_degrees.to_radians() * 0.5).tan();
            m[0] = 1.0 / (self.aspect * half_fov_tan);
            m[5] = 1.0 / half_fov_tan;
            m[10] = self.far / nf;
            m[11] = -1.0;
            m[14] = (self.far * self.near) / nf;
        }
        Mat4::from_cols_array(m)
    }
}

pub fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Mat4 {
    let forward = (target - eye).normalize();
    let right = up.cross(forward).normalize();
    let true_up = forward.cross(right);
    let back = -forward;

    let mut m = [0.0f32; 16];
    m[0] = right.x;
    m[4] = right.y;
    m[8] = right.z;
    m[12] = -right.dot(eye);

    m[1] = true_up.x;
    m[5] = true_up.y;
    m[9] = true_up.z;
    m[13] = -true_up.dot(eye);

    m[2] = back.x;
    m[6] = back.y;
    m[10] = back.z;
    m[14] = -back.dot(eye);

    m[15] = 1.0;
    Mat4::from_cols_array(m)
}
