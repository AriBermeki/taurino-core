// Portions of this file are derived from Tauri:
// https://github.com/tauri-apps/tauri
//
// Copyright 2019-2025 The Tauri Programme
// within The Commons Conservancy
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

pub use dpi::*;
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{borrow::Cow, fmt::Display, str::FromStr};

/// A rectangular region.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Rect {
    /// Rect position.
    pub position: dpi::Position,
    /// Rect size.
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

/// A rectangular region in physical pixels.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct PhysicalRect<P: dpi::Pixel, S: dpi::Pixel> {
    /// Rect position.
    pub position: dpi::PhysicalPosition<P>,
    /// Rect size.
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

/// A rectangular region in logical pixels.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct LogicalRect<P: dpi::Pixel, S: dpi::Pixel> {
    /// Rect position.
    pub position: dpi::LogicalPosition<P>,
    /// Rect size.
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

/// Window size constraints
#[derive(Clone, Copy, PartialEq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowSizeConstraints {
    /// The minimum width a window can be, If this is `None`, the window will have no minimum width.
    ///
    /// The default is `None`.
    pub min_width: Option<PixelUnit>,
    /// The minimum height a window can be, If this is `None`, the window will have no minimum height.
    ///
    /// The default is `None`.
    pub min_height: Option<PixelUnit>,
    /// The maximum width a window can be, If this is `None`, the window will have no maximum width.
    ///
    /// The default is `None`.
    pub max_width: Option<PixelUnit>,
    /// The maximum height a window can be, If this is `None`, the window will have no maximum height.
    ///
    /// The default is `None`.
    pub max_height: Option<PixelUnit>,
}

/// Monitor descriptor.
#[derive(Debug, Clone)]
pub struct Monitor {
    /// A human-readable name of the monitor.
    /// `None` if the monitor doesn't exist anymore.
    pub name: Option<String>,
    /// The monitor's resolution.
    pub size: PhysicalSize<u32>,
    /// The top-left corner position of the monitor relative to the larger full screen area.
    pub position: PhysicalPosition<i32>,
    /// The monitor's work_area.
    pub work_area: PhysicalRect<i32, u32>,
    /// Returns the scale factor that can be used to map logical pixels to physical pixels, and vice versa.
    pub scale_factor: f64,
}

/// A tuple struct of RGBA colors. Each value has minimum of 0 and maximum of 255.
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

