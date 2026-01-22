use std::time::Duration;

use clap::Parser;
use rslph::build::run_build_command;
use rslph::build::tokens::format_tokens;
use rslph::cli::{Cli, Commands};
use rslph::eval::{run_eval_command, run_retest_command};
use rslph::planning::run_plan_command;
use rslph::subprocess::setup_ctrl_c_handler;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();
    let config = cli.load_config()?;

    match cli.command {
        Commands::Plan { plan, adaptive } => {
            let working_dir = std::env::current_dir()?;

            // Set up Ctrl+C handling
            let cancel_token = setup_ctrl_c_handler();

            // Calculate timeout: max_iterations * 10 minutes per iteration
            let timeout = Duration::from_secs(config.max_iterations as u64 * 600);

            println!("Planning: {}", plan);
            println!("Working directory: {}", working_dir.display());
            if adaptive {
                println!("Mode: adaptive (with clarifying questions)");
            }

            match run_plan_command(&plan, adaptive, &config, &working_dir, cancel_token, timeout).await {
                Ok((output_path, _tokens)) => {
                    // Tokens already printed by run_plan_command
                    println!("Success! Progress file written to: {}", output_path.display());
                }
                Err(e) => {
                    eprintln!("Planning failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Build { plan, once, dry_run, no_tui } => {
            // Set up Ctrl+C handling
            let cancel_token = setup_ctrl_c_handler();

            // Determine if TUI will be used - if so, suppress startup messages
            let use_tui = config.tui_enabled && !no_tui && !dry_run;

            if !use_tui {
                println!("Building: {}", plan.display());
                if once {
                    println!("Mode: single iteration (--once)");
                }
                if dry_run {
                    println!("Mode: dry run (--dry-run)");
                }
                if no_tui {
                    println!("Mode: headless (--no-tui)");
                }
            }

            match run_build_command(plan, once, dry_run, no_tui, &config, cancel_token).await {
                Ok(_tokens) => {
                    // Tokens already printed by build command
                    if !use_tui {
                        println!("Build completed successfully.");
                    }
                }
                Err(e) => {
                    eprintln!("Build failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Eval { project, trials, keep, no_tui, list } => {
            // Handle --list flag
            if list {
                println!("Available built-in projects:");
                for name in rslph::eval::list_projects() {
                    println!("  - {}", name);
                }
                return Ok(());
            }

            // project is required when not listing
            let project = project.expect("project required when not listing");

            // Set up Ctrl+C handling
            let cancel_token = setup_ctrl_c_handler();

            println!("Evaluating: {}", project);
            if trials > 1 {
                println!("Trials: {}", trials);
            }
            if keep {
                println!("Mode: keep temp directory (--keep)");
            }
            if no_tui {
                println!("Mode: headless (--no-tui)");
            }

            match run_eval_command(project, keep, no_tui, &config, cancel_token).await {
                Ok(result) => {
                    println!("\n=== EVAL COMPLETE ===");
                    println!("Project: {}", result.project);
                    println!("Time: {:.1}s", result.elapsed_secs);
                    println!("Iterations: {}", result.iterations);
                    println!(
                        "Tokens: In: {} | Out: {} | CacheW: {} | CacheR: {}",
                        format_tokens(result.total_tokens.input_tokens),
                        format_tokens(result.total_tokens.output_tokens),
                        format_tokens(result.total_tokens.cache_creation_input_tokens),
                        format_tokens(result.total_tokens.cache_read_input_tokens),
                    );
                    if let Some(ref test_results) = result.test_results {
                        println!(
                            "Tests: {}/{} passed ({:.1}%)",
                            test_results.passed,
                            test_results.total,
                            test_results.pass_rate()
                        );
                    }
                    if let Some(path) = result.workspace_path {
                        println!("Workspace: {}", path.display());
                    }
                }
                Err(e) => {
                    eprintln!("Eval failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Retest { workspace } => {
            // Set up Ctrl+C handling
            let cancel_token = setup_ctrl_c_handler();

            println!("Re-running tests on: {}", workspace.display());

            match run_retest_command(workspace, &config, cancel_token).await {
                Ok(result) => {
                    println!("\n=== RETEST COMPLETE ===");
                    println!("Project: {}", result.project);
                    if let Some(ref test_results) = result.test_results {
                        println!(
                            "Tests: {}/{} passed ({:.1}%)",
                            test_results.passed,
                            test_results.total,
                            test_results.pass_rate()
                        );
                    } else {
                        println!("No test results available");
                    }
                    if let Some(path) = result.workspace_path {
                        println!("Workspace: {}", path.display());
                    }
                }
                Err(e) => {
                    eprintln!("Retest failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}
