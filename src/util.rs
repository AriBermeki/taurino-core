// Portions of this file are derived from Tauri:
// https://github.com/tauri-apps/tauri
//
// Copyright 2019-2025 The Tauri Programme
// within The Commons Conservancy
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[cfg_attr(not(windows), allow(unused_imports))]
pub use imp::*;
use semver::Version;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::ffi::OsString;
use std::io::BufRead;
#[cfg(windows)]
use webview2_com::FocusChangedEventHandler;
#[cfg(windows)]
use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Controller;
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

    pub fn encode_wide(string: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
        string.as_ref().encode_wide().chain(once(0)).collect()
    }

    // Helper function to dynamically load function pointer.
    // `library` and `function` must be zero-terminated.
    pub(super) fn get_function_impl(library: &str, function: &str) -> FARPROC {
        let library = encode_wide(library);
        assert_eq!(function.chars().last(), Some('\0'));

        // Library names we will use are ASCII so we can use the A version to avoid string conversion.
        let module =
            unsafe { LoadLibraryW(PCWSTR::from_raw(library.as_ptr())) }.unwrap_or_default();
        if module.is_invalid() {
            return None;
        }

        unsafe { GetProcAddress(module, PCSTR::from_raw(function.as_ptr())) }
    }

    macro_rules! get_function {
        ($lib:expr, $func:ident) => {
            $crate::util::get_function_impl($lib, concat!(stringify!($func), '\0'))
                .map(|f| unsafe { std::mem::transmute::<_, $func>(f) })
        };
    }

    type GetDpiForWindow = unsafe extern "system" fn(hwnd: HWND) -> u32;
    type GetDpiForMonitor = unsafe extern "system" fn(
        hmonitor: HMONITOR,
        dpi_type: MONITOR_DPI_TYPE,
        dpi_x: *mut u32,
        dpi_y: *mut u32,
    ) -> HRESULT;
    type GetSystemMetricsForDpi =
        unsafe extern "system" fn(nindex: SYSTEM_METRICS_INDEX, dpi: u32) -> i32;

    static GET_DPI_FOR_WINDOW: Lazy<Option<GetDpiForWindow>> =
        Lazy::new(|| get_function!("user32.dll", GetDpiForWindow));
    static GET_DPI_FOR_MONITOR: Lazy<Option<GetDpiForMonitor>> =
        Lazy::new(|| get_function!("shcore.dll", GetDpiForMonitor));
    static GET_SYSTEM_METRICS_FOR_DPI: Lazy<Option<GetSystemMetricsForDpi>> =
        Lazy::new(|| get_function!("user32.dll", GetSystemMetricsForDpi));

    #[allow(non_snake_case)]
    pub unsafe fn hwnd_dpi(hwnd: HWND) -> u32 {
        if let Some(GetDpiForWindow) = *GET_DPI_FOR_WINDOW {
            // We are on Windows 10 Anniversary Update (1607) or later.
            match unsafe { GetDpiForWindow(hwnd) } {
                0 => USER_DEFAULT_SCREEN_DPI, // 0 is returned if hwnd is invalid
                dpi => dpi,
            }
        } else if let Some(GetDpiForMonitor) = *GET_DPI_FOR_MONITOR {
            // We are on Windows 8.1 or later.
            let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
            if monitor.is_invalid() {
                return USER_DEFAULT_SCREEN_DPI;
            }

            let mut dpi_x = 0;
            let mut dpi_y = 0;
            if unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) }
                .is_ok()
            {
                dpi_x
            } else {
                USER_DEFAULT_SCREEN_DPI
            }
        } else {
            // We are on Vista or later.
            if unsafe { IsProcessDPIAware() }.as_bool() {
                let hdc = unsafe { GetDC(Some(hwnd)) };
                if hdc.is_invalid() {
                    return USER_DEFAULT_SCREEN_DPI;
                }
                // If the process is DPI aware, then scaling must be handled by the application using
                // this DPI value.
                let dpi = unsafe { GetDeviceCaps(Some(hdc), LOGPIXELSX) } as u32;
                unsafe { ReleaseDC(Some(hwnd), hdc) };
                dpi
            } else {
                // If the process is DPI unaware, then scaling is performed by the OS; we thus return
                // 96 (scale factor 1.0) to prevent the window from being re-scaled by both the
                // application and the WM.
                USER_DEFAULT_SCREEN_DPI
            }
        }
    }

    #[allow(non_snake_case)]
    pub unsafe fn get_system_metrics_for_dpi(nindex: SYSTEM_METRICS_INDEX, dpi: u32) -> i32 {
        if let Some(GetSystemMetricsForDpi) = *GET_SYSTEM_METRICS_FOR_DPI {
            unsafe { GetSystemMetricsForDpi(nindex, dpi) }
        } else {
            unsafe { GetSystemMetrics(nindex) }
        }
    }
}

