//! Package names and the public PyPI release metadata used for discovery.

use serde::Deserialize;
use std::collections::BTreeMap;

// #############################################################################
// MARK: Package names

#[derive(Debug, PartialEq, Eq)]
pub struct Package {
  pub name: String,
  pub extras: Vec<String>,
}

impl Package {
  /// Slash-separated extras fit proto's identifier grammar. Require canonical
  /// identifiers because proto owns the tool identity before calling our hooks.
  pub fn parse(id: &str) -> Result<Self, String> {
    let mut parts = id.split('/');
    let name = normalize_name(parts.next().unwrap_or_default())?;
    let mut extras = parts.map(normalize_name).collect::<Result<Vec<_>, _>>()?;
    extras.sort();
    extras.dedup();
    let package = Self { name, extras };
    let canonical = if package.extras.is_empty() {
      package.name.clone()
    } else {
      format!("{}/{}", package.name, package.extras.join("/"))
    };
    if canonical != id {
      return Err(format!("Use the canonical tool identifier uv:{canonical}"));
    }
    Ok(package)
  }

  pub fn requirement(&self) -> String {
    if self.extras.is_empty() {
      self.name.clone()
    } else {
      format!("{}[{}]", self.name, self.extras.join(","))
    }
  }
}

/// Accept distribution names only; never pass URLs, extras, or options to uv.
pub fn normalize_name(value: &str) -> Result<String, String> {
  let valid_edge = |byte: u8| byte.is_ascii_alphanumeric();
  if value.is_empty()
    || !valid_edge(value.as_bytes()[0])
    || !valid_edge(*value.as_bytes().last().unwrap())
    || !value
      .bytes()
      .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
  {
    return Err(format!(
      "Invalid PyPI package name: {value:?}. Use a distribution name without extras or a URL."
    ));
  }

  let mut normalized = String::new();
  for byte in value.bytes() {
    if b"-_.".contains(&byte) {
      if !normalized.ends_with('-') {
        normalized.push('-');
      }
    } else {
      normalized.push(byte.to_ascii_lowercase() as char);
    }
  }
  Ok(normalized)
}

// #############################################################################
// MARK: PyPI JSON API

#[derive(Debug, Deserialize)]
pub struct Project {
  pub releases: BTreeMap<String, Vec<ReleaseFile>>,
}

#[derive(Debug, Deserialize)]
pub struct ReleaseFile {
  #[serde(default)]
  pub yanked: bool,
  pub packagetype: String,
}

impl Project {
  /// Source builds are deliberately disabled; require a non-yanked wheel.
  pub fn available_versions(&self) -> impl Iterator<Item = &str> {
    self.releases.iter().filter_map(|(version, files)| {
      files
        .iter()
        .any(|file| !file.yanked && file.packagetype == "bdist_wheel")
        .then_some(version.as_str())
    })
  }
}

// #############################################################################
// MARK: Tests

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn extras_have_canonical_identity_and_python_requirement() {
    let package = Package::parse("black/colorama/d").unwrap();
    assert_eq!(package.name, "black");
    assert_eq!(package.requirement(), "black[colorama,d]");
    assert_eq!(Package::parse("black").unwrap().requirement(), "black");
    for id in ["black/d/colorama", "black/d/d", "Black/D", "black/dev_test"] {
      assert!(Package::parse(id).unwrap_err().contains("canonical"));
    }
    for id in [
      "black_colorama",
      "Black",
      "black/",
      "black//d",
      "black/../d",
      "black/d,other",
      "black[d]",
      "https://x",
      "black/--index",
    ] {
      assert!(Package::parse(id).is_err(), "{id}");
    }
  }

  #[test]
  fn normalizes_python_distribution_names() {
    assert_eq!(
      normalize_name("My_Package.Name").unwrap(),
      "my-package-name"
    );
    assert_eq!(normalize_name("a...___--b").unwrap(), "a-b");
    for name in [
      "",
      "-pip",
      "../pip",
      "foo/../bar",
      "foo[extra]",
      "foo==1",
      "https://x",
      "a b",
      "a\n",
      "é",
    ] {
      assert!(normalize_name(name).is_err(), "{name:?}");
    }
  }

  #[test]
  fn excludes_empty_yanked_and_source_only_releases() {
    let project: Project = serde_json::from_str(
      r#"{"releases": {
      "1.0": [],
      "1.1": [{"packagetype":"bdist_wheel","yanked":true}],
      "1.2": [{"packagetype":"sdist","yanked":false}],
      "1.3": [{"packagetype":"bdist_wheel","yanked":false}]
    }}"#,
    )
    .unwrap();
    assert_eq!(project.available_versions().collect::<Vec<_>>(), ["1.3"]);
  }
}
