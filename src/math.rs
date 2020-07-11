#![allow(dead_code)]

use std::ops::{Add, Div, Mul, Neg};

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default)]
pub struct Vec3([f32; 3]);

impl From<[f32; 3]> for Vec3 {
    fn from(value: [f32; 3]) -> Self {
        Self(value)
    }
}

impl From<Vec3> for [f32; 3] {
    fn from(value: Vec3) -> Self {
        value.0
    }
}

impl Add for Vec3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        let [lx, ly, lz] = self.0;
        let [rx, ry, rz] = rhs.0;
        [lx + rx, ly + ry, lz + rz].into()
    }
}

impl Mul for Vec3 {
    type Output = Vec3;
    fn mul(self, rhs: Vec3) -> Vec3 {
        let [lx, ly, lz] = self.0;
        let [rx, ry, rz] = rhs.0;
        [lx * rx, ly * ry, lz * rz].into()
    }
}

impl Mul<f32> for Vec3 {
    type Output = Vec3;
    fn mul(self, rhs: f32) -> Vec3 {
        let [x, y, z] = self.0;
        [x * rhs, y * rhs, z * rhs].into()
    }
}

impl Div for Vec3 {
    type Output = Vec3;
    fn div(self, rhs: Vec3) -> Vec3 {
        let [lx, ly, lz] = self.0;
        let [rx, ry, rz] = rhs.0;
        [lx / rx, ly / ry, lz / rz].into()
    }
}

impl Div<f32> for Vec3 {
    type Output = Vec3;
    fn div(self, rhs: f32) -> Vec3 {
        let [x, y, z] = self.0;
        [x / rhs, y / rhs, z / rhs].into()
    }
}

impl Neg for Vec3 {
    type Output = Self;
    fn neg(self) -> Self {
        let [x, y, z] = self.0;
        [-x, -y, -z].into()
    }
}

impl Vec3 {
    pub const ZERO: Self = Self([0.0, 0.0, 0.0]);
    pub const ONE: Self = Self([1.0, 1.0, 1.0]);
    pub const X_POS: Self = Self([1.0, 0.0, 0.0]);
    pub const Y_POS: Self = Self([0.0, 1.0, 0.0]);
    pub const Z_POS: Self = Self([0.0, 0.0, 1.0]);
    pub const X_NEG: Self = Self([-1.0, 0.0, 0.0]);
    pub const Y_NEG: Self = Self([0.0, -1.0, 0.0]);
    pub const Z_NEG: Self = Self([0.0, 0.0, -1.0]);

    pub fn len2(&self) -> f32 {
        let [x, y, z] = self.0;
        x * x + y * y + z * z
    }

    pub fn len(&self) -> f32 {
        self.len2().sqrt()
    }

    pub fn normalized(self) -> Self {
        self / self.len()
    }

    pub fn dot(self, rhs: Self) -> f32 {
        let [lx, ly, lz] = self.0;
        let [rx, ry, rz] = rhs.0;
        lx * rx + ly * ry + lz * rz
    }

    pub fn cross(self, rhs: Self) -> Self {
        let [lx, ly, lz] = self.0;
        let [rx, ry, rz] = rhs.0;
        [ly * rz - lz * ry, lz * rx - lx * rz, lx * ry - ly * rx].into()
    }
}

#[repr(transparent)]
#[derive(Copy, Clone, Debug)]
pub struct Vec4([f32; 4]);

impl Default for Vec4 {
    fn default() -> Self {
        Vec4([0.0, 0.0, 0.0, 1.0])
    }
}

impl From<Vec3> for Vec4 {
    fn from(Vec3([x, y, z]): Vec3) -> Self {
        Self([x, y, z, 1.0])
    }
}

impl From<[f32; 3]> for Vec4 {
    fn from([x, y, z]: [f32; 3]) -> Self {
        Self([x, y, z, 1.0])
    }
}

impl From<[f32; 4]> for Vec4 {
    fn from(value: [f32; 4]) -> Self {
        Self(value)
    }
}

impl From<Vec4> for [f32; 4] {
    fn from(value: Vec4) -> Self {
        value.0
    }
}

impl Vec4 {
    pub const ZERO: Self = Self([0.0, 0.0, 0.0, 1.0]);
    pub const ONE: Self = Self([1.0, 1.0, 1.0, 1.0]);
    pub const X: Self = Self([1.0, 0.0, 0.0, 1.0]);
    pub const Y: Self = Self([0.0, 1.0, 0.0, 1.0]);
    pub const Z: Self = Self([0.0, 0.0, 1.0, 1.0]);

    pub fn dot(self, rhs: Self) -> f32 {
        let [lx, ly, lz, lw] = self.0;
        let [rx, ry, rz, rw] = rhs.0;
        lx * rx + ly * ry + lz * rz + lw * rw
    }
}

#[repr(transparent)]
#[derive(Copy, Clone, Debug)]
pub struct Quaternion([f32; 4]);

impl Quaternion {
    pub const ZERO: Self = Self([0.0, 0.0, 0.0, 1.0]);

    pub fn axis_angle(axis: Vec3, angle: f32) -> Self {
        let (s, c) = f32::sin_cos(angle / 2.0);
        (axis * s, c).into()
    }

    pub fn from_to(from: Vec3, to: Vec3) -> Self {
        // Partial solution, doesn't handle 180deg rotations.
        // Should negate something when the dot is negative (e.g. they are opposing)
        let im = Vec3::cross(from, to);
        let r = Vec3::dot(from, to) * f32::sqrt(from.len2() * to.len2());
        (im / r, 1.0).into()
    }

    pub fn rotate(self, p: Vec3) -> Vec3 {
        (self * Quaternion::from((p, 0.0)) * -self).im()
    }

