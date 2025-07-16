use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Run(RunArgs),
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