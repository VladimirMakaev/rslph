use clap::Parser;
use rslph::cli::{Cli, Commands};

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();
    let config = cli.load_config()?;

    match cli.command {
        Commands::Plan { plan, adaptive } => {
            println!("Planning: {}", plan);
            println!("Adaptive mode: {}", adaptive);
            println!("Using config: {:?}", config);
            // Actual implementation in Phase 3
            println!("Plan command not yet implemented (Phase 3)");
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