fn default_alpha() -> u8 {
    255
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
enum InnerColor {
    /// Color hex string, for example: #fff, #ffffff, or #ffffffff.
    String(String),
    /// Array of RGB colors. Each value has minimum of 0 and maximum of 255.
    Rgb((u8, u8, u8)),
    /// Array of RGBA colors. Each value has minimum of 0 and maximum of 255.
    Rgba((u8, u8, u8, u8)),
    /// Object of red, green, blue, alpha color values. Each value has minimum of 0 and maximum of 255.
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

#[cfg(feature = "schema")]
impl schemars::JsonSchema for Color {
    fn schema_name() -> String {
        "Color".to_string()
    }

    fn json_schema(r#gen: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
        let mut schema = schemars::schema_for!(InnerColor).schema;
        schema.metadata = None; // Remove `title: InnerColor` from schema

        // add hex color pattern validation
        let any_of = schema.subschemas().any_of.as_mut().unwrap();
        let schemars::schema::Schema::Object(str_schema) = any_of.first_mut().unwrap() else {
            unreachable!()
        };
        str_schema.string().pattern =
            Some("^#?([A-Fa-f0-9]{3}|[A-Fa-f0-9]{6}|[A-Fa-f0-9]{8})$".into());

        schema.into()
    }
}

/// System theme.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum Theme {
    /// Light theme.
    Light,
    /// Dark theme.
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

/// Window icon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Icon<'a> {
    /// RGBA bytes of the icon.
    pub rgba: Cow<'a, [u8]>,
    /// Icon width.
    pub width: u32,
    /// Icon height.
    pub height: u32,
}


/// How the window title bar should be displayed on macOS.
#[derive(Debug, Clone, PartialEq, Eq, Copy, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum TitleBarStyle {
  /// A normal title bar.
  #[default]
  Visible,
  /// Makes the title bar transparent, so the window background color is shown instead.
  ///
  /// Useful if you don't need to have actual HTML under the title bar. This lets you avoid the caveats of using `TitleBarStyle::Overlay`. Will be more useful when Tauri lets you set a custom window background color.
  Transparent,
  /// Shows the title bar as a transparent overlay over the window's content.
  ///
  /// Keep in mind:
  /// - The height of the title bar is different on different OS versions, which can lead to window the controls and title not being where you don't expect.
  /// - You need to define a custom drag region to make your window draggable, however due to a limitation you can't drag the window when it's not in focus <https://github.com/tauri-apps/tauri/issues/4316>.
  /// - The color of the window title depends on the system theme.
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

/*

Jetzt ist der Unterschied klar: Tauri erhält bereits die WebViews des betreffenden Fensters als geordneten Slice, während du zuerst per WindowId suchst und anschließend eine beliebige WebView aus einer HashMap nimmst.

So kannst du die Frage an das Tauri-Team stellen:
*/

/*


### Does `inner_size` on macOS depend on the first WebView being the primary WebView?

I am implementing behavior similar to Tauri’s macOS-specific `inner_size` function.

Tauri currently receives the WebViews associated with the window as a slice and uses the first WebView:

```rust
#[cfg(target_os = "macos")]
fn inner_size(
  window: &Window,
  webviews: &[Webview],
  has_children: bool,
) -> TaoPhysicalSize<u32> {
  if !has_children && !webviews.is_empty() {
    use wry::WebViewExtMacOS;

    let webview = webviews.first().unwrap();

    let view = unsafe {
      Retained::cast_unchecked::<objc2_app_kit::NSView>(
        webview.webview()
      )
    };

    let view_frame = view.frame();

    let logical: TaoLogicalSize<f64> =
      (view_frame.size.width, view_frame.size.height).into();

    return logical.to_physical(window.scale_factor());
  }

  window.inner_size()
}
```

In my implementation, WebViews are stored by window ID and WebView label:

```rust
type WindowWebviews =
    HashMap<WindowId, HashMap<String, WebView>>;
```

My equivalent implementation currently uses the first value returned by the inner `HashMap`:

```rust
#[cfg(target_os = "macos")]
fn inner_size(
    window_id: WindowId,
    window: &Window,
    webviews: &WindowWebviews,
    has_children: bool,
) -> TaoPhysicalSize<u32> {
    if !has_children {
        let webview = webviews
            .get(&window_id)
            .and_then(|window_webviews| {
                window_webviews.values().next()
            });

        if let Some(webview) = webview {
            use wry::WebViewExtMacOS;

            let view = unsafe {
                Retained::cast_unchecked::<objc2_app_kit::NSView>(
                    webview.webview(),
                )
            };

            let frame = view.frame();

            let logical: TaoLogicalSize<f64> =
                (frame.size.width, frame.size.height).into();

            return logical.to_physical(window.scale_factor());
        }
    }

    window.inner_size()
}
```

However, unlike a slice or `Vec`, a `HashMap` does not guarantee iteration order. Therefore, `values().next()` may return any WebView associated with the window.

Does Tauri intentionally rely on `webviews.first()` being the primary or root WebView?

More specifically:

1. Is the first WebView guaranteed to be the WebView whose native `NSView` frame represents the window’s inner size?
2. Would selecting an arbitrary WebView from a `HashMap` be safe when `has_children` is `false`?
3. Are all WebViews expected to have the same native frame in this case?
4. Should I explicitly store the primary WebView label or ID instead of using `HashMap::values().next()`?
5. Does the ordering of Tauri’s `webviews` slice have semantic meaning, or is `first()` only used because any WebView would produce the same size?

I am trying to determine whether I need an ordered collection such as `Vec<WebView>`, or whether a `HashMap` is sufficient as long as I explicitly identify the primary WebView.


*/

/*

Technisch würde ich in deiner Implementierung nicht auf values().next() vertrauen. Du musst die HashMap nicht durch einen Vec ersetzen, aber die primäre WebView sollte ausdrücklich gespeichert werden, beispielsweise über primary_webview_label: String.

*/
