use std::path::PathBuf;

use clap::{
    Parser, Subcommand, ValueEnum,
    builder::Styles,
    builder::styling::{AnsiColor, Effects},
};

/// Clap 4: `Styles::default()` is plain (no ANSI).
/// Use `styled()` + accents so `--help` is not all gray.
/// UI language for config wizard and localized `--help` (`--lang zh`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum UiLang {
    #[default]
    En,
    Zh,
}

impl std::fmt::Display for UiLang {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UiLang::En => write!(f, "en"),
            UiLang::Zh => write!(f, "zh"),
        }
    }
}

fn embystream_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Yellow.on_default() | Effects::BOLD)
        .usage(AnsiColor::Green.on_default() | Effects::BOLD)
        .literal(AnsiColor::Cyan.on_default())
        .placeholder(AnsiColor::Green.on_default())
}

#[derive(Parser, Debug)]
#[command(
    name = "embystream",
    author,
    version,
    about = "Emby streaming proxy: run frontend, backend, or dual gateways from TOML config.",
    long_about = None
)]
#[command(propagate_version = true)]
#[command(styles = embystream_styles())]
#[command(color = clap::ColorChoice::Auto)]
pub struct Cli {
    /// UI language: en (default) or zh (Simplified Chinese).
    /// Affects config wizard and `--help`.
    #[arg(
        long = "lang",
        global = true,
        default_value_t = UiLang::En,
        value_name = "LANG"
    )]
    pub lang: UiLang,
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start HTTP gateways.
    /// When there is no subcommand, use `run` explicitly.
    Run(RunArgs),
    /// Start or administrate the web configuration studio.
    Web(WebArgs),
    /// OAuth helper commands for external providers.
    Auth(AuthArgs),
    /// Interactive TOML configuration wizard.
    /// Prompt language follows `--lang`.
    Config(ConfigArgs),
}

#[derive(Parser, Debug)]
pub struct AuthArgs {
    #[command(subcommand)]
    pub sub: AuthSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum AuthSubcommand {
    /// Start Google OAuth installed-flow authentication.
    Google(GoogleAuthCliArgs),
}

#[derive(Parser, Debug, Clone)]
pub struct GoogleAuthCliArgs {
    /// Google OAuth client ID.
    #[arg(long = "client-id", value_name = "CLIENT_ID")]
    pub client_id: String,
    /// Google OAuth client secret.
    #[arg(long = "secret", value_name = "CLIENT_SECRET")]
    pub client_secret: String,
    /// Do not try to open a browser automatically.
    /// Print the authorization URL only.
    #[arg(long = "no-browser")]
    pub no_browser: bool,
}

#[derive(Parser, Debug)]
pub struct ConfigArgs {
    /// List valid configs in the current directory.
    /// Print one with masked secrets.
    #[command(subcommand)]
    pub sub: Option<ConfigSubcommand>,
}

#[derive(Subcommand, Debug, Clone, Copy)]
pub enum ConfigSubcommand {
    /// List valid TOML configs here and print one.
    /// Mask secrets unless you confirm.
    Show,
    /// Interactive: pick `stream_mode` and write a starter TOML.
    /// Uses a temporary file and atomic rename.
    Template,
}

#[derive(Parser, Debug)]
pub struct RunArgs {
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Also start the web configuration studio alongside `run`.
    #[arg(long)]
    pub web: bool,

    /// Listen address for the web service when `--web` is enabled.
    #[arg(long, value_name = "ADDR", default_value = "0.0.0.0:6888")]
    pub web_listen: String,

    /// Data directory for SQLite, sessions, artifacts, and audit logs when `--web` is enabled.
    #[arg(long, value_name = "DIR", default_value = "web-config/data")]
    pub web_data_dir: PathBuf,

    /// TMDB API key for trending login backgrounds when `--web` is enabled.
    #[arg(long, value_name = "KEY")]
    pub web_tmdb_api_key: Option<String>,

    /// Runtime log directory for the admin log browser when `--web` is enabled.
    #[arg(long, value_name = "DIR", default_value = "web-config/logs")]
    pub web_runtime_log_dir: PathBuf,

    #[arg(long, value_name = "FILE")]
    pub ssl_cert_file: Option<PathBuf>,

    #[arg(long, value_name = "FILE")]
    pub ssl_key_file: Option<PathBuf>,
}

#[derive(Parser, Debug)]
pub struct WebArgs {
    #[command(subcommand)]
    pub sub: WebSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum WebSubcommand {
    /// Start the web configuration studio service.
    Serve(WebServeArgs),
    /// Administrative operations for the web configuration studio.
    Admin(WebAdminArgs),
}

#[derive(Parser, Debug, Clone)]
pub struct WebServeArgs {
    /// Listen address for the web service.
    #[arg(long, value_name = "ADDR", default_value = "0.0.0.0:6888")]
    pub listen: String,

    /// Data directory for SQLite, sessions, artifacts, and audit logs.
    #[arg(long, value_name = "DIR", default_value = "web-config/data")]
    pub data_dir: PathBuf,

    /// TMDB API key for trending login backgrounds.
    #[arg(long, value_name = "KEY")]
    pub tmdb_api_key: Option<String>,

    /// Runtime log directory for the admin log browser.
    #[arg(long, value_name = "DIR", default_value = "web-config/logs")]
    pub runtime_log_dir: PathBuf,

    /// Embystream stream log directory shown in the admin log browser.
    #[arg(long, value_name = "DIR", default_value = "./logs")]
    pub stream_log_dir: PathBuf,
}

#[derive(Parser, Debug)]
pub struct WebAdminArgs {
    #[command(subcommand)]
    pub sub: WebAdminSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum WebAdminSubcommand {
    /// Reset an administrator password and print the new random password.
    ResetPassword(WebAdminResetPasswordArgs),
}

#[derive(Parser, Debug, Clone)]
pub struct WebAdminResetPasswordArgs {
    /// Administrator username to reset.
    #[arg(long, value_name = "NAME", default_value = "admin")]
    pub username: String,

    /// Data directory for SQLite, sessions, artifacts, and audit logs.
    #[arg(long, value_name = "DIR", default_value = "web_data")]
    pub data_dir: PathBuf,
}
