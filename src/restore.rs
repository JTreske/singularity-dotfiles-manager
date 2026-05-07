use crate::config::{AppSettings, DotfilesReleaseConfig, ReleaseConfigSource};
use crate::util;
use crate::util::backup::list_backups;
use crate::util::log;
use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::{Path, PathBuf};

fn select_backup_id(backup_dir: &Path) -> Result<String> {
  let backup_ids = util::backup::list_backup_ids(backup_dir);
  if backup_ids.is_empty() {
    return Err(anyhow!("No backups available"));
  }

  let initial_value = backup_ids.first().unwrap().clone();
  let items: Vec<(String, String, &str)> = backup_ids
    .into_iter()
    .map(|id| (id.clone(), id, ""))
    .collect();

  let selected_id = util::select(
    "Choose the dotfiles ID you wish to restore",
    initial_value,
    &items,
  )?;

  Ok(selected_id)
}

fn select_backup_timestamp(backup_id_dir: &Path) -> Result<DateTime<Utc>> {
  let mut timestamps = list_backups(backup_id_dir);
  if timestamps.is_empty() {
    return Err(anyhow!("No backup timestamps available"));
  }

  timestamps.sort();
  timestamps.reverse();

  let initial_value = *timestamps.first().unwrap();
  let items: Vec<(DateTime<Utc>, String, &str)> = timestamps
    .into_iter()
    .map(|ts| (ts, ts.to_string(), ""))
    .collect();

  let selected_timestamp = util::select(
    "Choose the backup timestamp you wish to restore",
    initial_value,
    &items,
  )?;

  Ok(selected_timestamp)
}

fn probe_old_config(old_config_path: PathBuf) -> Result<Option<DotfilesReleaseConfig>> {
  if old_config_path.exists() {
    match DotfilesReleaseConfig::try_from(&ReleaseConfigSource::Local(old_config_path)) {
      Ok(old_config) => Ok(Some(old_config)),
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

pub fn restore(
  app_settings: &mut AppSettings,
  install_dir: &Path,
  mut backup: bool,
  mut activate: bool,
  id: &Option<String>,
  timestamp: &Option<DateTime<Utc>>,
) -> Result<()> {
  let absolute_install_dir = util::path::ensure_dir(install_dir)?;
  let backup_dir = absolute_install_dir.join("backups");

  if !backup_dir.is_dir() {
    log::info(format!(
      "No backups exist for install dir `{}`",
      absolute_install_dir.display()
    ));
    return Ok(());
  }

  let selected_id = match id {
    Some(id) => id.to_string(),
    None => select_backup_id(&backup_dir)?,
  };
  let backup_id_dir = util::backup::compose_id_backups_path(&absolute_install_dir, &selected_id);

  // needed to check pre-selcted ID
  if !backup_id_dir.is_dir() {
    log::info(format!(
      "No backups exist for ID `{selected_id}` in install dir `{}`",
      absolute_install_dir.display()
    ));
    return Ok(());
  }

  let selected_timestamp = match timestamp {
    Some(timestamp) => *timestamp,
    None => select_backup_timestamp(&backup_id_dir)?,
  };
  let backup_target_dir =
    util::backup::compose_backup_target_path(&backup_id_dir, Some(selected_timestamp));

  // needed to check pre-selcted timestamp
  if !backup_target_dir.is_dir() {
    return Err(anyhow!(
      "Backup does not exist: `{}`",
      backup_target_dir.display()
    ));
  }

  let release_config_source =
    ReleaseConfigSource::Local(backup_target_dir.join(format!("{selected_id}.json")));
  let dotfiles_config = DotfilesReleaseConfig::try_from(&release_config_source)?;

  dotfiles_config.present(false, true);
  let c = util::confirm("Do you wish to continue?", true)?;
  if !c {
    util::log::info("Aborting...");
    return Ok(());
  }

  let mut remove_active_symlinks = activate;
  let home_dir = dirs::home_dir().with_context(|| "Failed to resolve $HOME")?;
  if let Some(active_config) = &app_settings.active_config
    && dotfiles_config.id == active_config.id
  {
    log::warn("The selected config will overwrite the currently active config");
    if !util::confirm(
      "Continuing will overwrite the active config. Backup will be created. Do you wish to continue?",
      false,
    )? {
      util::log::info("Aborting...");
      return Ok(());
    }

    backup = true;
    activate = true;

    // in this case, symlinks must be removed before restoring to avoid dangling symlinks
    util::symlink::remove_active_symlinks(app_settings, &home_dir)?;

    remove_active_symlinks = false;
  }

  let absolute_profile_dir =
    util::path::ensure_dir(absolute_install_dir.join(&dotfiles_config.id))?;

  let restore_config_path = absolute_install_dir.join(format!("{}.json", &dotfiles_config.id));
  if backup && absolute_profile_dir.read_dir()?.next().is_some() {
    log::step("Backing up current profile...");
    let old_config_opt = probe_old_config(restore_config_path.clone())?;
    let old_release_backup_dir = util::path::ensure_dir(util::backup::compose_backup_target_path(
      &backup_id_dir,
      None,
    ))?;

    util::backup_dotfiles(
      &absolute_profile_dir,
      &old_release_backup_dir,
      &old_config_opt,
    )?;
  }

  let _ = fs::remove_dir_all(&absolute_profile_dir);
  util::path::ensure_dir(&absolute_profile_dir)?;

  util::backup::restore_all_from_backup(backup_target_dir, absolute_profile_dir)?;
  util::path::write_str_to_file(
    serde_json::to_string(&dotfiles_config)?,
    &restore_config_path,
    false,
  )?;

  if activate {
    if remove_active_symlinks && app_settings.active_config.is_some() {
      util::symlink::remove_active_symlinks(app_settings, &home_dir)?;
    }

    app_settings.active_config = Some(dotfiles_config);
    app_settings.active_install_dir = absolute_install_dir;
    util::symlink::create_active_symlinks(app_settings, &home_dir)?;
  }

  Ok(())
}
