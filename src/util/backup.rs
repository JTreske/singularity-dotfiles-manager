use crate::config::DotfilesReleaseConfig;
use crate::util;
use anyhow::{Result, anyhow};
use chrono::{DateTime, NaiveDateTime, Utc};
use std::fs;
use std::path::{Path, PathBuf};

const DOTFILES_SUB_DIR: &str = "dotfiles";
const DATE_TIME_BACKUP_FORMAT: &str = "%Y%m%d%H%M%S";

/// The user must make sure that both paths exist.
pub fn backup_dotfiles(
  src: impl AsRef<Path>,
  dst: impl AsRef<Path>,
  config_opt: &Option<DotfilesReleaseConfig>,
) -> Result<()> {
  let src = src.as_ref();
  let dst = dst.as_ref();

  util::log::step(format!(
    "Backup from `{}` to `{}`...",
    src.display(),
    dst.display()
  ));

  if std::fs::read_dir(dst)?.next().is_some() {
    return Err(anyhow!("Backup target `{}` is not empty", dst.display()));
  }

  let dotfiles_sub_dir = util::path::ensure_dir(dst.join(DOTFILES_SUB_DIR))?;

  if let Some(config) = config_opt {
    util::path::write_str_to_file(
      serde_json::to_string(config)?,
      dst.join(format!("{}.json", &config.id)),
      false,
    )?;
  }

  util::path::copy_recursive(src, dotfiles_sub_dir)?;

  util::log::success("Backup complete!");
  Ok(())
}

pub fn list_backups(backup_path: impl AsRef<Path>) -> Vec<DateTime<Utc>> {
  let backup_path = backup_path.as_ref();

  if !backup_path.is_dir() {
    return Vec::new();
  }

  let targets = match fs::read_dir(backup_path) {
    Ok(t) => t,
    Err(_) => return Vec::new(),
  };

  let mut backup_targets = Vec::new();
  for target_res in targets {
    if let Ok(target) = target_res
      && target.path().is_dir()
      && let Ok(timestamp) = NaiveDateTime::parse_from_str(
        &target.file_name().to_string_lossy(),
        DATE_TIME_BACKUP_FORMAT,
      )
    {
      backup_targets.push(timestamp.and_utc());
    }
  }

  Vec::new()
}

/// Returns the `backups_path`.
pub fn compose_id_backups_path(
  install_dir: impl AsRef<Path>,
  dotfiles_id: impl AsRef<Path>,
) -> PathBuf {
  install_dir.as_ref().join("backups").join(dotfiles_id)
}

/// Creates the backup target using the current time or the given UTC DateTime.
pub fn compose_backup_target_path(
  backup_path: impl AsRef<Path>,
  date_time_opt: Option<DateTime<Utc>>,
) -> PathBuf {
  let date_time = match date_time_opt {
    Some(dt) => dt,
    None => Utc::now(),
  };
  backup_path
    .as_ref()
    .join(date_time.format(DATE_TIME_BACKUP_FORMAT).to_string())
}

pub fn restore_from_backup(
  restore_paths: &[impl AsRef<Path>],
  backup_dir: impl AsRef<Path>,
  profile_dir: impl AsRef<Path>,
) -> Result<()> {
  let backup_dir = backup_dir.as_ref().join(DOTFILES_SUB_DIR);
  let profile_dir = profile_dir.as_ref();

  for item in restore_paths {
    let src = backup_dir.join(item);
    let dst = profile_dir.join(item);

    if src.is_dir() {
      let _ = fs::remove_dir_all(&dst);
      util::path::copy_recursive(&src, &dst)?;
    } else {
      let _ = fs::remove_file(&dst);
      fs::copy(&src, &dst)?;
    }
  }

  Ok(())
}
