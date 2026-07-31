// Portions of this file are derived from Tauri:
// https://github.com/tauri-apps/tauri
//
// Copyright 2019-2025 The Tauri Programme
// within The Commons Conservancy
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::{
    MonitorExt,
    dpi::{LogicalPosition, LogicalSize, PhysicalRect},
};
use gtk::prelude::MonitorExt;
use tao::platform::unix::MonitorHandleExtUnix;

impl MonitorExt for tao::monitor::MonitorHandle {
    fn work_area(&self) -> PhysicalRect<i32, u32> {
        let rect = self.gdk_monitor().workarea();
        let scale_factor = self.scale_factor();
        PhysicalRect {
            size: LogicalSize::new(rect.width() as u32, rect.height() as u32)
                .to_physical(scale_factor),
            position: LogicalPosition::new(rect.x(), rect.y()).to_physical(scale_factor),
        }
    }
}
