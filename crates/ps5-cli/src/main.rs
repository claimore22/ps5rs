use clap::Parser;

mod analyze;
mod batch_load;
mod catalog;
mod cli;
mod dataset;
mod export_scan;
mod extract;
mod inspect;
mod load;
mod strings;
mod terminal;
mod util;

use cli::{CatalogCommand, Cli, Commands};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Inspect { file, json, output } => inspect::cmd_inspect(&file, json, &output),
        Commands::Imports { file, json, output } => inspect::cmd_imports(&file, json, &output),
        Commands::Segments { file } => inspect::cmd_segments(&file),
        Commands::Dynamic { file } => inspect::cmd_dynamic(&file),
        Commands::Symbols { file } => inspect::cmd_symbols(&file),
        Commands::Nid { name } => inspect::cmd_nid(&name),
        Commands::Scan {
            path,
            output,
            nids,
            include_modules,
        } => analyze::cmd_scan(&path, &output, &nids, include_modules),
        Commands::Analyze {
            nids,
            include_modules,
            command,
        } => analyze::cmd_analyze(command, &nids, include_modules),
        Commands::Extract { file, output } => extract::cmd_extract(&file, &output),
        Commands::BatchExtract {
            path,
            output,
            nids,
            include_modules,
        } => extract::cmd_batch_extract(&path, &output, &nids, include_modules),
        Commands::Validate { path, output } => dataset::cmd_validate(&path, &output),
        Commands::Dashboard { path, output } => dataset::cmd_dashboard(&path, &output),
        Commands::ExportUnknown {
            path,
            group_by,
            output,
        } => dataset::cmd_export_unknown(&path, &group_by, &output),
        Commands::Exports {
            file,
            json,
            search,
            output,
        } => inspect::cmd_exports(&file, json, &search, &output),
        Commands::Load {
            file,
            prx_dir,
            json,
        } => load::cmd_load(&file, prx_dir, json),
        Commands::ExportScan { path, output } => export_scan::cmd_export_scan(&path, &output),
        Commands::BatchLoad {
            path,
            output,
            offline_dir,
            json,
        } => batch_load::cmd_batch_load(&path, &output, &offline_dir, json),
        Commands::Strings {
            file,
            min_length,
            offsets,
            detect,
            output,
        } => strings::cmd_strings(&file, min_length, offsets, detect, &output),
        Commands::Catalog { command } => match command {
            CatalogCommand::Sync { key, catalog_dir } => {
                catalog::cmd_sync(key.as_deref(), &catalog_dir)
            }
            CatalogCommand::PushUnknown {
                input,
                key,
                url,
                submitter,
            } => catalog::cmd_push_unknown(
                &input,
                key.as_deref(),
                url.as_deref(),
                submitter.as_deref(),
            ),
        },
    }
}
