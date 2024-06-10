#![no_std]
use spirv_std::spirv;
use spirv_std::glam::{Vec3, Vec2, Vec4, Mat4};
use spirv_std::num_traits::Float;
use miau_shared::SceneConst;
use game_shared::FurConst;

#[spirv(vertex)]
pub fn main_v(
  pos: Vec3,
  uv: Vec2,
  normal: Vec3,
  #[spirv(instance_index)] n: u32,
  #[spirv(push_constant)] transform: &Mat4,
  #[spirv(uniform, descriptor_set = 0, binding = 0)] scene: &SceneConst,
  #[spirv(uniform, descriptor_set = 1, binding = 0)] consts: &FurConst,
  #[spirv(position)] out_pos: &mut Vec4,
  out_uv: &mut Vec2,
  out_normal: &mut Vec3,
  out_n: &mut u32,
) {
  let layer = n as f32 / consts.layers as f32;
  *out_pos = scene.cam * *transform * (pos + normal * layer * consts.height).extend(1.0);
  *out_uv = uv;
  *out_normal = normal;
  *out_n = n;
}

#[spirv(fragment)]
pub fn main_f(
  uv: Vec2,
  normal: Vec3,
  #[spirv(flat)] n: u32,
  #[spirv(uniform, descriptor_set = 1, binding = 0)] consts: &FurConst,
  out_color: &mut Vec4,
) {
  let layer = n as f32 / consts.layers as f32;
  let local_uv = uv * consts.density;
  let hash = hash(local_uv.trunc());
  let distance = (local_uv.fract() * 2.0 - 1.0).length();

  if n > 0 && distance > consts.thickness * (hash - layer) {
    spirv_std::arch::kill();
  }
  let color = (normal + 1.0) / 2.0;
  let ao = (0.1 + layer.powf(2.0)).min(1.0);
  *out_color = (color * ao).extend(1.0);
}

fn hash(x: Vec2) -> f32 {
  let x = (1.0 / 4320.0) * x + Vec2::new(0.25, 0.0);
  let state = (x * x).dot(Vec2::splat(3571.0)).fract();
  (state * state * 3571.0 * 2.0).fract()
}
