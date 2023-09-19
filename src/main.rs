#![feature(downcast_unchecked, const_collections_with_hasher, const_type_id)]
mod gfx;
mod ecs;
mod assets;
mod scene;

use winit::window::WindowBuilder;
use winit::event_loop::EventLoop;
use winit::event::{Event, WindowEvent};
use log::LevelFilter;
use glam::Vec3;
use crate::ecs::World;
use crate::scene::{Transform, Model};

pub type Result<T = (), E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

fn main() -> Result {
  env_logger::builder()
    .filter_level(LevelFilter::Info)
    .filter(Some("wgpu_core"), LevelFilter::Warn)
    .filter(Some("wgpu_hal"), LevelFilter::Warn)
    .init();
  let event_loop = EventLoop::new()?;
  let window = WindowBuilder::new().build(&event_loop)?;
  let assets = assets::init();
  let renderer = gfx::init(&window);
  let world = World::new();

  world.spawn().insert(Transform::new()).insert(Model {
    mesh: assets.load("garfield.obj")?,
    tex: assets.load("garfield.png")?,
  });
  world
    .spawn()
    .insert(
      Transform::new()
        .pos(Vec3::new(-4.0, 0.0, 2.0))
        .scale(Vec3::splat(0.5)),
    )
    .insert(Model {
      mesh: assets.load("garfield.obj")?,
      tex: assets.load("garfield.png")?,
    });
  world.save()?;

  world.load()?;

  event_loop.run(move |event, elwt| match event {
    Event::WindowEvent { event, .. } => match event {
      WindowEvent::Resized(size) => renderer.resize(size),
      WindowEvent::CloseRequested => elwt.exit(),
      WindowEvent::RedrawRequested => renderer.frame(&world),
      _ => {}
    },
    Event::AboutToWait => window.request_redraw(),
    _ => {}
  })?;
  Ok(())
}
