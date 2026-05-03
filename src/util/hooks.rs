use crate::os::Runner;
use anyhow::anyhow;
use rhai::plugin::*;
use rhai::{AST, CustomType, Engine, Scope, TypeBuilder, exported_module};
use std::path::PathBuf;
use std::sync::Arc;

pub type HookRunner = Arc<dyn Runner>;
pub type TmpDir = Arc<tempfile::TempDir>;

#[derive(Debug, Clone, CustomType)]
pub struct HookInformation {
  pub update: bool,
  pub backup: bool,
  pub install_dir: ImmutableString,
  pub profile_dir: ImmutableString,
  pub dotfiles_dir: ImmutableString,
}

pub struct Hooks {
  engine: Engine,
  ast: AST,
  scope: Scope<'static>,
}

impl Hooks {
  pub fn new(
    hook_path: PathBuf,
    runner: HookRunner,
    info: HookInformation,
  ) -> anyhow::Result<Hooks> {
    let mut engine = Engine::new();

    engine
      .build_type::<HookInformation>()
      .register_type_with_name::<TmpDir>("TmpDir")
      .register_type_with_name::<HookRunner>("Runner")
      .register_static_module("api", exported_module!(api).into());

    let mut scope = Scope::new();
    scope
      .push_constant("runner", runner)
      .push_constant("info", info);

    // Pre run complete script for setting up potential global variables
    let ast = engine
      .compile_file(hook_path)
      .map_err(|_| anyhow!("Failed to compile hook script"))?;

    engine
      .run_ast_with_scope(&mut scope, &ast)
      .map_err(|_| anyhow!("Failed to run hook script"))?;

    Ok(Hooks { engine, ast, scope })
  }

  pub fn run(&mut self, fn_name: &str) -> anyhow::Result<()> {
    if self
      .ast
      .iter_functions()
      .any(|f| f.name == fn_name && f.params.is_empty())
    {
      self
        .engine
        .call_fn::<()>(&mut self.scope, &self.ast, fn_name, ())
        .map_err(|e| anyhow!("Failed to run hook function `{fn_name}`: {e}"))?;
    }
    Ok(())
  }

  pub fn register_functions(&mut self) {
    self
      .engine
      .register_static_module("api", exported_module!(api).into());
  }
}

pub mod api_helper {
  use crate::util::hooks::HookRunner;
  use rhai::{EvalAltResult, ImmutableString};
  use std::collections::HashMap;
  use std::path::Path;

