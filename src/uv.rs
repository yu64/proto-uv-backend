//! uv command construction and executable registration, independent of WASM.

use crate::config::UvBackendConfig;
use crate::package::normalize_name;
use proto_pdk_api::{ExecCommandInput, ExecutableConfig, LocateExecutablesOutput};
use std::collections::BTreeMap;

// #############################################################################
// MARK: Isolated installation

pub fn install_command(
  root: &str,
  package: &str,
  version: &str,
  config: &UvBackendConfig,
  force: bool,
) -> ExecCommandInput {
  let mut command = ExecCommandInput {
    command: "uv".into(),
    args: vec![
      "--no-config".into(),
      "--no-progress".into(),
      "tool".into(),
      "install".into(),
      "--managed-python".into(),
      "--python".into(),
      config.python.clone(),
      "--no-build".into(),
      "--default-index".into(),
      "https://pypi.org/simple".into(),
      "--index-strategy".into(),
      "first-index".into(),
    ],
    stream: false,
    ..Default::default()
  };
  if force {
    command.args.push("--reinstall".into());
  }
  command
    .args
    .extend(["--".into(), format!("{package}=={version}")]);
  for (key, suffix) in [
    ("UV_TOOL_DIR", "tools"),
    ("UV_TOOL_BIN_DIR", "bin"),
    ("UV_PYTHON_INSTALL_DIR", "python"),
    ("UV_CACHE_DIR", "cache"),
    ("UV_PYTHON_CACHE_DIR", "cache/python"),
    ("TMPDIR", "tmp"),
    ("TMP", "tmp"),
    ("TEMP", "tmp"),
  ] {
    command.env.insert(key.into(), format!("{root}/{suffix}"));
  }
  for (key, value) in [
    ("PROTO_UV_VERSION?", "*"),
    ("UV_PYTHON_INSTALL_BIN", "0"),
    ("UV_PYTHON_INSTALL_REGISTRY", "0"),
    ("UV_NO_MODIFY_PATH", "1"),
    ("UV_PYTHON_DOWNLOADS", "automatic"),
    ("UV_LINK_MODE", "copy"),
    ("UV_INDEX", ""),
    ("UV_INDEX_URL", ""),
    ("UV_EXTRA_INDEX_URL", ""),
    ("UV_FIND_LINKS", ""),
    ("UV_NO_INDEX", "false"),
  ] {
    command.env.insert(key.into(), value.into());
  }
  command
}

// #############################################################################
// MARK: Executables

/// Only inspect uv's dedicated exported bin directory, never a venv's bin.
pub fn executable_config(
  package: &str,
  files: Vec<String>,
  windows: bool,
) -> Result<LocateExecutablesOutput, String> {
  let mut executables = BTreeMap::new();
  for file in files {
    if file.is_empty() || file.contains(['/', '\\']) || file.starts_with('.') {
      continue;
    }
    let name = if windows {
      let Some((stem, extension)) = file.rsplit_once('.') else {
        continue;
      };
      if !["exe", "cmd", "bat"]
        .iter()
        .any(|allowed| extension.eq_ignore_ascii_case(allowed))
      {
        continue;
      }
      stem.to_string()
    } else {
      file.clone()
    };
    if name.is_empty()
      || !name
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
    {
      return Err(format!("Unsupported executable name: {file:?}"));
    }
    if executables.insert(name.clone(), file).is_some() {
      return Err(format!(
        "Multiple uv executables have the same command name: {name}"
      ));
    }
  }
  let primary = executables
    .keys()
    .find(|name| normalize_name(name).is_ok_and(|name| name == package))
    .or_else(|| executables.keys().next())
    .cloned()
    .ok_or_else(|| format!("The package {package} did not export any supported commands."))?;

  let mut output = LocateExecutablesOutput::default();
  for (name, file) in executables {
    let mut config = ExecutableConfig::new(format!("bin/{file}"));
    config.primary = name == primary;
    config.no_bin = true;
    output.exes.insert(name, config);
  }
  output.exes_dirs.push("bin".into());
  Ok(output)
}

// #############################################################################
// MARK: Tests

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn extras_are_part_of_the_pinned_requirement() {
    let package = crate::package::Package::parse("black/colorama/d").unwrap();
    let command = install_command(
      "/tool",
      &package.requirement(),
      "25.1.0",
      &UvBackendConfig::default(),
      false,
    );
    assert_eq!(command.args.last().unwrap(), "black[colorama,d]==25.1.0");
    assert_eq!(command.args[command.args.len() - 2], "--");
  }

  #[test]
  fn install_keeps_paths_and_package_arguments_separate() {
    let command = install_command(
      "C:/work space/tool",
      "httpie",
      "3.2.4",
      &UvBackendConfig::default(),
      false,
    );
    assert_eq!(command.command, "uv");
    assert_eq!(command.args.last().unwrap(), "httpie==3.2.4");
    assert!(command.args.contains(&"--no-build".into()));
    assert!(command.args.contains(&"--managed-python".into()));
    for key in [
      "UV_TOOL_DIR",
      "UV_TOOL_BIN_DIR",
      "UV_PYTHON_INSTALL_DIR",
      "UV_CACHE_DIR",
      "TEMP",
    ] {
      assert!(command.env[key].starts_with("C:/work space/tool/"), "{key}");
    }
  }

  #[test]
  fn forced_install_reinstalls_in_the_same_isolated_environment() {
    let command = install_command(
      "/tool",
      "ruff",
      "0.11.13",
      &UvBackendConfig::default(),
      true,
    );
    let reinstall = command
      .args
      .iter()
      .position(|arg| arg == "--reinstall")
      .unwrap();
    let separator = command.args.iter().position(|arg| arg == "--").unwrap();
    assert!(reinstall < separator);
    assert_eq!(command.env["UV_TOOL_DIR"], "/tool/tools");
  }

  #[test]
  fn registers_all_commands_and_chooses_the_package_command() {
    let output = executable_config(
      "httpie",
      vec!["https".into(), "httpie".into(), "http".into()],
      false,
    )
    .unwrap();
    assert_eq!(output.exes.len(), 3);
    assert!(output.exes["httpie"].primary);
    assert!(!output.exes["http"].primary);
    assert_eq!(output.exes_dirs, [std::path::PathBuf::from("bin")]);
  }

  #[test]
  fn chooses_a_deterministic_primary_when_the_name_differs() {
    for files in [vec!["z".into(), "a".into()], vec!["a".into(), "z".into()]] {
      let output = executable_config("different-package", files, false).unwrap();
      assert!(output.exes["a"].primary);
    }
  }

  #[test]
  fn windows_names_strip_only_launcher_extensions() {
    let output = executable_config(
      "ruff",
      vec!["ruff.exe".into(), "helper.cmd".into(), "helper.ps1".into()],
      true,
    )
    .unwrap();
    assert_eq!(output.exes.len(), 2);
    assert_eq!(
      output.exes["ruff"]
        .exe_path
        .as_ref()
        .unwrap()
        .to_str()
        .unwrap(),
      "bin/ruff.exe"
    );
    assert!(output.exes["ruff"].primary);
    assert!(executable_config("a", vec!["a.exe".into(), "a.cmd".into()], true).is_err());
  }

  #[test]
  fn empty_exports_and_invalid_paths_are_not_registered() {
    assert!(executable_config("a", vec![], false).is_err());
    assert!(executable_config("a", vec!["../a".into(), ".lock".into()], false).is_err());
  }
}
