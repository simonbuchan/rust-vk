use crate::math::*;

pub struct OrthographicProjection {
    pub width: f32,
    pub height: f32,
    pub depth: f32,
}

pub struct PerspectiveProjection {
    pub aspect: f32,
    pub fov_height: f32,
    pub near: f32,
    pub far: f32,
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
        let t = (self.fov_height / 2.0).tan();
        let r = t / self.aspect;
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

pub struct Transform {
    pub position: Vec3,
    pub rotation: Quaternion,
}

impl Transform {
    pub fn matrix(&self) -> Mat4 {
        Mat4::rotate(self.rotation) * Mat4::translate(-self.position)
    }
}

pub struct Camera<P: Projection> {
    pub transform: Transform,
    pub projection: P,
}

impl<P: Projection> Camera<P> {
    pub fn matrix(&self) -> Mat4 {
        self.projection.matrix() * self.transform.matrix()
    }
}
