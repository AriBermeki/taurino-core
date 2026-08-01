// Portions of this file are derived from Tauri:
// https://github.com/tauri-apps/tauri
//
// Copyright 2019-2025 The Tauri Programme
// within The Commons Conservancy
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::{
    borrow::Cow,
    fmt::{self, Display},
    path::PathBuf,
    str::FromStr,
};

pub use dpi::*;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use serde_with::skip_serializing_none;
use tao::window::Theme as TaoTheme;
use url::Url;

#[cfg(feature = "schema")]
use schemars::JsonSchema;

/// Defines how background execution is throttled.
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum BackgroundThrottlingPolicy {
    Disabled,
    Suspend,
    Throttle,
}

/// Represents an RGBA color using 8-bit channels.
#[derive(Debug, PartialEq, Eq, Serialize, Default, Clone, Copy)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Color(pub u8, pub u8, pub u8, pub u8);

impl From<Color> for (u8, u8, u8, u8) {
    fn from(value: Color) -> Self {
        (value.0, value.1, value.2, value.3)
    }
}

impl From<Color> for (u8, u8, u8) {
    fn from(value: Color) -> Self {
        (value.0, value.1, value.2)
    }
}

impl From<(u8, u8, u8, u8)> for Color {
    fn from(value: (u8, u8, u8, u8)) -> Self {
        Color(value.0, value.1, value.2, value.3)
    }
}

impl From<(u8, u8, u8)> for Color {
    fn from(value: (u8, u8, u8)) -> Self {
        Color(value.0, value.1, value.2, 255)
    }
}

impl From<Color> for [u8; 4] {
    fn from(value: Color) -> Self {
        [value.0, value.1, value.2, value.3]
    }
}

impl From<Color> for [u8; 3] {
    fn from(value: Color) -> Self {
        [value.0, value.1, value.2]
    }
}

impl From<[u8; 4]> for Color {
    fn from(value: [u8; 4]) -> Self {
        Color(value[0], value[1], value[2], value[3])
    }
}

impl From<[u8; 3]> for Color {
    fn from(value: [u8; 3]) -> Self {
        Color(value[0], value[1], value[2], 255)
    }
}

impl FromStr for Color {
    type Err = String;
    fn from_str(mut color: &str) -> Result<Self, Self::Err> {
        color = color.trim().strip_prefix('#').unwrap_or(color);
        let color = match color.len() {
            3 => color
                .chars()
                .flat_map(|c| std::iter::repeat_n(c, 2))
                .chain(std::iter::repeat_n('f', 2))
                .collect(),
            6 => format!("{color}FF"),
            8 => color.to_string(),
            _ => {
                return Err(
                    "Invalid hex color length, must be either 3, 6 or 8, for example: #fff, #ffffff, or #ffffffff"
                        .into(),
                );
            }
        };

        let r = u8::from_str_radix(&color[0..2], 16).map_err(|e| e.to_string())?;
        let g = u8::from_str_radix(&color[2..4], 16).map_err(|e| e.to_string())?;
        let b = u8::from_str_radix(&color[4..6], 16).map_err(|e| e.to_string())?;
        let a = u8::from_str_radix(&color[6..8], 16).map_err(|e| e.to_string())?;

        Ok(Color(r, g, b, a))
    }
}

#[cfg(feature = "schema")]
impl schemars::JsonSchema for Color {
    fn schema_name() -> String {
        "Color".to_string()
    }

    fn json_schema(_gen: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
        let mut schema = schemars::schema_for!(InnerColor).schema;
        schema.metadata = None;

        let any_of = schema.subschemas().any_of.as_mut().unwrap();
        let schemars::schema::Schema::Object(str_schema) = any_of.first_mut().unwrap() else {
            unreachable!()
        };
        str_schema.string().pattern =
            Some("^#?([A-Fa-f0-9]{3}|[A-Fa-f0-9]{6}|[A-Fa-f0-9]{8})$".into());

        schema.into()
    }
}

