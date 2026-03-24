use std::path::PathBuf;

use clap::{
    Parser, Subcommand,
    builder::Styles,
    builder::styling::{AnsiColor, Effects},
};

/// Clap 4: `Styles::default()` is plain (no ANSI). Use `styled()` + accents so `--help` is not all gray.
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
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start HTTP gateways (default when no subcommand: use `run` explicitly).
    Run(RunArgs),
    /// Interactive TOML configuration wizard (English prompts).
    Config(ConfigArgs),
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
