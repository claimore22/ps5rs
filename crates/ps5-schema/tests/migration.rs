use ps5_schema::{SCHEMA_VERSION, migrations};

#[test]
fn schema_version_is_1() {
    assert_eq!(SCHEMA_VERSION, 1);
}

#[test]
fn migration_v1_to_v2_is_noop_for_unknown() {
    let data = r#"{"schema_version":1,"tool":"test"}"#;
    let out = migrations::migrate_v1_to_v2(data);
    assert!(out.contains("schema_version"));
}
