#![no_std]
use glam::{Vec3, Mat4};

#[repr(C)]
pub struct Vertex {
  pub pos: Vec3,
  pub color: Vec3,
}

#[repr(C)]
pub struct SceneConst {
  pub cam: Mat4,
}

#[repr(C)]
pub struct ObjConst {
  pub transform: Mat4,
}
