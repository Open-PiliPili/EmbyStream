use std::{error::Error, fs, path::Path, process, str::FromStr, sync::Arc};

use clap::Parser;
use figlet_rs::FIGfont;
use hyper::{StatusCode, body::Incoming};
use tokio::signal as TokioSignal;

use embystream::gateway::reverse_proxy_filter::ReverseProxyFilterMiddleware;
use embystream::{
    AppState, GATEWAY_LOGGER_DOMAIN, INIT_LOGGER_DOMAIN, debug_log, error_log,
    info_log,
};
use embystream::{
    backend::{service::AppStreamService, stream::StreamMiddleware},
    cli::{Cli, Commands, RunArgs},
    config::{core::Config, general::StreamMode},
    frontend::{forward::ForwardMiddleware, service::AppForwardService},
    gateway::{
        CorsMiddleware, LoggerMiddleware, OptionsMiddleware, chain::Handler,
        client_filter::ClientAgentFilterMiddleware, context::Context,
        core::Gateway, response::ResponseBuilder,
    },
    logger::{LogLevel, Logger},
    system::SystemInfo,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Run(run_args)) => {
            run_app(&run_args).await?;
        }
        None => {}
    }
    Ok(())
}

async fn run_app(
    run_args: &RunArgs,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    setup_figlet();

    let config = setup_load_config(run_args);
    setup_logger(&config)?;
    setup_print_info(&config);

    if let Err(e) = validate_dual_mode_ports(&config) {
        error_log!(INIT_LOGGER_DOMAIN, "{}", e);
        process::exit(1);
    }

    setup_crypto_provider()?;

    let app_state = setup_cache(&config).await;

    setup_rate_limiters(&app_state).await;

    let mode = {
        let config_guard = app_state.get_config().await;
        config_guard.general.stream_mode.clone()
    };

    if matches!(mode, StreamMode::Frontend | StreamMode::Dual) {
        let frontend_state = app_state.clone();
        tokio::spawn(async move {
            if let Err(e) = setup_frontend_gateway(&frontend_state).await {
                error_log!(
                    INIT_LOGGER_DOMAIN,
                    "Frontend gateway failed: {}",
                    e
                );
            }
        });
    }

    if matches!(mode, StreamMode::Backend | StreamMode::Dual) {
        let backend_state = app_state.clone();
        tokio::spawn(async move {
            if let Err(e) = setup_backend_gateway(&backend_state).await {
                error_log!(INIT_LOGGER_DOMAIN, "Backend gateway failed: {}", e);
            }
        });
    }

    TokioSignal::ctrl_c().await?;
    info_log!(INIT_LOGGER_DOMAIN, "Shutting down EmbyStream...");

    Ok(())
}

fn setup_figlet() {
    if let Ok(standard_font) = FIGfont::standard() {
        if let Some(figure) = standard_font.convert("EMBYSTREAM") {
            println!("{figure}");
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
        config.log.level.as_str()
    );
    info_log!(
        INIT_LOGGER_DOMAIN,
        "Memory mode: {}",
        config.general.memory_mode.as_str()
    );
    info_log!(
        INIT_LOGGER_DOMAIN,
        "Stream mode: {}",
        config.general.stream_mode
    );
    info_log!(INIT_LOGGER_DOMAIN, "User agent: {}", config.user_agent)
}

fn setup_load_config(run_args: &RunArgs) -> Config {
    match Config::load_or_init(run_args) {
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
            process::exit(1);
        }
    }
}

fn setup_logger(config: &Config) -> Result<(), Box<dyn Error + Send + Sync>> {
    let log_path = Path::new(&config.log.root_path);
    fs::create_dir_all(log_path)?;

    let level = LogLevel::from_str(&config.log.level).unwrap_or(LogLevel::Info);
    Logger::builder()
        .with_level(level)
        .with_directory(&config.log.root_path)
        .with_file_prefix(&config.log.prefix)
        .build();

    Ok(())
}

async fn setup_cache(config: &Config) -> Arc<AppState> {
    let app_state = AppState::new(config.clone()).await;
    Arc::new(app_state)
}

fn validate_dual_mode_ports(config: &Config) -> Result<(), String> {
    if config.general.stream_mode == StreamMode::Dual {
        if let (Some(frontend), Some(backend)) =
            (&config.frontend, &config.backend)
        {
            if frontend.listen_port == backend.listen_port {
                return Err(format!(
                    "Dual mode port conflict: frontend & backend cannot both use {}.",
                    frontend.listen_port
                ));
            }
        }
    }
    Ok(())
}

fn setup_crypto_provider() -> Result<(), Box<dyn Error + Send + Sync>> {
    Gateway::setup_crypto_provider().map_err(|e| {
        error_log!(INIT_LOGGER_DOMAIN, "Setup crypto-provider failed: {:?}", e);
        e
    })
}

async fn setup_rate_limiters(app_state: &Arc<AppState>) {
    let rate_limiter_cache = app_state.get_rate_limiter_cache().await;
    rate_limiter_cache.start_refill_task();
    info_log!(INIT_LOGGER_DOMAIN, "Rate limiter refill task started.");
}

async fn setup_frontend_gateway(
    app_state: &Arc<AppState>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let config = app_state.get_config().await.clone();
    let mode = config.general.stream_mode;

    if !matches!(mode, StreamMode::Frontend | StreamMode::Dual) {
        debug_log!(
            INIT_LOGGER_DOMAIN,
            "Skipping frontend gateway setup - stream mode not enabled"
        );
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
        .add_middleware(Box::new(ClientAgentFilterMiddleware::new(
            app_state.clone(),
        )))
        .add_middleware(Box::new(ReverseProxyFilterMiddleware::new(
            frontend.clone().anti_reverse_proxy,
        )))
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
    let mode = config.general.clone().stream_mode;

    if !matches!(mode, StreamMode::Backend | StreamMode::Dual) {
        debug_log!(
            INIT_LOGGER_DOMAIN,
            "Skipping backend gateway setup - stream mode not enabled"
        );
        return Ok(());
    }

    debug_log!(INIT_LOGGER_DOMAIN, "Successfully start backend listener");

    let backend = config.backend.as_ref().ok_or_else(|| {
        error_log!(
            INIT_LOGGER_DOMAIN,
            "Error: Backend configuration not exist"
        );
        "Backend config missing"
    })?;

    let addr = format!("0.0.0.0:{}", backend.listen_port);
    let service = Arc::new(AppStreamService::new(app_state.clone()));

    let mut gateway = Gateway::new(&addr)
        .with_tls(config.get_ssl_cert_path(), config.get_ssl_key_path())
        .add_middleware(Box::new(LoggerMiddleware))
        .add_middleware(Box::new(ClientAgentFilterMiddleware::new(
            app_state.clone(),
        )))
        .add_middleware(Box::new(ReverseProxyFilterMiddleware::new(
            backend.clone().anti_reverse_proxy,
        )))
        .add_middleware(Box::new(CorsMiddleware))
        .add_middleware(Box::new(OptionsMiddleware))
        .add_middleware(Box::new(StreamMiddleware::new(
            &backend.path,
            service,
        )));

    gateway.set_handler(default_handler());
    gateway.listen().await?;

    Ok(())
}

fn default_handler() -> Handler {
    Arc::new(|_ctx: Context, _body: Option<Incoming>| {
        debug_log!(GATEWAY_LOGGER_DOMAIN, "Fallback to default middleware...");
        ResponseBuilder::with_status_code(StatusCode::SERVICE_UNAVAILABLE)
    })
}
