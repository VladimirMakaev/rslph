use clap::parser::ValueSource;
use clap::{CommandFactory, Parser, Subcommand};
use std::path::PathBuf;

use crate::config::{Config, PartialConfig};
use crate::prompts::PromptMode;

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

    /// Prompt mode selection (basic, gsd)
    #[arg(long, global = true, value_parser = clap::value_parser!(PromptMode))]
    pub mode: Option<PromptMode>,

    /// Append --dangerously-skip-permissions to all Claude invocations
    #[arg(long, global = true)]
    pub no_dsp: bool,

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

    /// Run evaluation in isolated environment (EVAL-01)
    Eval {
        /// Project directory or name to evaluate (optional with --list)
        #[arg(required_unless_present = "list")]
        project: Option<String>,

        /// Number of independent trials to run
        #[arg(long, default_value = "1")]
        trials: u32,

        /// Comma-separated list of modes to evaluate (basic,gsd)
        #[arg(long, value_delimiter = ',')]
        modes: Option<Vec<PromptMode>>,

        /// Keep temp directory after completion
        #[arg(long)]
        keep: bool,

        /// List available built-in projects
        #[arg(long)]
        list: bool,
    },

    /// Re-run tests only on an existing eval workspace
    Retest {
        /// Path to eval workspace directory
        workspace: PathBuf,
    },

    /// Compare two eval result files
    Compare {
        /// First result file (baseline)
        file1: PathBuf,

        /// Second result file (comparison)
        file2: PathBuf,
    },
}

