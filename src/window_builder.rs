// Portions of this file are derived from Tauri:
// https://github.com/tauri-apps/tauri
//
// Copyright 2019-2025 The Tauri Programme
// within The Commons Conservancy
//
// SPDX-License-Identifier: Apache-2.0 OR MIT



#[cfg(target_os = "macos")]
use dpi::Position;
use dpi::{PhysicalSize, Size};
use std::{
    fmt,
    path::PathBuf,
    sync::{Arc, Mutex, mpsc::Sender},
};
#[cfg(target_os = "android")]
use tao::platform::android::WindowBuilderExtAndroid;
#[cfg(target_os = "ios")]
use tao::platform::ios::WindowBuilderExtIOS;
#[cfg(target_os = "macos")]
use tao::platform::macos::WindowBuilderExtMacOS;
#[cfg(any(
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "linux",
    target_os = "netbsd",
    target_os = "openbsd"
))]
use tao::platform::unix::WindowBuilderExtUnix;
#[cfg(windows)]
use tao::platform::windows::WindowBuilderExtWindows;
use tao::{
    dpi::{LogicalPosition as TaoLogicalPosition, LogicalSize as TaoLogicalSize},
    window::{
        Fullscreen, Icon as TaoWindowIcon, Theme as TaoTheme, WindowBuilder as TaoWindowBuilder,
    },
};
#[cfg(windows)]
use windows::Win32::Foundation::HWND;

use crate::dpi::{Color, Icon, Theme, WindowSizeConstraints};
#[cfg(target_os = "macos")]
use crate::dpi::TitleBarStyle;
// window
pub type WindowEventHandler = Box<dyn Fn(&WindowEvent) + Send + 'static>;

pub type WindowEventListener = Arc<Mutex<Option<WindowEventHandler>>>;

/// The drag drop event payload.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum DragDropEvent {
    /// A drag operation has entered the webview.
    Enter {
        /// List of paths that are being dragged onto the webview.
        paths: Vec<PathBuf>,
        /// The position of the mouse cursor.
        position: dpi::PhysicalPosition<f64>,
    },
    /// A drag operation is moving over the webview.
    Over {
        /// The position of the mouse cursor.
        position: dpi::PhysicalPosition<f64>,
    },
    /// The file(s) have been dropped onto the webview.
    Drop {
        /// List of paths that are being dropped onto the window.
        paths: Vec<PathBuf>,
        /// The position of the mouse cursor.
        position: dpi::PhysicalPosition<f64>,
    },
    /// The drag operation has been cancelled or left the window.
    Leave,
}
/// An event from a window.
#[derive(Debug, Clone)]
pub enum WindowEvent {
    /// The size of the window has changed. Contains the client area's new dimensions.
    Resized(dpi::PhysicalSize<u32>),
    /// The position of the window has changed. Contains the window's new position.
    Moved(dpi::PhysicalPosition<i32>),
    /// The window has been requested to close.
    CloseRequested {
        /// A signal sender. If a `true` value is emitted, the window won't be closed.
        signal_tx: Sender<bool>,
    },
    /// The window has been destroyed.
    Destroyed,
    /// The window gained or lost focus.
    ///
    /// The parameter is true if the window has gained focus, and false if it has lost focus.
    Focused(bool),
    /// The window's scale factor has changed.
    ///
    /// The following user actions can cause DPI changes:
    ///
    /// - Changing the display's resolution.
    /// - Changing the display's scale factor (e.g. in Control Panel on Windows).
    /// - Moving the window to a display with a different scale factor.
    ScaleFactorChanged {
        /// The new scale factor.
        scale_factor: f64,
        /// The window inner size.
        new_inner_size: dpi::PhysicalSize<u32>,
    },
    /// An event associated with the drag and drop action.
    DragDrop(DragDropEvent),
    /// The system window theme has changed.
    ///
    /// Applications might wish to react to this to change the theme of the content of the window when the system changes the window theme.
    ThemeChanged(Theme),

    /// Emitted when the application has been suspended.
    ///
    /// ## Platform-specific
    ///
    /// - **Android**: This is triggered by `onPause` method of the Activity.
    /// - **iOS**: This is triggered by `applicationWillResignActive` method of the UIApplicationDelegate.
    /// - **Linux / macOS / Windows**: Unsupported.
    #[cfg(mobile)]
    Suspended,

