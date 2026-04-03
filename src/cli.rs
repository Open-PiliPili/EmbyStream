use std::path::PathBuf;

use clap::{
    Parser, Subcommand, ValueEnum,
    builder::Styles,
    builder::styling::{AnsiColor, Effects},
};

/// Clap 4: `Styles::default()` is plain (no ANSI). Use `styled()` + accents so `--help` is not all gray.
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
    /// UI language: en (default) or zh (Simplified Chinese); affects config wizard and --help.
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
    /// Start HTTP gateways (default when no subcommand: use `run` explicitly).
    Run(RunArgs),
    /// OAuth helper commands for external providers.
    Auth(AuthArgs),
    /// Interactive TOML configuration wizard (prompt language follows `--lang`).
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
    /// Do not try to open a browser automatically; print the authorization URL only.
    #[arg(long = "no-browser")]
    pub no_browser: bool,
}

#[derive(Parser, Debug)]
pub struct ConfigArgs {
    /// List valid configs in the current directory and print one (masked secrets).
    #[command(subcommand)]
    pub sub: Option<ConfigSubcommand>,
}

#[derive(Subcommand, Debug, Clone, Copy)]
pub enum ConfigSubcommand {
    /// List valid TOML configs here and print one (mask secrets unless you confirm).
    Show,
    /// Interactive: pick stream_mode and write a starter TOML (via temp file, then atomically).
    Template,
}

#[derive(Parser, Debug)]
pub struct RunArgs {
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    #[arg(long, value_name = "FILE")]
    pub ssl_cert_file: Option<PathBuf>,

    #[arg(long, value_name = "FILE")]
    pub ssl_key_file: Option<PathBuf>,
}