/// Reconstructs a path from its components using the platform separator then converts it to String and removes UNC prefixes on Windows if it exists.
pub fn display_path<P: AsRef<Path>>(p: P) -> String {
    dunce::simplified(&p.as_ref().components().collect::<PathBuf>())
        .display()
        .to_string()
}

/// Write the file only if the content of the existing file (if any) is different.
///
/// This will always write unless the file exists with identical content.
pub fn write_if_changed<P, C>(path: P, content: C) -> std::io::Result<()>
where
    P: AsRef<Path>,
    C: AsRef<[u8]>,
{
    if let Ok(existing) = std::fs::read(&path) {
        if existing == content.as_ref() {
            return Ok(());
        }
    }

    std::fs::write(path, content)
}

/// Information about environment variables.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Env {
    /// The APPIMAGE environment variable.
    #[cfg(target_os = "linux")]
    pub appimage: Option<std::ffi::OsString>,
    /// The APPDIR environment variable.
    #[cfg(target_os = "linux")]
    pub appdir: Option<std::ffi::OsString>,
    /// The command line arguments of the current process.
    pub args_os: Vec<OsString>,
}

#[allow(clippy::derivable_impls)]
impl Default for Env {
    fn default() -> Self {
        let args_os = std::env::args_os().collect();
        #[cfg(target_os = "linux")]
        {
            let env = Self {
                #[cfg(target_os = "linux")]
                appimage: std::env::var_os("APPIMAGE"),
                #[cfg(target_os = "linux")]
                appdir: std::env::var_os("APPDIR"),
                args_os,
            };
            if env.appimage.is_some() || env.appdir.is_some() {
                // validate that we're actually running on an AppImage
                // an AppImage is mounted to `/$TEMPDIR/.mount_${appPrefix}${hash}`
                // see <https://github.com/AppImage/AppImageKit/blob/1681fd84dbe09c7d9b22e13cdb16ea601aa0ec47/src/runtime.c#L501>
                // note that it is safe to use `std::env::current_exe` here since we just loaded an AppImage.
                let is_temp = std::env::current_exe()
                    .map(|p| {
                        p.display()
                            .to_string()
                            .starts_with(&format!("{}/.mount_", std::env::temp_dir().display()))
                    })
                    .unwrap_or(true);

                if !is_temp {
                    log::warn!(
                        "`APPDIR` or `APPIMAGE` environment variable found but this application was not detected as an AppImage; this might be a security issue."
                    );
                }
            }
            env
        }
        #[cfg(not(target_os = "linux"))]
        {
            Self { args_os }
        }
    }
}

/// Read all bytes until a newline (the `0xA` byte) or a carriage return (`\r`) is reached, and append them to the provided buffer.
///
/// Adapted from <https://doc.rust-lang.org/std/io/trait.BufRead.html#method.read_line>.
pub fn read_line<R: BufRead + ?Sized>(r: &mut R, buf: &mut Vec<u8>) -> std::io::Result<usize> {
    let mut read = 0;
    loop {
        let (done, used) = {
            let available = match r.fill_buf() {
                Ok(n) => n,
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,

                Err(e) => return Err(e),
            };
            match memchr::memchr(b'\n', available) {
                Some(i) => {
                    let end = i + 1;
                    buf.extend_from_slice(&available[..end]);
                    (true, end)
                }
                None => match memchr::memchr(b'\r', available) {
                    Some(i) => {
                        let end = i + 1;
                        buf.extend_from_slice(&available[..end]);
                        (true, end)
                    }
                    None => {
                        buf.extend_from_slice(available);
                        (false, available.len())
                    }
                },
            }
        };
        r.consume(used);
        read += used;
        if done || used == 0 {
            return Ok(read);
        }
    }
}

use std::fmt::{self, Display};
use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::rc::Rc;
#[cfg(windows)]
use std::sync::{Arc, Mutex};

// Import schemars nur wenn das feature aktiviert ist
#[cfg(feature = "schema")]
use schemars::JsonSchema;

const MIMETYPE_PLAIN: &str = "text/plain";