  pub fn array_to_vec<T: 'static>(array: rhai::Array) -> Result<Vec<T>, Box<EvalAltResult>> {
    array
      .into_iter()
      .enumerate()
      .map(|(i, d)| {
        d.try_cast::<T>()
          .ok_or_else(|| EvalAltResult::from(format!("Arg {i} could not be casted")).into())
      })
      .collect::<Result<Vec<_>, Box<EvalAltResult>>>()
  }

  pub fn array_to_select_tuple(
    array: rhai::Array,
  ) -> Result<Vec<(ImmutableString, ImmutableString, ImmutableString)>, Box<EvalAltResult>> {
    array
      .into_iter()
      .enumerate()
      .map(|(i, d)| {
        let inner_array = d.try_cast::<rhai::Array>().ok_or_else(|| {
          EvalAltResult::from(format!("Item at index {i} must be an array of 3 strings"))
        })?;

        if inner_array.len() != 3 {
          return Err(Box::new(EvalAltResult::from(format!(
            "Item at index {i} must have exactly 3 elements"
          ))));
        }

        let mut it = inner_array.into_iter();
        let v1 = it
          .next()
          .unwrap()
          .try_cast::<ImmutableString>()
          .ok_or("Field 1 not a string")?;
        let v2 = it
          .next()
          .unwrap()
          .try_cast::<ImmutableString>()
          .ok_or("Field 2 not a string")?;
        let v3 = it
          .next()
          .unwrap()
          .try_cast::<ImmutableString>()
          .ok_or("Field 3 not a string")?;

        Ok((v1, v2, v3))
      })
      .collect::<Result<Vec<_>, Box<EvalAltResult>>>()
  }

  pub fn run_command(
    runner: &mut HookRunner,
    program: &str,
    args: rhai::Array,
    interactive: bool,
    request_password_injection: bool,
    cwd_opt: Option<&Path>,
    env: rhai::Map,
  ) -> Result<(), Box<EvalAltResult>> {
    let arg_storage: Vec<ImmutableString> = array_to_vec(args)?;
    let arg_refs: Vec<&str> = arg_storage.iter().map(|s| s.as_str()).collect();

    let env_storage: Vec<(ImmutableString, ImmutableString)> = env
      .into_iter()
      .map(
        |(k, v)| -> Result<(ImmutableString, ImmutableString), Box<EvalAltResult>> {
          // Explicit return type here
          let val = v.try_cast::<ImmutableString>().ok_or_else(|| {
            Box::new(EvalAltResult::from(format!(
              "Env value for '{k}' is not a string"
            )))
          })?;
          Ok((k.into(), val))
        },
      )
      .collect::<Result<Vec<_>, Box<EvalAltResult>>>()?;
    let mut env_ref: HashMap<&str, &str> = HashMap::new();
    for (k, v) in &env_storage {
      env_ref.insert(k.as_str(), v.as_str());
    }

    runner
      .run_command(
        program,
        &arg_refs,
        interactive,
        request_password_injection,
        cwd_opt,
        &env_ref,
      )
      .map_err(|e| format!("{e}"))?;
    Ok(())
  }
}

/// Definition of the hooks API.
#[export_module]
pub mod api {
  use rhai::plugin::*;

  /// API functions for logging.
  pub mod log {
    use crate::util;

    /// Logs the given debug message using tracing.
    /// This message will not be shown to the user.
    ///
    /// # Args
    ///
    /// * msg - text that is logged
    ///
    /// # Example
    ///
    /// ```
    /// api::log::debug("debug message");
    /// ```
    ///
    /// # rhai-autodocs:index:1
    pub fn debug(msg: &str) {
      tracing::debug!(msg);
    }

    /// Logs the given step message using cliclack and tracing.
    ///
    /// # Args
    ///
    /// * msg - text that is logged (may be multiline)
    ///
    /// # Example
    ///
    /// ```
    /// api::log::step("step message");
    /// ```
    ///
    /// # rhai-autodocs:index:2
    pub fn step(msg: &str) {
      util::log::step(msg);
    }

    /// Logs the given info message using cliclack and tracing.
    ///
    /// # Args
    ///
    /// * msg - text that is logged (may be multiline)
    ///
    /// # Example
    ///
    /// ```
    /// api::log::info("info message");
    /// ```
    ///
    /// # rhai-autodocs:index:3
    pub fn info(msg: &str) {
      util::log::info(msg);
    }

    /// Logs the given success message using cliclack and tracing.
    ///
    /// # Args
    ///
    /// * msg - text that is logged (may be multiline)
    ///
    /// # Example
    ///
    /// ```
    /// api::log::success("success message");
    /// ```
    ///
    /// # rhai-autodocs:index:4
    pub fn success(msg: &str) {
      util::log::success(msg);
    }

    /// Logs the given remark message using cliclack and tracing.
    ///
    /// # Args
    ///
    /// * msg - text that is logged (may be multiline)
    ///
    /// # Example
    ///
    /// ```
    /// api::log::remark("remark message");
    /// ```
    ///
    /// # rhai-autodocs:index:5
    pub fn remark(msg: &str) {
      util::log::remark(msg);
    }

