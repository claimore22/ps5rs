use clap::{Parser, Subcommand, ValueHint};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "ps5rs", version, about = "PS5 binary inspector")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Inspect {
        file: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    Imports {
        file: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        catalog: bool,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    Segments {
        file: PathBuf,
    },
    Dynamic {
        file: PathBuf,
    },
    Symbols {
        file: PathBuf,
    },
    Nid {
        name: String,
    },
    Scan {
        path: PathBuf,
        #[arg(short, long, value_hint = ValueHint::DirPath)]
        output: PathBuf,
        #[arg(long = "nids", value_hint = ValueHint::FilePath)]
        nids: Vec<PathBuf>,
        #[arg(long)]
        include_modules: bool,
    },
    Analyze {
        #[arg(long = "nids", value_hint = ValueHint::FilePath)]
        nids: Vec<PathBuf>,
        #[arg(long)]
        include_modules: bool,
        #[command(subcommand)]
        command: AnalyzeCommand,
    },
    Extract {
        file: PathBuf,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    BatchExtract {
        path: PathBuf,
        #[arg(short, long, value_hint = ValueHint::DirPath)]
        output: PathBuf,
        #[arg(long = "nids", value_hint = ValueHint::FilePath)]
        nids: Vec<PathBuf>,
        #[arg(long)]
        include_modules: bool,
    },
    Validate {
        #[command(subcommand)]
        command: ValidateCommand,
    },
    Dashboard {
        path: PathBuf,
        #[arg(short, long, value_hint = ValueHint::DirPath, default_value = "analysis/dashboard")]
        output: PathBuf,
    },
    ExportUnknown {
        path: PathBuf,
        #[arg(long, default_value = "frequency")]
        group_by: String,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    Strings {
        file: PathBuf,
        #[arg(short = 'n', long, default_value_t = 4)]
        min_length: u8,
        #[arg(long)]
        offsets: bool,
        #[arg(long)]
        detect: bool,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    /// List exports from a PS5 binary
    Exports {
        file: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        search: Option<String>,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    /// Load a PS5 binary into the virtual memory model
    Load {
        file: PathBuf,
        #[arg(long, value_hint = ValueHint::DirPath)]
        prx_dir: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Batch-scan ELF/PRX files and produce offline export files for the loader
    ExportScan {
        /// Directory containing ELF/PRX/SELF files to scan
        path: PathBuf,
        /// Output directory for .exports.json files
        #[arg(short, long, default_value = "system_modules")]
        output: PathBuf,
    },
    /// Batch-load all games: run virtual loader, collect reports, aggregate stats
    BatchLoad {
        /// Games directory containing subdirectories with eboot.bin
        path: PathBuf,
        /// Output directory for per-game reports and summary
        #[arg(short, long, default_value = "analysis/load")]
        output: PathBuf,
        /// Path to offline export directory (system_modules)
        #[arg(long = "offline-dir", default_value = "system_modules")]
        offline_dir: PathBuf,
        /// Print combined JSON to stdout instead of writing files
        #[arg(long)]
        json: bool,
    },
    /// Manage NID catalog (sync from Supabase, push unknowns)
    Catalog {
        #[command(subcommand)]
        command: CatalogCommand,
    },
    /// Find NIDs imported by a games corpus that are missing from the catalog
    UnknownNids {
        /// Games directory containing subdirectories with eboot.bin
        path: PathBuf,
        /// REmu CLI binary (remu.exe) for name cross-reference
        #[arg(long, value_hint = ValueHint::FilePath)]
        remu: Option<PathBuf>,
        /// Emit JSON report
        #[arg(long)]
        json: bool,
        /// Write the report to a file instead of stdout
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
pub enum CatalogCommand {
    /// Download latest NID catalog from Supabase
    Sync {
        #[arg(long)]
        key: Option<String>,
        #[arg(short, long, default_value = "analysis/catalog")]
        catalog_dir: PathBuf,
    },
    /// Upload unknown NIDs as submission candidates
    PushUnknown {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(long)]
        key: Option<String>,
        #[arg(long)]
        url: Option<String>,
        #[arg(short, long)]
        submitter: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ValidateCommand {
    /// Validate a PS5 binary's structural metrics
    Binary {
        file: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    /// Validate a scan dataset directory
    Dataset {
        path: PathBuf,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
pub enum AnalyzeCommand {
    Stats {
        path: PathBuf,
        #[arg(long, value_enum, default_value = "terminal")]
        format: OutputFormat,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    Heatmap {
        path: PathBuf,
        #[arg(long, value_enum, default_value = "terminal")]
        format: OutputFormat,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    Frequency {
        path: PathBuf,
        #[arg(long, value_enum, default_value = "terminal")]
        format: OutputFormat,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    Unresolved {
        path: PathBuf,
        #[arg(long, value_enum, default_value = "terminal")]
        format: OutputFormat,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    Graph {
        path: PathBuf,
        #[arg(long)]
        include_nids: bool,
        #[arg(long, value_enum, default_value = "dot")]
        format: OutputFormat,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    Imports {
        path: PathBuf,
        #[arg(long, value_enum, default_value = "terminal")]
        format: OutputFormat,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    Unknown {
        path: PathBuf,
        #[arg(long, value_enum, default_value = "terminal")]
        format: OutputFormat,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    Collect {
        path: PathBuf,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    LibraryVersions {
        path: PathBuf,
        #[arg(long, value_enum, default_value = "terminal")]
        format: OutputFormat,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    Engines {
        path: PathBuf,
        #[arg(long, value_enum, default_value = "terminal")]
        format: OutputFormat,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
}

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum OutputFormat {
    Terminal,
    Csv,
    Json,
    Dot,
}
