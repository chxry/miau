mod fur;

use log::LevelFilter;
use miau::{Engine, Result};
use miau::ecs::{World, stage};
use miau::scene::{Transform, Model};
use miau::assets::Assets;
use miau::math::{Vec3, Quat};
use crate::fur::{FurPass, FurModel};

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
  world.add_resource(FurPass::new(world)?);
  let assets = world.get_resource::<Assets>().unwrap();
  world
    .spawn()
    .insert(Transform::new())
    .insert(FurModel::new(world, assets.load("garfield.obj")?))
    .insert(Spin);

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
  // world.load()?;
  Ok(())
}

struct Spin;

fn spin(world: &World) -> Result {
  for (e, _) in world.get::<Spin>() {
    e.get_one_mut::<Transform>().unwrap().rotation *= Quat::from_rotation_y(0.02);
  }
  Ok(())
}
