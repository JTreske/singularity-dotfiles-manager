use clap::{Args, Parser};
pub use commands::Commands;
use std::path::PathBuf;

mod commands;

#[derive(Args, Debug)]
pub struct GlobalOptions {
  #[arg(long, short = 'l', default_value = "INFO", global = true)]
  /// Log level used during execution (ERROR, WARN, INFO, DEBUG, TRACE)
  pub log_level: tracing::Level,
  #[arg(long, global = true)]
  /// Enables logging to the given file path
  pub log_file: Option<PathBuf>,
}

#[derive(Parser, Debug)]
#[command(name = "singularity-dotfiles-manager", author, version, about)]
pub struct Cli {
  #[command(flatten)]
  pub global: GlobalOptions,

  #[command(subcommand)]
  pub command: Commands,
}
