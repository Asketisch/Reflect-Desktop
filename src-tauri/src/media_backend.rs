//! Real Media Studio + Computer Use backends (Phase 3 item 13 complete impl).
//!
//! 用 `image` crate (`ImageBackend`) + `xcap`/`enigo` (`ComputerBackend`)
//! 替换默认的 metadata-only / unavailable 实现。
//!
//! ## 平台
//!
//! - **ImageBackend** —— 跨平台(`image` crate)。
//! - **ComputerBackend** —— macOS / Windows / Linux 都由 xcap + enigo 处理;
//!   macOS 首次调用 `Enigo::new()` 时会触发权限弹窗(若需要)。
//!
//! ## 失败模式
//!
//! - 截屏失败(权限拒绝 / 无显示) → `MediaError::Unavailable`。
//! - `Enigo::new()` 没权限 → `MediaError::Unavailable { capability: "input_permission" }`。
//! - 鼠标键盘动作实际错误 → `MediaError::Other`。

use std::io::Cursor;
use std::path::Path;

use enigo::{
    Axis::{Horizontal, Vertical},
    Button, Coordinate::Abs,
    Direction::{self, Click},
    Enigo, Key, Keyboard, Mouse, Settings,
};
use image::{DynamicImage, ImageFormat as ImgFmt};
use reflect_app_core::media::{
    BackendCapability, ComputerBackend, ComputerUseAction, ImageBackend, ImageFormat,
    ImageProcessResult, ImageProcessSpec, MediaAsset, MediaError,
};


/// `image` crate-backed image processor。
///
/// Pure-local,不依赖 native binding。读取文件 → 解码为
/// [`image::DynamicImage`] → 缩放 → 写回为指定格式。
pub struct RealImageBackend;

impl RealImageBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RealImageBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageBackend for RealImageBackend {
    fn capability(&self) -> BackendCapability {
        BackendCapability::Full
    }

    fn load_metadata(&self, path: &Path) -> Result<MediaAsset, MediaError> {
        let meta = std::fs::metadata(path).map_err(|e| MediaError::IoError {
            message: e.to_string(),
        })?;
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| MediaError::Other {
                message: "invalid filename encoding".into(),
            })?
            .to_string();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let format = ImageFormat::from_ext(ext);
        let mime = format.map(crate_format_to_mime);
        let modified_at_ms = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        // 尽力读维(失败不算 error,返回 None)。
        let (width, height) = image::image_dimensions(path).ok().map(|(w, h)| (Some(w), Some(h))).unwrap_or((None, None));
        Ok(MediaAsset {
            path: path.to_path_buf(),
            filename,
            size_bytes: meta.len(),
            mime_type: mime,
            width,
            height,
            modified_at_ms,
        })
    }

    fn process(&self, spec: &ImageProcessSpec) -> Result<ImageProcessResult, MediaError> {
        let input = Path::new(&spec.input_path);
        let output = Path::new(&spec.output_path);
        if !input.is_file() {
            return Err(MediaError::NotFound {
                message: format!("input not found: {}", input.display()),
            });
        }
        // 推断输出格式:spec.format 优先,否则用 output_path 的扩展名。
        let resolved_format = spec
            .format
            .or_else(|| {
                output
                    .extension()
                    .and_then(|e| e.to_str())
                    .and_then(ImageFormat::from_ext)
            })
            .ok_or_else(|| MediaError::UnsupportedFormat {
                message: "cannot infer output format from path or spec".into(),
            })?;

        let img = image::open(input).map_err(|e| MediaError::Other {
            message: format!("decode failed: {e}"),
        })?;

        // 缩放。
        let scaled: DynamicImage = match (spec.width, spec.height) {
            (Some(w), Some(h)) => img.resize_exact(w, h, image::imageops::FilterType::Lanczos3),
            (Some(w), None) => img.resize(w, img.height() * w / img.width().max(1), image::imageops::FilterType::Lanczos3),
            (None, Some(h)) => img.resize(img.width() * h / img.height().max(1), h, image::imageops::FilterType::Lanczos3),
            (None, None) => img,
        };

        if let Some(parent) = output.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let mut buf = std::fs::File::create(output).map_err(|e| MediaError::IoError {
            message: e.to_string(),
        })?;
        let img_fmt = match resolved_format {
            ImageFormat::Png => ImgFmt::Png,
            ImageFormat::Jpeg => ImgFmt::Jpeg,
            ImageFormat::Gif => ImgFmt::Gif,
            ImageFormat::WebP => ImgFmt::WebP,
            ImageFormat::Bmp => ImgFmt::Bmp,
        };
        scaled
            .to_rgb8()
            .write_to(&mut buf, img_fmt)
            .map_err(|e| MediaError::Other {
                message: format!("encode failed: {e}"),
            })?;
        let bytes = std::fs::metadata(output).map(|m| m.len()).unwrap_or(0);
        Ok(ImageProcessResult {
            output_path: spec.output_path.clone(),
            width: scaled.width(),
            height: scaled.height(),
            format: resolved_format,
            bytes,
        })
    }
}

