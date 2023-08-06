use spirv_builder::SpirvBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  SpirvBuilder::new("shaders", "spirv-unknown-spv1.5").build()?;
  Ok(())
}
