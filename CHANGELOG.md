# Changelog

## Unreleased

- Add release builds that remap and check machine-specific source paths.

- Support canonical slash-separated extras combinations as separate proto tools.
- Move integration tests into Podman and remove workspace-local setup scripts.

## 0.1.0

- Add a proto backend for public PyPI CLI packages installed by uv.
- Isolate tool environments, managed Python, caches, and exported commands per
  package version.
- Add supported PEP 440 version conversion and deterministic command selection.
- Add workspace-local setup scripts, unit tests, and Podman integration tests.
