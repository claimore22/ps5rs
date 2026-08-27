use ps5_shader::{ShaderBinary, shader_metadata::ShaderMetadata};
use ps5_shader::shader_binary::ShaderStage;

#[test]
fn shader_binary_creates() {
    let bin = ShaderBinary::parse(b"vertex shader").unwrap();
    assert_eq!(bin.stage, ShaderStage::Vertex);
}

#[test]
fn shader_metadata_creates() {
    let bin = ShaderBinary::parse(b"pixel data").unwrap();
    let meta = ShaderMetadata::from_binary(&bin, "main");
    assert_eq!(meta.stage, ShaderStage::Pixel);
    assert_eq!(meta.entry_point, "main");
}
