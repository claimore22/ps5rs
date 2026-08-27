use ps5_shader::shader_binary::ShaderStage;
use ps5_shader::{ShaderBinary, shader_metadata::ShaderMetadata};

#[test]
fn shader_binary_creates() {
    let mut data = vec![0u8; 50];
    data[32..36].copy_from_slice(b"Shdr");
    data[44] = 1;
    let bin = ShaderBinary::parse(&data).unwrap();
    assert_eq!(bin.stage, ShaderStage::Vertex);
}

#[test]
fn shader_metadata_creates() {
    let mut data = vec![0u8; 50];
    data[32..36].copy_from_slice(b"Shdr");
    data[44] = 2;
    let bin = ShaderBinary::parse(&data).unwrap();
    let meta = ShaderMetadata::from_binary(&bin, "main");
    assert_eq!(meta.stage, ShaderStage::Pixel);
    assert_eq!(meta.entry_point, "main");
}
