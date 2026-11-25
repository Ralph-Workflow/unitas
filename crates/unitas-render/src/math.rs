use std::ops::{Add, Index, IndexMut, Mul, Neg, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn scale(self, factor: f32) -> Self {
        Self {
            x: self.x * factor,
            y: self.y * factor,
            z: self.z * factor,
        }
    }

    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(self, other: Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    pub fn normalize(self) -> Self {
        let len = self.length();
        if len > 1e-8 {
            self.scale(1.0 / len)
        } else {
            Self::ZERO
        }
    }
}

impl Add for Vec3 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl Sub for Vec3 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        self.scale(rhs)
    }
}

impl Mul<Vec3> for f32 {
    type Output = Vec3;

    fn mul(self, rhs: Vec3) -> Self::Output {
        rhs.scale(self)
    }
}

impl Neg for Vec3 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vec4 {
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    pub const fn from_point(p: Vec3) -> Self {
        Self {
            x: p.x,
            y: p.y,
            z: p.z,
            w: 1.0,
        }
    }

    pub const fn from_direction(d: Vec3) -> Self {
        Self {
            x: d.x,
            y: d.y,
            z: d.z,
            w: 0.0,
        }
    }

    pub const fn truncate(self) -> Vec3 {
        Vec3 {
            x: self.x,
            y: self.y,
            z: self.z,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat4 {
    pub data: [f32; 16],
}

impl Default for Mat4 {
    fn default() -> Self {
        Self::identity()
    }
}

impl Mat4 {
    pub const fn identity() -> Self {
        Self {
            data: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    pub const fn zero() -> Self {
        Self { data: [0.0; 16] }
    }

    pub const fn from_cols_array(data: [f32; 16]) -> Self {
        Self { data }
    }

    pub const fn as_array(&self) -> &[f32; 16] {
        &self.data
    }

    pub const fn get(&self, col: usize, row: usize) -> f32 {
        self.data[col * 4 + row]
    }

    pub fn set(&mut self, col: usize, row: usize, val: f32) {
        self.data[col * 4 + row] = val;
    }

    pub fn mul_mat4(&self, rhs: &Self) -> Self {
        let mut out = [0.0f32; 16];
        for c in 0..4 {
            for r in 0..4 {
                let mut sum = 0.0f32;
                for k in 0..4 {
                    sum += self.data[k * 4 + r] * rhs.data[c * 4 + k];
                }
                out[c * 4 + r] = sum;
            }
        }
        Self { data: out }
    }

    pub fn mul_vec4(&self, v: Vec4) -> Vec4 {
        let m = &self.data;
        Vec4 {
            x: m[0] * v.x + m[4] * v.y + m[8] * v.z + m[12] * v.w,
            y: m[1] * v.x + m[5] * v.y + m[9] * v.z + m[13] * v.w,
            z: m[2] * v.x + m[6] * v.y + m[10] * v.z + m[14] * v.w,
            w: m[3] * v.x + m[7] * v.y + m[11] * v.z + m[15] * v.w,
        }
    }

    /// Inverse for an affine matrix (last row 0,0,0,1); `None` when the
    /// linear part is singular.
    pub fn inverse(&self) -> Option<Self> {
        let m = &self.data;
        // 3x3 linear part, column-major: a(c, r) = m[c * 4 + r].
        let a = [m[0], m[4], m[8], m[1], m[5], m[9], m[2], m[6], m[10]];
        let c0 = a[4] * a[8] - a[5] * a[7];
        let c1 = a[2] * a[7] - a[1] * a[8];
        let c2 = a[1] * a[5] - a[2] * a[4];
        let det = a[0] * c0 + a[3] * c1 + a[6] * c2;
        if det.abs() < 1e-12 {
            return None;
        }
        let inv_det = 1.0 / det;
        // Adjugate (transposed cofactor matrix), row-major pairs (row, col).
        let cof = [
            c0,
            c1,
            c2,
            a[5] * a[6] - a[3] * a[8],
            a[0] * a[8] - a[2] * a[6],
            a[2] * a[3] - a[0] * a[5],
            a[3] * a[7] - a[4] * a[6],
            a[1] * a[6] - a[0] * a[7],
            a[0] * a[4] - a[1] * a[3],
        ];
        // The cofactor table is column-major: cof[c * 3 + r] holds C(r, c).
        // The inverse needs the adjugate, A^-1 = C^T / det, so read the
        // cofactor table row-major here.
        let b = |row: usize, col: usize| cof[row * 3 + col] * inv_det;
        let tx = m[12];
        let ty = m[13];
        let tz = m[14];
        let itx = -(b(0, 0) * tx + b(0, 1) * ty + b(0, 2) * tz);
        let ity = -(b(1, 0) * tx + b(1, 1) * ty + b(1, 2) * tz);
        let itz = -(b(2, 0) * tx + b(2, 1) * ty + b(2, 2) * tz);
        Some(Self {
            data: [
                b(0, 0),
                b(1, 0),
                b(2, 0),
                0.0,
                b(0, 1),
                b(1, 1),
                b(2, 1),
                0.0,
                b(0, 2),
                b(1, 2),
                b(2, 2),
                0.0,
                itx,
                ity,
                itz,
                1.0,
            ],
        })
    }

    pub fn transform_point(&self, p: Vec3) -> Vec4 {
        self.mul_vec4(Vec4::from_point(p))
    }

    pub fn project_point(&self, p: Vec3) -> Vec3 {
        let clip = self.transform_point(p);
        if clip.w.abs() > 1e-8 {
            Vec3::new(clip.x / clip.w, clip.y / clip.w, clip.z / clip.w)
        } else {
            clip.truncate()
        }
    }
}

impl Index<usize> for Mat4 {
    type Output = f32;

    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}

impl IndexMut<usize> for Mat4 {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.data[index]
    }
}

impl Mul<Mat4> for Mat4 {
    type Output = Mat4;

    fn mul(self, rhs: Mat4) -> Self::Output {
        self.mul_mat4(&rhs)
    }
}

impl Mul<Vec4> for Mat4 {
    type Output = Vec4;

    fn mul(self, rhs: Vec4) -> Self::Output {
        self.mul_vec4(rhs)
    }
}

impl Mul<Vec3> for Mat4 {
    type Output = Vec3;

    fn mul(self, rhs: Vec3) -> Self::Output {
        self.project_point(rhs)
    }
}
