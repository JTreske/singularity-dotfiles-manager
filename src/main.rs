use clap::Parser;
use singularity_dotfiles_manager::{cli, config, util};
use std::fs;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Layer, fmt, registry};

fn main() {
  let cli = cli::Cli::parse();

  if let Some(log_file) = cli.global.log_file {
    let log_file = util::path::resolve_home(log_file).unwrap();
    let layer = fmt::layer()
      .with_ansi(false)
      .with_writer(
        fs::OpenOptions::new()
          .create(true)
          .append(true)
          .open(log_file)
          .unwrap(),
      )
      .with_filter(LevelFilter::from_level(cli.global.log_level));
    registry().with(layer).init();
  }

  let mut app_settings = config::AppSettings::from_filesystem();

  let mut fail = false;
  if let Err(e) = cli.command.run(&mut app_settings) {
    util::log::error(e.to_string());
    fail = true;
  }

  if let Err(e) = app_settings.to_filesystem() {
    util::log::error(e.to_string());
    fail = true;
  }

  if fail {
    std::process::exit(1);
  }
}