/// Defines the cursor displayed by the window.
#[non_exhaustive]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub enum CursorIcon {
    #[default]
    Default,
    Crosshair,
    Hand,
    Arrow,
    Move,
    Text,
    Wait,
    Help,
    Progress,
    NotAllowed,
    ContextMenu,
    Cell,
    VerticalText,
    Alias,
    Copy,
    NoDrop,
    Grab,
    Grabbing,
    AllScroll,
    ZoomIn,
    ZoomOut,
    EResize,
    NResize,
    NeResize,
    NwResize,
    SResize,
    SeResize,
    SwResize,
    WResize,
    EwResize,
    NsResize,
    NeswResize,
    NwseResize,
    ColResize,
    RowResize,
}

impl<'de> Deserialize<'de> for CursorIcon {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.to_lowercase().as_str() {
            "default" => CursorIcon::Default,
            "crosshair" => CursorIcon::Crosshair,
            "hand" => CursorIcon::Hand,
            "arrow" => CursorIcon::Arrow,
            "move" => CursorIcon::Move,
            "text" => CursorIcon::Text,
            "wait" => CursorIcon::Wait,
            "help" => CursorIcon::Help,
            "progress" => CursorIcon::Progress,
            "notallowed" => CursorIcon::NotAllowed,
            "contextmenu" => CursorIcon::ContextMenu,
            "cell" => CursorIcon::Cell,
            "verticaltext" => CursorIcon::VerticalText,
            "alias" => CursorIcon::Alias,
            "copy" => CursorIcon::Copy,
            "nodrop" => CursorIcon::NoDrop,
            "grab" => CursorIcon::Grab,
            "grabbing" => CursorIcon::Grabbing,
            "allscroll" => CursorIcon::AllScroll,
            "zoomin" => CursorIcon::ZoomIn,
            "zoomout" => CursorIcon::ZoomOut,
            "eresize" => CursorIcon::EResize,
            "nresize" => CursorIcon::NResize,
            "neresize" => CursorIcon::NeResize,
            "nwresize" => CursorIcon::NwResize,
            "sresize" => CursorIcon::SResize,
            "seresize" => CursorIcon::SeResize,
            "swresize" => CursorIcon::SwResize,
            "wresize" => CursorIcon::WResize,
            "ewresize" => CursorIcon::EwResize,
            "nsresize" => CursorIcon::NsResize,
            "neswresize" => CursorIcon::NeswResize,
            "nwseresize" => CursorIcon::NwseResize,
            "colresize" => CursorIcon::ColResize,
            "rowresize" => CursorIcon::RowResize,
            _ => CursorIcon::Default,
        })
    }
}

impl Serialize for CursorIcon {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(match self {
            CursorIcon::Default => "default",
            CursorIcon::Crosshair => "crosshair",
            CursorIcon::Hand => "hand",
            CursorIcon::Arrow => "arrow",
            CursorIcon::Move => "move",
            CursorIcon::Text => "text",
            CursorIcon::Wait => "wait",
            CursorIcon::Help => "help",
            CursorIcon::Progress => "progress",
            CursorIcon::NotAllowed => "notallowed",
            CursorIcon::ContextMenu => "contextmenu",
            CursorIcon::Cell => "cell",
            CursorIcon::VerticalText => "verticaltext",
            CursorIcon::Alias => "alias",
            CursorIcon::Copy => "copy",
            CursorIcon::NoDrop => "nodrop",
            CursorIcon::Grab => "grab",
            CursorIcon::Grabbing => "grabbing",
            CursorIcon::AllScroll => "allscroll",
            CursorIcon::ZoomIn => "zoomin",
            CursorIcon::ZoomOut => "zoomout",
            CursorIcon::EResize => "eresize",
            CursorIcon::NResize => "nresize",
            CursorIcon::NeResize => "neresize",
            CursorIcon::NwResize => "nwresize",
            CursorIcon::SResize => "sresize",
            CursorIcon::SeResize => "seresize",
            CursorIcon::SwResize => "swresize",
            CursorIcon::WResize => "wresize",
            CursorIcon::EwResize => "ewresize",
            CursorIcon::NsResize => "nsresize",
            CursorIcon::NeswResize => "neswresize",
            CursorIcon::NwseResize => "nwseresize",
            CursorIcon::ColResize => "colresize",
            CursorIcon::RowResize => "rowresize",
        })
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(tag = "type")]
pub enum DeviceEventFilter {
    Always,
    #[default]
    Unfocused,
    Never,
}

