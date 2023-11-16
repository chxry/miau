#![no_std]

#[repr(C)]
pub struct FurConst {
  pub layers: u32,
  pub density: f32,
  pub height: f32,
}
