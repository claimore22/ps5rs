use clap::Parser;

mod analyze;
mod catalog;
mod cli;
mod dataset;
mod extract;
mod inspect;
mod terminal;
mod util;

use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Inspect { file, json, output } => inspect::cmd_inspect(&file, json, &output),
        Commands::Imports { file, json, output } => inspect::cmd_imports(&file, json, &output),
        Commands::Segments { file } => inspect::cmd_segments(&file),
        Commands::Dynamic { file } => inspect::cmd_dynamic(&file),
        Commands::Symbols { file } => inspect::cmd_symbols(&file),
        Commands::Nid { name } => inspect::cmd_nid(&name),
        Commands::Scan { path, output, nids, include_modules } => {
            analyze::cmd_scan(&path, &output, &nids, include_modules)
        }
        Commands::Analyze { nids, include_modules, command } => {
            analyze::cmd_analyze(command, &nids, include_modules)
        }
        Commands::Extract { file, output } => extract::cmd_extract(&file, &output),
        Commands::BatchExtract { path, output, nids, include_modules } => {
            extract::cmd_batch_extract(&path, &output, &nids, include_modules)
        }
        Commands::Validate { path, output } => dataset::cmd_validate(&path, &output),
        Commands::Dashboard { path, output } => dataset::cmd_dashboard(&path, &output),
        Commands::ExportUnknown { path, group_by, output } => {
            dataset::cmd_export_unknown(&path, &group_by, &output)
        }
    }
}
