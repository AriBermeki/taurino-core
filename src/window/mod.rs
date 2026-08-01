// Portions of this file are derived from Tauri:
// https://github.com/tauri-apps/tauri
//
// Copyright 2019-2025 The Tauri Programme
// within The Commons Conservancy
//
// SPDX-License-Identifier: Apache-2.0 OR MIT
#[cfg(target_os = "android")]
use tao::platform::android::WindowExtAndroid;
#[cfg(target_os = "ios")]
use tao::platform::ios::WindowExtIOS;
#[cfg(target_os = "linux")]
use tao::platform::linux::WindowExtLinux;
#[cfg(target_os = "macos")]
use tao::platform::macos::WindowExtMacOS;
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
use tao::platform::unix::WindowExtUnix;
#[cfg(windows)]
use tao::platform::windows::WindowExtWindows;
use tao::window::{Fullscreen, Window as TaoWindow, WindowId};

use dpi::{PhysicalPosition, PhysicalSize, Position, Size};
use std::{
    fmt,
    sync::{Arc, Mutex},
};

#[cfg(desktop)]
use crate::monitor::MonitorExt;
use crate::{
    types::{
        Color, CursorIcon, Icon, Monitor, ProgressBarState, ResizeDirection, Theme,
        WindowSizeConstraints, to_tao_theme,
    },
    wrapper::{
        CursorIconWrapper, MonitorHandleWrapper, PhysicalPositionWrapper, PhysicalSizeWrapper,
        PositionWrapper, ProgressBarStateWrapper, SizeWrapper, TaoIcon, UserAttentionTypeWrapper,
        map_theme,
    },
};

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

pub(crate) mod factory;

pub trait WindowExt {
    /// Centers the window on its current monitor.
    ///
    /// ## Platform-specific
    ///
    /// - **Android / iOS:** Unsupported.
    fn center(&self) {}

    /// Clears the window surface using the configured background color.
    #[cfg(windows)]
    fn draw_surface(
        &self,
        surface: &mut softbuffer::Surface<Arc<TaoWindow>, Arc<TaoWindow>>,
        background_color: Option<tao::window::RGBA>,
    );

    /// Returns whether the window is enabled.
    ///
    /// ## Platform-specific
    ///
    /// - **Android / iOS:** Unsupported; always returns `true`.
    fn is_enabled(&self) -> bool;

    /// Enables or disables the window.
    ///
    /// ## Platform-specific
    ///
    /// - **Android / iOS:** Unsupported.
    fn set_enabled(&self, enabled: bool);
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

/// Metadata associated with a window event.
#[derive(Debug, Clone, Copy)]
pub struct WindowMeta<'a> {
    window_label: &'a str,
    window_id: WindowId,
}

impl<'a> WindowMeta<'a> {
    /// Creates metadata for a window event.
    pub fn new(window_label: &'a str, window_id: WindowId) -> Self {
        Self {
            window_label,
            window_id,
        }
    }

    /// Returns the window label.
    pub fn label(&self) -> &'a str {
        self.window_label
    }

    /// Returns the native window identifier.
    pub fn id(&self) -> WindowId {
        self.window_id
    }
}

/// Callback invoked for a window event.
pub type WindowEventHandler = Box<dyn for<'a> Fn(&WindowMeta<'a>, &str) + Send + 'static>;

/// Shared storage for an optional window event handler.
pub type WindowEventListener = Arc<Mutex<Option<WindowEventHandler>>>;

/// Thread-safe wrapper around a Tao window and its runtime state.
pub struct Window {
    label: String,
    pub(crate) inner: Arc<TaoWindow>,
    event_listener: WindowEventListener,

    #[cfg(windows)]
    background_color: Option<tao::window::RGBA>,

    #[cfg(windows)]
    is_window_transparent: bool,

    #[cfg(windows)]
    surface: Option<softbuffer::Surface<Arc<TaoWindow>, Arc<TaoWindow>>>,
}

impl Window {
    /// Returns the configured Windows background color.
    #[cfg(windows)]
    pub fn background_color(&self) -> Option<tao::window::RGBA> {
        self.background_color
    }

    /// Removes the registered window event handler.
    pub fn clear_event_handler(&self) {
        let mut listener = self
            .event_listener
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        *listener = None;
    }

    /// Emits an event to the registered window event handler.
    pub fn emit_event(&self, event: &str) {
        let meta = WindowMeta::new(&self.label, self.inner.id());

        let listener = self
            .event_listener
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        if let Some(handler) = listener.as_ref() {
            handler(&meta, event);
        }
    }