/// [Web Compatible MimeTypes](https://developer.mozilla.org/en-US/docs/Web/HTTP/Basics_of_HTTP/MIME_types#important_mime_types_for_web_developers)
#[allow(missing_docs)]
pub enum MimeType {
    Css,
    Csv,
    Html,
    Ico,
    Js,
    Json,
    Jsonld,
    Mp4,
    OctetStream,
    Rtf,
    Svg,
    Txt,
}

impl std::fmt::Display for MimeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mime = match self {
            MimeType::Css => "text/css",
            MimeType::Csv => "text/csv",
            MimeType::Html => "text/html",
            MimeType::Ico => "image/vnd.microsoft.icon",
            MimeType::Js => "text/javascript",
            MimeType::Json => "application/json",
            MimeType::Jsonld => "application/ld+json",
            MimeType::Mp4 => "video/mp4",
            MimeType::OctetStream => "application/octet-stream",
            MimeType::Rtf => "application/rtf",
            MimeType::Svg => "image/svg+xml",
            MimeType::Txt => MIMETYPE_PLAIN,
        };
        write!(f, "{mime}")
    }
}

impl MimeType {
    /// parse a URI suffix to convert text/plain mimeType to their actual web compatible mimeType.
    pub fn parse_from_uri(uri: &str) -> MimeType {
        Self::parse_from_uri_with_fallback(uri, Self::Html)
    }

    /// parse a URI suffix to convert text/plain mimeType to their actual web compatible mimeType with specified fallback for unknown file extensions.
    pub fn parse_from_uri_with_fallback(uri: &str, fallback: MimeType) -> MimeType {
        let suffix = uri.split('.').next_back();
        match suffix {
            Some("bin") => Self::OctetStream,
            Some("css" | "less" | "sass" | "styl") => Self::Css,
            Some("csv") => Self::Csv,
            Some("html") => Self::Html,
            Some("ico") => Self::Ico,
            Some("js") => Self::Js,
            Some("json") => Self::Json,
            Some("jsonld") => Self::Jsonld,
            Some("mjs") => Self::Js,
            Some("mp4") => Self::Mp4,
            Some("rtf") => Self::Rtf,
            Some("svg") => Self::Svg,
            Some("txt") => Self::Txt,
            // Assume HTML when a TLD is found for eg. `wry:://tauri.app` | `wry://hello.com`
            Some(_) => fallback,
            // using octet stream according to this:
            // <https://developer.mozilla.org/en-US/docs/Web/HTTP/Basics_of_HTTP/MIME_types/Common_types>
            None => Self::OctetStream,
        }
    }

    /// infer mimetype from content (or) URI if needed.
    pub fn parse(content: &[u8], uri: &str) -> String {
        Self::parse_with_fallback(content, uri, Self::Html)
    }
    /// infer mimetype from content (or) URI if needed with specified fallback for unknown file extensions.
    pub fn parse_with_fallback(content: &[u8], uri: &str, fallback: MimeType) -> String {
        let mime = if uri.ends_with(".svg") {
            // when reading svg, we can't use `infer`
            None
        } else {
            infer::get(content).map(|info| info.mime_type())
        };

        match mime {
            Some(mime) if mime == MIMETYPE_PLAIN => {
                Self::parse_from_uri_with_fallback(uri, fallback).to_string()
            }
            None => Self::parse_from_uri_with_fallback(uri, fallback).to_string(),
            Some(mime) => mime.to_string(),
        }
    }
}

#[cfg(target_os = "android")]
pub const ANDROID_ASSET_PROTOCOL_URI_PREFIX: &str = "asset://localhost/";

/// Platform target.
#[derive(PartialEq, Eq, Copy, Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum Target {
    /// MacOS.
    #[serde(rename = "macOS")]
    MacOS,
    /// Windows.
    Windows,
    /// Linux.
    Linux,
    /// Android.
    Android,
    /// iOS.
    #[serde(rename = "iOS")]
    Ios,
}

impl Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::MacOS => "macOS",
                Self::Windows => "windows",
                Self::Linux => "linux",
                Self::Android => "android",
                Self::Ios => "iOS",
            }
        )
    }
}

impl Target {
    /// Parses the target from the given target triple.
    pub fn from_triple(target: &str) -> Self {
        if target.contains("darwin") {
            Self::MacOS
        } else if target.contains("windows") {
            Self::Windows
        } else if target.contains("android") {
            Self::Android
        } else if target.contains("ios") {
            Self::Ios
        } else {
            Self::Linux
        }
    }

