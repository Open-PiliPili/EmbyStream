use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// 指定配置文件的路径
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,
}