use crate::math::*;

#[derive(Copy, Clone)]
pub struct OrthographicProjection {
    pub width: f32,
    pub height: f32,
    pub depth: f32,
}

impl Default for OrthographicProjection {
    fn default() -> Self {
        Self {
            width: 1.0,
            height: 1.0,
            depth: 1.0,
        }
    }
}

#[derive(Copy, Clone)]
pub struct PerspectiveProjection {
    pub aspect: f32,
    pub fov_deg_height: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for PerspectiveProjection {
    fn default() -> Self {
        Self {
            aspect: 1.0,
            fov_deg_height: 60.0,
            near: 0.01,
            far: 1000.0,
        }
    }
}

pub trait Projection {
    fn matrix(&self) -> Mat4;
}

impl Projection for OrthographicProjection {
    fn matrix(&self) -> Mat4 {
        Mat4::scale(Vec3::from([
            1.0 / self.width,
            1.0 / self.height,
            1.0 / self.depth,
        ]))
    }
}

impl Projection for PerspectiveProjection {
    fn matrix(&self) -> Mat4 {
        let r = (self.fov_deg_height * std::f32::consts::PI / 360.0).tan();
        let t = r * self.aspect;
        let x = 1.0 / t;
        let y = 1.0 / r;
        let z = self.far / (self.near - self.far);
        let w = self.far * self.near / (self.near - self.far);
        Mat4::from([
            Vec4::from([x, 0.0, 0.0, 0.0]),
            Vec4::from([0.0, -y, 0.0, 0.0]),
            Vec4::from([0.0, 0.0, z, -1.0]),
            Vec4::from([0.0, 0.0, w, 0.0]),
        ])
    }
}

#[derive(Copy, Clone, Default)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quaternion,
}

impl Transform {
    pub fn matrix(&self) -> Mat4 {
        Mat4::rotate(self.rotation) * Mat4::translate(-self.position)
    }
}

#[derive(Copy, Clone, Default)]
pub struct Camera<P: Projection> {
    pub transform: Transform,
    pub projection: P,
}

impl<P: Projection> Camera<P> {
    pub fn matrix(&self) -> Mat4 {
        self.projection.matrix() * self.transform.matrix()
    }
}
