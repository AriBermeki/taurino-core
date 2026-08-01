// Portions of this file are derived from Tauri:
// https://github.com/tauri-apps/tauri
//
// Copyright 2019-2025 The Tauri Programme
// within The Commons Conservancy
//
// SPDX-License-Identifier: Apache-2.0 OR MIT
use crate::types::PhysicalRect;

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

pub trait MonitorExt {
    /// Get the work area of this monitor
    ///
    /// ## Platform-specific:
    ///
    /// - **Android / iOS**: Unsupported.
    fn work_area(&self) -> PhysicalRect<i32, u32>;
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