    /// Returns the shared event-listener storage.
    pub fn event_listener(&self) -> WindowEventListener {
        Arc::clone(&self.event_listener)
    }

    /// Redraws the transparent Windows surface when required.
    #[cfg(windows)]
    pub fn handle_redraw_requested(&mut self) {
        if !self.is_window_transparent {
            return;
        }

        if let Some(surface) = self.surface.as_mut() {
            self.inner.draw_surface(surface, self.background_color);
        }
    }

    /// No-op on platforms without the Windows transparent surface.
    #[cfg(not(windows))]
    pub fn handle_redraw_requested(&mut self) {}

    /// Returns whether an event handler is registered.
    pub fn has_event_handler(&self) -> bool {
        self.event_listener
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_some()
    }

    /// Returns the native window identifier.
    pub fn id(&self) -> WindowId {
        self.inner.id()
    }

    /// Returns the underlying Tao window.
    pub fn inner(&self) -> &TaoWindow {
        self.inner.as_ref()
    }

    /// Returns a shared reference to the underlying Tao window.
    pub fn inner_arc(&self) -> Arc<TaoWindow> {
        Arc::clone(&self.inner)
    }

    /// Returns whether the window is enabled.
    pub fn is_enabled(&self) -> bool {
        #[cfg(desktop)]
        return self.inner.is_enabled();
        #[cfg(mobile)]
        return true;
    }

    /// Returns whether Windows transparency is enabled.
    #[cfg(windows)]
    pub fn is_window_transparent(&self) -> bool {
        self.is_window_transparent
    }

    /// Requests a window redraw.
    pub fn request_redraw(&self) {
        self.inner.request_redraw();
    }

    /// Registers the window event handler.
    pub fn set_event_handler<F>(&self, handler: F)
    where
        F: for<'a> Fn(&WindowMeta<'a>, &str) + Send + 'static,
    {
        let mut listener = self
            .event_listener
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        *listener = Some(Box::new(handler));
    }

    /// Updates the window label.
    pub fn set_label(&mut self, label: impl Into<String>) {
        self.label = label.into();
    }
}

impl fmt::Debug for Window {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Window")
            .field("label", &self.label)
            .field("id", &self.inner.id())
            .field("inner", &self.inner)
            .field("has_event_handler", &self.has_event_handler())
            .finish_non_exhaustive()
    }
}

impl Window {
    // Accessors

    /// Returns the Android activity name.
    #[cfg(target_os = "android")]
    pub fn activity_name(&self) -> crate::Result<String> {
        Ok(self.inner.activity_name())
    }

    /// Returns all available monitors.
    pub fn available_monitors(&self) -> crate::Result<Vec<Monitor>> {
        Ok(self
            .inner
            .available_monitors()
            .map(|monitor| MonitorHandleWrapper(monitor).into())
            .collect())
    }

    /// Returns the monitor containing the window.
    ///
    /// Returns `None` when the current monitor cannot be detected.
    pub fn current_monitor(&self) -> crate::Result<Option<Monitor>> {
        Ok(self
            .inner
            .current_monitor()
            .map(|monitor| MonitorHandleWrapper(monitor).into()))
    }

    /// Returns the default GTK vertical box.
    ///
    /// ## Platform-specific
    ///
    /// - **Linux / BSD:** Supported through Tao's Unix platform extension.
    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    pub fn default_vbox(&self) -> crate::Result<gtk::Box> {
        Ok(self
            .inner
            .default_vbox()
            .expect("Tao did not provide a default GTK vbox for this window")
            .clone())
    }

    /// Returns the underlying GTK application window.
    ///
    /// ## Platform-specific
    ///
    /// - **Linux / BSD:** Supported through Tao's Unix platform extension.
    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    pub fn gtk_window(&self) -> crate::Result<gtk::ApplicationWindow> {
        Ok(self.inner.gtk_window().clone())
    }

    /// Returns the client-area position in physical pixels.
    pub fn inner_position(&self) -> crate::Result<PhysicalPosition<i32>> {
        self.inner
            .inner_position()
            .map(PhysicalPositionWrapper)
            .map(Into::into)
            .map_err(crate::Error::NotSupportedError)
    }

    /// Returns whether the window stays above other windows.
    ///
    /// ## Platform-specific
    ///
    /// - **iOS / Android:** Unsupported.
    pub fn is_always_on_top(&self) -> crate::Result<bool> {
        Ok(self.inner.is_always_on_top())
    }

