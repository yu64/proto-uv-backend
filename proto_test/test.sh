#!/usr/bin/env bash

# #############################################################################
# MARK: Test environment

set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
# This script installs and removes tools; run through the test Dockerfile.
if [[ "${PROTO_UV_TEST_CONTAINER:-0}" != 1 ]]; then
  echo 'Run integration tests with Podman using proto_test/Dockerfile.' >&2
  exit 1
fi
: "${PROTO_HOME:?The test container must set PROTO_HOME}"
export PROTO_CONFIG_MODE=local
export PROTO_REPORTER=text

# These sentinel directories must remain untouched by the backend.
export UV_TOOL_DIR="$PROTO_HOME/ambient-tools"
export UV_TOOL_BIN_DIR="$PROTO_HOME/ambient-bin"
export UV_CACHE_DIR="$PROTO_HOME/ambient-cache"
export UV_PYTHON_INSTALL_DIR="$PROTO_HOME/ambient-python"
export UV_PYTHON_INSTALL_BIN=0
export UV_NO_MODIFY_PATH=1
export HTTPIE_CONFIG_DIR="$PROTO_HOME/httpie-config"

on_error() {
  for log in ./*-install.log; do
    if [[ -f "$log" ]]; then cat "$log" >&2; fi
  done
}
trap on_error ERR

assert_output() {
  local expected=$1
  shift
  local actual
  actual=$("$@")
  if [[ "$actual" != "$expected" ]]; then
    printf 'Expected: %s\nActual: %s\n' "$expected" "$actual" >&2
    return 1
  fi
}

# #############################################################################
# MARK: Installation and multiple commands

proto --version
proto use
assert_output 'ruff 0.11.13' proto run uv:ruff -- --version
assert_output '3.2.4' proto run uv:httpie --exe http -- --version
assert_output '3.2.4' proto run uv:httpie --exe https -- --version
proto run uv:httpie -- --help >/dev/null

# The backend must also work without an explicit [backends.uv] section.
(cd defaults && assert_output 'ruff 0.11.13' proto run uv:ruff -- --version)

ruff_bin=$(proto bin uv:ruff)
ruff_dir=$(dirname -- "$(dirname -- "$ruff_bin")")
test -d "$ruff_dir/tools"
test -d "$ruff_dir/python"
test -d "$ruff_dir/cache"
test ! -e "$UV_TOOL_DIR"
test ! -e "$UV_TOOL_BIN_DIR"
test ! -e "$UV_CACHE_DIR"
test ! -e "$UV_PYTHON_INSTALL_DIR"
if [[ "${PROTO_UV_TEST_CONTAINER:-0}" == 1 ]]; then
  test ! -e "$HOME/.local/share/uv"
fi

# Repeated install must leave the existing tool usable.
proto use
assert_output 'ruff 0.11.13' proto run uv:ruff -- --version

# Force must also work when the environment already exists.
proto install uv:ruff 0.11.13 --force
assert_output 'ruff 0.11.13' proto run uv:ruff -- --version

# #############################################################################
# MARK: Version coexistence and uninstall

proto install uv:ruff 0.11.12
older_bin=$(proto bin uv:ruff 0.11.12)
older_dir=$(dirname -- "$(dirname -- "$older_bin")")
test "$older_dir" != "$ruff_dir"
assert_output 'ruff 0.11.12' proto run uv:ruff 0.11.12 -- --version
assert_output 'ruff 0.11.13' proto run uv:ruff -- --version
proto uninstall uv:ruff 0.11.12 --yes
test ! -e "$older_dir"
assert_output 'ruff 0.11.13' proto run uv:ruff -- --version

# #############################################################################
# MARK: Shell activation and shims

activation=$(proto activate bash --export)
eval "$activation"
assert_output 'ruff 0.11.13' ruff --version
assert_output '3.2.4' http --version
test "$(command -v ruff)" = "$ruff_bin"
case ":$PATH:" in
  *"/tools/ruff/bin:"*|*"/tools/httpie/bin:"*)
    echo 'Activation exposed a Python virtual environment bin directory' >&2
    exit 1
    ;;
esac
ruff_shim=$(proto bin uv:ruff --shim)
assert_output 'ruff 0.11.13' "$ruff_shim" --version

# #############################################################################
# MARK: Failure reporting

# A distribution without CLI entry points must not be reported as installed.
if proto install uv:six 1.17.0 >"$PROTO_HOME/no-cli.log" 2>&1; then
  echo 'Installing a package without commands unexpectedly succeeded' >&2
  exit 1
fi
grep -q 'executable\|entry point\|commands' "$PROTO_HOME/no-cli.log"
assert_output 'ruff 0.11.13' proto run uv:ruff -- --version

# Extras combinations have separate inventories, even at the same version.
proto install uv:black 25.1.0
proto install uv:black/colorama 25.1.0
proto install uv:black/colorama/d 25.1.0
base_bin=$(proto bin uv:black 25.1.0)
extra_bin=$(proto bin uv:black/colorama 25.1.0)
multi_bin=$(proto bin uv:black/colorama/d 25.1.0)
base_dir=$(dirname -- "$(dirname -- "$base_bin")")
extra_dir=$(dirname -- "$(dirname -- "$extra_bin")")
multi_dir=$(dirname -- "$(dirname -- "$multi_bin")")
test "$base_dir" != "$extra_dir"
test "$extra_dir" != "$multi_dir"
test "$base_dir" != "$multi_dir"
"$base_dir/tools/black/bin/python" -c 'import importlib.util; assert importlib.util.find_spec("colorama") is None; assert importlib.util.find_spec("aiohttp") is None'
"$extra_dir/tools/black/bin/python" -c 'import colorama; import importlib.util; assert importlib.util.find_spec("aiohttp") is None'
"$multi_dir/tools/black/bin/python" -c 'import colorama, aiohttp'
for tool in uv:black uv:black/colorama uv:black/colorama/d; do
  proto run "$tool" 25.1.0 -- --version | grep -q '25.1.0'
done
# Reject alternate spellings rather than creating duplicate tool identities.
if proto install uv:black/d/colorama 25.1.0 >"$PROTO_HOME/extras-order.log" 2>&1; then
  echo 'Noncanonical extras order unexpectedly succeeded' >&2
  exit 1
fi
grep -q 'uv:black/colorama/d' "$PROTO_HOME/extras-order.log"
# A plain underscore name must not alias the extras inventory.
if proto install uv:black_colorama 25.1.0 >"$PROTO_HOME/extras-collision.log" 2>&1; then
  echo 'A noncanonical plain identifier unexpectedly succeeded' >&2
  exit 1
fi
grep -q 'uv:black-colorama' "$PROTO_HOME/extras-collision.log"
proto install uv:black/colorama 25.1.0 --force
"$extra_dir/tools/black/bin/python" -c 'import colorama'
proto uninstall uv:black/colorama 25.1.0 --yes
test ! -e "$extra_dir"
proto run uv:black 25.1.0 -- --version | grep -q '25.1.0'
proto run uv:black/colorama/d 25.1.0 -- --version | grep -q '25.1.0'
"$multi_dir/tools/black/bin/python" -c 'import colorama, aiohttp'

echo 'proto uv backend integration checks passed'
