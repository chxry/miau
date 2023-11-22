use glam::{Vec3, Quat, EulerRot, Mat4};
use serde::{Serialize, Deserialize};
use crate::ecs::component;

pub use crate::gfx::standard::Model;

#[component]
#[derive(Serialize, Deserialize)]
pub struct Transform {
  pub position: Vec3,
  pub rotation: Quat,
  pub scale: Vec3,
}

impl Transform {
  pub fn new() -> Self {
    Self {
      position: Vec3::ZERO,
      rotation: Quat::IDENTITY,
      scale: Vec3::ONE,
    }
  }

  pub fn pos(mut self, position: Vec3) -> Self {
    self.position = position;
    self
  }

  pub fn rot(mut self, rotation: Quat) -> Self {
    self.rotation = rotation;
    self
  }

  pub fn rot_euler(mut self, y: f32, p: f32, r: f32) -> Self {
    self.rotation = Quat::from_euler(
      EulerRot::YXZ,
      y.to_radians(),
      p.to_radians(),
      r.to_radians(),
    );
    self
  }

  pub fn scale(mut self, scale: Vec3) -> Self {
    self.scale = scale;
    self
  }

  pub fn as_mat4(&self) -> Mat4 {
    Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
  }
}