    /// Gets the current build target.
    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::MacOS
        } else if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "ios") {
            Self::Ios
        } else if cfg!(target_os = "android") {
            Self::Android
        } else {
            Self::Linux
        }
    }

    /// Whether the target is mobile or not.
    pub fn is_mobile(&self) -> bool {
        matches!(self, Target::Android | Target::Ios)
    }

    /// Whether the target is desktop or not.
    pub fn is_desktop(&self) -> bool {
        !self.is_mobile()
    }
}

#[derive(Debug, Clone)]
pub struct PackageInfo {
    /// App name
    pub name: String,
    /// App version
    pub version: Version,
    /// The crate authors.
    pub authors: &'static str,
    /// The crate description.
    pub description: &'static str,
    /// The crate name.
    pub crate_name: &'static str,
}

/// A bundle referenced by tauri-bundler.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "schema", schemars(rename_all = "lowercase"))]
pub enum BundleType {
    /// The debian bundle (.deb).
    Deb,
    /// The RPM bundle (.rpm).
    Rpm,
    /// The AppImage bundle (.appimage).
    AppImage,
    /// The Microsoft Installer bundle (.msi).
    Msi,
    /// The NSIS bundle (.exe).
    Nsis,
    /// The macOS application bundle (.app).
    App,
    /// The Apple Disk Image bundle (.dmg).
    Dmg,
}

impl BundleType {
    /// All bundle types.
    fn all() -> &'static [Self] {
        &[
            BundleType::Deb,
            BundleType::Rpm,
            BundleType::AppImage,
            BundleType::Msi,
            BundleType::Nsis,
            BundleType::App,
            BundleType::Dmg,
        ]
    }
}

impl Display for BundleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Deb => "deb",
                Self::Rpm => "rpm",
                Self::AppImage => "appimage",
                Self::Msi => "msi",
                Self::Nsis => "nsis",
                Self::App => "app",
                Self::Dmg => "dmg",
            }
        )
    }
}

impl Serialize for BundleType {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

impl<'de> Deserialize<'de> for BundleType {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "deb" => Ok(Self::Deb),
            "rpm" => Ok(Self::Rpm),
            "appimage" => Ok(Self::AppImage),
            "msi" => Ok(Self::Msi),
            "nsis" => Ok(Self::Nsis),
            "app" => Ok(Self::App),
            "dmg" => Ok(Self::Dmg),
            _ => Err(serde::de::Error::custom(format!(
                "unknown bundle target '{s}'"
            ))),
        }
    }
}

/// Targets to bundle. Each value is case insensitive.
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub enum BundleTarget {
    /// Bundle all targets.
    #[default]
    All,
    /// A list of bundle targets.
    List(Vec<BundleType>),
    /// A single bundle target.
    One(BundleType),
}

#[cfg(feature = "schema")]
impl schemars::JsonSchema for BundleTarget {
    fn schema_name() -> std::string::String {
        "BundleTarget".to_owned()
    }

    fn json_schema(_gen: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
        // Vereinfachte Schema-Implementierung ohne Verwendung von gen.subschema_for
        // um die 'gen' keyword Fehler zu vermeiden
        let any_of = vec![
            schemars::schema::SchemaObject {
                const_value: Some("all".into()),
                metadata: Some(Box::new(schemars::schema::Metadata {
                    description: Some("Bundle all targets.".to_owned()),
                    ..Default::default()
                })),
                ..Default::default()
            }
            .into(),
            // Statt gen.subschema_for verwenden wir eine einfachere String-Repräsentation
            schemars::schema::SchemaObject {
                metadata: Some(Box::new(schemars::schema::Metadata {
                    description: Some("A list of bundle targets.".to_owned()),
                    ..Default::default()
                })),
                ..Default::default()
            }
            .into(),
            schemars::schema::SchemaObject {
                metadata: Some(Box::new(schemars::schema::Metadata {
                    description: Some("A single bundle target.".to_owned()),
                    ..Default::default()
                })),
                ..Default::default()
            }
            .into(),
        ];

        schemars::schema::SchemaObject {
            subschemas: Some(Box::new(schemars::schema::SubschemaValidation {
                any_of: Some(any_of),
                ..Default::default()
            })),
            metadata: Some(Box::new(schemars::schema::Metadata {
                description: Some("Targets to bundle. Each value is case insensitive.".to_owned()),
                ..Default::default()
            })),
            ..Default::default()
        }
        .into()
    }
}