    /// Logs the given waring message using cliclack and tracing.
    ///
    /// # Args
    ///
    /// * msg - text that is logged (may be multiline)
    ///
    /// # Example
    ///
    /// ```
    /// api::log::warn("warning message");
    /// ```
    ///
    /// # rhai-autodocs:index:6
    pub fn warn(msg: &str) {
      util::log::warn(msg);
    }

    /// Logs the given error message using cliclack and tracing.
    ///
    /// # Args
    ///
    /// * msg - text that is logged (may be multiline)
    ///
    /// # Example
    ///
    /// ```
    /// api::log::error("error message");
    /// ```
    ///
    /// # rhai-autodocs:index:7
    pub fn error(msg: &str) {
      util::log::error(msg);
    }
  }

  /// API functions for system operations and package management.
  pub mod runner {
    use crate::util;
    use std::path::Path;

    /// Installs the provided package.
    ///
    /// # Args
    ///
    /// * runner - Runner instance provided in the scope (runner)
    /// * package - Name of the package to install
    ///
    /// # Example
    ///
    /// ```
    /// api::runner::run(runner, "git");
    /// ```
    ///
    /// # rhai-autodocs:index:1
    #[rhai_fn(pure, return_raw)]
    pub fn install_package(
      runner: &mut HookRunner,
      package: &str,
    ) -> Result<(), Box<EvalAltResult>> {
      if let Err(e) = runner.install_package(package) {
        util::log::error(e.to_string());
        Err(format!("{e}").into())
      } else {
        Ok(())
      }
    }

    /// Returns the short ID of the current operating system (e.g., "arch, "debian").
    ///
    /// # Args
    ///
    /// * runner - Runner instance provided in the scope
    ///
    /// # Example
    ///
    /// ```
    /// let os = api::runner::os_short_name(runner);
    /// ```
    ///
    /// # rhai-autodocs:index:2
    #[rhai_fn(pure)]
    pub fn os_short_name(runner: &mut HookRunner) -> ImmutableString {
      runner.os_short_name().into()
    }

    /// Runs any command with the stored permissions.
    ///
    /// If root privileges are needed, set `program` to `sudo` or `interactive` to `true`.
    ///
    /// Note: `interactive` mode disrupts the clean UI.
    ///
    /// # Args
    ///
    /// * runner - Runner instance provided in the scope
    /// * program - The executable to run
    /// * args - An array of string arguments
    /// * interactive - Whether to run in an interactive TTY
    /// * request_password_injection - If set to `true` the runner will prepare the sudo cache
    /// * env - A map of environment variables
    /// * cwd - The directory in which to execute the command (may use ~ for $HOME)
    ///
    /// # Example
    ///
    /// ```
    /// api::runner::run_command(runner, "git", ["sparse-checkout", "set", "test"], false, #{}, "/path/to/repo");
    /// ```
    ///
    /// # rhai-autodocs:index:3
    #[rhai_fn(name = "run_command", pure, return_raw)]
    pub fn run_command_with_cwd(
      runner: &mut HookRunner,
      program: &str,
      args: rhai::Array,
      interactive: bool,
      request_password_injection: bool,
      env: rhai::Map,
      cwd: &str,
    ) -> Result<(), Box<EvalAltResult>> {
      let cwd_path = util::path::resolve_home(Path::new(cwd)).map_err(|e| format!("{e}"))?;

      api_helper::run_command(
        runner,
        program,
        args,
        interactive,
        request_password_injection,
        Some(&cwd_path),
        env,
      )
    }

    #[rhai_fn(name = "run_command", pure, return_raw)]
    pub fn run_command_without_cwd(
      runner: &mut HookRunner,
      program: &str,
      args: rhai::Array,
      interactive: bool,
      request_password_injection: bool,
      env: rhai::Map,
    ) -> Result<(), Box<EvalAltResult>> {
      api_helper::run_command(
        runner,
        program,
        args,
        interactive,
        request_password_injection,
        None,
        env,
      )
    }