    /// Emitted when the application has been resumed.
    ///
    /// ## Platform-specific
    ///
    /// - **Android**: This is triggered by `onResume` method of the Activity. The first onResume() is ignored to match the iOS implementation, since that is called on activity creation.
    /// - **iOS**: This is triggered by `applicationWillEnterForeground` method of the UIApplicationDelegate.
    /// - **Linux / macOS / Windows**: Unsupported.
    #[cfg(mobile)]
    Resumed,
}

/// Builder for configuring and creating a single application window.
///
/// `WindowBuilder` wraps Tao's [`TaoWindowBuilder`] and exposes a stable,
/// application-facing API for common window options. Platform-specific options
/// are guarded with `#[cfg(...)]` so unsupported methods are not compiled for
/// the wrong target.
#[derive(Clone, Default)]
pub struct FrameBuilder {
    pub label: String,
    pub center: bool,
    pub inner: TaoWindowBuilder,
    pub prevent_overflow: Option<Size>,
    #[cfg(windows)]
    pub background_color: Option<tao::window::RGBA>,
    #[cfg(windows)]
    pub is_window_transparent: bool,
    #[cfg(target_os = "macos")]
    pub tabbing_identifier: Option<String>,
    pub window_event_listener: WindowEventListener,
}

impl fmt::Debug for FrameBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("FrameBuilder");

        debug
            .field("center", &self.center)
            .field("inner", &self.inner)
            .field("label", &self.label)
            .field("prevent_overflow", &self.prevent_overflow);

        #[cfg(windows)]
        {
            debug
                .field("background_color", &self.background_color)
                .field("is_window_transparent", &self.is_window_transparent);
        }

        #[cfg(target_os = "macos")]
        {
            debug.field("tabbing_identifier", &self.tabbing_identifier);
        }

        debug.finish()
    }
}

impl FrameBuilder {
    /// Creates a new window builder with application defaults.
    ///
    /// Defaults:
    /// - focused window
    /// - title: `Taurino App`
    /// - label: `main`
    /// - Windows class name: `Taurino Window`
    #[inline]
    pub fn new() -> Self {
        #[allow(unused_mut)]
        let mut builder = Self::default()
            .focused(true)
            .label("main")
            .title("Taurino App");

        #[cfg(target_os = "macos")]
        {
            // Tao/webview workaround: the visible title bar keeps the content
            // view inside the native window bounds when devtools are open.
            builder = builder.title_bar_style(TitleBarStyle::Visible);
        }

        #[cfg(windows)]
        {
            builder = builder.window_classname("Taurino Window");
        }

        builder
    }

    /*

    let builder = WindowBuilder::new().on_window_event(|event| {
        println!("Window event: {event}");
    });

    */
    #[inline]
    pub fn on_window_event<F>(self, handler: F) -> Self
    where
        F: Fn(&WindowEvent) + Send + 'static,
    {
        *self
            .window_event_listener
            .lock()
            .expect("Window-event listener mutex was poisoned") = Some(Box::new(handler));

        self
    }

    /// Sets the native Android activity name used by this window.
    #[cfg(target_os = "android")]
    #[inline]
    pub fn activity_name<S: Into<String>>(mut self, class_name: S) -> Self {
        self.inner = self.inner.with_activity_name(class_name.into());
        self
    }

    /// Enables or disables the always-on-bottom window flag.
    #[inline]
    pub fn always_on_bottom(mut self, always_on_bottom: bool) -> Self {
        self.inner = self.inner.with_always_on_bottom(always_on_bottom);
        self
    }

    /// Enables or disables the always-on-top window flag.
    #[inline]
    pub fn always_on_top(mut self, always_on_top: bool) -> Self {
        self.inner = self.inner.with_always_on_top(always_on_top);
        self
    }

    /// Sets the window background color.
    #[inline]
    pub fn background_color(mut self, color: Color) -> Self {
        #[cfg(windows)]
        {
            let color = color.into();
            self.background_color = Some(color);
            self.inner = self.inner.with_background_color(color);
        }

        #[cfg(not(windows))]
        {
            self.inner = self.inner.with_background_color(color.into());
        }

        self
    }

    /// Centers the window after it has been created.
    #[inline]
    pub fn center(mut self) -> Self {
        self.center = true;
        self
    }

