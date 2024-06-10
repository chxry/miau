use std::fs;
use spirv_builder::SpirvBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let out = SpirvBuilder::new("shaders", "spirv-unknown-spv1.5").build()?;
  fs::copy(out.module.unwrap_single(), "../assets/miau_shaders.spv")?;
  Ok(())
}
