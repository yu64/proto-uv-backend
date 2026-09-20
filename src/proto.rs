//! WASM hooks for public PyPI discovery and isolated uv tool installations.

use crate::{
  config::UvBackendConfig,
  package::{Package, Project},
  uv, version,
};
use extism_pdk::*;
use proto_pdk::*;
use schematic::SchemaBuilder;

// #############################################################################
// MARK: Registration

#[plugin_fn]
pub fn register_backend(_: Json<RegisterBackendInput>) -> FnResult<Json<RegisterBackendOutput>> {
  Ok(Json(RegisterBackendOutput {
    backend_id: get_plugin_id()?,
    ..Default::default()
  }))
}

#[plugin_fn]
pub fn register_tool(Json(input): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
  Package::parse(&input.id).map_err(|message| plugin_err!("{message}"))?;
  backend_config()?;
  Ok(Json(RegisterToolOutput {
    name: format!("uv:{}", input.id),
    type_of: PluginType::CommandLine,
    requires: vec!["uv".into()],
    minimum_proto_version: Some(Version::new(0, 60, 0)),
    plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
    inventory_options: ToolInventoryOptions {
      scoped_backend_dir: true,
      ..Default::default()
    },
    lock_options: ToolLockOptions {
      no_record: true,
      ..Default::default()
    },
    ..Default::default()
  }))
}

#[plugin_fn]
pub fn define_backend_config() -> FnResult<Json<DefineBackendConfigOutput>> {
  Ok(Json(DefineBackendConfigOutput {
    schema: SchemaBuilder::build_root::<UvBackendConfig>(),
  }))
}

fn backend_config() -> FnResult<UvBackendConfig> {
  let config = get_backend_config::<UvBackendConfig>()?;
  config
    .validate()
    .map_err(|message| plugin_err!("{message}"))?;
  Ok(config)
}

fn package_name() -> FnResult<Package> {
  Package::parse(&get_plugin_id()?).map_err(|message| plugin_err!("{message}"))
}

// #############################################################################
// MARK: Version discovery

#[plugin_fn]
pub fn load_versions(_: Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
  let package = package_name()?;
  let project: Project = fetch_json(format!("https://pypi.org/pypi/{}/json", package.name))?;
  let output = version::load_versions(project.available_versions())
    .map_err(|message| plugin_err!("{}: {message}", package.name))?;
  Ok(Json(output))
}

// #############################################################################
// MARK: Native installation

#[plugin_fn]
pub fn native_install(
  Json(input): Json<NativeInstallInput>,
) -> FnResult<Json<NativeInstallOutput>> {
  let config = backend_config()?;
  let package = package_name()?;
  let version =
    version::to_python(&input.context.version).map_err(|message| plugin_err!("{message}"))?;
  let real_dir = input
    .install_dir
    .to_real_path()?
    .ok_or_else(|| plugin_err!("The install directory has no host path."))?;
  std::fs::create_dir_all(input.install_dir.join("tmp"))?;
  let mut command = uv::install_command(
    &real_dir.to_string(),
    &package.requirement(),
    &version,
    &config,
    input.force,
  );
  command.cwd = Some(input.install_dir);
  let result = exec(command)?;
  Ok(Json(NativeInstallOutput {
    installed: result.exit_code == 0,
    error: (result.exit_code != 0).then(|| {
      format!(
        "uv tool install failed (exit {}):\n{}\n{}",
        result.exit_code, result.stderr, result.stdout
      )
    }),
    ..Default::default()
  }))
}

// #############################################################################
// MARK: Executable discovery

#[plugin_fn]
pub fn locate_executables(
  Json(input): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
  let env = get_host_environment()?;
  let package = package_name()?;
  let mut files = Vec::new();
  // proto may refresh shim registrations after removing an installation.
  if !input.install_dir.exists() {
    return Ok(Json(LocateExecutablesOutput::default()));
  }
  for entry in std::fs::read_dir(input.install_dir.join("bin"))? {
    let entry = entry?;
    let kind = entry.file_type()?;
    if kind.is_file() || kind.is_symlink() {
      files.push(
        entry
          .file_name()
          .into_string()
          .map_err(|_| plugin_err!("uv exported a non-UTF-8 command name."))?,
      );
    }
  }
  Ok(Json(
    uv::executable_config(&package.name, files, env.os.is_windows())
      .map_err(|message| plugin_err!("{message}"))?,
  ))
}
