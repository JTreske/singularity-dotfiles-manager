use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AppConfigPath {
  /// Optional path to the configuration relative to the repository root
  pub src: PathBuf,
  /// Absolute path to the destination directory (~ may be used for $HOME)
  pub dst: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AppPackage {
  pub subcategory: String,
  /// The exact package name for the package manager
  pub package_name: String,
  pub description: String,
  /// Optional definition of an application configuration directory.
  ///
  /// If set the dotfiles manager will copy the directory from the profile into the $HOME directory.
  /// Only possible if part of a dotfiles profile.
  #[serde(default)]
  pub config_path: Vec<AppConfigPath>,
  /// Packages that are installed with this package
  #[serde(default)]
  pub dependencies: Vec<String>,
  /// If set to true this item will be preselected
  #[serde(default)]
  pub preselected: bool,
  /// Non-interactive bash script called after the installation of this item
  pub post_install_script: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
/// Curated list of apps ordered by category
pub struct Apps {
  #[serde(flatten)]
  pub categories: BTreeMap<String, Vec<AppPackage>>,
}
