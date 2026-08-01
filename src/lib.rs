// Portions of this file are derived from Tauri:
// https://github.com/tauri-apps/tauri
//
// Copyright 2019-2025 The Tauri Programme
// within The Commons Conservancy
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

pub use http::{
    header::{InvalidHeaderName, InvalidHeaderValue},
    method::InvalidMethod,
    status::InvalidStatusCode,
};
use std::path::PathBuf;
use tao::error::OsError;
pub use tao;
pub use wry;
#[cfg(windows)]
pub use webview2_com;
#[cfg(windows)]
pub use windows;
#[cfg(target_os = "linux")]
pub use gtk;
#[cfg(target_vendor = "apple")]
pub use objc2_app_kit; 
pub mod build;
pub mod dialog;
pub mod event;
pub mod monitor;
pub mod types;
pub mod undecorated_resizing;
pub mod utils;
pub mod web;
pub mod window;
pub mod wrapper;
/// The result type of `tauri-utils`.
pub type Result<T> = std::result::Result<T, Error>;

/// The error type of `tauri-utils`.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Failed to create the webview.
    #[error("failed to create webview: {0}")]
    CreateWebview(#[source] Box<dyn std::error::Error + Send + Sync>),

    /// Failed to create the window.
    #[error("failed to create window: {0}")]
    CreateWindow(#[from] OsError),
    /// Target triple architecture error
    #[error("Unable to determine target-architecture")]
    Architecture,
    /// Target triple OS error
    #[error("Unable to determine target-os")]
    Os,
    /// Target triple environment error
    #[error("Unable to determine target-environment")]
    Environment,
    /// Tried to get resource on an unsupported platform
    #[error("Unsupported platform for reading resources")]
    UnsupportedPlatform,
    /// Get parent process error
    #[error("Could not get parent process")]
    ParentProcess,
    /// Get parent process PID error
    #[error("Could not get parent PID")]
    ParentPid,
    /// Get child process error
    #[error("Could not get child process")]
    ChildProcess,
    /// IO error
    #[error("{0}")]
    Io(#[from] std::io::Error),
    /// Invalid pattern.
    #[error("invalid pattern `{0}`. Expected either `brownfield` or `isolation`.")]
    InvalidPattern(String),
    /// Invalid glob pattern.
    #[error("{0}")]
    GlobPattern(#[from] glob::PatternError),
    /// Failed to use glob pattern.
    #[error("`{0}`")]
    Glob(#[from] glob::GlobError),
    /// Glob pattern did not find any results.
    #[error("glob pattern {0} path not found or didn't match any files.")]
    GlobPathNotFound(String),
    /// Error walking directory.
    #[error("{0}")]
    WalkdirError(#[from] walkdir::Error),
    /// Not allowed to walk dir.
    #[error(
        "could not walk directory `{0}`, try changing `allow_walk` to true on the `ResourcePaths` constructor."
    )]
    NotAllowedToWalkDir(std::path::PathBuf),
    /// Resource path doesn't exist
    #[error("resource path `{0}` doesn't exist")]
    ResourcePathNotFound(std::path::PathBuf),
    /// The image file extension is not supported.
    #[error(
        "unsupported image extension `{extension:?}` for image `{path:?}`; \
         expected `ico` or `png`"
    )]
    InvalidImageExtension { extension: PathBuf, path: PathBuf },

    /// Failed to serialize or deserialize JSON data.
    #[error("failed to process JSON data: {0}")]
    Json(#[from] serde_json::Error),

    /// Failed to get the current cursor position.
    #[error("failed to retrieve the current cursor position")]
    FailedToGetCursorPosition,

    /// Invalid HTTP header name.
    #[error("invalid HTTP header name: {0}")]
    InvalidHeaderName(#[from] InvalidHeaderName),

    /// Invalid HTTP header value.
    #[error("invalid HTTP header value: {0}")]
    InvalidHeaderValue(#[from] InvalidHeaderValue),

    /// Invalid HTTP status code.
    #[error("invalid HTTP status code: {0}")]
    InvalidStatusCode(#[from] InvalidStatusCode),

    /// Invalid HTTP method.
    #[error("invalid HTTP method: {0}")]
    InvalidMethod(#[from] InvalidMethod),

    /// An infallible operation unexpectedly failed.
    #[error("an unexpected infallible error occurred: {0}")]
    Infallible(#[from] std::convert::Infallible),

    /// Invalid proxy URL.
    #[error("invalid proxy URL")]
    InvalidProxyUrl,

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    /// Failed to remove the webview data store.
    #[error("failed to remove the webview data store")]
    FailedToRemoveDataStore,

    /// The required webview runtime is not installed.
    #[error(
        "webview runtime not found; \
         please make sure the required runtime is installed"
    )]
    WebviewRuntimeNotInstalled,

    /// Window label must be unique.
    #[error("a window with the label `{0}` already exists")]
    WindowLabelAlreadyExists(String),

    /// Webview label must be unique.
    #[error("a webview with the label `{0}` already exists")]
    WebviewLabelAlreadyExists(String),
    #[error("WebView creation failed: {0}")]
    WebViewCreationFailed(String),

    #[error("Failed to lock context store")]
    ContextLockFailed,

    /// Failed to load or validate a window icon from an IO source.
    #[error("failed to load icon: {0}")]
    InvalidIcon(#[source] std::io::Error),

    /// Failed to validate a Tao window icon.
    #[error("invalid window icon: {0}")]
    InvalidTaoIcon(#[source] Box<tao::window::BadIcon>),

    /// A URL is malformed or invalid.
    #[error("invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    /// The requested operation is not supported.
    #[error("operation is not supported: {0}")]
    NotSupportedError(tao::error::NotSupportedError),

    /// An external error occurred outside of Tao's control.
    #[error("external platform error: {0}")]
    ExternalError(tao::error::ExternalError),
}

fn main() {}
