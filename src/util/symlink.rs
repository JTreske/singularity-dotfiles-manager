use crate::config::AppSettings;
use crate::util;
use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::Path;
use tracing::debug;

fn backup_existing_file(file_path: impl AsRef<Path>) -> Result<()> {
  let file_path = file_path.as_ref();

  if !file_path.exists() {
    return Ok(());
  }
  let bak = file_path.with_added_extension("bak");

  util::log::warn(format!(
    "File `{}` already exists. Creating backup `{}`...",
    file_path.file_name().unwrap().display(),
    bak.display()
  ));

  if bak.exists() {
    fs::remove_file(&bak)?;
  }

  fs::rename(file_path, &bak).with_context(|| "Backup creation failed")?;

  Ok(())
}

pub fn remove_symlink(src: impl AsRef<Path>, target: impl AsRef<Path>) -> Result<()> {
  let src = src.as_ref();
  let target = target.as_ref();

  if !target.is_symlink() {
    return Ok(());
  }

  let real_src = fs::read_link(target)?;
  if real_src != src {
    // ignoring links that are not expected
    return Ok(());
  }

  Ok(fs::remove_file(target)?)
}

pub fn remove_symlinks(src_root: impl AsRef<Path>, symlink_root: impl AsRef<Path>) -> Result<()> {
  util::log::step(format!(
    "Removing symlinks from `{}` to `{}`...",
    src_root.as_ref().display(),
    symlink_root.as_ref().display()
  ));

  for entry in walkdir::WalkDir::new(&src_root)
    .into_iter()
    .filter_map(|e| e.ok())
  {
    if entry.file_type().is_dir() {
      continue;
    }

    let relative = entry.path().strip_prefix(&src_root).with_context(|| {
      format!(
        "Failed to create relative path for `{}`",
        entry.path().display()
      )
    })?;
    let target_path = symlink_root.as_ref().join(relative);
    remove_symlink(entry.path(), target_path)?;
  }

  util::log::success("Symlinks removed!");

  Ok(())
}

pub fn remove_active_symlinks(
  app_settings: &mut AppSettings,
  symlink_root: impl AsRef<Path>,
) -> Result<()> {
  util::log::step("Removing symlinks for active profile...");

  let active_profile_dir = match &app_settings.active_config {
    Some(active_config) => app_settings
      .active_install_dir
      .join(&active_config.id)
      .join(&active_config.dotfiles),
    None => return Err(anyhow!("No active ID found in app settings")),
  };

  remove_symlinks(active_profile_dir, symlink_root)?;
  app_settings.active_config = None;

  Ok(())
}

pub fn create_symlink(src: impl AsRef<Path>, target: impl AsRef<Path>) -> Result<()> {
  let target = target.as_ref();

  if let Some(parent) = target.parent() {
    util::path::ensure_dir(parent)?;
  }

  backup_existing_file(target)?;

  std::os::unix::fs::symlink(src, target)
    .with_context(|| format!("Failed to create symlink `{}`", target.display()))
}

pub fn create_symlinks(src_root: impl AsRef<Path>, symlink_root: impl AsRef<Path>) -> Result<()> {
  util::log::step(format!(
    "Creating symlinks from `{}` to `{}`...",
    src_root.as_ref().display(),
    symlink_root.as_ref().display()
  ));
  for entry in walkdir::WalkDir::new(&src_root)
    .into_iter()
    .filter_map(|e| e.ok())
  {
    if entry.file_type().is_dir() {
      continue;
    }

    let relative = entry.path().strip_prefix(&src_root).with_context(|| {
      format!(
        "Failed to create relative path for `{}`",
        entry.path().display()
      )
    })?;
    let target_path = symlink_root.as_ref().join(relative);
    create_symlink(entry.path(), target_path)?;
  }

  util::log::success("Symlinks created!");

  Ok(())
}

pub fn create_active_symlinks(
  app_settings: &AppSettings,
  symlink_root: impl AsRef<Path>,
) -> Result<()> {
  util::log::step("Creating symlinks for active profile...");

  let active_profile_dir = match &app_settings.active_config {
    Some(active_config) => app_settings
      .active_install_dir
      .join(&active_config.id)
      .join(&active_config.dotfiles),
    None => return Err(anyhow!("No active ID found in app settings")),
  };

  create_symlinks(active_profile_dir, symlink_root)
}

pub fn sync_active_profile(app_settings: &AppSettings) -> Result<()> {
  let active_config = match &app_settings.active_config {
    Some(active_config) => active_config,
    None => {
      debug!("Aborting active profile sync: No active config set");
      return Ok(());
    }
  };

  util::log::step("Syncing active profile...");

  let profile_dir = app_settings.active_install_dir.join(&active_config.id);
  let dotfiles_dir = profile_dir.join(&active_config.dotfiles);
  let home_dir = dirs::home_dir().with_context(|| "Failed to resolve $HOME")?;

  for entry in walkdir::WalkDir::new(&dotfiles_dir)
    .into_iter()
    .filter_map(|e| e.ok())
  {
    if entry.file_type().is_dir() {
      continue;
    }

    let relative = entry.path().strip_prefix(&dotfiles_dir).with_context(|| {
      format!(
        "Failed to create relative path for `{}`",
        entry.path().display()
      )
    })?;
    let target_path = home_dir.join(relative);

    if target_path.is_symlink()
      && let Ok(link) = target_path.canonicalize()
      && let Ok(orig) = entry.path().canonicalize()
      && orig == link
    {
      // skip syncing for this file since it already points to the correct location
      continue;
    }

    util::log::info(format!("Syncing `{}`...", target_path.display()));
    if target_path.is_file() {
      std::fs::copy(&target_path, entry.path()).with_context(|| {
        format!(
          "Failed to copy `{}` to `{}`",
          target_path.display(),
          entry.path().display()
        )
      })?;
      std::fs::remove_file(&target_path)
        .with_context(|| format!("Failed to remove `{}`", target_path.display()))?;

      create_symlink(entry.path(), target_path)?;
    } else {
      util::log::warn(format!(
        "Unexpected file type for `{}`",
        target_path.display()
      ));
    }
  }

  util::log::success("Active profile synced!");

  Ok(())
}
