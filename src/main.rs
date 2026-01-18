use std::time::Duration;

use clap::Parser;
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
            if adaptive {
                // Adaptive mode not yet implemented
                eprintln!("Adaptive mode not yet implemented (Phase 3, Plan 02)");
                std::process::exit(1);
            }

            let working_dir = std::env::current_dir()?;

            // Set up Ctrl+C handling
            let cancel_token = setup_ctrl_c_handler();

            // Calculate timeout: max_iterations * 10 minutes per iteration
            let timeout = Duration::from_secs(config.max_iterations as u64 * 600);

            println!("Planning: {}", plan);
            println!("Working directory: {}", working_dir.display());

            match run_plan_command(&plan, &config, &working_dir, cancel_token, timeout).await {
                Ok(output_path) => {
                    println!("Success! Progress file written to: {}", output_path.display());
                }
                Err(e) => {
                    eprintln!("Planning failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Build { plan, once, dry_run } => {
            println!("Building: {}", plan.display());
            println!("Once mode: {}", once);
            println!("Dry run: {}", dry_run);
            println!("Using config: {:?}", config);
            // Actual implementation in Phase 4
            println!("Build command not yet implemented (Phase 4)");
        }
    }

    Ok(())
}