    /// Enables or disables the native close button.
    #[inline]
    pub fn closable(mut self, closable: bool) -> Self {
        self.inner = self.inner.with_closable(closable);
        self
    }

    /// Enables or disables content protection for the window.
    ///
    /// When enabled, the operating system is asked to prevent other
    /// applications from capturing the window content.
    #[inline]
    pub fn content_protected(mut self, protected: bool) -> Self {
        self.inner = self.inner.with_content_protection(protected);
        self
    }

    /// Sets the Android activity name that created this window.
    #[cfg(target_os = "android")]
    #[inline]
    pub fn created_by_activity_name<S: Into<String>>(mut self, class_name: S) -> Self {
        self.inner = self.inner.with_created_by_activity_name(class_name.into());
        self
    }

    /// Enables or disables native window decorations.
    #[inline]
    pub fn decorations(mut self, decorations: bool) -> Self {
        self.inner = self.inner.with_decorations(decorations);
        self
    }

    /// Enables or disables drag-and-drop support.
    #[cfg(windows)]
    #[inline]
    pub fn drag_and_drop(mut self, enabled: bool) -> Self {
        self.inner = self.inner.with_drag_and_drop(enabled);
        self
    }

    /// Enables or disables window focus on creation.
    #[inline]
    pub fn focusable(mut self, focusable: bool) -> Self {
        self.inner = self.inner.with_focusable(focusable);
        self
    }

    /// Requests initial focus for the window.
    #[inline]
    pub fn focused(mut self, focused: bool) -> Self {
        self.inner = self.inner.with_focused(focused);
        self
    }

    /// Enables or disables fullscreen mode.
    ///
    /// Fullscreen uses a borderless fullscreen configuration on the current
    /// monitor when enabled.
    #[inline]
    pub fn fullscreen(mut self, fullscreen: bool) -> Self {
        self.inner = if fullscreen {
            self.inner
                .with_fullscreen(Some(Fullscreen::Borderless(None)))
        } else {
            self.inner.with_fullscreen(None)
        };

        self
    }

    /// Returns the configured window label.
    #[inline]
    pub fn get_label(&self) -> &str {
        &self.label
    }

    /// Returns the configured preferred theme.
    #[inline]
    pub fn get_theme(&self) -> Option<Theme> {
        self.inner.window.preferred_theme.map(|theme| match theme {
            TaoTheme::Dark => Theme::Dark,
            _ => Theme::Light,
        })
    }

    /// Returns whether a window icon has been configured.
    #[inline]
    pub fn has_icon(&self) -> bool {
        self.inner.window.window_icon.is_some()
    }

    /// Hides the native title text while keeping the title bar controls.
    #[cfg(target_os = "macos")]
    #[inline]
    pub fn hidden_title(mut self, hidden: bool) -> Self {
        self.inner = self.inner.with_title_hidden(hidden);
        self
    }

    /// Sets the window icon.
    #[inline]
    pub fn icon(mut self, icon: Icon) -> crate::Result<Self> {
        let tao_icon = TaoIcon::try_from(icon)?.0;
        self.inner = self.inner.with_window_icon(Some(tao_icon));
        Ok(self)
    }

    /// Sets the initial inner window size in logical pixels.
    #[inline]
    pub fn inner_size(mut self, width: f64, height: f64) -> Self {
        self.inner = self
            .inner
            .with_inner_size(TaoLogicalSize::new(width, height));
        self
    }

    /// Sets all inner-size constraints for the window.
    #[inline]
    pub fn inner_size_constraints(mut self, constraints: WindowSizeConstraints) -> Self {
        self.inner.window.inner_size_constraints = tao::window::WindowSizeConstraints {
            min_width: constraints.min_width,
            min_height: constraints.min_height,
            max_width: constraints.max_width,
            max_height: constraints.max_height,
        };

        self
    }

    /// Sets the application-specific window label.
    #[inline]
    pub fn label<S: Into<String>>(mut self, label: S) -> Self {
        self.label = label.into();
        self
    }

    /// Sets the maximum inner window size in logical pixels.
    #[inline]
    pub fn max_inner_size(mut self, max_width: f64, max_height: f64) -> Self {
        self.inner = self
            .inner
            .with_max_inner_size(TaoLogicalSize::new(max_width, max_height));
        self
    }

