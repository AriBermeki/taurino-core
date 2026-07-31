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
#[cfg(target_os = "linux")]
mod monitor_linux;
#[cfg(target_os = "macos")]
mod monitor_macos;
#[cfg(windows)]
mod monitor_windows;
pub mod undecorated_resizing;
pub mod util;

#[cfg(target_os = "linux")]
mod window_linux;
#[cfg(target_os = "macos")]
mod window_macos;
#[cfg(windows)]
mod window_windows;

#[cfg(target_os = "linux")]
mod dialog_linux;
#[cfg(target_os = "macos")]
mod dialog_macos;
#[cfg(windows)]
mod dialog_windows;
mod window_builder;

pub use window_builder::{
    DragDropEvent, FrameBuilder, TaoIcon, WindowEvent, WindowEventHandler, WindowEventListener,
};

pub mod dpi;
pub mod starting_binary;

pub mod dialog {

    #[cfg(target_os = "linux")]
    use super::dialog_linux::error;
    #[cfg(target_os = "macos")]
    use super::dialog_macos::error;
    #[cfg(windows)]
    use super::dialog_windows::error;
}
pub mod monitor {
    #[cfg(target_os = "linux")]
    use super::monitor_linux;
    #[cfg(target_os = "macos")]
    use super::monitor_macos;
    #[cfg(windows)]
    use super::monitor_windows;

    pub trait MonitorExt {
        /// Get the work area of this monitor
        ///
        /// ## Platform-specific:
        ///
        /// - **Android / iOS**: Unsupported.
        fn work_area(&self) -> crate::dpi::PhysicalRect<i32, u32>;
    }

    #[cfg(mobile)]
    impl MonitorExt for tao::monitor::MonitorHandle {
        fn work_area(&self) -> PhysicalRect<i32, u32> {
            PhysicalRect {
                size: self.size(),
                position: self.position(),
            }
        }
    }
}

pub mod window {
    #[cfg(target_os = "linux")]
    use super::window_linux;
    #[cfg(target_os = "macos")]
    use super::window_macos;
    #[cfg(windows)]
    use super::window_windows;
    use crate::monitor::MonitorExt;
    pub use tao;
    #[cfg(windows)]
    pub use windows;
    pub trait WindowExt {
        /// Enable or disable the window
        ///
        /// ## Platform-specific:
        ///
        /// - **Android / iOS**: Unsupported.
        fn set_enabled(&self, enabled: bool);

        /// Whether the window is enabled or disabled.
        ///
        /// ## Platform-specific:
        ///
        /// - **Android / iOS**: Unsupported, always returns `true`.
        fn is_enabled(&self) -> bool;

        /// Center the window
        ///
        /// ## Platform-specific:
        ///
        /// - **Android / iOS**: Unsupported.
        fn center(&self) {}

        /// Clears the window surface. i.e make it transparent.
        #[cfg(windows)]
        fn draw_surface(
            &self,
            surface: &mut softbuffer::Surface<
                std::sync::Arc<tao::window::Window>,
                std::sync::Arc<tao::window::Window>,
            >,
            background_color: Option<tao::window::RGBA>,
        );
    }

    #[cfg(mobile)]
    impl WindowExt for tao::window::Window {
        fn set_enabled(&self, _: bool) {}
        fn is_enabled(&self) -> bool {
            true
        }
    }

    #[cfg(desktop)]
    pub fn calculate_window_center_position(
        window_size: tao::dpi::PhysicalSize<u32>,
        target_monitor: tao::monitor::MonitorHandle,
    ) -> tao::dpi::PhysicalPosition<i32> {
        let work_area = target_monitor.work_area();

        tao::dpi::PhysicalPosition::new(
            (work_area.size.width as i32 - window_size.width as i32) / 2 + work_area.position.x,
            (work_area.size.height as i32 - window_size.height as i32) / 2 + work_area.position.y,
        )
    }
}

pub mod webview {
    #[cfg(windows)]
    pub use webview2_com;
    pub use wry;
    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    mod imp {
        pub type Webview = webkit2gtk::WebView;
    }

    #[cfg(target_vendor = "apple")]
    mod imp {
        use std::ffi::c_void;

        pub struct Webview {
            pub webview: *mut c_void,
            pub manager: *mut c_void,
            #[cfg(target_os = "macos")]
            pub ns_window: *mut c_void,
            #[cfg(target_os = "ios")]
            pub view_controller: *mut c_void,
        }
    }

    #[cfg(windows)]
    mod imp {
        use webview2_com::Microsoft::Web::WebView2::Win32::{
            ICoreWebView2Controller, ICoreWebView2Environment,
        };
        pub struct Webview {
            pub controller: ICoreWebView2Controller,
            pub environment: ICoreWebView2Environment,
        }
    }

    #[cfg(target_os = "android")]
    mod imp {
        use wry::JniHandle;
        pub type Webview = JniHandle;
    }

    pub use imp::*;
}

/// The result type of `tauri-utils`.
pub type Result<T> = std::result::Result<T, Error>;

/// The error type of `tauri-utils`.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
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

    /// Failed to create the webview.
    #[error("failed to create webview: {0}")]
    CreateWebview(#[source] Box<dyn std::error::Error + Send + Sync>),

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
