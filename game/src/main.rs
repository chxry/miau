use log::LevelFilter;
use miau::{Engine, Result};
use miau::ecs::{World, stage};
use miau::scene::{Transform, Model};
use miau::assets::Assets;
use miau::math::Vec3;

fn main() -> Result {
  env_logger::builder()
    .filter_level(LevelFilter::Info)
    .filter(Some("wgpu_core"), LevelFilter::Warn)
    .filter(Some("wgpu_hal"), LevelFilter::Warn)
    .init();
  Engine::new().add_system(stage::START, start).run()
}

fn start(world: &World) -> Result {
  let assets = world.get_resource::<Assets>().unwrap();
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
  Ok(())
}
