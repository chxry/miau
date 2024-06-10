#![no_std]
use glam::{Vec3, Vec2, Mat4};

#[repr(C)]
pub struct Vertex {
  pub pos: Vec3,
  pub uv: Vec2,
  pub normal: Vec3,
}

#[repr(C)]
pub struct SceneConst {
  pub cam: Mat4,
  pub size: Vec2,
}
