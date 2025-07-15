use std::{error::Error, path::PathBuf, str::FromStr, sync::Arc};

use clap::Parser;
use figlet_rs::FIGfont;
use hyper::StatusCode;

use embystream::{
    AppState, GATEWAY_LOGGER_DOMAIN, INIT_LOGGER_DOMAIN, debug_log, error_log, info_log,
};
use embystream::{
    backend::{service::AppStreamService, stream::StreamMiddleware},
    cli::{Cli, Commands},
    config::{config::Config, general::StreamMode},
    frontend::{forward::ForwardMiddleware, service::AppForwardService},
    gateway::{
        CorsMiddleware, LoggerMiddleware, OptionsMiddleware, chain::Handler, context::Context,
        gateway::Gateway, response::ResponseBuilder, ua_filter::UserAgentFilterMiddleware,
    },
    logger::{LogLevel, Logger},
    system::SystemInfo,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Run(run_args)) => {
            run_app(run_args.config).await?;
        }
        None => {}
    }
    Ok(())
}

async fn run_app(config_path: Option<PathBuf>) -> Result<(), Box<dyn Error + Send + Sync>> {
    setup_figlet();
    let config = setup_load_config(config_path);
    setup_logger(&config);
    setup_print_info(&config);

    let app_state = setup_cache(&config).await;
    let frontend_state = app_state.clone();
    let backend_state = app_state.clone();

    tokio::try_join!(
        setup_frontend_gateway(&frontend_state),
        setup_backend_gateway(&backend_state)
    )?;

    Ok(())
}

fn setup_figlet() {
    if let Ok(standard_font) = FIGfont::standard() {
        if let Some(figure) = standard_font.convert("EMBYSTREAM") {
            println!("{}", figure);
        }
    }
}

fn setup_print_info(config: &Config) {
    info_log!(INIT_LOGGER_DOMAIN, "Initializing EmbyStream...");

    let system_info = SystemInfo::new();
    let configurarion = if cfg!(debug_assertions) {
        "Development"
    } else {
        "Production"
    };
    info_log!(
        INIT_LOGGER_DOMAIN,
        "Environment: {:?} [{:?}], Version: {:?}",
        system_info.environment,
        &configurarion,
        system_info.version
    );
    info_log!(
        INIT_LOGGER_DOMAIN,
        "Log level: {}",
        config.general.log_level.as_str()
    );
}

fn setup_load_config(config_path: Option<PathBuf>) -> Config {
    match Config::load_or_init(config_path) {
        Ok(config) => {
            info_log!(INIT_LOGGER_DOMAIN, "Configuration loaded successfully.");
            config
        }
        Err(e) => {
            error_log!(
                INIT_LOGGER_DOMAIN,
                "Configuration initialization failed: {}",
                e
            );
            std::process::exit(1);
        }
    }
}

fn setup_logger(config: &Config) {
    let level = LogLevel::from_str(&config.general.log_level).unwrap_or(LogLevel::Info);
    Logger::builder().with_level(level).build();
}

async fn setup_cache(config: &Config) -> Arc<AppState> {
    let app_state = AppState::new(config.clone()).await;
    Arc::new(app_state)
}

async fn setup_frontend_gateway(
    app_state: &Arc<AppState>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let config = app_state.get_config().await.clone();
    let mode = config.general.stream_mode;

    if !(mode == StreamMode::Frontend || mode == StreamMode::Dual) {
        return Ok(());
    }

    debug_log!(INIT_LOGGER_DOMAIN, "Successfully start frontend listener");

    let frontend = config.frontend.as_ref().ok_or_else(|| {
        error_log!(
            INIT_LOGGER_DOMAIN,
            "Error: Frontend configuration not exist"
        );
        "Frontend config missing"
    })?;

    let addr = format!("0.0.0.0:{}", frontend.listen_port);
    let service = Arc::new(AppForwardService::new(app_state.clone()));

    let mut gateway = Gateway::new(&addr)
        .add_middleware(Box::new(LoggerMiddleware))
        .add_middleware(Box::new(UserAgentFilterMiddleware::new(app_state.clone())))
        .add_middleware(Box::new(CorsMiddleware))
        .add_middleware(Box::new(OptionsMiddleware))
        .add_middleware(Box::new(ForwardMiddleware::new(service)));

    gateway.set_handler(default_handler());
    gateway.listen().await?;

    Ok(())
}

async fn setup_backend_gateway(
    app_state: &Arc<AppState>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let config = app_state.get_config().await.clone();
    let mode = config.general.stream_mode;

    if !(mode == StreamMode::Backend || mode == StreamMode::Dual) {
        return Ok(());
    }

    debug_log!(INIT_LOGGER_DOMAIN, "Successfully start backend listener");

    let backend = config.backend.as_ref().ok_or_else(|| {
        error_log!(INIT_LOGGER_DOMAIN, "Error: Backend configuration not exist");
        "Backend config missing"
    })?;

    let addr = format!("0.0.0.0:{}", backend.listen_port);
    let service = Arc::new(AppStreamService::new(app_state.clone()));

    let mut gateway = Gateway::new(&addr)
        .add_middleware(Box::new(LoggerMiddleware))
        .add_middleware(Box::new(UserAgentFilterMiddleware::new(app_state.clone())))
        .add_middleware(Box::new(CorsMiddleware))
        .add_middleware(Box::new(OptionsMiddleware))
        .add_middleware(Box::new(StreamMiddleware::new(&backend.path, service)));

    gateway.set_handler(default_handler());
    gateway.listen().await?;

    Ok(())
}

fn default_handler() -> Handler {
    Arc::new(|_ctx: Context| {
        debug_log!(GATEWAY_LOGGER_DOMAIN, "Starting default middleware...");
        ResponseBuilder::with_status_code(StatusCode::SERVICE_UNAVAILABLE)
    })
}
