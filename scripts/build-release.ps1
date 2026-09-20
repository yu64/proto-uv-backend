# Build a release WASM with local source paths replaced by /build.
$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot

# Restore the caller's environment and working directory when finished,
# including when the build or the path check fails.
$previousFlags = $env:CARGO_ENCODED_RUSTFLAGS
$previousTarget = $env:CARGO_TARGET_DIR

Push-Location $root

try {
  # Cargo's encoded flags use ASCII 31 as a separator. Unlike spaces,
  # this separator allows a single flag to contain a path with spaces.
  $flagSeparator = [char]31
  $flags = @()

  if ($previousFlags) {
    $flags += $previousFlags.Split($flagSeparator)
  } elseif ($env:RUSTFLAGS) {
    $flags += $env:RUSTFLAGS -split '\s+' |
      Where-Object { $_ }
  }

  # Include custom tool homes as well as the user's home and workspace.
  # Add longer prefixes last: rustc uses the last matching remap rule.
  $paths = @(
    $HOME
    $env:CARGO_HOME
    $env:RUSTUP_HOME
    $root
  ) |
    Where-Object { $_ } |
    ForEach-Object { [IO.Path]::GetFullPath($_) } |
    Sort-Object -Unique |
    Sort-Object Length

  foreach ($path in $paths) {
    # Source paths can use either Windows or forward-slash separators.
    $pathForms = @(
      $path
      $path.Replace('\', '/')
    ) | Select-Object -Unique

    foreach ($form in $pathForms) {
      $flags += "--remap-path-prefix=$form=/build"
    }
  }

  $env:CARGO_ENCODED_RUSTFLAGS = $flags -join $flagSeparator
  $env:CARGO_TARGET_DIR = Join-Path $root 'target'

  # Use the Rust version selected by the project's .prototools file.
  $buildArgs = @(
    'run'
    'rust'
    '--'
    'build'
    '--locked'
    '--target'
    'wasm32-wasip1'
    '--release'
  )

  try {
    # Windows PowerShell can treat native stderr as error records,
    # even for normal compiler progress. Check the exit code instead.
    $ErrorActionPreference = 'Continue'

    proto @buildArgs 2>&1 |
      ForEach-Object { $_.ToString() }

    $buildExit = $LASTEXITCODE
  } finally {
    $ErrorActionPreference = 'Stop'
  }

  if ($buildExit -ne 0) {
    throw 'WASM build failed'
  }

  # Inspect the generated binary for any original path prefixes.
  # Do not print a matching path: it may contain private information.
  $artifactDirectory = Join-Path $env:CARGO_TARGET_DIR `
    'wasm32-wasip1/release'
  $artifact = Join-Path $artifactDirectory 'proto_uv_backend.wasm'

  $bytes = [IO.File]::ReadAllBytes($artifact)
  $text = [Text.Encoding]::UTF8.GetString($bytes)

  foreach ($path in $paths) {
    $hasNativePath = $text.Contains($path)
    $hasForwardPath = $text.Contains($path.Replace('\', '/'))

    if ($hasNativePath -or $hasForwardPath) {
      throw (
        'WASM still contains a machine-specific path; ' +
        'do not distribute it.'
      )
    }
  }

  Write-Output 'Release WASM built and checked for machine-specific paths.'
} finally {
  $env:CARGO_ENCODED_RUSTFLAGS = $previousFlags
  $env:CARGO_TARGET_DIR = $previousTarget

  Pop-Location
}
