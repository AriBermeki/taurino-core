// Portions of this file are derived from Tauri:
// https://github.com/tauri-apps/tauri
//
// Copyright 2019-2025 The Tauri Programme
// within The Commons Conservancy
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[cfg_attr(not(windows), allow(unused_imports))]
pub use imp::*;

#[cfg(not(windows))]
mod imp {}

#[cfg(windows)]
mod imp {
    use std::{iter::once, os::windows::ffi::OsStrExt};

    use once_cell::sync::Lazy;
    use windows::{
        Win32::{
            Foundation::*,
            Graphics::Gdi::*,
            System::LibraryLoader::{GetProcAddress, LoadLibraryW},
            UI::{HiDpi::*, WindowsAndMessaging::*},
        },
        core::{HRESULT, PCSTR, PCWSTR},
    };

    type GetDpiForMonitorFn = unsafe extern "system" fn(
        monitor: HMONITOR,
        dpi_type: MONITOR_DPI_TYPE,
        dpi_x: *mut u32,
        dpi_y: *mut u32,
    ) -> HRESULT;

    type GetDpiForWindowFn = unsafe extern "system" fn(hwnd: HWND) -> u32;

    type GetSystemMetricsForDpiFn =
        unsafe extern "system" fn(index: SYSTEM_METRICS_INDEX, dpi: u32) -> i32;

    macro_rules! get_function {
        ($library:expr, $function_type:ty, $symbol:literal) => {{
            // SAFETY: The symbol is null-terminated and the requested type must
            // match the exported function's documented ABI and signature.
            unsafe {
                get_function_impl($library, concat!($symbol, '\0'))
                    .map(|function| std::mem::transmute::<_, $function_type>(function))
            }
        }};
    }

    static GET_DPI_FOR_MONITOR: Lazy<Option<GetDpiForMonitorFn>> =
        Lazy::new(|| get_function!("shcore.dll", GetDpiForMonitorFn, "GetDpiForMonitor"));

    static GET_DPI_FOR_WINDOW: Lazy<Option<GetDpiForWindowFn>> =
        Lazy::new(|| get_function!("user32.dll", GetDpiForWindowFn, "GetDpiForWindow"));

    static GET_SYSTEM_METRICS_FOR_DPI: Lazy<Option<GetSystemMetricsForDpiFn>> = Lazy::new(|| {
        get_function!(
            "user32.dll",
            GetSystemMetricsForDpiFn,
            "GetSystemMetricsForDpi"
        )
    });

    /// Encodes a Windows string as a null-terminated UTF-16 buffer.
    pub fn encode_wide(value: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
        value.as_ref().encode_wide().chain(once(0)).collect()
    }

    /// Returns the effective DPI for the specified native window.
    ///
    /// Uses the newest DPI API available on the current Windows version and
    /// falls back to the process or device DPI when required.
    ///
    /// # Safety
    ///
    /// `hwnd` must either be a valid window handle or a handle accepted by the
    /// underlying Win32 APIs. The caller must ensure that the handle remains
    /// valid for the duration of this call.
    pub unsafe fn hwnd_dpi(hwnd: HWND) -> u32 {
        if let Some(get_dpi_for_window) = *GET_DPI_FOR_WINDOW {
            // SAFETY: The function pointer is loaded from user32.dll under the
            // exact `GetDpiForWindow` symbol and uses the documented ABI.
            let dpi = unsafe { get_dpi_for_window(hwnd) };

            return if dpi == 0 {
                USER_DEFAULT_SCREEN_DPI
            } else {
                dpi
            };
        }

        if let Some(get_dpi_for_monitor) = *GET_DPI_FOR_MONITOR {
            // SAFETY: The caller guarantees that `hwnd` is suitable for the
            // Win32 call. No ownership is transferred.
            let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };

            if monitor.is_invalid() {
                return USER_DEFAULT_SCREEN_DPI;
            }

            let mut dpi_x = 0;
            let mut dpi_y = 0;

            // SAFETY: The function pointer is loaded from shcore.dll under the
            // exact `GetDpiForMonitor` symbol. Both output pointers are valid
            // for writes for the duration of the call.
            let result =
                unsafe { get_dpi_for_monitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) };

            return if result.is_ok() {
                dpi_x
            } else {
                USER_DEFAULT_SCREEN_DPI
            };
        }

        // SAFETY: This Win32 query has no pointer arguments and requires no
        // additional invariants from the caller.
        if unsafe { IsProcessDPIAware().as_bool() } {
            // SAFETY: The caller guarantees that `hwnd` is valid for the call.
            // The returned device context is released before returning.
            let device_context = unsafe { GetDC(Some(hwnd)) };

            if device_context.is_invalid() {
                return USER_DEFAULT_SCREEN_DPI;
            }

            // SAFETY: `device_context` was successfully acquired above and is
            // valid until it is released below.
            let dpi = unsafe { GetDeviceCaps(Some(device_context), LOGPIXELSX) as u32 };

            // SAFETY: `device_context` was acquired for `hwnd` by `GetDC` and
            // is released exactly once here.
            unsafe {
                ReleaseDC(Some(hwnd), device_context);
            }

            dpi
        } else {
            USER_DEFAULT_SCREEN_DPI
        }
    }

    /// Returns a system metric scaled for the specified DPI.
    ///
    /// Uses `GetSystemMetricsForDpi` when available and otherwise falls back to
    /// the unscaled `GetSystemMetrics` API.
    ///
    /// # Safety
    ///
    /// `index` must identify a valid Windows system metric. The dynamically
    /// loaded function pointer must continue to refer to the module from which
    /// it was resolved.
    pub unsafe fn get_system_metrics_for_dpi(index: SYSTEM_METRICS_INDEX, dpi: u32) -> i32 {
        if let Some(get_system_metrics_for_dpi) = *GET_SYSTEM_METRICS_FOR_DPI {
            // SAFETY: The function pointer is loaded from user32.dll under the
            // exact `GetSystemMetricsForDpi` symbol and uses the documented ABI.
            unsafe { get_system_metrics_for_dpi(index, dpi) }
        } else {
            // SAFETY: `index` is guaranteed by the caller to be valid.
            unsafe { GetSystemMetrics(index) }
        }
    }

    /// Loads an exported function from a Windows dynamic library.
    ///
    /// Returns `None` when the library cannot be loaded or the symbol cannot be
    /// resolved.
    ///
    /// # Safety
    ///
    /// `function` must be an ASCII symbol name terminated by a null byte. The
    /// returned pointer must only be called through a function type matching
    /// the exported symbol's ABI and signature.
    pub(super) unsafe fn get_function_impl(library: &str, function: &str) -> FARPROC {
        assert_eq!(function.as_bytes().last(), Some(&0));

        let library = encode_wide(library);

        // SAFETY: `library` is a valid null-terminated UTF-16 buffer and remains
        // alive for the duration of the call.
        let module =
            unsafe { LoadLibraryW(PCWSTR::from_raw(library.as_ptr())) }.unwrap_or_default();

        if module.is_invalid() {
            return None;
        }

        // SAFETY: `function` is required to be a null-terminated ASCII symbol
        // name. The loaded module remains resident for the process lifetime.
        unsafe { GetProcAddress(module, PCSTR::from_raw(function.as_ptr())) }
    }
}