    pub fn im(&self) -> Vec3 {
        let [x, y, z, _] = self.0;
        [x, y, z].into()
    }

    pub fn real(&self) -> f32 {
        self.0[3]
    }

    pub fn normalized(&self) -> Self {
        (self.im() / self.real(), 1.0).into()
    }
}

impl Default for Quaternion {
    fn default() -> Self {
        Self([0.0, 0.0, 0.0, 1.0])
    }
}

impl From<(Vec3, f32)> for Quaternion {
    fn from((im, real): (Vec3, f32)) -> Self {
        Self([im.0[0], im.0[1], im.0[2], real])
    }
}

impl From<Quaternion> for (Vec3, f32) {
    fn from(value: Quaternion) -> Self {
        let [x, y, z, w] = value.0;
        ([x, y, z].into(), w)
    }
}

impl From<[f32; 4]> for Quaternion {
    fn from(value: [f32; 4]) -> Self {
        Self(value)
    }
}

impl Neg for Quaternion {
    type Output = Self;

    fn neg(self) -> Self {
        (-self.im(), self.real()).into()
    }
}

impl Mul for Quaternion {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        // Order is b, c, d, a using the names from
        // https://en.wikipedia.org/wiki/Quaternion#Hamilton_product
        let [lx, ly, lz, lw] = self.0;
        let [rx, ry, rz, rw] = rhs.0;
        Self([
            lw * rx + lx * rw + ly * rz - lz * ry,
            lw * ry - lx * rz + ly * rw + lz * rx,
            lw * rz + lx * ry - ly * rx + lz * rw,
            lw * rw - lx * rx - ly * ry - lz * rz,
        ])
    }
}

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct Mat4([[f32; 4]; 4]);

impl Default for Mat4 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl From<[Vec4; 4]> for Mat4 {
    fn from([x, y, z, w]: [Vec4; 4]) -> Self {
        Self([x.0, y.0, z.0, w.0])
    }
}

impl Mat4 {
    pub const IDENTITY: Self = Self([
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]);

    pub fn translate(value: Vec3) -> Self {
        let [x, y, z] = value.0;
        Self([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [x, y, z, 1.0],
        ])
    }

    pub fn rotate(value: Quaternion) -> Self {
        // https://en.wikipedia.org/wiki/Quaternions_and_spatial_rotation#Quaternion-derived_rotation_matrix
        // Assuming unit quaternion.
        let [x, y, z, w] = value.0;
        Self([
            [
                1.0 - 2.0 * (y * y - z * z),
                2.0 * (x * y - z * w),
                2.0 * (x * z + y * w),
                0.0,
            ],
            [
                2.0 * (x * y + z * w),
                1.0 - 2.0 * (x * x + z * z),
                2.0 * (y * z - x * w),
                0.0,
            ],
            [
                2.0 * (x * z - y * w),
                2.0 * (y * z + x * w),
                1.0 - 2.0 * (x * x + y * y),
                0.0,
            ],
            [0.0, 0.0, 0.0, 1.0],
        ])
        //https://en.wikipedia.org/wiki/Quaternions_and_spatial_rotation#Conversion_to_and_from_the_matrix_representation
        // let [b, c, d, a] = (value.0).0;
        // Self([
        //     [
        //         a * a + b * b - c * c - d * d,
        //         2 * b * c - 2 * a * d,
        //         2 * b * d + 2 * a * c,
        //         0.0,
        //     ],
        //     [
        //         2 * b * c + 2 * a * d,
        //         a * a - b * b + c * c - d * d,
        //         2 * c * d - 2 * a * b,
        //         0.0,
        //     ],
        //     [
        //         2 * b * d - 2 * a * c,
        //         2 * c * d + 2 * a * b,
        //         a * a - b * b - c * c + d * d,
        //         0.0,
        //     ],
        //     [0.0, 0.0, 0.0, 1.0],
        // ])
    }

    pub fn scale(value: Vec3) -> Self {
        let [x, y, z] = value.0;
        Self([
            [x, 0.0, 0.0, 0.0],
            [0.0, y, 0.0, 0.0],
            [0.0, 0.0, z, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    pub fn row(&self, index: usize) -> Vec4 {
        self.0[index].into()
    }

    pub fn col(&self, index: usize) -> Vec4 {
        Vec4([
            self.0[0][index],
            self.0[1][index],
            self.0[2][index],
            self.0[3][index],
        ])
    }
}

impl Mul for Mat4 {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        // Out[x,y] = sum(i in 0..4 { lhs[x,i] * rhs[i,y] })
        //          = Vec4::dot(lhs.col(x), rhs.row(y))
        let lx = self.col(0);
        let ly = self.col(1);
        let lz = self.col(2);
        let lw = self.col(3);
        let rx = rhs.row(0);
        let ry = rhs.row(1);
        let rz = rhs.row(2);
        let rw = rhs.row(3);
        Self([
            [
                Vec4::dot(lx, rx),
                Vec4::dot(ly, rx),
                Vec4::dot(lz, rx),
                Vec4::dot(lw, rx),
            ],
            [
                Vec4::dot(lx, ry),
                Vec4::dot(ly, ry),
                Vec4::dot(lz, ry),
                Vec4::dot(lw, ry),
            ],
            [
                Vec4::dot(lx, rz),
                Vec4::dot(ly, rz),
                Vec4::dot(lz, rz),
                Vec4::dot(lw, rz),
            ],
            [
                Vec4::dot(lx, rw),
                Vec4::dot(ly, rw),
                Vec4::dot(lz, rw),
                Vec4::dot(lw, rw),
            ],
        ])
    }
}
