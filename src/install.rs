use crate::config::dotfiles_config::RestoreItem;
use crate::config::{AppSettings, DotfilesReleaseConfig, ReleaseConfigSource};
use crate::util::Hooks;
use crate::util::hooks::HookInformation;
use crate::{os, util};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use url::Url;

#[derive(Debug, Default, Clone, Copy)]
struct State {
  pub update: bool,
  pub backup: bool,
}

fn probe_old_config(
  state: &mut State,
  old_config_path: PathBuf,
) -> Result<Option<DotfilesReleaseConfig>> {
  if old_config_path.exists() {
    match DotfilesReleaseConfig::try_from(&ReleaseConfigSource::Local(old_config_path)) {
      Ok(old_config) => {
        state.update = true;
        Ok(Some(old_config))
      }
      Err(e) => {
        if util::confirm(
          "Failed to read stored release config. Do you wish to continue? (Backup will be created)",
          true,
        )? {
          return Ok(None);
        }
        Err(e)
      }
    }
  } else {
    Ok(None)
  }
}

fn check_versions(old_tag: &Option<String>, new_tag: &Option<String>) -> Result<()> {
  match (new_tag, old_tag) {
    (Some(new_tag), Some(old_tag)) => {
      let new_ver = semver::Version::parse(new_tag).ok();
      let old_ver = semver::Version::parse(old_tag).ok();

      let log_msg = format!(
        "Current version of the dotfiles `{old_tag}` is newer or equal than the new version `{new_tag}`"
      );
      let ui_msg = format!(
        "A newer or equal version '{old_tag}' of the dotfiles was previously installed. Do you want to continue with installing an older version '{new_tag}'?"
      );
      if let (Some(new_v), Some(old_v)) = (new_ver, old_ver) {
        if old_v >= new_v {
          tracing::warn!(log_msg);
          util::confirm(ui_msg, false)?;
        }
      } else {
        // Fallback: compare raw strings lexicographically.
        if old_tag >= new_tag {
          tracing::warn!(log_msg);
          util::confirm(ui_msg, false)?;
        }
      }
    }
    (None, Some(old_tag)) => {
      tracing::warn!(
        "Version information was found for the previous installation of the dotfiles ({old_tag}) but not for this release."
      );
      util::confirm(
        format!(
          "Version information was found for the previous installation of the dotfiles ({old_tag}) but not for this release. Do you want to continue installing unversioned dotfiles?"
        ),
        false,
      )?;
    }
    _ => {}
  }

  Ok(())
}

pub fn merge_config(
  old_config: &DotfilesReleaseConfig,
  new_config: &DotfilesReleaseConfig,
) -> DotfilesReleaseConfig {
  let mut merged_config = new_config.clone();

  let old_restore_map: std::collections::HashMap<&str, &RestoreItem> = old_config
    .restore
    .iter()
    .map(|item| (item.path.as_str(), item))
    .collect();

  for new_item in merged_config.restore.iter_mut() {
    if let Some(old_item) = old_restore_map.get(new_item.path.as_str()) {
      new_item.selected = old_item.selected;
    }
  }

  merged_config
}

fn select_restore(config: &mut DotfilesReleaseConfig) -> Result<()> {
  let mut initial_values = Vec::new();
  let mut items = Vec::new();
  for item in &config.restore {
    items.push((
      item.clone(),
      item.name.clone(),
      item.information.clone().unwrap_or(item.path.clone()),
    ));
    if item.selected {
      initial_values.push(item.clone());
    }
  }

  let selection = util::multi_select("Select restore paths", initial_values, &items)?;
  let selection_map: std::collections::HashMap<&str, &RestoreItem> = selection
    .iter()
    .map(|item| (item.path.as_str(), item))
    .collect();

  for item in &mut config.restore {
    item.selected = selection_map.contains_key(item.name.as_str());
  }

  Ok(())
}

fn restore(config: &DotfilesReleaseConfig, backup_dir: &Path, profile_dir: &Path) -> Result<()> {
  util::log::step("Restoring selected items...");

  let selected_items: Vec<String> = config
    .restore
    .iter()
    .filter(|item| item.selected)
    .map(|item| item.path.clone())
    .collect();

  util::backup::restore_from_backup(&selected_items, backup_dir, profile_dir)?;

  util::log::success("Restore complete!");

  Ok(())
}

