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
        image: serde_json::json!({ "test": 1 }),
    };
    let json = serde_json::to_string(&doc).unwrap();
    assert!(json.contains("schema_version"));
}
