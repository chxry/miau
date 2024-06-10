use std::fs::{File, self};
use spirv_builder::SpirvBuilder;
use vach::builder::{Builder, BuilderConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let out = SpirvBuilder::new("shaders", "spirv-unknown-spv1.5").build()?;
  fs::copy(out.module.unwrap_single(), "../assets/game_shaders.spv")?;
  let mut builder = Builder::new();
  builder.add_dir("../assets", None)?;
  builder.dump(File::create("../assets.vach")?, &BuilderConfig::default())?;
  Ok(())
}