/// Defines the frontend source loaded by the application.
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(untagged, deny_unknown_fields)]
#[non_exhaustive]
pub enum FrontendDist {
    Url(Url),
    Directory(PathBuf),
}

impl std::fmt::Display for FrontendDist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Url(url) => write!(f, "{url}"),
            Self::Directory(p) => write!(f, "{}", p.display()),
        }
    }
}

/// Represents a window icon in RGBA format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Icon<'a> {
    pub rgba: Cow<'a, [u8]>,
    pub width: u32,
    pub height: u32,
}

/// Represents a rectangle in logical pixels.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct LogicalRect<P: dpi::Pixel, S: dpi::Pixel> {
    pub position: dpi::LogicalPosition<P>,
    pub size: dpi::LogicalSize<S>,
}

impl<P: dpi::Pixel, S: dpi::Pixel> Default for LogicalRect<P, S> {
    fn default() -> Self {
        Self {
            position: (0, 0).into(),
            size: (0, 0).into(),
        }
    }
}

/// Describes an available display monitor.
#[derive(Debug, Clone)]
pub struct Monitor {
    pub name: Option<String>,
    pub size: PhysicalSize<u32>,
    pub position: PhysicalPosition<i32>,
    pub work_area: PhysicalRect<i32, u32>,
    pub scale_factor: f64,
}

/// Represents a rectangle in physical pixels.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct PhysicalRect<P: dpi::Pixel, S: dpi::Pixel> {
    pub position: dpi::PhysicalPosition<P>,
    pub size: dpi::PhysicalSize<S>,
}

impl<P: dpi::Pixel, S: dpi::Pixel> Default for PhysicalRect<P, S> {
    fn default() -> Self {
        Self {
            position: (0, 0).into(),
            size: (0, 0).into(),
        }
    }
}

/// Defines the taskbar progress state.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProgressBarStatus {
    None,
    Normal,
    Indeterminate,
    Paused,
    Error,
}

/// Configures taskbar progress reporting.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressBarState {
    pub status: Option<ProgressBarStatus>,
    pub progress: Option<u64>,
    pub desktop_filename: Option<String>,
}

/// Represents a rectangle using logical or physical units.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Rect {
    pub position: dpi::Position,
    pub size: dpi::Size,
}

impl Default for Rect {
    fn default() -> Self {
        Self {
            position: Position::Logical((0, 0).into()),
            size: Size::Logical((0, 0).into()),
        }
    }
}

/// Defines the direction of an interactive window resize.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum ResizeDirection {
    East,
    North,
    NorthEast,
    NorthWest,
    South,
    SouthEast,
    SouthWest,
    West,
}

/// Defines the scrollbar style used by a webview.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default)]
pub enum ScrollBarStyle {
    #[default]
    Default,

    #[cfg(windows)]
    FluentOverlay,
}

/// Defines the preferred system theme.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum Theme {
    Light,
    Dark,
}

impl Serialize for Theme {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

impl<'de> Deserialize<'de> for Theme {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.to_lowercase().as_str() {
            "dark" => Self::Dark,
            _ => Self::Light,
        })
    }
}

impl Display for Theme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Light => "light",
                Self::Dark => "dark",
            }
        )
    }
}

/// Defines the macOS title-bar appearance.
#[derive(Debug, Clone, PartialEq, Eq, Copy, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum TitleBarStyle {
    #[default]
    Visible,
    Transparent,
    Overlay,
}

impl Serialize for TitleBarStyle {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

impl<'de> Deserialize<'de> for TitleBarStyle {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.to_lowercase().as_str() {
            "transparent" => Self::Transparent,
            "overlay" => Self::Overlay,
            _ => Self::Visible,
        })
    }
}

impl Display for TitleBarStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Visible => "Visible",
                Self::Transparent => "Transparent",
                Self::Overlay => "Overlay",
            }
        )
    }
}

/// Defines the urgency of a window attention request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(tag = "type")]
pub enum UserAttentionType {
    Critical,
    Informational,
}

