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
    /// Show header, segments, imports, and metadata summary for a binary
    Inspect {
        file: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    /// List NID imports from a binary, with optional catalog name resolution
    Imports {
        file: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        catalog: bool,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    /// List ELF program headers and SELF segments for a binary
    Segments { file: PathBuf },
    /// Show dynamic entries and needed libraries for a binary
    Dynamic { file: PathBuf },
    /// List symbol table entries for a binary
    Symbols { file: PathBuf },
    /// Compute the Sony NID hash for a symbol name
    Nid { name: String },
    /// Scan a games directory and build a JSON analysis dataset
    Scan {
        path: PathBuf,
        #[arg(short, long, value_hint = ValueHint::DirPath)]
        output: PathBuf,
        #[arg(long = "nids", value_hint = ValueHint::FilePath)]
        nids: Vec<PathBuf>,
        #[arg(long)]
        include_modules: bool,
    },
    /// Run reports over a scan dataset or games directory
    Analyze {
        #[arg(long = "nids", value_hint = ValueHint::FilePath)]
        nids: Vec<PathBuf>,
        #[arg(long)]
        include_modules: bool,
        #[command(subcommand)]
        command: AnalyzeCommand,
    },
    /// Extract the inner ELF from a SELF container
    Extract {
        file: PathBuf,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    /// Batch-extract ELFs for every game in a directory
    BatchExtract {
        path: PathBuf,
        #[arg(short, long, value_hint = ValueHint::DirPath)]
        output: PathBuf,
        #[arg(long = "nids", value_hint = ValueHint::FilePath)]
        nids: Vec<PathBuf>,
        #[arg(long)]
        include_modules: bool,
    },
    /// Validate binaries and scan datasets
    Validate {
        #[command(subcommand)]
        command: ValidateCommand,
    },
    /// Generate a static HTML dashboard from a scan dataset
    Dashboard {
        path: PathBuf,
        #[arg(short, long, value_hint = ValueHint::DirPath, default_value = "analysis/dashboard")]
        output: PathBuf,
        #[arg(long = "games", value_hint = ValueHint::DirPath)]
        games: Option<PathBuf>,
    },
    /// Export unresolved NIDs from a dataset to CSV
    ExportUnknown {
        path: PathBuf,
        #[arg(long, default_value = "frequency")]
        group_by: String,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    /// Extract printable strings and detect engine/SDK fingerprints
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
    /// Run a PS5 eboot in the host emulator and print the execution report
    Run {
        file: PathBuf,
        /// Directory containing PRX modules (default: `file parent/sce_module`)
        #[arg(long, value_hint = ValueHint::DirPath)]
        prx_dir: Option<PathBuf>,
        /// Emit the execution report as JSON
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
    /// Detect third-party middleware libraries in game /prx folders
    Middleware {
        /// Games directory containing subdirectories with eboot.bin
        path: PathBuf,
        #[arg(long, value_enum, default_value = "terminal")]
        format: OutputFormat,
        /// Write the report to a file instead of stdout
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    /// Show module dependency graph (uses ps5-deps)
    Deps {
        /// Games directory or single binary path
        path: PathBuf,
        #[arg(long, value_enum, default_value = "terminal")]
        format: OutputFormat,
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
    /// Import NID/name pairs from *.a libraries
    ImportStubs {
        /// Directory containing *.a files (fallback directory also checked)
        #[arg(value_hint = ValueHint::DirPath)]
        library_dir: PathBuf,
        /// Append net-new NID lines to this catalog file
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
        /// Cross-check .scenid NIDs against hash(name)
        #[arg(long)]
        verify: bool,
    },
    /// Print every symbol from *.a libraries
    DumpStubs {
        /// Directory containing *.a files or a single archive file (fallback checked)
        #[arg(value_hint = ValueHint::DirPath)]
        path: PathBuf,
        /// Write the dump to this file instead of stdout
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
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
    /// Show per-game and corpus-wide import statistics
    Stats {
        path: PathBuf,
        #[arg(long, value_enum, default_value = "terminal")]
        format: OutputFormat,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    /// Show library usage heatmap across games
    Heatmap {
        path: PathBuf,
        #[arg(long, value_enum, default_value = "terminal")]
        format: OutputFormat,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    /// Show NID frequency across the corpus
    Frequency {
        path: PathBuf,
        #[arg(long, value_enum, default_value = "terminal")]
        format: OutputFormat,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    /// List imports that failed catalog resolution
    Unresolved {
        path: PathBuf,
        #[arg(long, value_enum, default_value = "terminal")]
        format: OutputFormat,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    /// Export the module dependency graph (dot/json)
    Graph {
        path: PathBuf,
        #[arg(long)]
        include_nids: bool,
        #[arg(long, value_enum, default_value = "dot")]
        format: OutputFormat,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    /// Show per-library import inventory
    Imports {
        path: PathBuf,
        #[arg(long, value_enum, default_value = "terminal")]
        format: OutputFormat,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    /// List unknown NIDs in a scan dataset
    Unknown {
        path: PathBuf,
        #[arg(long, value_enum, default_value = "terminal")]
        format: OutputFormat,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    /// Collect a games directory into a JSON analysis file
    Collect {
        path: PathBuf,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    /// Show library version strings per game
    LibraryVersions {
        path: PathBuf,
        #[arg(long, value_enum, default_value = "terminal")]
        format: OutputFormat,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    /// Detect game engines from dataset strings
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
