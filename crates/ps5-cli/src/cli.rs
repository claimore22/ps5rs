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
        path: PathBuf,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
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
    /// Manage NID catalog (sync from Supabase, push unknowns)
    Catalog {
        #[command(subcommand)]
        command: CatalogCommand,
    },
}

#[derive(Subcommand)]
pub enum CatalogCommand {
    /// Download latest NID catalog from Supabase
    Sync {
        #[arg(long, env = "PS5RS_SUPABASE_KEY")]
        key: Option<String>,
        #[arg(short, long, default_value = "analysis/catalog")]
        catalog_dir: PathBuf,
    },
    /// Upload unknown NIDs as submission candidates
    PushUnknown {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(long, env = "PS5RS_SUPABASE_KEY")]
        key: String,
        #[arg(long)]
        url: Option<String>,
        #[arg(short, long)]
        submitter: Option<String>,
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
