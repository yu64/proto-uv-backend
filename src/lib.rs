//! A proto backend for isolated Python CLI installations managed by uv.

// #############################################################################
// MARK: Modules

#[cfg(any(test, all(feature = "wasm", target_arch = "wasm32")))]
mod config;
#[cfg(any(test, all(feature = "wasm", target_arch = "wasm32")))]
mod package;
#[cfg(any(test, all(feature = "wasm", target_arch = "wasm32")))]
mod uv;
#[cfg(any(test, all(feature = "wasm", target_arch = "wasm32")))]
mod version;

#[cfg(all(feature = "wasm", target_arch = "wasm32"))]
mod proto;

// #############################################################################
// MARK: WASM exports

#[cfg(all(feature = "wasm", target_arch = "wasm32"))]
pub use proto::*;
