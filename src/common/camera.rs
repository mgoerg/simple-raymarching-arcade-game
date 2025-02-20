use cgmath::InnerSpace;



#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);


pub struct Camera {
    pub eye: cgmath::Point3<f32>,
    pub target: cgmath::Point3<f32>,
    pub up: cgmath::Vector3<f32>,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    pub fn new(eye: cgmath::Point3<f32>, target: cgmath::Point3<f32>, up: cgmath::Vector3<f32>, aspect: f32, fovy: f32, znear: f32, zfar: f32) -> Self {
        let u_dir = (target - eye).cross(up).normalize();
        let v_dir = u_dir.cross(target - eye).normalize();
        Self {
            eye,
            target,
            up,
            aspect,
            fovy,
            znear,
            zfar,
        }
    }
    
    pub fn new_dir(eye: cgmath::Point3<f32>, direction: cgmath::Vector3<f32>, up: cgmath::Vector3<f32>, aspect: f32, fovy: f32, znear: f32, zfar: f32) -> Self {
        let target = eye + direction;
        Self {
            eye,
            target,
            up,
            aspect,
            fovy,
            znear,
            zfar,
        }
    }

    pub fn direction(&self) -> cgmath::Vector3<f32> {
        (self.target - self.eye).normalize()
    }

    pub fn u_dir(&self) -> cgmath::Vector3<f32> {
        (self.target - self.eye).cross(self.up).normalize()
    }

    pub fn v_dir(&self) -> cgmath::Vector3<f32> {
        self.u_dir().cross(self.target - self.eye).normalize()
    }

    pub fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
        return OPENGL_TO_WGPU_MATRIX * proj * view;
    }
}
