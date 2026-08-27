use ps5_shader::shader_binary::ShaderStage;
use ps5_shader::{ShaderBinary, agc::AgcShader};

#[test]
fn agc_shader_from_roms() {
    let roms = r"C:\Users\claimoar\Documents\ROMS\PS5";
    let path = std::path::Path::new(roms);
    if !path.exists() {
        return;
    }
    let mut found = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten().take(1) {
            let p = entry.path();
            if p.is_dir() {
                let data = b"vertex shader binary";
                let shader = ShaderBinary::parse(data).unwrap();
                let agc = AgcShader {
                    data: vec![0u8; 16],
                };
                assert_eq!(shader.stage, ShaderStage::Vertex);
                assert_eq!(agc.data.len(), 16);
                found += 1;
            }
        }
    }
    assert!(found >= 0);
}
