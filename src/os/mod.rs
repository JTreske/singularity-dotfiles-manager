use crate::config::dotfiles_config::Options;
use crate::util::log;
use anyhow::{Result, anyhow};
pub use command_runner::CommandRunner;
use os_release::OsRelease;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

mod arch;
mod command_runner;

#[derive(Debug, Clone, Copy)]
pub struct Permissions {
  pub run_as_root: bool,
}

pub trait Runner {
  fn install_package(&self, package: &str) -> Result<()>;

  fn install_package_list(&self, packages: &[&str]) -> Result<()>;

  fn is_installed(&self, package: &str) -> bool;

  fn install_from_dependency_dir(&self, dependency_dir: &Path) -> Result<()>;

  // Can be used for the hook script or dependency file path
  fn os_short_name(&self) -> &str;

  fn run_command(
    &self,
    program: &str,
    args: &[&str],
    interactive: bool,
    request_password_injection: bool,
    cwd_opt: Option<&Path>,
    env: &HashMap<&str, &str>,
  ) -> Result<()>;

  fn clone_git_repository(
    &self,
    url: &url::Url,
    tag_opt: Option<&str>,
    target: &Path,
  ) -> Result<()>;
}

pub fn create_runner(
  permissions: Permissions,
  options: Options,
  command_runner: Arc<CommandRunner>,
) -> Result<Arc<dyn Runner>> {
  log::step("Creating OS runner...");
  let release = OsRelease::new()?;

  let match_key = if !release.id_like.is_empty() {
    release.id_like.as_str()
  } else {
    release.id.as_str()
  };

  match match_key {
    "arch" => Ok(Arc::new(arch::Arch::new(
      permissions,
      options,
      command_runner,
    )?)),
    _ => Err(anyhow!(
      "Unsupported distribution: {} ({})",
      release.pretty_name,
      release.id_like
    )),
  }
}
