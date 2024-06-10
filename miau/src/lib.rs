#![feature(
  downcast_unchecked,
  const_collections_with_hasher,
  const_type_id,
  trait_alias
)]
#![allow(clippy::new_without_default)]
extern crate self as miau;
pub mod gfx;
pub mod ecs;
pub mod assets;
pub mod scene;
pub mod ui;

use std::mem::MaybeUninit;
use std::any::Any;
use winit::window::{WindowBuilder, Window};
use winit::event_loop::EventLoop;
use winit::event::{Event, WindowEvent};
use crate::gfx::Renderer;
use crate::ecs::{World, System, stage};
use crate::assets::Assets;

#[doc(hidden)]
pub use erased_serde;
pub use glam as math;

pub type Result<T = (), E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

pub struct Engine(World);

impl Engine {
  pub fn new() -> Self {
    Self(World::new()).add_system(stage::INIT, init)
  }

  pub fn add_system<S: System + 'static>(self, stage: u64, s: S) -> Self {
    self.0.add_system(stage, s);
    self
  }

  pub fn add_resource<T: Any>(self, resource: T) -> Self {
    self.0.add_resource(resource);
    self
  }

  pub fn run(self) -> Result {
    unsafe { WORLD.write(self.0) };
    world().run_system(stage::INIT);
    world().run_system(stage::START);
    Ok(())
  }
}

fn init(world: &World) -> Result {
  let event_loop = EventLoop::new()?;
  let window = WindowBuilder::new().build(&event_loop)?;
  world.add_resource(event_loop);
  world.add_resource(window);
  Assets::init(world)?;
  pollster::block_on(Renderer::init(world))?;
  world.add_system(stage::START, start);
  world.add_system(stage::UPDATE, update);
  Ok(())
}

fn start(world: &World) -> Result {
  world
    .take_resource::<EventLoop<()>>()
    .unwrap()
    .run(move |event, elwt| match event {
      Event::WindowEvent { event, .. } => {
        match event {
          WindowEvent::RedrawRequested => world.run_system(stage::UPDATE),
          WindowEvent::Resized(size) => world.get_resource_mut::<Renderer>().unwrap().resize(size),
          WindowEvent::CloseRequested => elwt.exit(),
          _ => {}
        }
        world.add_resource(event);
        world.run_system(stage::EVENT);
      }
      Event::AboutToWait => world.get_resource::<Window>().unwrap().request_redraw(),
      _ => {}
    })?;
  Ok(())
}

fn update(world: &World) -> Result {
  world.get_resource_mut::<Renderer>().unwrap().frame(world);
  Ok(())
}

static mut WORLD: MaybeUninit<World> = MaybeUninit::uninit();

#[inline(always)]
fn world() -> &'static mut World {
  unsafe { WORLD.assume_init_mut() }
}
