use rhai::exported_module;
use schemars::schema_for;
use singularity_dotfiles_manager::config::{Apps, DotfilesReleaseConfig};
use singularity_dotfiles_manager::util::hooks::HookRunner;
use std::path::Path;
use std::{env, fs};

fn main() {
  let workspace_root = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is not set");

  let schema_path = Path::new(&workspace_root).join("docs/src/schema");
  let _ = fs::remove_dir_all(&schema_path);
  singularity_dotfiles_manager::util::path::ensure_dir(&schema_path)
    .unwrap_or_else(|e| panic!("{e}"));
  let release_config_schema = schema_for!(DotfilesReleaseConfig);
  let release_config_schema_file = schema_path.join("dotfiles_release_config.json");
  let release_config_schema_string = serde_json::to_string_pretty(&release_config_schema).unwrap();
  singularity_dotfiles_manager::util::path::write_str_to_file(
    &release_config_schema_string,
    &release_config_schema_file,
    false,
  )
  .unwrap_or_else(|_| {
    panic!(
      "Cannot write to file {}",
      release_config_schema_file.display()
    )
  });
  let apps_schema = schema_for!(Apps);
  let apps_schema_file = schema_path.join("apps.json");
  let apps_schema_string = serde_json::to_string_pretty(&apps_schema).unwrap();
  singularity_dotfiles_manager::util::path::write_str_to_file(
    &apps_schema_string,
    &apps_schema_file,
    false,
  )
  .unwrap_or_else(|_| panic!("Cannot write to file {}", apps_schema_file.display()));

  let api_docs_path = Path::new(&workspace_root).join("docs/src/api");
  let _ = fs::remove_dir_all(&api_docs_path);
  singularity_dotfiles_manager::util::path::ensure_dir(&api_docs_path)
    .unwrap_or_else(|e| panic!("{e}"));
  let mut engine = rhai::Engine::new();
  engine.register_type_with_name::<HookRunner>("Runner");
  engine.register_static_module(
    "api",
    exported_module!(singularity_dotfiles_manager::util::hooks::api).into(),
  );

  let functions = engine
    .gen_fn_metadata_to_json(true)
    .expect("Failed to generate fn metadata");
  singularity_dotfiles_manager::util::path::write_str_to_file(
    functions,
    api_docs_path.join("defs.json"),
    false,
  )
  .expect("Failed to write defs.json");

  let docs = rhai_autodocs::export::options()
    .include_standard_packages(true)
    .order_items_with(rhai_autodocs::export::ItemsOrder::ByIndex)
    .export(&engine)
    .expect("failed to export documentation");

  let mdx = rhai_autodocs::generate::mdbook()
    .generate(&docs)
    .expect("failed to generate mdx for mdbook");

  for (name, docs) in mdx {
    let file_name = api_docs_path.join(format!("{name}.mdx"));
    // Create or truncate the file.
    singularity_dotfiles_manager::util::path::write_str_to_file(&docs, &file_name, false)
      .unwrap_or_else(|_| panic!("Cannot write to file {}", file_name.display()));

    println!("Wrote docs for module `{}` → {}", name, file_name.display())
  }
}