impl Serialize for BundleTarget {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::All => serializer.serialize_str("all"),
            Self::List(l) => l.serialize(serializer),
            Self::One(t) => serializer.serialize_str(t.to_string().as_ref()),
        }
    }
}

impl<'de> Deserialize<'de> for BundleTarget {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize, Serialize)]
        #[serde(untagged)]
        pub enum BundleTargetInner {
            List(Vec<BundleType>),
            One(BundleType),
            All(String),
        }

        match BundleTargetInner::deserialize(deserializer)? {
            BundleTargetInner::All(s) if s.to_lowercase() == "all" => Ok(Self::All),
            BundleTargetInner::All(t) => Err(serde::de::Error::custom(format!(
                "invalid bundle type {t}, expected one of `all`, {}",
                BundleType::all()
                    .iter()
                    .map(|b| format!("`{b}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ))),
            BundleTargetInner::List(l) => Ok(Self::List(l)),
            BundleTargetInner::One(t) => Ok(Self::One(t)),
        }
    }
}

impl BundleTarget {
    /// Gets the bundle targets as a [`Vec`]. The vector is empty when set to [`BundleTarget::All`].
    #[allow(dead_code)]
    pub fn to_vec(&self) -> Vec<BundleType> {
        match self {
            Self::All => BundleType::all().to_vec(),
            Self::List(list) => list.clone(),
            Self::One(i) => vec![i.clone()],
        }
    }
}

/// Configuration for AppImage bundles.
///
/// See more: <https://v2.tauri.app/reference/config/#appimageconfig>
#[derive(Debug, Default, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppImageConfig {
    /// Include additional gstreamer dependencies needed for audio and video playback.
    /// This increases the bundle size by ~15-35MB depending on your build system.
    #[serde(default, alias = "bundle-media-framework")]
    pub bundle_media_framework: bool,
    /// The files to include in the Appimage Binary.
    #[serde(default)]
    pub files: HashMap<PathBuf, PathBuf>,
}

pub fn current_exe() -> std::io::Result<PathBuf> {
    crate::starting_binary::STARTING_BINARY.cloned()
}

/// Try to determine the current target triple.
///
/// Returns a target triple (e.g. `x86_64-unknown-linux-gnu` or `i686-pc-windows-msvc`) or an
/// `Error::Config` if the current config cannot be determined or is not some combination of the
/// following values:
/// `linux, mac, windows` -- `i686, x86, armv7` -- `gnu, musl, msvc`
///
/// * Errors:
///     * Unexpected system config
pub fn target_triple() -> crate::Result<String> {
    let arch = if cfg!(target_arch = "x86") {
        "i686"
    } else if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "arm") {
        "armv7"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else if cfg!(target_arch = "riscv64") {
        "riscv64"
    } else {
        return Err(crate::Error::Architecture);
    };

    let os = if cfg!(target_os = "linux") {
        "unknown-linux"
    } else if cfg!(target_os = "macos") {
        "apple-darwin"
    } else if cfg!(target_os = "windows") {
        "pc-windows"
    } else if cfg!(target_os = "freebsd") {
        "unknown-freebsd"
    } else {
        return Err(crate::Error::Os);
    };

    let os = if cfg!(target_os = "macos") || cfg!(target_os = "freebsd") {
        String::from(os)
    } else {
        let env = if cfg!(target_env = "gnu") {
            "gnu"
        } else if cfg!(target_env = "musl") {
            "musl"
        } else if cfg!(target_env = "msvc") {
            "msvc"
        } else {
            return Err(crate::Error::Environment);
        };

        format!("{os}-{env}")
    };

    Ok(format!("{arch}-{os}"))
}

#[cfg(all(not(test), not(target_os = "android")))]
fn is_cargo_output_directory(path: &std::path::Path) -> bool {
    path.join(".cargo-lock").exists()
}

#[cfg(test)]
const CARGO_OUTPUT_DIRECTORIES: &[&str] = &["debug", "release", "custom-profile"];

#[cfg(test)]
fn is_cargo_output_directory(path: &std::path::Path) -> bool {
    let Some(last_component) = path.components().next_back() else {
        return false;
    };
    CARGO_OUTPUT_DIRECTORIES
        .iter()
        .any(|dirname| &last_component.as_os_str() == dirname)
}

pub fn resource_dir(package_info: &PackageInfo, env: &Env) -> crate::Result<PathBuf> {
    #[cfg(target_os = "android")]
    return resource_dir_android(package_info, env);
    #[cfg(not(target_os = "android"))]
    {
        let exe = current_exe()?;
        resource_dir_from(exe, package_info, env)
    }
}

#[cfg(target_os = "android")]
fn resource_dir_android(_package_info: &PackageInfo, _env: &Env) -> crate::Result<PathBuf> {
    Ok(PathBuf::from(ANDROID_ASSET_PROTOCOL_URI_PREFIX))
}

#[cfg(not(target_os = "android"))]
#[allow(unused_variables)]
fn resource_dir_from<P: AsRef<std::path::Path>>(
    exe: P,
    package_info: &PackageInfo,
    env: &Env,
) -> crate::Result<PathBuf> {
    let exe_dir = exe.as_ref().parent().expect("failed to get exe directory");
    let curr_dir = exe_dir.display().to_string();

    let parts: Vec<&str> = curr_dir.split(std::path::MAIN_SEPARATOR).collect();
    let len = parts.len();

    // Check if running from the Cargo output directory, which means it's an executable in a development machine
    // We check if the binary is inside a `target` folder which can be either `target/$profile` or `target/$triple/$profile`
    // and see if there's a .cargo-lock file along the executable
    // This ensures the check is safer so it doesn't affect apps in production
    // Windows also includes the resources in the executable folder so we check that too
    if cfg!(target_os = "windows")
        || ((len >= 2 && parts[len - 2] == "target") || (len >= 3 && parts[len - 3] == "target"))
            && is_cargo_output_directory(exe_dir)
    {
        return Ok(exe_dir.to_path_buf());
    }

    #[allow(unused_mut, unused_assignments)]
    let mut res = Err(crate::Error::UnsupportedPlatform);

    #[cfg(target_os = "linux")]
    {
        // (canonicalize checks for existence, so there's no need for an extra check)
        res = if let Ok(bundle_dir) = exe_dir
            .join(format!("../lib/{}", package_info.name))
            .canonicalize()
        {
            Ok(bundle_dir)
        } else if let Some(appdir) = &env.appdir {
            let appdir: &std::path::Path = appdir.as_ref();
            Ok(PathBuf::from(format!(
                "{}/usr/lib/{}",
                appdir.display(),
                package_info.name
            )))
        } else {
            // running bundle
            Ok(PathBuf::from(format!("/usr/lib/{}", package_info.name)))
        };
    }

    #[cfg(target_os = "macos")]
    {
        res = exe_dir
            .join("../Resources")
            .canonicalize()
            .map_err(Into::into);
    }

    #[cfg(target_os = "ios")]
    {
        res = exe_dir.join("assets").canonicalize().map_err(Into::into);
    }

    res
}

// Variable holding the type of bundle the executable is stored in. This is modified by binary
// patching during build
#[used]
// Marked as `mut` because it could get optimized away without it,
// see https://github.com/tauri-apps/tauri/pull/13812
static mut __TAURINO_BUNDLE_TYPE: &str = "__TAURINO_BUNDLE_TYPE_VAR_UNK";

/// Get the type of the bundle current binary is packaged in.
/// If the bundle type is unknown, it returns [`Option::None`].
pub fn bundle_type() -> Option<BundleType> {
    unsafe {
        match __TAURINO_BUNDLE_TYPE {
            "__TAURINO_BUNDLE_TYPE_VAR_DEB" => Some(BundleType::Deb),
            "__TAURINO_BUNDLE_TYPE_VAR_RPM" => Some(BundleType::Rpm),
            "__TAURINO_BUNDLE_TYPE_VAR_APP" => Some(BundleType::AppImage),
            "__TAURINO_BUNDLE_TYPE_VAR_MSI" => Some(BundleType::Msi),
            "__TAURINO_BUNDLE_TYPE_VAR_NSS" => Some(BundleType::Nsis),
            _ => {
                if cfg!(target_os = "macos") {
                    Some(BundleType::App)
                } else {
                    None
                }
            }
        }
    }
}

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
    window: &crate::window::tao::window::Window,
    webview_label: String,
    kind: bool,
    focused_webview: Arc<Mutex<FocusState>>,
    webview_builder: crate::webview::wry::WebViewBuilder<'a>,
    callback: Option<WebViewHostEventCallback>,
) -> crate::Result<crate::webview::wry::WebView> {
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
        use wry::WebViewExtWindows;
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
