#![no_std]
use spirv_std::spirv;
use spirv_std::glam::Vec4;

#[spirv(vertex)]
pub fn main_v(#[spirv(vertex_index)] idx: usize, #[spirv(position)] pos: &mut Vec4) {
  *pos = [
    Vec4::new(0.0, -1.0, 0.0, 1.0),
    Vec4::new(1.0, 1.0, 0.0, 1.0),
    Vec4::new(-1.0, 1.0, 0.0, 1.0),
  ][idx];
}

#[spirv(fragment)]
pub fn main_f(color: &mut Vec4) {
  *color = Vec4::new(1.0, 0.0, 0.0, 1.0);
}
