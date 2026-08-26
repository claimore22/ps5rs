use ps5_schema::{BinaryImageDocument, SCHEMA_VERSION};

#[test]
fn schema_version_is_1() {
    assert_eq!(SCHEMA_VERSION, 1);
}

#[test]
fn binary_image_document_serializes() {
    let doc = BinaryImageDocument {
        schema_version: SCHEMA_VERSION,
        tool: "test".to_string(),
        image: ps5_image::BinaryImage {
            sha256: "a".repeat(64),
            platform: ps5_image::Platform::Ps5,
            is_self: true,
            file_size: 100,
            entry_point: 0,
            metadata: ps5_image::BinaryMetadata::default(),
            segments: vec![],
            imports: vec![],
            exports: vec![],
            relocations: vec![],
            tls: None,
            init_va: 0,
            init_array_va: 0,
            init_array_sz: 0,
            fini_va: 0,
            fini_array_va: 0,
            fini_array_sz: 0,
            preinit_array_va: 0,
            preinit_array_sz: 0,
            import_libs: Default::default(),
            needed_files: vec![],
            dynamic_entries: vec![],
            version_defs: vec![],
            lib_versions: vec![],
        },
    };
    let json = serde_json::to_string(&doc).unwrap();
    assert!(json.contains("schema_version"));
}