    /// Enables or disables the native maximize button.
    #[inline]
    pub fn maximizable(mut self, maximizable: bool) -> Self {
        self.inner = self.inner.with_maximizable(maximizable);
        self
    }

    /// Enables or disables maximized state on creation.
    #[inline]
    pub fn maximized(mut self, maximized: bool) -> Self {
        self.inner = self.inner.with_maximized(maximized);
        self
    }

    /// Sets the minimum inner window size in logical pixels.
    #[inline]
    pub fn min_inner_size(mut self, min_width: f64, min_height: f64) -> Self {
        self.inner = self
            .inner
            .with_min_inner_size(TaoLogicalSize::new(min_width, min_height));
        self
    }

    /// Enables or disables the native minimize button.
    #[inline]
    pub fn minimizable(mut self, minimizable: bool) -> Self {
        self.inner = self.inner.with_minimizable(minimizable);
        self
    }

    /// Sets the Windows owner window.
    #[cfg(windows)]
    #[inline]
    pub fn owner(mut self, owner: HWND) -> Self {
        self.inner = self.inner.with_owner_window(owner.0 as _);
        self
    }

    /// Sets the Windows parent window.
    #[cfg(windows)]
    #[inline]
    pub fn parent(mut self, parent: HWND) -> Self {
        self.inner = self.inner.with_parent_window(parent.0 as _);
        self
    }

    /// Sets the macOS parent window pointer.
    #[cfg(target_os = "macos")]
    #[inline]
    pub fn parent(mut self, parent: *mut std::ffi::c_void) -> Self {
        self.inner = self.inner.with_parent_window(parent);
        self
    }

    /// Sets the initial outer window position in logical pixels.
    #[inline]
    pub fn position(mut self, x: f64, y: f64) -> Self {
        self.inner = self.inner.with_position(TaoLogicalPosition::new(x, y));
        self
    }

    /// Prevents the initial window bounds from overflowing the working area.
    ///
    /// The working area is the usable monitor area excluding system UI such as
    /// taskbars, docks, or panels.
    ///
    /// ## Platform-specific
    ///
    /// - **iOS / Android:** Unsupported.
    #[inline]
    pub fn prevent_overflow(mut self) -> Self {
        self.prevent_overflow
            .replace(PhysicalSize::new(0, 0).into());
        self
    }

    /// Prevents the initial window bounds from overflowing the working area
    /// while preserving the given margin.
    ///
    /// ## Platform-specific
    ///
    /// - **iOS / Android:** Unsupported.
    #[inline]
    pub fn prevent_overflow_with_margin(mut self, margin: Size) -> Self {
        self.prevent_overflow.replace(margin);
        self
    }

    /// Requests an iOS scene identifier for this window.
    #[cfg(target_os = "ios")]
    #[inline]
    pub fn requested_by_scene_identifier<S: Into<String>>(mut self, identifier: S) -> Self {
        self.inner = self
            .inner
            .with_requesting_scene_identifier(identifier.into());
        self
    }

