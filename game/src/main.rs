use log::LevelFilter;
use miau::{Engine, Result};
use miau::ecs::{World, stage};
use miau::scene::{Transform, Model};
use miau::assets::Assets;
use miau::math::{Vec3, Quat};

fn main() -> Result {
  env_logger::builder()
    .filter_level(LevelFilter::Info)
    .filter(Some("wgpu_core"), LevelFilter::Warn)
    .filter(Some("wgpu_hal"), LevelFilter::Warn)
    .init();
  Engine::new()
    .add_system(stage::START, start)
    .add_system(stage::UPDATE, spin)
    .run()
}

fn start(world: &World) -> Result {
  let assets = world.get_resource::<Assets>().unwrap();
  world.spawn().insert(Transform::new()).insert(Model {
    mesh: assets.load("assets/garfield.obj")?,
    tex: assets.load("assets/garfield.png")?,
    instances: 1,
  });

  world
    .spawn()
    .insert(
      Transform::new()
        .pos(Vec3::new(-4.0, 0.0, 2.0))
        .scale(Vec3::splat(0.5)),
    )
    .insert(Model {
      mesh: assets.load("assets/garfield.obj")?,
      tex: assets.load("assets/garfield.png")?,
      instances: 1,
    });
  world.save()?;

  world.load()?;
  Ok(())
}

fn spin(world: &World) -> Result {
  world.get_mut::<Transform>()[0].1.rotation *= Quat::from_rotation_y(0.02);
  Ok(())
}
