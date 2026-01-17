use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "rslph")]
#[command(about = "Ralph Wiggum Loop - autonomous AI coding agent")]
#[command(version)]
pub struct Cli {
    /// Override config file path
    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,

    /// Override claude command path
    #[arg(long, global = true)]
    pub claude_path: Option<String>,

    /// Maximum iterations (overrides config)
    #[arg(long, global = true)]
    pub max_iterations: Option<u32>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Transform idea/plan into structured progress file (CMD-01)
    Plan {
        /// Path to the plan/idea file or inline text
        plan: String,

        /// Use adaptive mode with clarifying questions
        #[arg(long)]
        adaptive: bool,
    },

    /// Execute tasks iteratively with fresh context (CMD-02)
    Build {
        /// Path to the progress file
        plan: PathBuf,

        /// Run single iteration only
        #[arg(long)]
        once: bool,

        /// Preview without executing
        #[arg(long)]
        dry_run: bool,
    },
}
