use std::fs;
use spirv_builder::{SpirvBuilder, MetadataPrintout};

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let out = SpirvBuilder::new("shaders", "spirv-unknown-spv1.5")
    .print_metadata(MetadataPrintout::None)
    .build()?;
  fs::copy(out.module.unwrap_single(), "../assets/miau_shaders.spv")?;
  Ok(())
}
