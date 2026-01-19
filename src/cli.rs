use clap::parser::ValueSource;
use clap::{CommandFactory, Parser, Subcommand};
use std::path::PathBuf;

use crate::config::{Config, PartialConfig};

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

        /// Disable TUI and use simple output
        #[arg(long)]
        no_tui: bool,
    },
}

impl Cli {
    /// Build PartialConfig from explicitly provided CLI arguments only.
    /// Values that are CLI defaults are NOT included, allowing config file
    /// values to take precedence over CLI defaults.
    pub fn to_overrides(&self, matches: &clap::ArgMatches) -> PartialConfig {
        PartialConfig {
            claude_path: self.extract_if_explicit(matches, "claude_path", &self.claude_path),
            max_iterations: self.extract_if_explicit(matches, "max_iterations", &self.max_iterations),
            ..Default::default()
        }
    }

    fn extract_if_explicit<T: Clone>(
        &self,
        matches: &clap::ArgMatches,
        name: &str,
        value: &Option<T>,
    ) -> Option<T> {
        // Only include if value was explicitly provided by user
        if matches.value_source(name) == Some(ValueSource::CommandLine) {
            value.clone()
        } else {
            None
        }
    }

    /// Load config with CLI overrides applied (main entry point)
    pub fn load_config(&self) -> color_eyre::Result<Config> {
        let matches = Cli::command().get_matches_from(std::env::args_os());
        let overrides = self.to_overrides(&matches);
        Config::load_with_overrides(self.config.as_deref(), overrides)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_plan_command() {
        let cli = Cli::try_parse_from(["rslph", "plan", "my-idea.txt"]).expect("Should parse");
        match cli.command {
            Commands::Plan { plan, adaptive } => {
                assert_eq!(plan, "my-idea.txt");
                assert!(!adaptive);
            }
            _ => panic!("Expected Plan command"),
        }
    }

    #[test]
    fn test_parse_build_command() {
        let cli =
            Cli::try_parse_from(["rslph", "build", "progress.md", "--once"]).expect("Should parse");
        match cli.command {
            Commands::Build {
                plan,
                once,
                dry_run,
                no_tui,
            } => {
                assert_eq!(plan, PathBuf::from("progress.md"));
                assert!(once);
                assert!(!dry_run);
                assert!(!no_tui);
            }
            _ => panic!("Expected Build command"),
        }
    }

    #[test]
    fn test_global_flags() {
        let cli = Cli::try_parse_from([
            "rslph",
            "--max-iterations",
            "50",
            "--claude-path",
            "/usr/bin/claude",
            "plan",
            "idea.txt",
        ])
        .expect("Should parse");

        assert_eq!(cli.max_iterations, Some(50));
        assert_eq!(cli.claude_path, Some("/usr/bin/claude".to_string()));
    }

    #[test]
    fn test_parse_plan_with_adaptive() {
        let cli = Cli::try_parse_from(["rslph", "plan", "idea.txt", "--adaptive"])
            .expect("Should parse");
        match cli.command {
            Commands::Plan { plan, adaptive } => {
                assert_eq!(plan, "idea.txt");
                assert!(adaptive);
            }
            _ => panic!("Expected Plan command"),
        }
    }

    #[test]
    fn test_parse_build_with_dry_run() {
        let cli = Cli::try_parse_from(["rslph", "build", "progress.md", "--dry-run"])
            .expect("Should parse");
        match cli.command {
            Commands::Build {
                plan,
                once,
                dry_run,
                no_tui,
            } => {
                assert_eq!(plan, PathBuf::from("progress.md"));
                assert!(!once);
                assert!(dry_run);
                assert!(!no_tui);
            }
            _ => panic!("Expected Build command"),
        }
    }

    #[test]
    fn test_parse_build_with_no_tui() {
        let cli = Cli::try_parse_from(["rslph", "build", "progress.md", "--no-tui"])
            .expect("Should parse");
        match cli.command {
            Commands::Build {
                plan,
                once,
                dry_run,
                no_tui,
            } => {
                assert_eq!(plan, PathBuf::from("progress.md"));
                assert!(!once);
                assert!(!dry_run);
                assert!(no_tui);
            }
            _ => panic!("Expected Build command"),
        }
    }

    #[test]
    fn test_config_override_flag() {
        let cli = Cli::try_parse_from([
            "rslph",
            "-c",
            "/custom/config.toml",
            "plan",
            "idea.txt",
        ])
        .expect("Should parse");

        assert_eq!(cli.config, Some(PathBuf::from("/custom/config.toml")));
    }
}
