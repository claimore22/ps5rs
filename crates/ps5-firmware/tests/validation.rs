use ps5_firmware::{FirmwareCatalog, FirmwareVersion};

#[test]
fn firmware_catalog_creates() {
    let ver = FirmwareVersion {
        major: 10,
        minor: 0,
        patch: 0,
    };
    let catalog = FirmwareCatalog::new(ver);
    assert_eq!(catalog.version.major, 10);
}
