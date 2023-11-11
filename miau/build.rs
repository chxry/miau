use std::fs::File;
use spirv_builder::SpirvBuilder;
use vach::builder::{Builder, BuilderConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let out = SpirvBuilder::new("shaders", "spirv-unknown-spv1.5").build()?;
  let path = out.module.unwrap_single();
  let mut builder = Builder::new();
  builder.add_dir("../assets", None)?;
  builder.add(File::open(path)?, "shaders.spv")?;
  builder.dump(File::create("../assets.vach")?, &BuilderConfig::default())?;
  Ok(())
}
