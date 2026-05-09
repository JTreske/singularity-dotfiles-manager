use crate::config::DotfilesReleaseConfig;
use crate::util;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const APP_SETTINGS_PATH: &str = "~/.local/share/singularity-dotfiles-manager/settings.json";

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AppSettings {
  #[serde(default)]
  pub active_config: Option<DotfilesReleaseConfig>,
  #[serde(default)]
  pub active_install_dir: PathBuf,
}

impl AppSettings {
  pub fn from_filesystem() -> AppSettings {
    util::path::resolve_home(Path::new(APP_SETTINGS_PATH))
      .and_then(util::path::read_file_to_string)
      .and_then(|s| serde_json::from_str(&s).context("Failed to serialize app settings"))
      .unwrap_or_default()
  }

  pub fn to_filesystem(&self) -> Result<()> {
    let settings_path = util::path::resolve_home(Path::new(APP_SETTINGS_PATH))?;
    util::path::create_parents(&settings_path)?;
    util::path::write_str_to_file(
      serde_json::to_string(&self).with_context(|| "Failed to serialize app settings")?,
      settings_path,
      false,
    )?;

    Ok(())
  }
}
