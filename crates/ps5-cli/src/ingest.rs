use std::path::{Path, PathBuf};

pub(crate) fn cmd_archive(
    game: &Option<PathBuf>,
    dataset: &Path,
    offline_dir: &Path,
    mark_deleted: &Option<String>,
) {
    if let Err(e) = ps5_farm::ingest::cmd_archive(game, dataset, offline_dir, mark_deleted) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