impl Cli {
    /// Build PartialConfig from explicitly provided CLI arguments only.
    /// Values that are CLI defaults are NOT included, allowing config file
    /// values to take precedence over CLI defaults.
    pub fn to_overrides(&self, matches: &clap::ArgMatches) -> PartialConfig {
        PartialConfig {
            claude_path: self.extract_if_explicit(matches, "claude_path", &self.claude_path),
            max_iterations: self.extract_if_explicit(
                matches,
                "max_iterations",
                &self.max_iterations,
            ),
            prompt_mode: self.extract_if_explicit(matches, "mode", &self.mode),
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
            Commands::Plan {
                plan,
                adaptive,
            } => {
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
            } => {
                assert_eq!(plan, PathBuf::from("progress.md"));
                assert!(once);
                assert!(!dry_run);
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
        let cli =
            Cli::try_parse_from(["rslph", "plan", "idea.txt", "--adaptive"]).expect("Should parse");
        match cli.command {
            Commands::Plan {
                plan,
                adaptive,
            } => {
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
            } => {
                assert_eq!(plan, PathBuf::from("progress.md"));
                assert!(!once);
                assert!(dry_run);
            }
            _ => panic!("Expected Build command"),
        }
    }

    #[test]
    fn test_config_override_flag() {
        let cli = Cli::try_parse_from(["rslph", "-c", "/custom/config.toml", "plan", "idea.txt"])
            .expect("Should parse");

        assert_eq!(cli.config, Some(PathBuf::from("/custom/config.toml")));
    }

    #[test]
    fn test_parse_eval_command() {
        let cli = Cli::try_parse_from(["rslph", "eval", "calculator"]).expect("Should parse");
        match cli.command {
            Commands::Eval {
                project,
                trials,
                modes,
                keep,
                list,
            } => {
                assert_eq!(project, Some("calculator".to_string()));
                assert_eq!(trials, 1); // default value
                assert!(modes.is_none());
                assert!(!keep);
                assert!(!list);
            }
            _ => panic!("Expected Eval command"),
        }
    }

    #[test]
    fn test_parse_eval_with_trials() {
        let cli = Cli::try_parse_from(["rslph", "eval", "calculator", "--trials", "5"])
            .expect("Should parse");
        match cli.command {
            Commands::Eval {
                project,
                trials,
                modes,
                keep,
                list,
            } => {
                assert_eq!(project, Some("calculator".to_string()));
                assert_eq!(trials, 5);
                assert!(modes.is_none());
                assert!(!keep);
                assert!(!list);
            }
            _ => panic!("Expected Eval command"),
        }
    }

    #[test]
    fn test_parse_eval_with_keep() {
        let cli =
            Cli::try_parse_from(["rslph", "eval", "calculator", "--keep"]).expect("Should parse");
        match cli.command {
            Commands::Eval {
                project,
                trials,
                modes,
                keep,
                list,
            } => {
                assert_eq!(project, Some("calculator".to_string()));
                assert_eq!(trials, 1);
                assert!(modes.is_none());
                assert!(keep);
                assert!(!list);
            }
            _ => panic!("Expected Eval command"),
        }
    }

    #[test]
    fn test_parse_eval_with_list() {
        let cli = Cli::try_parse_from(["rslph", "eval", "--list"]).expect("Should parse");
        match cli.command {
            Commands::Eval {
                project,
                trials,
                modes,
                keep,
                list,
            } => {
                assert!(project.is_none());
                assert_eq!(trials, 1);
                assert!(modes.is_none());
                assert!(!keep);
                assert!(list);
            }
            _ => panic!("Expected Eval command"),
        }
    }

    #[test]
    fn test_parse_eval_with_modes() {
        let cli = Cli::try_parse_from([
            "rslph",
            "eval",
            "calculator",
            "--modes",
            "basic,gsd",
        ])
        .expect("Should parse");
        match cli.command {
            Commands::Eval {
                project,
                trials,
                modes,
                keep,
                list,
            } => {
                assert_eq!(project, Some("calculator".to_string()));
                assert_eq!(trials, 1);
                let modes = modes.expect("modes should be present");
                assert_eq!(modes.len(), 2);
                assert_eq!(modes[0], PromptMode::Basic);
                assert_eq!(modes[1], PromptMode::Gsd);
                assert!(!keep);
                assert!(!list);
            }
            _ => panic!("Expected Eval command"),
        }
    }

    #[test]
    fn test_parse_eval_with_modes_and_trials() {
        let cli = Cli::try_parse_from([
            "rslph",
            "eval",
            "calculator",
            "--modes",
            "basic,gsd",
            "--trials",
            "3",
        ])
        .expect("Should parse");
        match cli.command {
            Commands::Eval {
                project,
                trials,
                modes,
                keep,
                list,
            } => {
                assert_eq!(project, Some("calculator".to_string()));
                assert_eq!(trials, 3);
                let modes = modes.expect("modes should be present");
                assert_eq!(modes.len(), 2);
                assert_eq!(modes[0], PromptMode::Basic);
                assert_eq!(modes[1], PromptMode::Gsd);
                assert!(!keep);
                assert!(!list);
            }
            _ => panic!("Expected Eval command"),
        }
    }

    #[test]
    fn test_parse_retest_command() {
        let cli =
            Cli::try_parse_from(["rslph", "retest", "/path/to/workspace"]).expect("Should parse");
        match cli.command {
            Commands::Retest { workspace } => {
                assert_eq!(workspace, PathBuf::from("/path/to/workspace"));
            }
            _ => panic!("Expected Retest command"),
        }
    }

    #[test]
    fn test_parse_with_mode_flag() {
        let cli = Cli::try_parse_from(["rslph", "--mode", "gsd", "plan", "idea.txt"])
            .expect("Should parse");

        assert_eq!(cli.mode, Some(PromptMode::Gsd));
    }

    #[test]
    fn test_mode_flag_values() {
        // Test all valid mode values
        for (input, expected) in [
            ("basic", PromptMode::Basic),
            ("gsd", PromptMode::Gsd),
        ] {
            let cli = Cli::try_parse_from(["rslph", "--mode", input, "plan", "idea.txt"])
                .expect("Should parse");
            assert_eq!(cli.mode, Some(expected));
        }
    }

    #[test]
    fn test_parse_compare_command() {
        let cli = Cli::try_parse_from([
            "rslph",
            "compare",
            "/path/to/baseline.json",
            "/path/to/comparison.json",
        ])
        .expect("Should parse");
        match cli.command {
            Commands::Compare { file1, file2 } => {
                assert_eq!(file1, PathBuf::from("/path/to/baseline.json"));
                assert_eq!(file2, PathBuf::from("/path/to/comparison.json"));
            }
            _ => panic!("Expected Compare command"),
        }
    }

    #[test]
    fn test_parse_no_dsp_flag() {
        let cli =
            Cli::try_parse_from(["rslph", "--no-dsp", "plan", "idea.txt"]).expect("Should parse");
        assert!(cli.no_dsp);
    }

    #[test]
    fn test_no_dsp_default_false() {
        let cli = Cli::try_parse_from(["rslph", "plan", "idea.txt"]).expect("Should parse");
        assert!(!cli.no_dsp);
    }
}
