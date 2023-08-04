use spirv_builder::{SpirvBuilder, MetadataPrintout};

fn main() -> Result<(), Box<dyn std::error::Error>> {
  SpirvBuilder::new("shaders", "spirv-unknown-spv1.5")
    .print_metadata(MetadataPrintout::Full)
    .build()?;
  Ok(())
}
