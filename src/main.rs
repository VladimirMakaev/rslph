use std::time::Duration;

use clap::Parser;
use rslph::build::run_build_command;
use rslph::cli::{Cli, Commands};
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
                Ok(output_path) => {
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

            match run_build_command(plan, once, dry_run, no_tui, &config, cancel_token).await {
                Ok(()) => {
                    println!("Build completed successfully.");
                }
                Err(e) => {
                    eprintln!("Build failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}
