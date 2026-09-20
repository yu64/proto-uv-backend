//! Explicit conversion of the supported PEP 440 subset to proto versions.

use pep440_rs::{PrereleaseKind, Version as PythonVersion};
use proto_pdk_api::{LoadVersionsOutput, VersionSpec};
use std::collections::BTreeSet;

// #############################################################################
// MARK: Version conversion

/// Numeric releases and a/b/rc prereleases have an unambiguous semver mapping.
/// Epoch, post, dev, local, and four-part releases are intentionally excluded.
pub fn to_proto(value: &str) -> Option<VersionSpec> {
  let version: PythonVersion = value.parse().ok()?;
  if version.epoch() != 0
    || version.release().len() > 3
    || version.post().is_some()
    || version.dev().is_some()
    || version.is_local()
  {
    return None;
  }
  let release = version.release();
  let mut mapped = format!(
    "{}.{}.{}",
    release[0],
    release.get(1).unwrap_or(&0),
    release.get(2).unwrap_or(&0)
  );
  if let Some(pre) = version.pre() {
    let kind = match pre.kind {
      PrereleaseKind::Alpha => "a",
      PrereleaseKind::Beta => "b",
      PrereleaseKind::Rc => "rc",
    };
    mapped.push_str(&format!("-{kind}.{}", pre.number));
  }
  VersionSpec::parse(mapped).ok()
}

pub fn to_python(spec: &VersionSpec) -> Result<String, String> {
  let invalid = || {
    "Use a numeric release (1.2.3) or an a/b/rc prerelease (1.2.3-rc.1). Epoch, post, dev, local, and four-part versions are not supported.".to_string()
  };
  let version = spec.as_version().ok_or_else(invalid)?;
  if version.scope.is_some() || version.build.is_some() {
    return Err(invalid());
  }
  let mut python = format!("{}.{}.{}", version.major, version.minor, version.patch);
  if let Some(pre) = &version.prerelease {
    let (kind, number) = pre.split_once('.').ok_or_else(invalid)?;
    if !matches!(kind, "a" | "b" | "rc")
      || number.is_empty()
      || !number.bytes().all(|b| b.is_ascii_digit())
    {
      return Err(invalid());
    }
    python.push_str(&format!("{kind}{number}"));
  }
  Ok(python)
}

// #############################################################################
// MARK: Version discovery

pub fn load_versions<'a>(
  values: impl Iterator<Item = &'a str>,
) -> Result<LoadVersionsOutput, String> {
  let versions: BTreeSet<_> = values.filter_map(to_proto).collect();
  if versions.is_empty() {
    return Err("No supported, non-yanked wheel releases were found on PyPI.".into());
  }
  let latest = versions
    .iter()
    .rev()
    .find(|spec| spec.as_version().is_some_and(|v| v.prerelease.is_none()))
    .map(VersionSpec::to_unresolved_spec);
  let mut output = LoadVersionsOutput {
    versions: versions.into_iter().collect(),
    latest: latest.clone(),
    ..Default::default()
  };
  if let Some(latest) = latest {
    output.aliases.insert("latest".into(), latest);
  }
  Ok(output)
}

// #############################################################################
// MARK: Tests

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn maps_common_python_versions_without_losing_prerelease_order() {
    for (python, proto) in [
      ("1", "1.0.0"),
      ("24.10", "24.10.0"),
      ("1.2.3a2", "1.2.3-a.2"),
      ("1.2.3rc12", "1.2.3-rc.12"),
    ] {
      let spec = to_proto(python).unwrap();
      assert_eq!(spec.to_string(), proto);
      let roundtrip: PythonVersion = to_python(&spec).unwrap().parse().unwrap();
      assert_eq!(roundtrip, python.parse::<PythonVersion>().unwrap());
    }
  }

  #[test]
  fn unsupported_versions_are_never_silently_changed() {
    for value in [
      "1!2.0",
      "1.2.post1",
      "1.2.dev1",
      "1.2+local",
      "1.2.3.4",
      "garbage",
    ] {
      assert!(to_proto(value).is_none(), "{value}");
    }
    for value in ["latest", "canary", "1.2.3+local", "1.2.3-preview.1"] {
      assert!(to_python(&VersionSpec::parse(value).unwrap()).is_err());
    }
  }

  #[test]
  fn latest_ignores_prereleases_and_duplicates() {
    let output = load_versions(["1.0", "1.0.0", "2.0rc1", "1.9", "broken"].into_iter()).unwrap();
    assert_eq!(output.versions.len(), 3);
    assert_eq!(output.latest.unwrap().to_string(), "1.9.0");
    assert!(
      load_versions(["1.0rc1"].into_iter())
        .unwrap()
        .latest
        .is_none()
    );
    assert!(load_versions(["1.0.post1"].into_iter()).is_err());
  }
}
