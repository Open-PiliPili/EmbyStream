use std::{error::Error as StdError, path::PathBuf};

use clap::Parser;

use embystream::cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<(), Box<dyn StdError>> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Run(run_args)) => {
            run_app(run_args.config).await?;
        }
        None => {}
    }

    Ok(())
}

async fn run_app(config_path: Option<PathBuf>) -> Result<(), Box<dyn StdError>> {
    println!("Starting the application asynchronously...");

    if let Some(path) = &config_path {
        println!("Using configuration file: {:?}", path);
    } else {
        println!("No configuration file specified, using defaults.");
    }

    // --- Initalize service ---
    // example
    // let app_state = Arc::new(AppState::new(config_path).await?);
    // let stream_service = Arc::new(AppStreamService::new(app_state.clone()));
    // let stream_middleware = StreamHandler::new("/stream", stream_service);
    //
    // let mut gateway = Gateway::new("127.0.0.1:8080")
    //     .add_middleware(Box::new(stream_middleware));
    //
    // gateway.set_handler(...);
    // gateway.listen().await?;

    println!("Application finished.");
    Ok(())
}
