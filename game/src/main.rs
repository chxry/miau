mod fur;

use std::fs::File;
use log::LevelFilter;
use miau::{Engine, Result};
use miau::ecs::{World, Scene, stage};
use miau::scene::{Transform, Model};
use miau::assets::Assets;
use miau::math::{Vec3, Quat};
use miau::ui::imgui::Ui;
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
    .add_system(stage::DRAW, ui)
    .run()
}

fn start(world: &World) -> Result {
  world.add_resource(FurPass::new(world)?);
  let assets = world.get_resource::<Assets>().unwrap();
  world
    .spawn()
    .insert(Transform::new())
    .insert(FurModel::new(world, assets.load("garfield.obj")?));

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
    })
    .insert(Spin);

  Scene::from_world(world).save(File::create("assets/test.scene")?)?;
  // assets.load::<Scene>("test.scene")?.into_world(world);
  Ok(())
}

struct Spin;

fn spin(world: &World) -> Result {
  for (e, _) in world.get::<Spin>() {
    e.get_one_mut::<Transform>().unwrap().rotation *= Quat::from_rotation_y(0.02);
  }
  Ok(())
}

fn ui(world: &World) -> Result {
  let ui = world.get_resource::<Ui>().unwrap();
  let model = &mut world.get_mut::<FurModel>()[0].1;
  let consts = &mut model.consts.data_mut();
  ui.show_demo_window(&mut true);
  ui.window("fur").always_auto_resize(true).build(|| {
    ui.slider("layers", 1, 500, &mut consts.layers);
    ui.slider("density", 50.0, 5000.0, &mut consts.density);
    ui.slider("height", 0.0, 2.5, &mut consts.height);
    ui.slider("thickness", 0.0, 5.0, &mut consts.thickness);
  });
  Ok(())
}
