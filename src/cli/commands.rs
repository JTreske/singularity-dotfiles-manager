use crate::apps::apps;
use crate::config::{AppSettings, ReleaseConfigSource};
use crate::install;
use crate::util::path::resolve_home;
use anyhow::Result;
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum Commands {
  /// Installs the dotfiles repository specified by the `RELEASE_CONFIG`.
  Install {
    #[arg(value_name = "RELEASE_CONFIG")]
    /// URL or local path to the release config JSON file
    release_config: ReleaseConfigSource,
    #[arg(long, short = 'o', default_value = "~/.mydotfiles")]
    install_dir: PathBuf,
  },
  /// Installs apps from a curated list.
  Apps {
    /// If set all apps from the app list will be shown even those already installed
    #[arg(long)]
    all: bool,
    /// If not specified uses the app list from the active profile
    #[arg(long, short = 'a', default_value = None)]
    app_list: Option<PathBuf>,
  },
}

impl Commands {
  pub fn run(&self, app_settings: &mut AppSettings) -> Result<()> {
    match self {
      Commands::Install {
        release_config,
        install_dir,
      } => {
        let install_dir = resolve_home(install_dir)?;
        install(app_settings, release_config, &install_dir)
      }
      Commands::Apps { all, app_list } => apps(app_settings, *all, app_list),
    }
  }
}
