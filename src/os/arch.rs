use crate::config::dotfiles_config::Options;
use crate::os::command_runner::CommandRunner;
use crate::os::{Permissions, Runner};
use crate::util::{self, log};
use anyhow::{Context, Result, anyhow};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tracing::debug;
use url::Url;
use which::which;

pub struct Arch {
  aur_helper: Option<String>,
  permissions: Permissions,
  command_runner: Arc<CommandRunner>,
}

impl Arch {
  fn install_paru(&self) -> Result<()> {
    if which("rustc").is_err() {
      debug!("Rust is not installed. Installing rustup...");
      self.install_package("rustup")?;
      debug!("Setting up Rust toolchain...");
      self.command_runner.run_command(
        "rustup",
        &["default", "stable"],
        &Permissions { run_as_root: false },
        false,
        false,
        None,
        &HashMap::new(),
      )?;
    }

    let tmp_dir = tempfile::tempdir()?;

    self.command_runner.git_clone(
      &Url::parse("https://aur.archlinux.org/paru.git")?,
      &None,
      tmp_dir.path(),
    )?;

    debug!("Building paru package...");
    self.command_runner.run_command(
      "makepkg",
      &["-si", "--noconfirm"],
      &self.permissions,
      false,
      true,
      Some(tmp_dir.path()),
      &HashMap::new(),
    )?;

    log::success("Paru installed!");
    Ok(())
  }

  fn install_yay(&self) -> Result<()> {
    let tmp_dir = tempfile::tempdir()?;

    self.command_runner.git_clone(
      &Url::parse("https://aur.archlinux.org/yay-bin.git")?,
      &None,
      tmp_dir.path(),
    )?;

    debug!("Building yay package...");
    self.command_runner.run_command(
      "makepkg",
      &["-si", "--noconfirm"],
      &self.permissions,
      false,
      true,
      Some(tmp_dir.path()),
      &HashMap::new(),
    )?;

    log::success("Yay installed!");
    Ok(())
  }

  fn install_aur(&mut self) -> Result<()> {
    let supported_helpers = vec!["paru", "yay"];
    log::step("Setting up AUR helper...");
    log::info(format!("Supported AUR helpers: {:?}", supported_helpers));

    for helper in supported_helpers {
      debug!("AUR helper check: {helper}");
      if which(helper).is_ok() {
        log::success(format!("AUR helper detected: {helper}"));
        self.aur_helper = Some(helper.to_string());
        return Ok(());
      }
    }
    log::info("No AUR helper detected.");

    let selected_helper = util::select(
      "Select your preferred AUR helper",
      "paru",
      &[("paru", "paru", "recommended"), ("yay", "yay", "")],
    )?;
    debug!("Selected AUR helper: {selected_helper}");

    log::step(format!("Installing selected AUR helper: {selected_helper}"));
    match selected_helper {
      "paru" => self.install_paru()?,
      "yay" => self.install_yay()?,
      _ => {
        return Err(anyhow!("Unsupported AUR helper: {selected_helper}"));
      }
    }

    self.aur_helper = Some(selected_helper.to_string());

    Ok(())
  }

  fn install_deps(&self, git_lfs: bool) -> Result<()> {
    log::step("Installing dotfiles manager dependencies...");
    self.install_package("base-devel")?;
    self.install_package("git")?;
    self.install_package("make")?;

    if git_lfs {
      self.install_package("git-lfs")?;

      log::step("Setting up Git LFS...");
      self.command_runner.run_command(
        "git",
        &["lfs", "install"],
        &Permissions { run_as_root: false },
        false,
        false,
        None,
        &HashMap::new(),
      )?
    }

    Ok(())
  }

  pub fn new(
    permissions: Permissions,
    options: Options,
    command_runner: Arc<CommandRunner>,
  ) -> Result<Arch> {
    let mut arch = Self {
      aur_helper: None,
      permissions,
      command_runner,
    };
    if options.request_aur {
      arch.install_aur()?;
    }
    arch.install_deps(options.git_lfs)?;
    Ok(arch)
  }
}

impl Runner for Arch {
  fn install_package(&self, package: &str) -> Result<()> {
    let mut args = Vec::new();
    let program = match self.aur_helper.as_deref() {
      Some(helper) => helper,
      None => {
        args.push("pacman");
        "sudo"
      }
    };

    log::step(format!("Installing package `{package}`..."));

    args.extend_from_slice(&["-S", "--needed", "--noconfirm", "--quiet", package]);
    self.command_runner.run_command(
      program,
      &args,
      &self.permissions,
      false,
      true,
      None,
      &HashMap::new(),
    )?;

    log::success(format!("Package `{package}` installed!"));

    Ok(())
  }

  fn install_package_list(&self, packages: &[&str]) -> Result<()> {
    let mut args = Vec::new();
    let program = match self.aur_helper.as_deref() {
      Some(helper) => helper,
      None => {
        args.push("pacman");
        "sudo"
      }
    };

    log::step(format!("Installing packages `{:?}`...", packages));
    args.extend_from_slice(&["-S", "--needed", "--noconfirm", "--quiet"]);
    args.extend_from_slice(packages);
    self.command_runner.run_command(
      program,
      &args,
      &self.permissions,
      false,
      true,
      None,
      &HashMap::new(),
    )?;

    log::success(format!("Package `{:?}` installed!", packages));

    Ok(())
  }

  fn is_installed(&self, package: &str) -> bool {
    let program = self.aur_helper.as_deref().unwrap_or("pacman");

    let args = vec!["-Qq", package];

    self
      .command_runner
      .run_command(
        program,
        &args,
        &self.permissions,
        false,
        false,
        None,
        &HashMap::new(),
      )
      .is_ok()
  }

  fn install_from_dependency_dir(&self, dependency_dir: &Path) -> Result<()> {
    log::step("Installing dotfile dependencies...");

    let dependency_file = dependency_dir.join("arch");
    if !dependency_file.is_file() {
      return Err(anyhow!("Missing dependency file `arch`"));
    }

    let content =
      fs::read_to_string(dependency_file).context("Failed to read `arch` dependency file")?;

    for line in content.lines() {
      self.install_package(line)?;
    }

    log::success("Dependencies installed!");

    Ok(())
  }

  fn os_short_name(&self) -> &str {
    "arch"
  }

  fn run_command(
    &self,
    program: &str,
    args: &[&str],
    interactive: bool,
    request_password_injection: bool,
    cwd_opt: Option<&Path>,
    env: &HashMap<&str, &str>,
  ) -> Result<()> {
    self.command_runner.run_command(
      program,
      args,
      &self.permissions,
      interactive,
      request_password_injection,
      cwd_opt,
      env,
    )
  }

  fn clone_git_repository(
    &self,
    url: &url::Url,
    tag_opt: Option<&str>,
    target: &Path,
  ) -> Result<()> {
    self.command_runner.git_clone(url, &tag_opt, target)
  }
}
