//! Avatar image loading
//!
//! Loads an image from disk and prepares it for rendering.
//! Auto-detects the best terminal graphics protocol (kitty, sixel, etc.)
//! and falls back to halfblocks on bare TTYs.
//! Returns `None` on any error — avatar is optional, never blocks login.

use ratatui_image::{picker::Picker, protocol::StatefulProtocol};
use std::fs;
use std::io::{BufRead, Seek};

/// Maximum source dimension accepted by the decoder.
const MAX_SOURCE_DIM: u32 = 4096;
/// Maximum compressed source file size accepted by the loader.
const MAX_SOURCE_BYTES: u64 = 64 * 1024 * 1024;
/// Maximum memory the decoder may allocate at once.
const MAX_DECODE_BYTES: u64 = 64 * 1024 * 1024;
/// Maximum dimension retained for terminal rendering.
const MAX_RENDER_DIM: u32 = 512;

/// Loaded avatar render protocol.
pub struct Avatar {
    pub protocol: StatefulProtocol,
}

/// Load an avatar image and return a render-ready protocol state.
///
/// Queries the terminal for graphics support (kitty, sixel, iterm2).
/// Falls back to halfblocks which works on bare TTYs.
pub fn load(path: &str) -> Option<Avatar> {
    let metadata = fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_SOURCE_BYTES {
        return None;
    }

    let reader = image::ImageReader::open(path).ok()?;
    let dyn_img = decode_with_limits(reader)?;

    // Keep protocol state small after the decoder has enforced source limits.
    let dyn_img = dyn_img.thumbnail(MAX_RENDER_DIM, MAX_RENDER_DIM);

    // Try auto-detecting the best protocol; fall back to halfblocks
    let picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());

    Some(Avatar {
        protocol: picker.new_resize_protocol(dyn_img),
    })
}

fn decode_with_limits<R>(mut reader: image::ImageReader<R>) -> Option<image::DynamicImage>
where
    R: BufRead + Seek,
{
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_SOURCE_DIM);
    limits.max_image_height = Some(MAX_SOURCE_DIM);
    limits.max_alloc = Some(MAX_DECODE_BYTES);
    reader.limits(limits);
    reader.decode().ok()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "tests can unwrap")]
mod tests {
    use super::*;
    use image::{codecs::png::PngEncoder, ColorType, ImageEncoder, ImageFormat};
    use std::io::Cursor;

    #[test]
    fn missing_avatar_is_ignored() {
        assert!(load("/file/that/does/not/exist.png").is_none());
    }

    #[test]
    fn non_regular_avatar_is_ignored_before_opening() {
        assert!(load("/dev/null").is_none());
    }

    #[test]
    fn oversized_avatar_is_rejected_before_decode() {
        let width = MAX_SOURCE_DIM + 1;
        let pixels = vec![0; width as usize];
        let mut encoded = Vec::new();
        PngEncoder::new(&mut encoded)
            .write_image(&pixels, width, 1, ColorType::L8.into())
            .unwrap();
        let reader = image::ImageReader::with_format(Cursor::new(encoded), ImageFormat::Png);

        assert!(decode_with_limits(reader).is_none());
    }

    #[test]
    fn small_avatar_decodes_with_limits() {
        let pixels = [0, 127, 255, 63];
        let mut encoded = Vec::new();
        PngEncoder::new(&mut encoded)
            .write_image(&pixels, 2, 2, ColorType::L8.into())
            .unwrap();
        let reader = image::ImageReader::with_format(Cursor::new(encoded), ImageFormat::Png);

        let decoded = decode_with_limits(reader).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (2, 2));
    }
}