    /// Clones a git repository.
    ///
    /// # Args
    ///
    /// * runner - Runner instance provided in the scope
    /// * url - The git clone URL
    /// * target - The local path to clone into
    /// * tag - The specific tag to checkout
    ///
    /// # Example
    ///
    /// ```
    /// api::runner::clone_git_repository_with_tag("https://github.com/user/repo", "/tmp/dir", "1.0.0");
    /// ```
    ///
    /// # rhai-autodocs:index:4
    #[rhai_fn(name = "clone_git_repository", pure, return_raw)]
    pub fn clone_git_repository_with_tag(
      runner: &mut HookRunner,
      url: &str,
      target: &str,
      tag: &str,
    ) -> Result<(), Box<EvalAltResult>> {
      let target_path = util::path::resolve_home(Path::new(target)).map_err(|e| format!("{e}"))?;
      if let Err(e) = runner.clone_git_repository(
        &url::Url::parse(url).map_err(|e| Box::new(EvalAltResult::from(format!("{e}"))))?,
        Some(tag),
        &target_path,
      ) {
        util::log::error(e.to_string());
        Err(format!("{e}").into())
      } else {
        Ok(())
      }
    }

    #[rhai_fn(name = "clone_git_repository", pure, return_raw)]
    pub fn clone_git_repository_without_tag(
      runner: &mut HookRunner,
      url: &str,
      target: &str,
    ) -> Result<(), Box<EvalAltResult>> {
      let target_path = util::path::resolve_home(Path::new(target)).map_err(|e| format!("{e}"))?;
      if let Err(e) = runner.clone_git_repository(
        &url::Url::parse(url).map_err(|e| Box::new(EvalAltResult::from(format!("{e}"))))?,
        None,
        &target_path,
      ) {
        util::log::error(e.to_string());
        Err(format!("{e}").into())
      } else {
        Ok(())
      }
    }
  }

  /// General utility functions for file system operations and user interaction.
  pub mod utility {
    use crate::util;
    use std::fs;
    use std::path::Path;

    /// Displays a note to the user.
    ///
    /// # Args
    ///
    /// * prompt - The heading of the note
    /// * msg - The message to display
    ///
    /// # Example
    ///
    /// ```
    /// api::utility::note("Next Steps", "Set your preferred shell, ...");
    /// ```
    ///
    /// # rhai-autodocs:index:1
    #[rhai_fn(return_raw)]
    pub fn note(prompt: &str, msg: &str) -> Result<(), Box<EvalAltResult>> {
      util::note(prompt, msg).map_err(|e| format!("{e}").into())
    }

    /// Displays a confirmation prompt to the user.
    ///
    /// # Args
    ///
    /// * msg - The message to display
    /// * initial_value - The default selection (true/false)
    ///
    /// # Example
    ///
    /// ```
    /// if api::utility::confirm("Do you want to proceed?", true) { ... }
    /// ```
    ///
    /// # rhai-autodocs:index:2
    #[rhai_fn(return_raw)]
    pub fn confirm(msg: &str, initial_value: bool) -> Result<bool, Box<EvalAltResult>> {
      util::confirm(msg, initial_value).map_err(|e| format!("{e}").into())
    }

    /// Displays a single-choice selection menu.
    ///
    /// # Args
    ///
    /// * msg - The message to display
    /// * initial_value - The initially highlighted value
    /// * items - An array of strings to choose from
    ///
    /// # Example
    ///
    /// ```
    /// let choice = api::utility::select(
    ///   "Pick a color",
    ///   "red",
    ///   [["red", "Red", "a red color"],
    ///   ["green", "Green", "a green color"],
    ///   ["blue", "Blue", "a blue color"]]
    /// );
    /// ```
    ///
    /// # rhai-autodocs:index:3
    #[rhai_fn(return_raw)]
    pub fn select(
      msg: &str,
      initial_value: ImmutableString,
      items: rhai::Array,
    ) -> Result<ImmutableString, Box<EvalAltResult>> {
      let items_vec = api_helper::array_to_select_tuple(items)?;

      util::select(msg, initial_value, &items_vec).map_err(|e| format!("{e}").into())
    }