    /// Returns whether the window can be closed.
    ///
    /// ## Platform-specific
    ///
    /// - **iOS / Android:** Unsupported.
    pub fn is_closable(&self) -> crate::Result<bool> {
        Ok(self.inner.is_closable())
    }

    /// Returns whether window decorations are enabled.
    pub fn is_decorated(&self) -> crate::Result<bool> {
        Ok(self.inner.is_decorated())
    }

    /// Returns whether the window has focus.
    pub fn is_focused(&self) -> crate::Result<bool> {
        Ok(self.inner.is_focused())
    }

    /// Returns whether the window is fullscreen.
    pub fn is_fullscreen(&self) -> crate::Result<bool> {
        Ok(self.inner.fullscreen().is_some())
    }

    /// Returns whether the window can be maximized.
    ///
    /// ## Platform-specific
    ///
    /// - **Linux / iOS / Android:** Unsupported.
    pub fn is_maximizable(&self) -> crate::Result<bool> {
        Ok(self.inner.is_maximizable())
    }

    /// Returns whether the window is maximized.
    pub fn is_maximized(&self) -> crate::Result<bool> {
        Ok(self.inner.is_maximized())
    }

    /// Returns whether the window can be minimized.
    ///
    /// ## Platform-specific
    ///
    /// - **Linux / iOS / Android:** Unsupported.
    pub fn is_minimizable(&self) -> crate::Result<bool> {
        Ok(self.inner.is_minimizable())
    }

    /// Returns whether the window is minimized.
    pub fn is_minimized(&self) -> crate::Result<bool> {
        Ok(self.inner.is_minimized())
    }

    /// Returns whether the window is resizable.
    pub fn is_resizable(&self) -> crate::Result<bool> {
        Ok(self.inner.is_resizable())
    }

    /// Returns whether the window is visible.
    pub fn is_visible(&self) -> crate::Result<bool> {
        Ok(self.inner.is_visible())
    }

    /// Returns the window label.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Returns the monitor containing the specified point.
    pub fn monitor_from_point(&self, x: f64, y: f64) -> crate::Result<Option<Monitor>> {
        Ok(self
            .inner
            .monitor_from_point(x, y)
            .map(|monitor| MonitorHandleWrapper(monitor).into()))
    }

    /// Returns the outer window position in physical pixels.
    ///
    /// Includes window decorations.
    pub fn outer_position(&self) -> crate::Result<PhysicalPosition<i32>> {
        self.inner
            .outer_position()
            .map(PhysicalPositionWrapper)
            .map(Into::into)
            .map_err(crate::Error::NotSupportedError)
    }

    /// Returns the outer window size in physical pixels.
    ///
    /// Includes the title bar and borders.
    pub fn outer_size(&self) -> crate::Result<PhysicalSize<u32>> {
        Ok(PhysicalSizeWrapper(self.inner.outer_size()).into())
    }

    /// Returns the primary monitor.
    pub fn primary_monitor(&self) -> crate::Result<Option<Monitor>> {
        Ok(self
            .inner
            .primary_monitor()
            .map(|monitor| MonitorHandleWrapper(monitor).into()))
    }

    /// Returns the logical-to-physical scale factor.
    pub fn scale_factor(&self) -> crate::Result<f64> {
        Ok(self.inner.scale_factor())
    }

    /// Returns the iOS scene identifier.
    #[cfg(target_os = "ios")]
    pub fn scene_identifier(&self) -> crate::Result<String> {
        Ok(self.inner.scene_identifier())
    }

    /// Returns the window theme.
    pub fn theme(&self) -> crate::Result<Theme> {
        Ok(map_theme(&self.inner.theme()))
    }

    /// Returns the window title.
    pub fn title(&self) -> crate::Result<String> {
        Ok(self.inner.title())
    }

    // Mutators

    /// Hides the window.
    pub fn hide(&self) -> crate::Result<()> {
        self.inner.set_visible(false);
        Ok(())
    }

    /// Maximizes the window.
    pub fn maximize(&self) -> crate::Result<()> {
        self.inner.set_maximized(true);
        Ok(())
    }

    /// Minimizes the window.
    pub fn minimize(&self) -> crate::Result<()> {
        self.inner.set_minimized(true);
        Ok(())
    }