fn crate_format_to_mime(f: ImageFormat) -> String {
    match f {
        ImageFormat::Png => "image/png".into(),
        ImageFormat::Jpeg => "image/jpeg".into(),
        ImageFormat::Gif => "image/gif".into(),
        ImageFormat::WebP => "image/webp".into(),
        ImageFormat::Bmp => "image/bmp".into(),
    }
}

/// `xcap` (截图) + `enigo` (鼠标键盘输入) 组合的真实 backend。
///
/// 每次操作都新建 `Enigo`(无状态 action),不持有跨调用状态。
pub struct RealComputerBackend;

impl RealComputerBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RealComputerBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl ComputerBackend for RealComputerBackend {
    fn capability(&self) -> BackendCapability {
        BackendCapability::Full
    }

    fn screenshot(&self) -> Result<Vec<u8>, MediaError> {
        let monitors = xcap::Monitor::all().map_err(|e| MediaError::Unavailable {
            capability: "screenshot".into(),
            reason: format!("xcap enumerate failed: {e}"),
        })?;
        let monitor = monitors.first().ok_or_else(|| MediaError::Unavailable {
            capability: "screenshot".into(),
            reason: "no monitors detected".into(),
        })?;
        let rgba = monitor.capture_image().map_err(|e| MediaError::Other {
            message: format!("capture_image failed: {e}"),
        })?;
        // Encode RGBA → PNG bytes(前端 base64 后就能当 data: URL 用)。
        let mut buf: Vec<u8> = Vec::new();
        {
            let mut cursor = Cursor::new(&mut buf);
            rgba.write_to(&mut cursor, ImgFmt::Png).map_err(|e| MediaError::Other {
                message: format!("png encode failed: {e}"),
            })?;
        }
        Ok(buf)
    }

    fn execute(&self, action: &ComputerUseAction) -> Result<(), MediaError> {
        match action {
            ComputerUseAction::Screenshot => {
                // `screenshot` 走 `screenshot()` 独立命令;这里报告重复请求。
                Err(MediaError::Other {
                    message: "use reflect_screenshot for captures".into(),
                })
            }
            ComputerUseAction::MouseMove { x, y } => {
                let mut enigo = make_enigo()?;
                enigo.move_mouse(*x, *y, Abs).map_err(input_err)
            }
            ComputerUseAction::MouseClick { x, y, button } => {
                let mut enigo = make_enigo()?;
                enigo.move_mouse(*x, *y, Abs).map_err(input_err)?;
                let btn = parse_button(button);
                enigo.button(btn, Click).map_err(input_err)
            }
            ComputerUseAction::KeyType { text } => {
                let mut enigo = make_enigo()?;
                enigo.text(text).map_err(input_err)
            }
            ComputerUseAction::KeyCombo { keys } => {
                let mut enigo = make_enigo()?;
                press_combo(&mut enigo, keys)
            }
            ComputerUseAction::Scroll { dx, dy } => {
                let mut enigo = make_enigo()?;
                let mut last_err: Option<MediaError> = None;
                if *dy != 0 {
                    if let Err(e) = enigo.scroll(*dy, Vertical) {
                        last_err = Some(input_err(e));
                    }
                }
                if *dx != 0 {
                    if let Err(e) = enigo.scroll(*dx, Horizontal) {
                        last_err = Some(input_err(e));
                    }
                }
                match last_err {
                    None => Ok(()),
                    Some(e) => Err(e),
                }
            }
        }
    }
}

fn make_enigo() -> Result<Enigo, MediaError> {
    let settings = Settings::default();
    Enigo::new(&settings).map_err(|e| MediaError::Unavailable {
        capability: "input_permission".into(),
        reason: format!("enigo::Enigo::new failed: {e:?}"),
    })
}

fn input_err<E: std::fmt::Display>(e: E) -> MediaError {
    MediaError::Other {
        message: format!("input error: {e}"),
    }
}

fn parse_button(s: &str) -> Button {
    match s.to_lowercase().as_str() {
        "right" | "r" => Button::Right,
        "middle" | "m" => Button::Middle,
        _ => Button::Left,
    }
}

/// 把 `"ctrl+c"` / `"cmd+shift+p"` 字符串解析成组合键并依次按下 / 释放。
fn press_combo(enigo: &mut Enigo, keys: &str) -> Result<(), MediaError> {
    let parts: Vec<&str> = keys.split('+').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return Err(MediaError::Other {
            message: format!("empty combo: {keys:?}"),
        });
    }
    // 按下所有修饰键,最后一个是主键,然后 click 主键,释放所有修饰键(逆序)。
    let parsed: Vec<Key> = parts.iter().map(|p| parse_key(p)).collect();
    for k in &parsed[..parsed.len().saturating_sub(1)] {
        enigo.key(*k, Direction::Press).map_err(input_err)?;
    }
    if let Some(last) = parsed.last().copied() {
        enigo.key(last, Click).map_err(input_err)?;
    }
    for k in parsed[..parsed.len().saturating_sub(1)].iter().rev() {
        enigo.key(*k, Direction::Release).map_err(input_err)?;
    }
    Ok(())
}

