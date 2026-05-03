use crate::config::dotfiles_config::Options;
use crate::config::{AppConfigPath, AppPackage, AppSettings, Apps};
use crate::os::Runner;
use crate::{os, util};
use anyhow::{Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::debug;

fn install_config(profile_dir: &Path, config_path: &AppConfigPath) -> Result<()> {
  let src = profile_dir.join(&config_path.src);
  let dst = util::path::resolve_home(&config_path.dst)?;

  if !src.exists() {
    return Err(anyhow!(
      "App config source path `{}` not found",
      src.display()
    ));
  }

  if dst.exists() {
    debug!(
      "App config destination path `{}` already exists",
      dst.display()
    );
    let choice = util::confirm(
      format!(
        "App config destination path `{}` already exists. Do you want to overwrite?",
        dst.display()
      ),
      false,
    )?;

    if !choice {
      return Err(anyhow!("Aborted"));
    }

    fs::remove_dir_all(&dst)?;
  }

  util::path::ensure_dir(&dst)?;
  util::path::copy_recursive(&src, &dst)
}

fn process_package(
  runner: &Arc<dyn Runner>,
  profile_dir: Option<&Path>,
  package: &AppPackage,
) -> Result<()> {
  util::log::step(format!("Setting up `{}`...", package.package_name));

  if let Some(pd) = profile_dir {
    for config in &package.config_path {
      if let Err(e) = install_config(pd, config) {
        util::log::error(format!("Failed to install config: {e}"));
        let choice = util::confirm("Do you wish to install the application anyway?", true)?;

        if !choice {
          util::log::info(format!(
            "Installation of `{}` aborted",
            package.package_name
          ));
          return Ok(());
        }
      }
    }
  }

  let mut packages = vec![package.package_name.as_str()];
  for p in &package.dependencies {
    packages.push(p.as_str());
  }
  runner.install_package_list(&packages)?;

  // TODO: run post_install_script

  util::log::success(format!("App `{}` set up!", package.package_name));

  Ok(())
}

fn selector(apps: Apps, all: bool, runner: &Arc<dyn Runner>) -> Result<Vec<AppPackage>> {
  let mut menu_items = Vec::new();
  let mut initial_values = Vec::new();

  for (category, mut packages) in apps.categories {
    let mut help_items = Vec::new();
    help_items.push((None, format!("── {category} ──"), "".to_string()));

    packages.sort_by(|a, b| a.subcategory.cmp(&b.subcategory));

    for pkg in packages {
      if !all && runner.is_installed(&pkg.package_name) {
        continue;
      }

      let label = if pkg.subcategory.is_empty() {
        pkg.package_name.to_string()
      } else {
        format!("{} ({})", pkg.package_name, pkg.subcategory)
      };

      if pkg.preselected && !all {
        initial_values.push(Some(pkg.clone()));
      }

      help_items.push((Some(pkg.clone()), label, pkg.description.clone()));
    }

    if help_items.len() > 1 {
      menu_items.append(&mut help_items);
    }
  }

  let selection = util::multi_select(
    "Select the packages you want to install (SPACE to select, ENTER to confirm)",
    initial_values,
    &menu_items,
  )?
  .into_iter()
  .flatten()
  .collect();

  Ok(selection)
}

pub fn apps(
  app_settings: &mut AppSettings,
  all: bool,
  app_list_opt: &Option<PathBuf>,
) -> Result<()> {
  let run_as_root = util::confirm(
    "Do you wish to allow all commands that need root privileges to run? (If not you will be asked for each command)",
    false,
  )?;
  let permissions = os::Permissions { run_as_root };

  let password = util::password("Enter your password for later use")?;

  let command_runner = Arc::new(os::CommandRunner::new(password));
  let options = if let Some(config) = &app_settings.active_config {
    config.options
  } else {
    Options::default()
  };
  let runner = os::create_runner(permissions, options, command_runner.clone())?;

  let mut profile_dir = None;
  let apps = if let Some(app_list) = app_list_opt {
    let apps_json = util::path::read_file_to_string(app_list)?;
    serde_json::from_str(&apps_json)?
  } else {
    let active_config = match &app_settings.active_config {
      Some(c) => c,
      None => {
        util::log::info("No active config found");
        return Ok(());
      }
    };

    let profile_dir_help = app_settings.active_install_dir.join(&active_config.id);
    profile_dir = Some(profile_dir_help.clone());

    let apps_path = match &active_config.apps {
      Some(a) => profile_dir_help.join(a),
      None => {
        util::log::info("The active config does not specify an app list");
        return Ok(());
      }
    };

    if !apps_path.is_dir() {
      return Err(anyhow!("App path `{}` does not exist", apps_path.display()));
    }

    let apps_json_path = apps_path.join(format!("{}.json", runner.os_short_name()));
    if !apps_json_path.is_file() {
      return Err(anyhow!(
        "No app list found for `{}`",
        runner.os_short_name()
      ));
    }

    let apps_json = util::path::read_file_to_string(apps_json_path)?;
    serde_json::from_str(&apps_json)?
  };

  let selections = selector(apps, all, &runner)?;

  for item in &selections {
    if let Err(e) = process_package(&runner, profile_dir.as_deref(), item) {
      util::log::error(format!("Setup of `{}` failed: {e}", item.package_name));
      let choice = util::confirm("Do you wish to continue?", true)?;
      if !choice {
        util::log::info("Aborted");
        return Ok(());
      }
    }
  }

  Ok(())
}