    /// Displays a multi-choice selection menu.
    ///
    /// # Args
    ///
    /// * msg - The message to display
    /// * initial_values - Array of values to pre-select
    /// * items - An array of all available options
    ///
    /// # Example
    ///
    /// ```
    /// let choice = api::utility::multi_select(
    ///   "Pick colors",
    ///   ["red", "green"],
    ///   [["red", "Red", "a red color"],
    ///   ["green", "Green", "a green color"],
    ///   ["blue", "Blue", "a blue color"]]
    /// );
    /// ```
    ///
    /// # rhai-autodocs:index:4
    #[rhai_fn(return_raw)]
    pub fn multi_select(
      msg: &str,
      initial_values: rhai::Array,
      items: rhai::Array,
    ) -> Result<rhai::Array, Box<EvalAltResult>> {
      let initial_values_vec: Vec<ImmutableString> = api_helper::array_to_vec(initial_values)?;
      let items_vec = api_helper::array_to_select_tuple(items)?;

      let selected_vec = util::multi_select(msg, initial_values_vec, &items_vec)
        .map_err(|e| EvalAltResult::from(format!("{e}")))?;

      let selected: rhai::Array = selected_vec.into_iter().map(Into::into).collect();
      Ok(selected)
    }

    /// Resolves `~` in a path to the user's home directory.
    ///
    /// # Args
    ///
    /// * path - The path string to resolve
    ///
    /// # Example
    ///
    /// ```
    /// let full_path = api::utility::resolve_home("~/Downloads");
    /// ```
    ///
    /// # rhai-autodocs:index:5
    #[rhai_fn(return_raw)]
    pub fn resolve_home(path: &str) -> Result<ImmutableString, Box<EvalAltResult>> {
      let res_path = util::path::resolve_home(Path::new(path)).map_err(|e| format!("{e}"))?;
      Ok(res_path.to_string_lossy().as_ref().into())
    }

    /// Reads the entire content of a file into a string.
    ///
    /// # Args
    ///
    /// * path - Path to the file (may use ~ for $HOME)
    ///
    /// # Example
    ///
    /// ```
    /// let config = api::utility::read_file("~/.config/myapp/config.toml");
    /// ```
    ///
    /// # rhai-autodocs:index:6
    #[rhai_fn(return_raw)]
    pub fn read_file(path: &str) -> Result<ImmutableString, Box<EvalAltResult>> {
      let res_path = util::path::resolve_home(Path::new(path)).map_err(|e| format!("{e}"))?;
      let content = util::path::read_file_to_string(res_path).map_err(|e| format!("{e}"))?;

      Ok(content.into())
    }

    /// Writes a string to a file, optionally appending to existing content.
    ///
    /// # Args
    ///
    /// * path - Path to the file (may use ~ for $HOME)
    /// * content - The string data to write
    /// * append - If true, appends to the file; if false, overwrites it
    ///
    /// # Example
    ///
    /// ```
    /// api::utility::write_file("~/log.txt", "New entry\n", true);
    /// ```
    ///
    /// # rhai-autodocs:index:7
    #[rhai_fn(return_raw)]
    pub fn write_file(path: &str, content: &str, append: bool) -> Result<(), Box<EvalAltResult>> {
      let res_path = util::path::resolve_home(Path::new(path)).map_err(|e| format!("{e}"))?;
      util::path::write_str_to_file(content, res_path, append).map_err(|e| format!("{e}"))?;

      Ok(())
    }

