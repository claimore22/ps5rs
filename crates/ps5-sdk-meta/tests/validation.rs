use ps5_sdk_meta::{SdkDatabase, SdkFunction, VersionRange};

#[test]
fn sdk_database_inserts() {
    let mut db = SdkDatabase::new();
    let func = SdkFunction {
        nid: "test".to_string(),
        name: "testFunc".to_string(),
        library: "libTest".to_string(),
        module: None,
        sdk_versions: VersionRange {
            from: "1.0".to_string(),
            to: None,
        },
        category: "test".to_string(),
    };
    db.insert(func);
    assert_eq!(db.len(), 1);
    assert!(db.get("test").is_some());
}
