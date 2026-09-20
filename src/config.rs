//! Backend settings shared by the host tests and WASM hooks.

use serde::Deserialize;

// #############################################################################
// MARK: Backend settings

#[derive(Debug, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(schematic::Schematic))]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct UvBackendConfig {
  /// Managed Python version used to create each tool environment.
  pub python: String,
}

impl Default for UvBackendConfig {
  fn default() -> Self {
    Self {
      python: "3.12".into(),
    }
  }
}

impl UvBackendConfig {
  pub fn validate(&self) -> Result<(), String> {
    let parts: Vec<_> = self.python.split('.').collect();
    if !(2..=3).contains(&parts.len())
      || parts[0] != "3"
      || parts
        .iter()
        .any(|part| part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()))
    {
      return Err("The python setting must be a Python 3 version, such as 3.12 or 3.12.9.".into());
    }
    Ok(())
  }
}

// #############################################################################
// MARK: Tests

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn accepts_versions_but_not_paths_or_arguments() {
    for python in ["3.12", "3.13.2"] {
      assert!(
        UvBackendConfig {
          python: python.into()
        }
        .validate()
        .is_ok()
      );
    }
    for python in ["python", "../python", "3", "3.12 --system", "2.7", "3..1"] {
      assert!(
        UvBackendConfig {
          python: python.into()
        }
        .validate()
        .is_err()
      );
    }
  }

  #[test]
  fn rejects_misspelled_settings() {
    assert!(serde_json::from_str::<UvBackendConfig>(r#"{"pythno":"3.12"}"#).is_err());
    let config: UvBackendConfig = serde_json::from_str("{}").unwrap();
    assert_eq!(config.python, "3.12");
  }
}
