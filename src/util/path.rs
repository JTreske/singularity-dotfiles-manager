use anyhow::{Context, Result, anyhow};
use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

pub fn resolve_home(path: impl AsRef<Path>) -> Result<PathBuf> {
  let mut components = path.as_ref().components();
  if let Some(first) = components.next()
    && let Component::Normal(name) = first
    && name == OsStr::new("~")
  {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not determine $HOME"))?;

    let relative: PathBuf = components.collect();
    Ok(home.join(relative))
  } else {
    Ok(path.as_ref().to_path_buf())
  }
}

pub fn read_file_to_string(path: impl AsRef<Path>) -> Result<String> {
  Ok(fs::read_to_string(path)?)
}

pub fn create_parents(path: impl AsRef<Path>) -> Result<()> {
  let Some(parent) = path.as_ref().parent() else {
    return Ok(());
  };

  if parent.exists() {
    return Ok(());
  }

  fs::create_dir_all(parent)
    .with_context(|| format!("Failed to create directory `{}`", parent.display()))
}

/// Writes the content to the specified file.
///
/// Make sure the parent directory exists before calling this function.
pub fn write_str_to_file(
  content: impl AsRef<str>,
  path: impl AsRef<Path>,
  append: bool,
) -> Result<()> {
  if !append && path.as_ref().is_file() {
    fs::remove_file(&path)?;
  }

  let mut file = fs::OpenOptions::new()
    .write(true)
    .append(append)
    .create(true)
    .open(path)?;

  writeln!(file, "{}", content.as_ref())?;

  Ok(())
}

/// Makes sure the given path exists and is a directory.
/// Returns the absolute directory path.
pub fn ensure_dir(dir: impl AsRef<Path>) -> Result<PathBuf> {
  let dir_ref = dir.as_ref();
  match dir_ref.metadata() {
    Ok(md) if md.is_dir() => {
      tracing::debug!(
        "Directory `{}` already exists. Skipping creation",
        dir_ref.display()
      )
    }
    Ok(_) => {
      return Err(anyhow!(
        "`{}` already exists but is not a directory.",
        dir_ref.display()
      ));
    }
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
      tracing::debug!(
        "Directory `{}` does not yet exist. Creating...",
        dir_ref.display()
      );
      fs::create_dir_all(dir_ref)
        .with_context(|| format!("Failed to create directory `{}`", dir_ref.display()))?;
    }
    Err(e) => return Err(e.into()),
  }

  tracing::debug!("Creating absolute directory path...");
  fs::canonicalize(dir_ref).with_context(|| {
    format!(
      "Failed to get absolute directory path for `{}`",
      dir_ref.display()
    )
  })
}

/// Both src and dst must exist
pub fn copy_recursive(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<()> {
  if src.as_ref().is_file() {
    fs::copy(src.as_ref(), dst.as_ref())?;
  }

  if let Some(name) = src.as_ref().file_name()
    && name == ".git"
  {
    // Do not copy .git folder
    return Ok(());
  }

  for entry_res in fs::read_dir(src)? {
    let entry = entry_res?;

    let ty = entry.file_type()?;
    if ty.is_dir() {
      copy_recursive(entry.path(), dst.as_ref().join(entry.file_name()))?;
    } else {
      ensure_dir(&dst)?;
      fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
    }
  }

  Ok(())
}
