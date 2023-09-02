#![feature(downcast_unchecked, trait_alias)]
mod gfx;
mod ecs;
mod assets;

use std::io::BufReader;
use std::fs::File;
use winit::window::WindowBuilder;
use winit::event_loop::EventLoop;
use winit::event::{Event, WindowEvent};
use glam::{Vec3, Vec2, Quat, EulerRot, Mat4};
use log::LevelFilter;
use obj::{Obj, TexturedVertex};
use crate::gfx::{Pipeline, Mesh, Vertex};
use crate::ecs::World;
use crate::assets::Assets;

pub type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn load_mesh() -> Result<Mesh> {
  let obj: Obj<TexturedVertex, u32> = obj::load_obj(BufReader::new(File::open("garfield.obj")?))?;
  Ok(Mesh::new(
    &obj
      .vertices
      .iter()
      .map(|v| Vertex {
        pos: v.position.into(),
        uv: Vec2::new(v.texture[0], 1.0 - v.texture[1]),
      })
      .collect::<Vec<_>>(),
    &obj.indices,
  ))
}

fn main() -> Result {
  env_logger::builder()
    .filter_level(LevelFilter::Info)
    .filter(Some("wgpu_core"), LevelFilter::Warn)
    .init();
  let event_loop = EventLoop::new();
  let window = WindowBuilder::new().build(&event_loop)?;
  gfx::init(&window);
  let mut pipeline = Pipeline::new(window.inner_size())?;
  let mut assets = Assets::new();
  assets.register_loader::<Mesh>(load_mesh);
  let world = World::new();
  world
    .spawn()
    .insert(Transform::new())
    .insert(assets.load::<Mesh>("ass")?);

  event_loop.run(move |event, _, control_flow| match event {
    Event::WindowEvent { event, .. } => match event {
      WindowEvent::Resized(size) => {
        gfx::renderer().resize(size);
        pipeline.resize(size);
      }
      WindowEvent::CloseRequested => control_flow.set_exit(),
      _ => {}
    },
    Event::RedrawRequested(..) => pipeline.frame(&world),
    Event::MainEventsCleared => window.request_redraw(),
    _ => {}
  });
}

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