    /// Replaces occurrences in a string using a Regular Expression.
    ///
    /// # Args
    ///
    /// * input - The source string
    /// * pattern - The Regex pattern to match
    /// * replace - The replacement string
    ///
    /// # Example
    ///
    /// ```
    /// let cleaned = api::utility::regex_replace("Hello 123", "[0-9]+", "World");
    /// ```
    ///
    /// # rhai-autodocs:index:8
    #[rhai_fn(return_raw)]
    pub fn regex_replace(
      input: &str,
      pattern: &str,
      replace: &str,
    ) -> Result<ImmutableString, Box<EvalAltResult>> {
      let re = regex::Regex::new(pattern).map_err(|e| format!("Invalid regex pattern: {e}"))?;

      Ok(re.replace(input, replace).as_ref().into())
    }

    /// Creates a temporary directory that is deleted when the handle is dropped.
    ///
    /// # Example
    ///
    /// ```
    /// let tmp = api::utility::create_tmp_dir();
    /// let path = api::utility::get_tmp_path(tmp);
    /// ```
    ///
    /// # rhai-autodocs:index:9
    #[rhai_fn(return_raw)]
    pub fn create_tmp_dir() -> Result<TmpDir, Box<EvalAltResult>> {
      let tmp_dir =
        tempfile::TempDir::new().map_err(|e| format!("Failed to create tmp dir: {e}"))?;

      Ok(Arc::new(tmp_dir))
    }

    /// Returns the absolute path of a temporary directory handle.
    ///
    /// # Args
    ///
    /// * tmp_dir - The TmpDir handle
    ///
    /// # rhai-autodocs:index:10
    #[rhai_fn(pure)]
    pub fn get_tmp_path(tmp_dir: &mut TmpDir) -> ImmutableString {
      tmp_dir.path().to_string_lossy().as_ref().into()
    }

    /// Removes a file from the filesystem.
    ///
    /// # Args
    ///
    /// * path - Path to the file
    ///
    /// # rhai-autodocs:index:11
    #[rhai_fn(return_raw)]
    pub fn remove_file(path: &str) -> Result<(), Box<EvalAltResult>> {
      fs::remove_file(Path::new(path)).map_err(|e| {
        Box::new(EvalAltResult::from(format!(
          "Failed to remove file {path}: {e}"
        )))
      })
    }

    /// Recursively copies a directory or file to a destination.
    ///
    /// # Args
    ///
    /// * src - Source path
    /// * dst - Destination path
    ///
    /// # rhai-autodocs:index:12
    #[rhai_fn(return_raw)]
    pub fn copy_recursive(src: &str, dst: &str) -> Result<(), Box<EvalAltResult>> {
      util::path::copy_recursive(src, dst)
        .map_err(|e| Box::new(EvalAltResult::from(format!("{e}"))))
    }

    /// Parses a JSON string into a Rhai Map or Array.
    ///
    /// # Args
    ///
    /// * json - The raw JSON string
    ///
    /// # Example
    ///
    /// ```
    /// let data = api::utility::parse_json("{\"key\": \"value\"}");
    /// print(data.key);
    /// ```
    ///
    /// # rhai-autodocs:index:13
    #[rhai_fn(return_raw)]
    pub fn parse_json(
      context: NativeCallContext,
      json: &str,
    ) -> Result<Dynamic, Box<EvalAltResult>> {
      let res = context.engine().parse_json(json, true)?;
      Ok(res.into())
    }

    /// Checks if a path exists on the filesystem.
    ///
    /// # Args
    ///
    /// * path - The path to check
    ///
    /// # rhai-autodocs:index:14
    pub fn path_exists(path: &str) -> bool {
      Path::new(path).exists()
    }

    /// Ensures a directory exists, creating it and any parents if necessary.
    ///
    /// # Args
    ///
    /// * path - The directory path
    ///
    /// # rhai-autodocs:index:15
    #[rhai_fn(return_raw)]
    pub fn ensure_dir(path: &str) -> Result<(), Box<EvalAltResult>> {
      util::path::ensure_dir(path).map_err(|e| Box::new(EvalAltResult::from(format!("{e}"))))?;

      Ok(())
    }
  }
}
