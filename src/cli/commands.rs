use crate::apps::apps;
use crate::config::{AppSettings, ReleaseConfigSource};
use crate::install;
use crate::restore::restore;
use crate::util::path::resolve_home;
use anyhow::Result;
use chrono::{DateTime, NaiveDateTime, Utc};
use clap::Subcommand;
use std::path::PathBuf;

fn parse_timestamp(s: &str) -> Result<DateTime<Utc>, String> {
  Ok(
    NaiveDateTime::parse_from_str(s, "%Y%m%d%H%M%S")
      .map_err(|e| e.to_string())?
      .and_utc(),
  )
}

#[derive(Subcommand, Debug)]
pub enum Commands {
  /// Installs the dotfiles repository specified by the `RELEASE_CONFIG`.
  Install {
    /// URL or local path to the release config JSON file
    #[arg(value_name = "RELEASE_CONFIG")]
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
  /// Restores the profile from a backup.
  Restore {
    /// If set the current profile of the selected dotfiles is backed up
    #[arg(long)]
    backup: bool,
    /// If set the selected dotfiles are activated after restore
    #[arg(long)]
    activate: bool,
    #[arg(long, short = 'o', default_value = "~/.mydotfiles")]
    install_dir: PathBuf,
    /// The ID of dotfiles to restore
    #[arg(long, default_value = None)]
    id: Option<String>,
    /// The timestamp of the backup to restore in the format `YYYYmmddHHMMSS`
    #[arg(long, value_parser=parse_timestamp, default_value = None)]
    timestamp: Option<DateTime<Utc>>,
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
      Commands::Restore {
        backup,
        activate,
        install_dir,
        id,
        timestamp,
      } => {
        let install_dir = resolve_home(install_dir)?;
        restore(
          app_settings,
          &install_dir,
          *backup,
          *activate,
          id,
          timestamp,
        )
      }
    }
  }
}
