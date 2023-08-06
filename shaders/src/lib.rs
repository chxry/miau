#![no_std]
use spirv_std::spirv;
use spirv_std::glam::{Vec4, Vec3, Mat4};

// use this in vs
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

#[spirv(vertex)]
pub fn main_v(
  pos: Vec3,
  color: Vec3,
  #[spirv(push_constant)] (scene, obj): &(SceneConst, ObjConst),
  #[spirv(position)] out_pos: &mut Vec4,
  out_color: &mut Vec3,
) {
  *out_pos = scene.cam * obj.transform * pos.extend(1.0);
  *out_color = color;
}

#[spirv(fragment)]
pub fn main_f(color: Vec3, out_color: &mut Vec4) {
  *out_color = color.extend(1.0);
}
