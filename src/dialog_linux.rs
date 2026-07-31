// Portions of this file are derived from Tauri:
// https://github.com/tauri-apps/tauri
//
// Copyright 2019-2025 The Tauri Programme
// within The Commons Conservancy
//
// SPDX-License-Identifier: Apache-2.0 OR MIT


pub fn error(err: &'static str) {
    #[cfg(not(windows))]
    {
        unimplemented!("Error dialog is not implemented for this platform");
    }
}
