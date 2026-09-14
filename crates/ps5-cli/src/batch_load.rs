use std::path::Path;

pub(crate) fn cmd_batch_load(
    games_dir: &Path,
    output_dir: &Path,
    offline_dir: &Path,
    json: bool,
    force: bool,
) {
    if let Err(e) = ps5_farm::batch::run_batch_load(games_dir, output_dir, offline_dir, json, force)
    {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