    /// Requests user attention.
    ///
    /// Passing `None` clears the current attention request.
    pub fn request_user_attention(
        &self,
        request_type: Option<UserAttentionTypeWrapper>,
    ) -> crate::Result<()> {
        self.inner
            .request_user_attention(request_type.map(|request| request.0));
        Ok(())
    }

    /// Keeps the window below other windows when enabled.
    pub fn set_always_on_bottom(&self, always_on_bottom: bool) -> crate::Result<()> {
        self.inner.set_always_on_bottom(always_on_bottom);
        Ok(())
    }

    /// Keeps the window above other windows when enabled.
    pub fn set_always_on_top(&self, always_on_top: bool) -> crate::Result<()> {
        self.inner.set_always_on_top(always_on_top);
        Ok(())
    }

    /// Sets the window background color.
    pub fn set_background_color(&self, color: Option<Color>) -> crate::Result<()> {
        self.inner.set_background_color(color.map(Into::into));
        Ok(())
    }

    /// Sets the application badge count.
    ///
    /// `None` and `Some(0)` both clear the badge.
    ///
    /// ## Platform-specific
    ///
    /// - **iOS:** Values outside `i32` are clamped.
    /// - **Windows:** Unsupported. Use `set_overlay_icon` instead.
    /// - **Android:** Unsupported.
    #[cfg(target_os = "ios")]
    pub fn set_badge_count(
        &self,
        count: Option<i64>,
        _desktop_filename: Option<String>,
    ) -> crate::Result<()> {
        self.inner.set_badge_count(count.map_or(0, |value| {
            value.clamp(i32::MIN as i64, i32::MAX as i64) as i32
        }));
        Ok(())
    }

    /// Sets the macOS taskbar badge label.
    ///
    /// Passing `None` clears the badge label.
    ///
    /// ## Platform-specific
    ///
    /// - **macOS:** Supported.
    #[cfg(target_os = "macos")]
    pub fn set_badge_label(&self, label: Option<String>) -> crate::Result<()> {
        self.inner.set_badge_label(label);
        Ok(())
    }

    /// Enables or disables the native close button.
    ///
    /// ## Platform-specific
    ///
    /// - **Linux:** The window manager may ignore this request for visible windows.
    /// - **iOS / Android:** Unsupported.
    pub fn set_closable(&self, closable: bool) -> crate::Result<()> {
        self.inner.set_closable(closable);
        Ok(())
    }

    /// Protects the window content from being captured by other applications.
    pub fn set_content_protected(&self, protected: bool) -> crate::Result<()> {
        self.inner.set_content_protection(protected);
        Ok(())
    }

    /// Captures or releases the cursor for this window.
    ///
    /// Cursor grabbing does not guarantee that the cursor is hidden. Use
    /// `set_cursor_visible(false)` when the cursor should also be hidden.
    pub fn set_cursor_grab(&self, grab: bool) -> crate::Result<()> {
        self.inner
            .set_cursor_grab(grab)
            .map_err(crate::Error::ExternalError)
    }

    /// Sets the cursor icon used while the cursor is over this window.
    pub fn set_cursor_icon(&self, icon: CursorIcon) -> crate::Result<()> {
        self.inner.set_cursor_icon(CursorIconWrapper::from(icon).0);
        Ok(())
    }

    /// Moves the cursor to the provided window-relative position.
    pub fn set_cursor_position<Pos>(&self, position: Pos) -> crate::Result<()>
    where
        Pos: Into<Position>,
    {
        self.inner
            .set_cursor_position(PositionWrapper::from(position.into()).0)
            .map_err(crate::Error::ExternalError)
    }

    /// Shows or hides the cursor while it is over this window.
    pub fn set_cursor_visible(&self, visible: bool) -> crate::Result<()> {
        self.inner.set_cursor_visible(visible);
        Ok(())
    }

    /// Enables or disables native window decorations.
    pub fn set_decorations(&self, decorations: bool) -> crate::Result<()> {
        self.inner.set_decorations(decorations);
        Ok(())
    }

    /// Enables or disables the window.
    ///
    /// ## Platform-specific
    ///
    /// - **Android / iOS:** Unsupported.
    pub fn set_enabled(&self, enabled: bool) {
        #[cfg(desktop)]
        self.inner.set_enabled(enabled);
        #[cfg(mobile)]
        let _ = enabled;
    }

    /// Brings the window to the front and requests focus.
    pub fn set_focus(&self) -> crate::Result<()> {
        self.inner.set_focus();
        Ok(())
    }

