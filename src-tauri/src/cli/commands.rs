use clap::{Parser, Subcommand, Args, ValueEnum};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "taurihands")]
#[command(about = "TauriHands - AI-Driven Development Agent")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Configuration file path
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Use Codex CLI for local AI assistance
    #[arg(short = 'x', long)]
    pub use_codex: bool,

    /// Codex model to use (gpt-4-codex, gpt-3.5-codex)
    #[arg(long, value_name = "MODEL")]
    pub codex_model: Option<String>,

    /// Codex reasoning level (0-5)
    #[arg(long, value_name = "LEVEL")]
    pub codex_reasoning: Option<u8>,

    /// Codex approval mode (always, edit, ask)
    #[arg(long, value_name = "MODE")]
    pub codex_approval: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start the TauriHands agent
    Run(RunArgs),
    /// Start terminal UI mode
    Terminal(TerminalArgs),
    /// Start headless mode
    Headless(HeadlessArgs),
    /// Start web interface
    Web(WebArgs),
    /// Start GUI server
    Serve(ServeArgs),
    /// Configure settings
    Config(ConfigArgs),
    /// Show version information
    Version,
}

#[derive(Parser)]
pub struct RunArgs {
    /// Task description
    #[arg(short, long)]
    pub task: Option<String>,

    /// Workspace path
    #[arg(short, long, value_name = "DIR")]
    pub workspace: Option<PathBuf>,

    /// Model to use
    #[arg(short, long, value_name = "MODEL")]
    pub model: Option<String>,

    /// Run in headless mode
    #[arg(short, long)]
    pub headless: bool,

    /// Auto-confirm all actions
    #[arg(short, long)]
    pub yes: bool,

    /// Maximum steps
    #[arg(short, long, value_name = "NUM")]
    pub max_steps: Option<usize>,
}

#[derive(Parser)]
pub struct TerminalArgs {
    /// Workspace path
    #[arg(short, long, value_name = "DIR")]
    pub workspace: Option<PathBuf>,

    /// Enable mouse support
    #[arg(short, long)]
    pub mouse: bool,
}

#[derive(Parser)]
pub struct HeadlessArgs {
    /// Task description (required)
    #[arg(short, long)]
    pub task: String,

    /// Workspace path
    #[arg(short, long, value_name = "DIR")]
    pub workspace: Option<PathBuf>,

    /// Output format
    #[arg(short, long, value_name = "FORMAT", default_value = "json")]
    pub output: OutputFormat,

    /// Save output to file
    #[arg(short, long, value_name = "FILE")]
    pub output_file: Option<PathBuf>,
}

#[derive(Parser)]
pub struct WebArgs {
    /// Port to bind to
    #[arg(short, long, value_name = "PORT", default_value = "3000")]
    pub port: u16,

    /// Host to bind to
    #[arg(short, long, value_name = "HOST", default_value = "localhost")]
    pub host: String,

    /// Open browser automatically
    #[arg(short, long)]
    pub open: bool,
}

#[derive(Parser)]
pub struct ServeArgs {
    /// Port to bind to
    #[arg(short, long, value_name = "PORT", default_value = "8080")]
    pub port: u16,

    /// Host to bind to
    #[arg(short, long, value_name = "HOST", default_value = "localhost")]
    pub host: String,

    /// Enable API access
    #[arg(short, long)]
    pub api: bool,
}

#[derive(Parser)]
pub struct ConfigArgs {
    /// Show current configuration
    #[arg(short, long)]
    pub show: bool,

    /// Set configuration value
    #[arg(short, long, value_name = "KEY=VALUE")]
    pub set: Option<String>,

    /// Reset configuration to defaults
    #[arg(short, long)]
    pub reset: bool,

    /// List all configuration options
    #[arg(short, long)]
    pub list: bool,
}

#[derive(clap::ValueEnum, Clone, Debug, Serialize, Deserialize)]
pub enum OutputFormat {
    Json,
    Yaml,
    Text,
}

impl Default for OutputFormat {
    fn default() -> Self {
        OutputFormat::Json
    }
}
