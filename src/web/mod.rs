// Portions of this file are derived from Tauri:
// https://github.com/tauri-apps/tauri
//
// Copyright 2019-2025 The Tauri Programme
// within The Commons Conservancy
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[cfg(windows)]
use std::{
    rc::Rc,
    sync::{Arc, Mutex},
};

use tao::window::Window;
#[cfg(windows)]
use webview2_com::{
    FocusChangedEventHandler, Microsoft::Web::WebView2::Win32::ICoreWebView2Controller,
};
#[cfg(windows)]
use wry::WebViewExtWindows;
use wry::{WebView, WebViewBuilder};

#[cfg(windows)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WindowFrameAdjustment {
    /// Linker und rechter äußerer Fensterrahmen zusammen.
    pub shadow_width: u32,

    /// Oberer Fensterrahmen inklusive Titelleiste.
    pub top_height: u32,
}

#[cfg(windows)]
pub fn calculate_window_frame_adjustment(decorations: bool) -> WindowFrameAdjustment {
    use windows::Win32::{
        Foundation::RECT,
        UI::WindowsAndMessaging::{AdjustWindowRect, WS_OVERLAPPEDWINDOW},
    };

    if !decorations {
        return WindowFrameAdjustment::default();
    }

    let mut rect = RECT::default();

    let result = unsafe { AdjustWindowRect(&mut rect, WS_OVERLAPPEDWINDOW, false) };

    if result.is_err() {
        return WindowFrameAdjustment::default();
    }

    WindowFrameAdjustment {
        shadow_width: (rect.right - rect.left) as u32,
        // rect.bottom besteht hauptsächlich aus dem unteren Schatten
        // und wird in der ursprünglichen Logik nicht berücksichtigt.
        top_height: (-rect.top) as u32,
    }
}

/// Events emitted directly by the native WebView callbacks.
///
/// The callback runs on the WebView/UI thread. It does not use an
/// `EventLoopProxy` and must therefore not be called from worker threads.
#[cfg(windows)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebViewHostEvent {
    Focused {
        webview_label: String,
        focused: bool,
    },
    FullscreenChanged {
        webview_label: String,
        fullscreen: bool,
    },
}

#[cfg(windows)]
pub type WebViewHostEventCallback = Rc<dyn Fn(WebViewHostEvent) + 'static>;

/// Prevents duplicate focus events and tracks the last focused WebView.
#[cfg(windows)]
pub fn add_focus_change_listeners(
    callback: WebViewHostEventCallback,
    focused_webview: Arc<Mutex<FocusState>>,
    label: String,
    controller: &ICoreWebView2Controller,
) {
    let got_focus_callback = Rc::clone(&callback);
    let got_focus_state = Arc::clone(&focused_webview);
    let got_focus_label = label.clone();
    let mut got_focus_token = 0;

    if let Err(error) = unsafe {
        controller.add_GotFocus(
            &FocusChangedEventHandler::create(Box::new(move |_, _| {
                let should_emit = {
                    let mut state = got_focus_state.lock().unwrap();
                    let already_focused = matches!(
                        *state,
                        FocusState::WindowFocused | FocusState::WebviewFocused { .. }
                    );

                    *state = FocusState::WebviewFocused {
                        webview_label: got_focus_label.clone(),
                    };

                    !already_focused
                };

                // Invoke user code only after releasing the state mutex.
                if should_emit {
                    got_focus_callback(WebViewHostEvent::Focused {
                        webview_label: got_focus_label.clone(),
                        focused: true,
                    });
                }

                Ok(())
            })),
            &mut got_focus_token,
        )
    } {
        log::error!(
            "Failed to attach WebView2 `add_GotFocus` handler; focus events will be incomplete: {error}"
        );
        return;
    }

    let lost_focus_callback = callback;
    let lost_focus_label = label;
    let mut lost_focus_token = 0;

    if let Err(error) = unsafe {
        controller.add_LostFocus(
            &FocusChangedEventHandler::create(Box::new(move |_, _| {
                let should_emit = {
                    let mut state = focused_webview.lock().unwrap();

                    match &*state {
                        FocusState::WebviewFocused { webview_label }
                            if webview_label == &lost_focus_label =>
                        {
                            *state = FocusState::Blured {
                                last_focused_webview_label: Some(lost_focus_label.clone()),
                            };
                            true
                        }
                        _ => false,
                    }
                };

                // No EventLoopProxy: invoke the callback directly on the
                // WebView/UI thread after releasing the mutex.
                if should_emit {
                    lost_focus_callback(WebViewHostEvent::Focused {
                        webview_label: lost_focus_label.clone(),
                        focused: false,
                    });
                }

                Ok(())
            })),
            &mut lost_focus_token,
        )
    } {
        log::error!(
            "Failed to attach WebView2 `add_LostFocus` handler; focus events will be incomplete: {error}"
        );
    }
}

