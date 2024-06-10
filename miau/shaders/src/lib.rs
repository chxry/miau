#![no_std]
use spirv_std::{spirv, Image, Sampler};
use spirv_std::glam::{Vec4, Vec3, Vec2, Mat4};
use miau_shared::SceneConst;

#[spirv(vertex)]
pub fn main_v(
  pos: Vec3,
  uv: Vec2,
  _: Vec3,
  #[spirv(push_constant)] transform: &Mat4,
  #[spirv(uniform, descriptor_set = 0, binding = 0)] scene: &SceneConst,
  #[spirv(position)] out_pos: &mut Vec4,
  out_uv: &mut Vec2,
) {
  *out_pos = scene.cam * *transform * pos.extend(1.0);
  *out_uv = uv;
}

#[spirv(fragment)]
pub fn main_f(
  uv: Vec2,
  #[spirv(descriptor_set = 1, binding = 0)] tex: &Image!(2D, type=f32, sampled),
  #[spirv(descriptor_set = 1, binding = 1)] sampler: &Sampler,
  out_color: &mut Vec4,
) {
  *out_color = tex.sample(*sampler, uv);
}

#[spirv(vertex)]
pub fn ui_v(
  pos: Vec2,
  uv: Vec2,
  color: Vec4,
  #[spirv(uniform, descriptor_set = 0, binding = 0)] scene: &SceneConst,
  #[spirv(position)] out_pos: &mut Vec4,
  out_uv: &mut Vec2,
  out_color: &mut Vec4,
) {
  *out_pos = Vec4::new(
    2.0 * pos.x / scene.size.x - 1.0,
    1.0 - 2.0 * pos.y / scene.size.y,
    0.0,
    1.0,
  );
  *out_uv = uv;
  *out_color = color;
}

#[spirv(fragment)]
pub fn ui_f(
  uv: Vec2,
  color: Vec4,
  #[spirv(descriptor_set = 1, binding = 0)] tex: &Image!(2D, type=f32, sampled),
  #[spirv(descriptor_set = 1, binding = 1)] sampler: &Sampler,
  out_color: &mut Vec4,
) {
  *out_color = tex.sample(*sampler, uv) * color.powf(2.2);
}
