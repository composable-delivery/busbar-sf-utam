//! UTAM CLI
//!
//! Command-line interface for compiling UTAM page objects to Rust.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "utam")]
#[command(author, version, about = "UTAM Rust Compiler")]
struct Cli {
    /// Path to configuration file
    #[arg(short, long, default_value = "utam.config.json")]
    config: PathBuf,

    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile UTAM JSON files to Rust
    Compile {
        /// Input files or directories
        #[arg(required = true)]
        input: Vec<PathBuf>,

        /// Output directory
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Watch for changes
        #[arg(short, long)]
        watch: bool,
    },

    /// Validate UTAM JSON files
    Validate {
        /// Files to validate
        #[arg(required = true)]
        files: Vec<PathBuf>,

        /// Output format (text, json, sarif)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Initialize configuration
    Init {
        /// Force overwrite existing config
        #[arg(short, long)]
        force: bool,
    },

    /// Lint UTAM JSON files
    Lint {
        /// Files to lint
        #[arg(required = true)]
        files: Vec<PathBuf>,

        /// Output SARIF report
        #[arg(long)]
        sarif: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Compile { input, output, watch } => {
            println!("Compiling {:?} -> {:?} (watch: {})", input, output, watch);
            // TODO: Implement
        }
        Commands::Validate { files, format } => {
            println!("Validating {:?} (format: {})", files, format);
            // TODO: Implement
        }
        Commands::Init { force } => {
            println!("Initializing config (force: {})", force);
            // TODO: Implement
        }
        Commands::Lint { files, sarif } => {
            println!("Linting {:?} (sarif: {:?})", files, sarif);
            // TODO: Implement
        }
    }
}
