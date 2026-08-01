// Portions of this file are derived from Tauri:
// https://github.com/tauri-apps/tauri
//
// Copyright 2019-2025 The Tauri Programme
// within The Commons Conservancy
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::build::WindowBuilder;
use crate::monitor::MonitorExt;
use crate::window::Window;
#[cfg(windows)]
use crate::window::WindowExt;
use crate::window::calculate_window_center_position;
use crate::wrapper::find_monitor_for_position;
use crate::{Error, Result};
use std::sync::Arc;
use tao::{dpi::PhysicalSize as TaoPhysicalSize, event_loop::EventLoopWindowTarget};

pub(crate) fn create_window<T: 'static>(
    event_loop: &EventLoopWindowTarget<T>,
    mut window_builder: WindowBuilder,
) -> Result<Window> {
    #[cfg(windows)]
    let background_color = window_builder.inner.window.background_color;
    #[cfg(windows)]
    let is_window_transparent = window_builder.inner.window.transparent;

    #[cfg(target_os = "macos")]
    {
        if tabbing_identifier.is_none() || inner.window.transparent || !inner.window.decorations {
            inner = inner.with_automatic_window_tabbing(false);
        }
    }

    if window_builder.prevent_overflow.is_some() || window_builder.center {
        let monitor = if let Some(window_position) = &window_builder.inner.window.position {
            find_monitor_for_position(event_loop.available_monitors(), *window_position)
        } else {
            event_loop.primary_monitor()
        };
        if let Some(monitor) = monitor {
            let scale_factor = monitor.scale_factor();
            let desired_size = window_builder
                .inner
                .window
                .inner_size
                .unwrap_or_else(|| TaoPhysicalSize::new(800, 600).into());
            let mut inner_size = window_builder
                .inner
                .window
                .inner_size_constraints
                .clamp(desired_size, scale_factor)
                .to_physical::<u32>(scale_factor);
            let mut window_size = inner_size;
            #[allow(unused_mut)]
            // Left and right window shadow counts as part of the window on Windows
            // We need to include it when calculating positions, but not size
            let mut shadow_width = 0;
            #[cfg(windows)]
            if window_builder.inner.window.decorations {
                use windows::Win32::UI::WindowsAndMessaging::{
                    AdjustWindowRect, WS_OVERLAPPEDWINDOW,
                };
                let mut rect = windows::Win32::Foundation::RECT::default();
                let result = unsafe { AdjustWindowRect(&mut rect, WS_OVERLAPPEDWINDOW, false) };
                if result.is_ok() {
                    shadow_width = (rect.right - rect.left) as u32;
                    // rect.bottom is made out of shadow, and we don't care about it
                    window_size.height += -rect.top as u32;
                }
            }

            if let Some(margin) = window_builder.prevent_overflow {
                let work_area = monitor.work_area();
                let margin = margin.to_physical::<u32>(scale_factor);
                let constraint = TaoPhysicalSize::new(
                    work_area.size.width - margin.width,
                    work_area.size.height - margin.height,
                );
                if window_size.width > constraint.width || window_size.height > constraint.height {
                    if window_size.width > constraint.width {
                        inner_size.width = inner_size
                            .width
                            .saturating_sub(window_size.width - constraint.width);
                        window_size.width = constraint.width;
                    }
                    if window_size.height > constraint.height {
                        inner_size.height = inner_size
                            .height
                            .saturating_sub(window_size.height - constraint.height);
                        window_size.height = constraint.height;
                    }
                    window_builder.inner.window.inner_size = Some(inner_size.into());
                }
            }

            if window_builder.center {
                window_size.width += shadow_width;
                let position = calculate_window_center_position(window_size, monitor);
                let logical_position = position.to_logical::<f64>(scale_factor);

                window_builder.inner = window_builder.inner.with_position(logical_position);
            }
        }
    };

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    let (initial_position, is_fullscreen) =
        (inner.window.position, inner.window.fullscreen.is_some());

    // If fullscreen is requested with an explicit position, resolve the target
    // monitor up front so the window is created fullscreen on that display.
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    if let (true, Some(position)) = (is_fullscreen, initial_position) {
        if let Some(target_monitor) =
            find_monitor_for_position(event_loop.available_monitors(), position)
        {
            inner.window.fullscreen = Some(Fullscreen::Borderless(Some(target_monitor)));
        }
    }

    let window = window_builder
        .inner
        .build(event_loop)
        .inspect_err(|e| log::error!("Error creating window: {e:?}"))
        .map_err(Error::CreateWindow)?;

    // On macOS, `with_position` uses the content origin; the title bar is added
    // above it. `set_outer_position` is needed for precise window placement.
    #[cfg(target_os = "macos")]
    if !is_fullscreen {
        if let Some(position) = initial_position {
            window.set_outer_position(position);
        }
    }
    let window = Arc::new(window);
    #[cfg(windows)]
    let surface = if is_window_transparent {
        if let Ok(context) = softbuffer::Context::new(window.clone()) {
            if let Ok(mut surface) = softbuffer::Surface::new(&context, window.clone()) {
                window.draw_surface(&mut surface, background_color);
                Some(surface)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    Ok(Window {
        label: window_builder.label.to_string(),
        inner: window,
        #[cfg(windows)]
        background_color,
        #[cfg(windows)]
        is_window_transparent,
        event_listener: window_builder.window_event_listener,
        #[cfg(windows)]
        surface,
    })
}