    /// Enables or disables keyboard focus.
    pub fn set_focusable(&self, focusable: bool) -> crate::Result<()> {
        self.inner.set_focusable(focusable);
        Ok(())
    }

    /// Enables or disables borderless fullscreen mode.
    pub fn set_fullscreen(&self, fullscreen: bool) -> crate::Result<()> {
        if fullscreen {
            self.inner
                .set_fullscreen(Some(Fullscreen::Borderless(None)));
        } else {
            self.inner.set_fullscreen(None);
        }

        Ok(())
    }

    /// Sets the window icon.
    pub fn set_icon(&self, icon: Icon<'_>) -> crate::Result<()> {
        self.inner.set_window_icon(Some(TaoIcon::try_from(icon)?.0));
        Ok(())
    }

    /// Enables or disables cursor event passthrough for this window.
    pub fn set_ignore_cursor_events(&self, ignore: bool) -> crate::Result<()> {
        self.inner
            .set_ignore_cursor_events(ignore)
            .map_err(crate::Error::ExternalError)
    }

    /// Sets the maximum inner window size.
    pub fn set_max_size(&self, size: Option<Size>) -> crate::Result<()> {
        self.inner
            .set_max_inner_size(size.map(|size| SizeWrapper::from(size).0));
        Ok(())
    }

    /// Enables or disables the native maximize button.
    ///
    /// ## Platform-specific
    ///
    /// - **macOS:** Controls the zoom button in the title bar.
    /// - **Linux / iOS / Android:** Unsupported.
    pub fn set_maximizable(&self, maximizable: bool) -> crate::Result<()> {
        self.inner.set_maximizable(maximizable);
        Ok(())
    }

    /// Sets the minimum inner window size.
    pub fn set_min_size(&self, size: Option<Size>) -> crate::Result<()> {
        self.inner
            .set_min_inner_size(size.map(|size| SizeWrapper::from(size).0));
        Ok(())
    }

    /// Enables or disables the native minimize button.
    ///
    /// ## Platform-specific
    ///
    /// - **Linux / iOS / Android:** Unsupported.
    pub fn set_minimizable(&self, minimizable: bool) -> crate::Result<()> {
        self.inner.set_minimizable(minimizable);
        Ok(())
    }

    /// Sets or clears the Windows taskbar overlay icon.
    ///
    /// ## Platform-specific
    ///
    /// - **Windows:** Supported.
    #[cfg(windows)]
    pub fn set_overlay_icon(&self, icon: Option<Icon<'_>>) -> crate::Result<()> {
        let tao_icon = icon.map(TaoIcon::try_from).transpose()?;
        self.inner
            .set_overlay_icon(tao_icon.as_ref().map(|icon| &icon.0));
        Ok(())
    }

    /// Sets the outer window position.
    pub fn set_position(&self, position: Position) -> crate::Result<()> {
        self.inner
            .set_outer_position(PositionWrapper::from(position).0);
        Ok(())
    }

    /// Sets the taskbar progress state.
    ///
    /// ## Platform-specific
    ///
    /// - **Linux / macOS:** Progress is app-wide and requires a supported desktop environment.
    /// - **iOS / Android:** Unsupported.
    pub fn set_progress_bar(&self, progress_state: ProgressBarState) -> crate::Result<()> {
        self.inner
            .set_progress_bar(ProgressBarStateWrapper::from(progress_state).0);
        Ok(())
    }

    /// Enables or disables simple fullscreen mode.
    ///
    /// ## Platform-specific
    ///
    /// - **macOS:** Supported.
    #[cfg(target_os = "macos")]
    pub fn set_simple_fullscreen(&self, enable: bool) -> crate::Result<()> {
        self.inner.set_simple_fullscreen(enable);
        Ok(())
    }

    /// Sets the inner window size.
    pub fn set_size(&self, size: Size) -> crate::Result<()> {
        self.inner.set_inner_size(SizeWrapper::from(size).0);
        Ok(())
    }

    /// Sets the minimum and maximum inner size constraints.
    pub fn set_size_constraints(&self, constraints: WindowSizeConstraints) -> crate::Result<()> {
        self.inner
            .set_inner_size_constraints(tao::window::WindowSizeConstraints {
                min_width: constraints.min_width,
                min_height: constraints.min_height,
                max_width: constraints.max_width,
                max_height: constraints.max_height,
            });

        Ok(())
    }

