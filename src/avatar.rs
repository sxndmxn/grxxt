//! Avatar image loading
//!
//! Loads an image from disk and prepares it for rendering.
//! Auto-detects the best terminal graphics protocol (kitty, sixel, etc.)
//! and falls back to halfblocks on bare TTYs.
//! Returns `None` on any error — avatar is optional, never blocks login.

use ratatui_image::{
    picker::{Picker, ProtocolType},
    protocol::StatefulProtocol,
};

/// Maximum dimension (width or height) for the decoded image.
/// Caps memory usage for large source files.
const MAX_DIM: u32 = 512;

/// Loaded avatar: render protocol + aspect ratio for centering
pub struct Avatar {
    pub protocol: StatefulProtocol,
    /// Width / height ratio of the source image
    pub aspect_ratio: f32,
}

/// Load an avatar image and return a render-ready protocol state.
///
/// Queries the terminal for graphics support (kitty, sixel, iterm2).
/// Falls back to halfblocks which works on bare TTYs.
#[allow(
    clippy::cast_precision_loss,
    reason = "image dimensions are small u32 values, f32 is fine"
)]
pub fn load(path: &str) -> Option<Avatar> {
    let dyn_img = image::ImageReader::open(path).ok()?.decode().ok()?;
    let aspect_ratio = dyn_img.width() as f32 / dyn_img.height() as f32;

    // Pre-resize to cap memory; thumbnail preserves aspect ratio
    let dyn_img = dyn_img.thumbnail(MAX_DIM, MAX_DIM);

    // Try auto-detecting the best protocol; fall back to halfblocks
    let picker = Picker::from_query_stdio().unwrap_or_else(|_| {
        let mut p = Picker::from_fontsize((4, 8));
        p.set_protocol_type(ProtocolType::Halfblocks);
        p
    });

    Some(Avatar {
        protocol: picker.new_resize_protocol(dyn_img),
        aspect_ratio,
    })
}