/// Defines the content source loaded by a webview.
#[derive(PartialEq, Eq, Debug, Clone, Serialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum WebviewUrl {
    External(Url),
    App(PathBuf),
    CustomProtocol(Url),
}

impl<'de> Deserialize<'de> for WebviewUrl {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum WebviewUrlDeserializer {
            Url(Url),
            Path(PathBuf),
        }

        match WebviewUrlDeserializer::deserialize(deserializer)? {
            WebviewUrlDeserializer::Url(u) => {
                if u.scheme() == "https" || u.scheme() == "http" {
                    Ok(Self::External(u))
                } else {
                    Ok(Self::CustomProtocol(u))
                }
            }
            WebviewUrlDeserializer::Path(p) => Ok(Self::App(p)),
        }
    }
}

impl fmt::Display for WebviewUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::External(url) | Self::CustomProtocol(url) => {
                write!(f, "{url}")
            }
            Self::App(path) => write!(f, "{}", path.display()),
        }
    }
}

impl Default for WebviewUrl {
    fn default() -> Self {
        Self::External(Url::parse("https://tauri.app").unwrap())
    }
}

/// Defines native window effects.
#[allow(deprecated)]
mod window_effects {
    use super::*;

    #[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub enum WindowEffect {
        #[deprecated(
            since = "0.1.0",
            note = "You should instead choose an appropriate semantic material."
        )]
        AppearanceBased,
        #[deprecated(since = "0.1.0", note = "Use a semantic material instead.")]
        Light,
        #[deprecated(since = "0.1.0", note = "Use a semantic material instead.")]
        Dark,
        #[deprecated(since = "0.1.0", note = "Use a semantic material instead.")]
        MediumLight,
        #[deprecated(since = "0.1.0", note = "Use a semantic material instead.")]
        UltraDark,
        Titlebar,
        Selection,
        Menu,
        Popover,
        Sidebar,
        HeaderView,
        Sheet,
        WindowBackground,
        HudWindow,
        FullScreenUI,
        Tooltip,
        ContentBackground,
        UnderWindowBackground,
        UnderPageBackground,
        Mica,
        MicaDark,
        MicaLight,
        Tabbed,
        TabbedDark,
        TabbedLight,
        Blur,
        Acrylic,
    }

    #[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub enum WindowEffectState {
        FollowsWindowActiveState,
        Active,
        Inactive,
    }
}

pub use window_effects::{WindowEffect, WindowEffectState};

/// Configures native window effects.
#[skip_serializing_none]
#[derive(Debug, PartialEq, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WindowEffectsConfig {
    pub effects: Vec<WindowEffect>,
    pub state: Option<WindowEffectState>,
    pub radius: Option<f64>,
    pub color: Option<Color>,
}

/// Defines minimum and maximum window dimensions.
#[derive(Clone, Copy, PartialEq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowSizeConstraints {
    pub min_width: Option<PixelUnit>,
    pub min_height: Option<PixelUnit>,
    pub max_width: Option<PixelUnit>,
    pub max_height: Option<PixelUnit>,
}

fn default_alpha() -> u8 {
    255
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
enum InnerColor {
    String(String),
    Rgb((u8, u8, u8)),
    Rgba((u8, u8, u8, u8)),
    RgbaObject {
        red: u8,
        green: u8,
        blue: u8,
        #[serde(default = "default_alpha")]
        alpha: u8,
    },
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let color = InnerColor::deserialize(deserializer)?;
        let color = match color {
            InnerColor::String(string) => string.parse().map_err(serde::de::Error::custom)?,
            InnerColor::Rgb(rgb) => Color(rgb.0, rgb.1, rgb.2, 255),
            InnerColor::Rgba(rgb) => rgb.into(),
            InnerColor::RgbaObject {
                red,
                green,
                blue,
                alpha,
            } => Color(red, green, blue, alpha),
        };

        Ok(color)
    }
}

pub(crate) fn to_tao_theme(theme: Option<Theme>) -> Option<TaoTheme> {
    match theme {
        Some(Theme::Light) => Some(TaoTheme::Light),
        Some(Theme::Dark) => Some(TaoTheme::Dark),
        _ => None,
    }
}