    /// Enables or disables resize support.
    #[inline]
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.inner = self.inner.with_resizable(resizable);
        self
    }

    /// Enables or disables native window shadow.
    ///
    /// ## Platform-specific
    ///
    /// - **Windows:** Applies undecorated shadow support.
    /// - **macOS:** Applies native window shadow support.
    /// - **Other platforms:** No-op.
    #[inline]
    pub fn shadow(#[allow(unused_mut)] mut self, _enable: bool) -> Self {
        #[cfg(windows)]
        {
            self.inner = self.inner.with_undecorated_shadow(_enable);
        }

        #[cfg(target_os = "macos")]
        {
            self.inner = self.inner.with_has_shadow(_enable);
        }

        self
    }

    /// Shows or hides the window icon in the taskbar or window list.
    #[cfg(any(
        windows,
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "linux",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    #[inline]
    pub fn skip_taskbar(mut self, skip: bool) -> Self {
        self.inner = self.inner.with_skip_taskbar(skip);
        self
    }

    /// No-op implementation for platforms without taskbar support.
    #[cfg(any(target_os = "android", target_os = "ios", target_os = "macos"))]
    #[inline]
    pub fn skip_taskbar(self, _skip: bool) -> Self {
        self
    }

    /// Sets a macOS tabbing identifier used to group compatible windows.
    #[cfg(target_os = "macos")]
    #[inline]
    pub fn tabbing_identifier(mut self, identifier: &str) -> Self {
        self.inner = self.inner.with_tabbing_identifier(identifier);
        self.tabbing_identifier.replace(identifier.into());
        self
    }

    /// Sets the preferred window theme.
    #[inline]
    pub fn theme(mut self, theme: Option<Theme>) -> Self {
        self.inner = self.inner.with_theme(theme.map(|theme| match theme {
            Theme::Dark => TaoTheme::Dark,
            _ => TaoTheme::Light,
        }));

        self
    }

    /// Sets the initial window title.
    #[inline]
    pub fn title<S: Into<String>>(mut self, title: S) -> Self {
        self.inner = self.inner.with_title(title.into());
        self
    }

    /// Sets the macOS title bar style.
    #[cfg(target_os = "macos")]
    #[inline]
    pub fn title_bar_style(mut self, style: TitleBarStyle) -> Self {
        match style {
            TitleBarStyle::Visible => {
                self.inner = self.inner.with_titlebar_transparent(false);
                self.inner = self.inner.with_fullsize_content_view(true);
            }
            TitleBarStyle::Transparent => {
                self.inner = self.inner.with_titlebar_transparent(true);
                self.inner = self.inner.with_fullsize_content_view(false);
            }
            TitleBarStyle::Overlay => {
                self.inner = self.inner.with_titlebar_transparent(true);
                self.inner = self.inner.with_fullsize_content_view(true);
            }
            #[allow(unreachable_patterns)]
            unknown => {
                #[cfg(feature = "tracing")]
                tracing::warn!("unknown title bar style applied: {unknown:?}");

                #[cfg(not(feature = "tracing"))]
                eprintln!("unknown title bar style applied: {unknown:?}");
            }
        }

        self
    }

    /// Sets the macOS traffic-light control inset.
    ///
    /// Requires an overlay title bar style and enabled decorations.
    #[cfg(target_os = "macos")]
    #[inline]
    pub fn traffic_light_position<P: Into<Position>>(mut self, position: P) -> Self {
        self.inner = self.inner.with_traffic_light_inset(position.into());
        self
    }

    /// Enables or disables transparent window background support.
    #[cfg(any(not(target_os = "macos"), feature = "macos-private-api"))]
    #[inline]
    pub fn transparent(mut self, transparent: bool) -> Self {
        self.inner = self.inner.with_transparent(transparent);

        #[cfg(windows)]
        {
            self.is_window_transparent = transparent;
        }

        self
    }

    /// Sets the Unix parent window for transient behavior.
    #[cfg(any(
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "linux",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    #[inline]
    pub fn transient_for(mut self, parent: &impl gtk::glib::IsA<gtk::Window>) -> Self {
        self.inner = self.inner.with_transient_for(parent);
        self
    }

    /// Enables or disables initial visibility.
    #[inline]
    pub fn visible(mut self, visible: bool) -> Self {
        self.inner = self.inner.with_visible(visible);
        self
    }

    /// Shows the window on all workspaces where supported by the platform.
    #[inline]
    pub fn visible_on_all_workspaces(mut self, visible_on_all_workspaces: bool) -> Self {
        self.inner = self
            .inner
            .with_visible_on_all_workspaces(visible_on_all_workspaces);
        self
    }

    /// Sets the Windows window class name.
    #[cfg(windows)]
    #[inline]
    pub fn window_classname<S: Into<String>>(mut self, window_classname: S) -> Self {
        self.inner = self.inner.with_window_classname(window_classname);
        self
    }

    /// No-op implementation for non-Windows targets.
    #[cfg(not(windows))]
    #[inline]
    pub fn window_classname<S: Into<String>>(self, _window_classname: S) -> Self {
        self
    }
}

pub struct TaoIcon(pub TaoWindowIcon);

impl TryFrom<Icon<'_>> for TaoIcon {
    type Error = crate::Error;
    fn try_from(icon: Icon<'_>) -> std::result::Result<Self, Self::Error> {
        TaoWindowIcon::from_rgba(icon.rgba.to_vec(), icon.width, icon.height)
            .map(Self)
            .map_err(|e| crate::Error::InvalidTaoIcon(Box::new(e)))
    }
}
