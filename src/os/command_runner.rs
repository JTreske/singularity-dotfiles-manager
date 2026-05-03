use crate::os::Permissions;
use crate::util::{self, log};
use anyhow::{Context, Result, anyhow};
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use tracing::debug;
use url::Url;

#[derive(Debug, Clone)]
pub struct CommandRunner {
  password: String,
}

impl CommandRunner {
  pub fn new(password: String) -> CommandRunner {
    CommandRunner { password }
  }

  /// Runs the specified command.
  /// If a program needs interaction other than entering the password, `interactive` must be set to `true`.
  /// All programs that need root rights should be run with request_password_injection set to `true`.
  #[allow(clippy::too_many_arguments)]
  pub fn run_command(
    &self,
    program: &str,
    args: &[&str],
    permissions: &Permissions,
    interactive: bool,
    request_password_injection: bool,
    cwd_opt: Option<&Path>,
    env: &HashMap<&str, &str>,
  ) -> Result<()> {
    // prepare command
    let mut command = Command::new(program);
    command.envs(env);

    if let Some(cwd) = cwd_opt {
      command.current_dir(cwd);
    }

    let mut escaped_args = Vec::new();
    for arg in args {
      escaped_args.push(format!("\"{arg}\""));
    }
    let command_string = format!("{program} {}", escaped_args.join(" "));

    // check permissions
    let mut run_as_root = permissions.run_as_root;
    if !permissions.run_as_root && request_password_injection {
      let res = util::confirm(
        format!(
          "The manager is trying to run `{command_string}`, but run-as-root permission is not set. Do you want to allow this?",
        ),
        false,
      )?;
      if !res {
        return Err(anyhow!("Run as root is not allowed. Aborted!"));
      }

      run_as_root = true;
    }

    // set or remove sudo cache
    if run_as_root && request_password_injection {
      let mut setup = Command::new("sudo");
      setup
        .args(["-v", "--stdin", "-p", ""])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

      let mut setup_process = setup.spawn()?;

      let stdin = setup_process
        .stdin
        .as_mut()
        .context("Failed to open setup process stdin")?;
      let res = writeln!(stdin, "{}", &self.password);
      if res.is_err() {
        debug!("Failed to write to stdin. Killing setup process...");
        let _ = setup_process.kill();
      };

      let status = setup_process.wait();

      if !matches!(status, Ok(s) if s.success()) {
        log::warn("Failed to set sudo cache");
      }
    } else {
      let status = Command::new("sudo")
        .arg("-K")
        .spawn()
        .and_then(|mut child| child.wait());

      if !matches!(status, Ok(s) if s.success()) {
        log::warn("Failed to clear sudo cache");
      }
    }

    command.args(args);

    if !interactive {
      command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    }

    let mut child = command.spawn()?;

    let status = child.wait()?;
    if !status.success() {
      return Err(anyhow!("Command `{command_string}` failed: {status}"));
    }

    Ok(())
  }

  pub fn git_clone(&self, url: &Url, tag_opt: &Option<&str>, target: &Path) -> Result<()> {
    util::path::ensure_dir(target)?;
    let target_str = target.to_string_lossy();

    let mut args = vec!["clone"];
    if let Some(tag) = tag_opt {
      args.extend_from_slice(&["--depth", "1", "--branch", tag]);
    }
    args.push(url.as_str());
    args.push(&target_str);

    log::step(format!("Cloning repository `{url}`..."));

    self.run_command(
      "git",
      &args,
      &Permissions { run_as_root: false },
      false,
      false,
      None,
      &HashMap::new(),
    )?;

    log::success("Repository cloned!");

    Ok(())
  }
}
