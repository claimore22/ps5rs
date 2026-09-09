pub mod constants;
pub mod database;
pub mod functions;
pub mod libraries;
pub mod structures;
pub mod versions;

pub use database::SdkDatabase;
pub use functions::SdkFunction;
pub use libraries::{LibraryInfo, ModuleKind};
pub use structures::{FieldInfo, StructInfo};
pub use versions::VersionRange;

use std::path::Path;

pub fn load_sdk_database_from_stubs(stubs_dir: &Path) -> SdkDatabase {
    let mut db = SdkDatabase::new();
    db.populate_from_stubs_dir(stubs_dir);
    db
}

pub fn load_sdk_database_from_nids_csv(csv_path: &Path) -> SdkDatabase {
    let mut db = SdkDatabase::new();
    db.populate_from_nids_csv(csv_path);
    db
}
