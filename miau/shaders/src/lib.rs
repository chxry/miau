#![no_std]
use spirv_std::{spirv, Image, Sampler};
use spirv_std::glam::{Vec4, Vec3, Vec2};
use shared::{SceneConst, ObjConst};

#[spirv(vertex)]
pub fn main_v(
  pos: Vec3,
  uv: Vec2,
  #[spirv(push_constant)] (scene, obj): &(SceneConst, ObjConst),
  #[spirv(position)] out_pos: &mut Vec4,
  out_uv: &mut Vec2,
) {
  *out_pos = scene.cam * obj.transform * pos.extend(1.0);
  *out_uv = uv;
}

#[spirv(fragment)]
pub fn main_f(
  uv: Vec2,
  #[spirv(descriptor_set = 0, binding = 0)] tex: &Image!(2D, type=f32, sampled),
  #[spirv(descriptor_set = 0, binding = 1)] sampler: &Sampler,
  out_color: &mut Vec4,
) {
  *out_color = tex.sample(*sampler, uv);
}
