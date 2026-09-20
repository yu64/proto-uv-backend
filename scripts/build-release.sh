#!/usr/bin/env bash
# Build a release WASM with local source paths replaced by /build.
set -euo pipefail

# Run this script with bash; environment changes stay in its process.
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
root=$PWD

# Include custom tool homes as well as the user's home and workspace.
paths=(
  "$HOME"
  "${CARGO_HOME:-$HOME/.cargo}"
  "${RUSTUP_HOME:-$HOME/.rustup}"
  "$root"
)

# Preserve existing compiler flags. Cargo's encoded form uses ASCII 31
# between flags, so paths containing spaces remain a single argument.
flags=${CARGO_ENCODED_RUSTFLAGS:-}

if [[ -z "$flags" && -n "${RUSTFLAGS:-}" ]]; then
  read -r -a existing_flags <<< "$RUSTFLAGS"
  printf -v flags '%s\x1f' "${existing_flags[@]}"

  # Remove the trailing separator added by printf.
  flags=${flags%$'\x1f'}
fi

for path in "${paths[@]}"; do
  # Resolve symbolic links before passing the prefix to rustc.
  path=$(cd -- "$path" && pwd -P)

  if [[ -n "$flags" ]]; then
    flags+=$'\x1f'
  fi

  flags+="--remap-path-prefix=$path=/build"
done

export CARGO_ENCODED_RUSTFLAGS="$flags"
export CARGO_TARGET_DIR="$root/target"

# Use the Rust version selected by the project's .prototools file.
proto run rust -- build \
  --locked \
  --target wasm32-wasip1 \
  --release

# Inspect the generated binary for any original path prefixes.
# Do not print a matching path: it may contain private information.
artifact_directory="$CARGO_TARGET_DIR/wasm32-wasip1/release"
artifact="$artifact_directory/proto_uv_backend.wasm"

for path in "${paths[@]}"; do
  path=$(cd -- "$path" && pwd -P)

  # -a reads the binary as text; -F matches literally; -q hides matches.
  if LC_ALL=C grep -aFq -- "$path" "$artifact"; then
    echo 'WASM still contains a machine-specific path;' \
      'do not distribute it.' >&2
    exit 1
  fi
done

echo 'Release WASM built and checked for machine-specific paths.'
