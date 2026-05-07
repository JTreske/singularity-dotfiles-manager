use crate::util::log;
use anyhow::{Context, anyhow};
use reqwest::blocking;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use url::Url;

#[derive(Debug, Clone)]
pub enum ReleaseConfigSource {
  Remote(Url),
  Local(PathBuf),
}

impl FromStr for ReleaseConfigSource {
  type Err = anyhow::Error;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    if s.starts_with("http://") || s.starts_with("https://") {
      return Url::parse(s)
        .map(ReleaseConfigSource::Remote)
        .map_err(|e| anyhow!("Failed to parse release config url: {e}"));
    }

    let path = PathBuf::from(s);
    if path.is_file() {
      Ok(ReleaseConfigSource::Local(path))
    } else {
      Err(anyhow!("File not found: {s}"))
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RestoreItem {
  /// Name of the item shown to the user.
  pub name: String,
  /// Additional information for the user.
  #[serde(default)]
  pub information: Option<String>,
  /// Relative path from repository root.
  /// May be a single file or a complete folder.
  pub path: String,
  /// Defines if the restore item should be selected by default.
  /// The user selection from the last update takes precedence.
  #[serde(default)]
  pub selected: bool,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub struct Options {
  #[serde(default)]
  pub request_aur: bool,
  #[serde(default)]
  pub git_lfs: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DotfilesReleaseConfig {
  /// Name of the dotfiles release
  pub name: String,
  /// ID of the dotfiles release
  pub id: String,
  #[serde(default)]
  pub description: Option<String>,
  /// If specified the tagged version of the dotfiles repository will be installed
  #[serde(default)]
  pub tag: Option<String>,
  /// Name of the author of the dotfiles repository
  #[serde(default)]
  pub author: Option<String>,
  /// Link to the website where the user can find more information about the dotfiles
  #[serde(default)]
  pub website: Option<String>,
  /// Dotfiles git repository HTTPS link.
  pub repository: String,
  /// Relative path to the folder containing the dotfiles that should be installed.
  pub dotfiles: String,
  /// Relative path to the folder containing the RHAI hook scripts.
  #[serde(default)]
  pub hooks: Option<String>,
  /// Relative path to the folder containing the OS based package dependencies.
  #[serde(default)]
  pub dependencies: Option<String>,
  /// Relative path to the folder containing the OS based curated app lists
  pub apps: Option<String>,
  #[serde(default)]
  pub options: Options,
  /// List of restore options the user can choose
  #[serde(default)]
  pub restore: Vec<RestoreItem>,
}

impl DotfilesReleaseConfig {
  pub fn present(&self, update: bool, restore: bool) {
    log::info(format!(
      "PROFILE INFORMATION\n\
       Status:        {}\n\
       Name:          {}\n\
       Description:   {}\n\
       ID:            {}\n\
       Version:       {}\n\
       Author:        {}\n\
       Homepage:      {}\n\
       Source:        {}\n\
       Subfolder:     {}",
      if update {
        "Update of existing profile"
      } else if restore {
        "Restore profile"
      } else {
        "Install new profile"
      },
      &self.name,
      self.description.as_deref().unwrap_or(""),
      &self.id,
      self.tag.as_deref().unwrap_or("default branch"),
      self.author.as_deref().unwrap_or(""),
      self.website.as_deref().unwrap_or(""),
      self.repository,
      &self.dotfiles,
    ));
  }
}

impl TryFrom<&ReleaseConfigSource> for DotfilesReleaseConfig {
  type Error = anyhow::Error;

  fn try_from(value: &ReleaseConfigSource) -> Result<Self, Self::Error> {
    let bytes = match value {
      ReleaseConfigSource::Remote(url) => {
        log::step(format!("Fetching release config from `{url}`..."));
        blocking::Client::new()
          .get(url.clone())
          .send()
          .context("HTTP request failed")?
          .bytes()
          .context("Failed to read HTTP body")?
          .to_vec()
      }
      ReleaseConfigSource::Local(path_buf) => {
        log::step(format!(
          "Fetching release config from filesystem `{}`...",
          path_buf.display(),
        ));
        fs::read(path_buf).context("Failed to read local file")?
      }
    };

    serde_json::from_slice(&bytes).context("Release config JSON deserialization failed")
  }
}
