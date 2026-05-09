use clap::{Args, Parser};
pub use commands::Commands;
use std::path::PathBuf;

mod commands;

#[derive(Args, Debug)]
pub struct GlobalOptions {
  /// Disables the profile sync where all dotfiles from the active profile are updated with the actual files from $HOME
  #[arg(long, global = true)]
  pub no_profile_sync: bool,
  /// Log level used during execution (ERROR, WARN, INFO, DEBUG, TRACE)
  #[arg(long, short = 'l', default_value = "INFO", global = true)]
  pub log_level: tracing::Level,
  /// Enables logging to the given file path
  #[arg(long, global = true)]
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