#[cfg(windows)]
pub fn add_fullscreen_change_listener(
    callback: WebViewHostEventCallback,
    label: String,
    controller: &ICoreWebView2Controller,
) {
    use webview2_com::ContainsFullScreenElementChangedEventHandler;

    let Ok(core_webview) = (unsafe { controller.CoreWebView2() }) else {
        log::error!("Failed to obtain `ICoreWebView2` for fullscreen events");
        return;
    };

    let mut fullscreen_token = 0;

    if let Err(error) = unsafe {
        core_webview.add_ContainsFullScreenElementChanged(
            &ContainsFullScreenElementChangedEventHandler::create(Box::new(move |sender, _| {
                let mut contains_fullscreen_element = windows::core::BOOL::default();

                sender
                    .ok_or_else(windows::core::Error::empty)?
                    .ContainsFullScreenElement(&mut contains_fullscreen_element)?;

                callback(WebViewHostEvent::FullscreenChanged {
                    webview_label: label.clone(),
                    fullscreen: contains_fullscreen_element.as_bool(),
                });

                Ok(())
            })),
            &mut fullscreen_token,
        )
    } {
        log::error!("Failed to attach WebView2 fullscreen handler: {error}");
    }
}

#[derive(Debug)]
pub enum FocusState {
    WindowFocused,
    WebviewFocused {
        webview_label: String,
    },
    Blured {
        last_focused_webview_label: Option<String>,
    },
}

impl Default for FocusState {
    fn default() -> Self {
        Self::Blured {
            last_focused_webview_label: None,
        }
    }
}

pub fn build_webview<'a>(
    window: &Window,
    webview_label: String,
    kind: bool,
    focused_webview: Arc<Mutex<FocusState>>,
    webview_builder: WebViewBuilder<'a>,
    callback: Option<WebViewHostEventCallback>,
) -> crate::Result<WebView> {
    // Build the WebView

    let webview = match kind {
        #[cfg(not(any(
            target_os = "windows",
            target_os = "macos",
            target_os = "ios",
            target_os = "android"
        )))]
        true => {
            // only way to account for menu bar height, and also works for multiwebviews :)
            let vbox = window.default_vbox().unwrap();
            webview_builder
                .build_gtk(vbox)
                .map_err(|e| crate::Error::WebViewCreationFailed(e.to_string()))?;
        }
        #[cfg(any(
            target_os = "windows",
            target_os = "macos",
            target_os = "ios",
            target_os = "android"
        ))]
        true => webview_builder
            .build_as_child(&window)
            .map_err(|e| crate::Error::WebViewCreationFailed(e.to_string()))?,
        false => {
            #[cfg(any(
                target_os = "windows",
                target_os = "macos",
                target_os = "ios",
                target_os = "android"
            ))]
            let builder = webview_builder
                .build(&window)
                .map_err(|e| crate::Error::WebViewCreationFailed(e.to_string()))?;
            #[cfg(not(any(
                target_os = "windows",
                target_os = "macos",
                target_os = "ios",
                target_os = "android"
            )))]
            let builder = {
                let vbox = window.default_vbox().unwrap();
                webview_builder
                    .build_gtk(vbox)
                    .map_err(|e| crate::Error::WebViewCreationFailed(e.to_string()))?;
            };
            builder
        }
    };

    if !kind {
        #[cfg(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ))]
        undecorated_resizing::attach_resize_handler(&webview);
        #[cfg(windows)]
        if window.is_resizable() && !window.is_decorated() {
            use crate::undecorated_resizing;
            use tao::platform::windows::WindowExtWindows;

            undecorated_resizing::attach_resize_handler(
                window.hwnd(),
                window.has_undecorated_shadow(),
            );
        }
    }

    #[cfg(windows)]
    if let Some(callback) = callback {
        use std::rc::Rc;

        let controller = webview.controller();
        add_focus_change_listeners(
            Rc::clone(&callback),
            focused_webview,
            webview_label.clone(),
            &controller,
        );
        add_fullscreen_change_listener(callback, webview_label.clone(), &controller);
    }

    Ok(webview)
}


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