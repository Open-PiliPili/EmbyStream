use embystream::{info_log, logger::*};

fn setup_logger() {
    Logger::builder().with_level(LogLevel::Debug).build()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    setup_logger();

    info_log!("Starting Emby Stream application");

    Ok(())
}