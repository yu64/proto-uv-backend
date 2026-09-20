# uv Proto Backend

Install Python CLI tools from public PyPI through proto and uv.
Requires proto **0.60.0+**. Supports wheels only and a subset of PEP 440 versions.

## Usage

After publishing the WASM asset to [GitHub Releases](https://github.com/yu64/proto-uv-backend/releases),
add this to `.prototools`:

```toml
uv = "0.12.17"
"uv:ruff" = "0.11.13"

[plugins.backends]
uv = "github://yu64/proto-uv-backend"
```

Append `@<release-tag>` to the locator to pin a plugin release.

```sh
proto use
proto run uv:ruff -- --version
```

## Versions

Supports one to three numeric components (`24.10` becomes `24.10.0`) and
alpha/beta/RC releases (`1.2.3rc1` is specified as `1.2.3-rc.1` in proto).
`latest` selects a stable release.

Unsupported: nonzero epochs (`1!2.0`), post releases (`1.2.post1`), development
releases (`1.2.dev1`), local versions (`1.2+custom`), and four or more numeric
components (`1.2.3.4`). These releases are excluded from version discovery.

## Extras

Use `/` for extras, with normalized names sorted and deduplicated:

```toml
"uv:black/colorama/d" = "25.1.0" # black[colorama,d]==25.1.0
```

Each combination is a separate tool. Select it with `proto run uv:black/colorama/d`.

## Local build

```sh
proto use
rustup target add wasm32-wasip1 --toolchain 1.94.0
bash scripts/build-release.sh
```

On PowerShell, use `./scripts/build-release.ps1` instead of the Bash command.
The release scripts remap local paths and check the WASM before distribution.

Replace the GitHub locator with the local artifact path:

```toml
[plugins.backends]
uv = "file://./target/wasm32-wasip1/release/proto_uv_backend.wasm"
```

## Tests

```sh
podman build -f proto_test/Dockerfile -t uv-backend-test .
podman run --rm uv-backend-test
```
