// Portions of this file are derived from Tauri:
// https://github.com/tauri-apps/tauri
//
// Copyright 2019-2025 The Tauri Programme
// within The Commons Conservancy
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[cfg(windows)]
mod windows;

// Takes a `&'static str` here since we convert clickable hyperlinks,
// DO NOT pass in untrusted input
#[cfg_attr(not(windows), allow(unused))]
pub fn error(err: &'static str) {
    #[cfg(windows)]
    windows::error(err);

    #[cfg(not(windows))]
    {
        unimplemented!("Error dialog is not implemented for this platform");
    }
}
