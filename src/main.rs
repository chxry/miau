#![feature(downcast_unchecked, const_collections_with_hasher, cell_leak)]
mod gfx;
mod ecs;
mod assets;
mod scene;

use winit::window::WindowBuilder;
use winit::event_loop::EventLoop;
use winit::event::{Event, WindowEvent};
use log::LevelFilter;
use crate::ecs::World;
use crate::scene::{Transform, Model};

pub type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result {
  env_logger::builder()
    .filter_level(LevelFilter::Info)
    .filter(Some("wgpu_core"), LevelFilter::Warn)
    .init();
  let event_loop = EventLoop::new();
  let window = WindowBuilder::new().build(&event_loop)?;
  let assets = assets::init();
  let renderer = gfx::init(&window);
  let world = World::new();
  world.spawn().insert(Transform::new()).insert(Model {
    mesh: assets.load("garfield.obj")?,
    tex: assets.load("garfield.png")?,
  });

  world.save();

  // std::fs::write(
  //   "test",
  //   bincode::serialize(&assets.load::<gfx::Mesh>("garfield.obj")?)?,
  // )?;

  // let h: assets::Handle<gfx::Mesh> = bincode::deserialize(&std::fs::read("test")?)?;

  event_loop.run(move |event, _, control_flow| match event {
    Event::WindowEvent { event, .. } => match event {
      WindowEvent::Resized(size) => renderer.resize(size),
      WindowEvent::CloseRequested => control_flow.set_exit(),
      _ => {}
    },
    Event::RedrawRequested(..) => renderer.frame(&world),
    Event::MainEventsCleared => window.request_redraw(),
    _ => {}
  });
}