fn parse_key(name: &str) -> Key {
    let n = name.to_lowercase();
    match n.as_str() {
        "ctrl" | "control" => Key::Control,
        "shift" => Key::Shift,
        "alt" | "option" => Key::Alt,
        "cmd" | "command" | "meta" | "super" => Key::Meta,
        "enter" | "return" => Key::Return,
        "tab" => Key::Tab,
        "esc" | "escape" => Key::Escape,
        "backspace" => Key::Backspace,
        "space" => Key::Space,
        "left" => Key::LeftArrow,
        "right" => Key::RightArrow,
        "up" => Key::UpArrow,
        "down" => Key::DownArrow,
        other => Key::Unicode(other.chars().next().unwrap_or('?')),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_button_known() {
        assert!(matches!(parse_button("left"), Button::Left));
        assert!(matches!(parse_button("RIGHT"), Button::Right));
        assert!(matches!(parse_button("middle"), Button::Middle));
    }

    #[test]
    fn parse_key_modifiers() {
        assert!(matches!(parse_key("ctrl"), Key::Control));
        assert!(matches!(parse_key("Shift"), Key::Shift));
        assert!(matches!(parse_key("cmd"), Key::Meta));
    }

    #[test]
    fn parse_key_arrows() {
        assert!(matches!(parse_key("left"), Key::LeftArrow));
        assert!(matches!(parse_key("RIGHT"), Key::RightArrow));
    }

    #[test]
    fn real_image_backend_load_metadata_includes_dimensions() {
        // Create a 4x4 PNG in-memory and write to tempdir.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.png");
        let img = image::RgbImage::from_fn(4, 4, |_, _| image::Rgb([255, 0, 0]));
        img.save(&path).unwrap();
        let backend = RealImageBackend::new();
        let asset = backend.load_metadata(&path).unwrap();
        assert_eq!(asset.width, Some(4));
        assert_eq!(asset.height, Some(4));
        assert_eq!(asset.mime_type.as_deref(), Some("image/png"));
    }

    #[test]
    fn real_image_backend_process_resize_to_jpeg() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("in.png");
        let output = dir.path().join("out.jpg");
        let img = image::RgbImage::from_fn(8, 8, |_, _| image::Rgb([0, 255, 0]));
        img.save(&input).unwrap();
        let backend = RealImageBackend::new();
        let spec = ImageProcessSpec {
            input_path: input.to_string_lossy().into_owned(),
            output_path: output.to_string_lossy().into_owned(),
            format: Some(ImageFormat::Jpeg),
            width: Some(4),
            height: Some(2),
            quality: None,
        };
        let result = backend.process(&spec).unwrap();
        assert_eq!(result.width, 4);
        assert_eq!(result.height, 2);
        assert_eq!(result.format, ImageFormat::Jpeg);
        assert!(result.bytes > 0);
        // Verify it's a real JPEG: read magic bytes.
        let mut f = std::fs::File::open(&output).unwrap();
        let mut buf = [0u8; 2];
        use std::io::Read;
        f.read_exact(&mut buf).unwrap();
        assert_eq!(buf, [0xff, 0xd8]); // JPEG SOI marker.
    }

    #[test]
    fn real_image_backend_process_infers_format_from_output_path() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("in.png");
        let output = dir.path().join("out.png");
        std::fs::write(&input, b"").ok(); // 不在意内容
        // 用真 PNG content 而非空。
        let img = image::RgbImage::from_fn(2, 2, |_, _| image::Rgb([0, 0, 0]));
        img.save(&input).unwrap();
        let backend = RealImageBackend::new();
        let spec = ImageProcessSpec {
            input_path: input.to_string_lossy().into_owned(),
            output_path: output.to_string_lossy().into_owned(),
            format: None,
            width: None,
            height: None,
            quality: None,
        };
        let result = backend.process(&spec).unwrap();
        assert_eq!(result.format, ImageFormat::Png);
    }

    #[test]
    fn real_image_backend_load_metadata_ioerror() {
        let backend = RealImageBackend::new();
        let err = backend
            .load_metadata(Path::new("/nonexistent/path/xyz.png"))
            .unwrap_err();
        assert!(matches!(err, MediaError::IoError { .. }));
    }

    #[test]
    fn real_image_backend_returns_unavailable_capability_full() {
        let backend = RealImageBackend::new();
        assert_eq!(backend.capability(), BackendCapability::Full);
    }

    #[test]
    fn real_computer_backend_full_capability() {
        let backend = RealComputerBackend::new();
        assert_eq!(backend.capability(), BackendCapability::Full);
    }

    #[test]
    fn format_to_mime_roundtrip() {
        assert_eq!(crate_format_to_mime(ImageFormat::Png), "image/png");
        assert_eq!(crate_format_to_mime(ImageFormat::Jpeg), "image/jpeg");
    }
}
