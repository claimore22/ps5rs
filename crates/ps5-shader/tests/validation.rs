use ps5_shader::{ShaderBinary, shader_metadata::ShaderMetadata};

#[test]
fn shader_binary_creates() {
    let bin = ShaderBinary {
        stage: "vertex".to_string(),
    };
    assert_eq!(bin.stage, "vertex");
}

#[test]
fn shader_metadata_creates() {
    let meta = ShaderMetadata {
        stage: "pixel".to_string(),
        entry_point: "main".to_string(),
    };
    assert_eq!(meta.stage, "pixel");
}