    /// Shows or hides the window in the taskbar.
    ///
    /// ## Platform-specific
    ///
    /// - **Windows / Linux / BSD:** Supported.
    #[cfg(any(
        windows,
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    pub fn set_skip_taskbar(&self, skip: bool) -> crate::Result<()> {
        self.inner
            .set_skip_taskbar(skip)
            .map_err(crate::Error::ExternalError)
    }

    /// Sets the preferred theme for this window.
    ///
    /// ## Platform-specific
    ///
    /// - **Linux / macOS:** Theme is app-wide and not window-specific.
    /// - **iOS / Android:** Unsupported.
    pub fn set_theme(&self, theme: Option<Theme>) -> crate::Result<()> {
        self.inner.set_theme(to_tao_theme(theme));
        Ok(())
    }

    /// Updates the window title.
    pub fn set_title<S>(&self, title: S) -> crate::Result<()>
    where
        S: Into<String>,
    {
        let title = title.into();
        self.inner.set_title(&title);
        Ok(())
    }

    /// Sets the macOS title bar style.
    ///
    /// ## Platform-specific
    ///
    /// - **macOS:** Supported.
    /// - **Linux / Windows / iOS / Android:** Unsupported.
    #[cfg(target_os = "macos")]
    pub fn set_title_bar_style(&self, style: TitleBarStyle) -> crate::Result<()> {
        match style {
            TitleBarStyle::Overlay => {
                self.inner.set_titlebar_transparent(true);
                self.inner.set_fullsize_content_view(true);
            }
            TitleBarStyle::Transparent => {
                self.inner.set_titlebar_transparent(true);
                self.inner.set_fullsize_content_view(false);
            }
            TitleBarStyle::Visible => {
                self.inner.set_titlebar_transparent(false);
                self.inner.set_fullsize_content_view(false);
            }
            #[allow(unreachable_patterns)]
            unknown => {
                eprintln!("unknown title bar style applied: {unknown:?}");
            }
        }

        Ok(())
    }

    /// Sets the macOS traffic-light button position.
    ///
    /// This requires `TitleBarStyle::Overlay` and enabled decorations.
    ///
    /// ## Platform-specific
    ///
    /// - **macOS:** Supported.
    /// - **Linux / Windows / iOS / Android:** Unsupported.
    #[cfg(target_os = "macos")]
    pub fn set_traffic_light_position(&self, position: Position) -> crate::Result<()> {
        self.inner.set_traffic_light_inset(position);
        Ok(())
    }

    /// Shows the window on all workspaces when enabled.
    pub fn set_visible_on_all_workspaces(
        &self,
        visible_on_all_workspaces: bool,
    ) -> crate::Result<()> {
        self.inner
            .set_visible_on_all_workspaces(visible_on_all_workspaces);
        Ok(())
    }

    /// Shows the window.
    pub fn show(&self) -> crate::Result<()> {
        self.inner.set_visible(true);
        Ok(())
    }

    /// Starts an interactive window drag operation.
    pub fn start_dragging(&self) -> crate::Result<()> {
        self.inner
            .drag_window()
            .map_err(crate::Error::ExternalError)
    }

    /// Starts an interactive resize operation.
    pub fn start_resize_dragging(&self, direction: ResizeDirection) -> crate::Result<()> {
        let direction = match direction {
            ResizeDirection::East => tao::window::ResizeDirection::East,
            ResizeDirection::North => tao::window::ResizeDirection::North,
            ResizeDirection::NorthEast => tao::window::ResizeDirection::NorthEast,
            ResizeDirection::NorthWest => tao::window::ResizeDirection::NorthWest,
            ResizeDirection::South => tao::window::ResizeDirection::South,
            ResizeDirection::SouthEast => tao::window::ResizeDirection::SouthEast,
            ResizeDirection::SouthWest => tao::window::ResizeDirection::SouthWest,
            ResizeDirection::West => tao::window::ResizeDirection::West,
        };

        self.inner
            .drag_resize_window(direction)
            .map_err(crate::Error::ExternalError)
    }

    /// Restores the window from maximized state.
    pub fn unmaximize(&self) -> crate::Result<()> {
        self.inner.set_maximized(false);
        Ok(())
    }

    /// Restores the window from minimized state.
    pub fn unminimize(&self) -> crate::Result<()> {
        self.inner.set_minimized(false);
        Ok(())
    }
}

/*

Event::RedrawRequested(window_id) => {
    let mut windows = windows.0.borrow_mut();

    if let Some(window) = windows.get_mut(&window_id) {
        window.handle_redraw_requested();
    }
}

*/