pub fn install(
  app_settings: &mut AppSettings,
  release_config_source: &ReleaseConfigSource,
  install_dir: &Path,
) -> Result<()> {
  let mut state = State::default();

  let run_as_root = util::confirm(
    "Do you wish to allow all commands that need root privileges to run? (If not you will be asked for each command)",
    false,
  )?;
  let permissions = os::Permissions { run_as_root };

  let password = util::password("Enter your password for later use")?;

  let mut new_config: DotfilesReleaseConfig = release_config_source.try_into()?;

  let absolute_install_dir = util::path::ensure_dir(install_dir)?;
  let absolute_profile_dir = util::path::ensure_dir(absolute_install_dir.join(&new_config.id))?;
  let absolute_dotfiles_dir = absolute_profile_dir.join(&new_config.dotfiles);

  if absolute_profile_dir.read_dir()?.next().is_some() {
    state.backup = true;
  }

  let old_config_path = absolute_install_dir.join(format!("{}.json", &new_config.id));
  let old_config_opt = probe_old_config(&mut state, old_config_path.clone())?;

  if state.update
    && let Some(old_config) = &old_config_opt
  {
    check_versions(&old_config.tag, &new_config.tag)?;
    new_config = merge_config(old_config, &new_config);
  }

  new_config.present(state.update, false);
  let c = util::confirm("Do you wish to continue?", true)?;
  if !c {
    util::log::info("Aborting...");
    return Ok(());
  }

  let command_runner = Arc::new(os::CommandRunner::new(password));
  let runner = os::create_runner(permissions, new_config.options, command_runner.clone())?;

  let tmp_clone_dir = tempfile::tempdir()?;
  command_runner.git_clone(
    &Url::parse(&new_config.repository)?,
    &new_config.tag.as_deref(),
    tmp_clone_dir.path(),
  )?;

  let mut hooks_opt = match &new_config.hooks {
    Some(relative_hook_dir) => {
      let hook_path = tmp_clone_dir
        .path()
        .join(relative_hook_dir)
        .join(format!("{}.rhai", runner.os_short_name()));
      if hook_path.is_file() {
        Some(Hooks::new(
          hook_path,
          runner.clone(),
          HookInformation {
            update: state.update,
            backup: state.backup,
            install_dir: absolute_install_dir.to_string_lossy().as_ref().into(),
            profile_dir: absolute_profile_dir.to_string_lossy().as_ref().into(),
            dotfiles_dir: absolute_dotfiles_dir.to_string_lossy().as_ref().into(),
          },
        )?)
      } else {
        None
      }
    }
    None => None,
  };

  if let Some(hooks) = &mut hooks_opt {
    hooks.run("pre_backup")?
  }

  if state.update {
    select_restore(&mut new_config)?;
  }

  let mut absolute_backup_dir = PathBuf::new();
  if state.backup {
    let backup_path = util::backup::compose_id_backups_path(&absolute_install_dir, &new_config.id);
    absolute_backup_dir =
      util::path::ensure_dir(util::backup::compose_backup_target_path(&backup_path, None))?;

    util::backup_dotfiles(&absolute_profile_dir, &absolute_backup_dir, &old_config_opt)?;

    let backups = util::backup::list_backups(&backup_path);
    if backups.len() > 3 {
      for backup in &backups[..backups.len() - 3] {
        let target_path = util::backup::compose_backup_target_path(&backup_path, Some(*backup));
        if fs::remove_dir_all(&target_path).is_err() {
          util::log::error(format!(
            "Failed to delete backup `{}`",
            target_path.display()
          ));
        };
      }
    }
  }

  if let Some(hooks) = &mut hooks_opt {
    hooks.run("post_backup_pre_deps")?
  }

  // Install dependencies if configured
  if let Some(relative_deps_dir) = &new_config.dependencies {
    let deps_dir = tmp_clone_dir.path().join(relative_deps_dir);

    runner.install_from_dependency_dir(&deps_dir)?;
  }

  if let Some(hooks) = &mut hooks_opt {
    hooks.run("post_deps_pre_install")?
  }

  // Remove symbolic links of active dotfiles
  let home_dir = dirs::home_dir().with_context(|| "Failed to resolve $HOME")?;
  if app_settings.active_config.is_some() {
    util::symlink::remove_active_symlinks(app_settings, &home_dir)?;
  }

  // Install dotfiles in profile
  if state.backup {
    let _ = fs::remove_dir_all(&absolute_profile_dir);
    util::path::ensure_dir(&absolute_profile_dir)?;
  }
  util::path::copy_recursive(tmp_clone_dir.path(), &absolute_profile_dir)?;
  util::path::write_str_to_file(serde_json::to_string(&new_config)?, &old_config_path, false)?;

  if let Some(hooks) = &mut hooks_opt {
    hooks.run("post_install_pre_restore")?
  }

  if state.update {
    restore(&new_config, &absolute_backup_dir, &absolute_profile_dir)?;
  }

  if let Some(hooks) = &mut hooks_opt {
    hooks.run("post_restore_pre_link")?
  }

  app_settings.active_config = Some(new_config);
  app_settings.active_install_dir = absolute_install_dir;
  util::symlink::create_active_symlinks(app_settings, &home_dir)?;

  if let Some(hooks) = &mut hooks_opt {
    hooks.run("finalize")?
  }

  Ok(())
}
