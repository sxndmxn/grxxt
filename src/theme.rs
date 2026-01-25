//! Zodiac brutalist theme for iced
//!
//! TODO: Add rem-like scalable sizing system
//!   - Define BASE_SIZE constant (e.g., 16.0)
//!   - Create size helper: `fn rem(multiplier: f32) -> f32 { BASE_SIZE * multiplier }`
//!   - Replace hardcoded pixel values with rem(1.0), rem(1.25), etc.

use iced::widget::{button, container, text, text_input};
use iced::{Border, Color, Theme};

// Zodiac color palette
pub const BACKGROUND: Color = Color::from_rgb(
    0x0b as f32 / 255.0,
    0x0a as f32 / 255.0,
    0x13 as f32 / 255.0,
);

pub const FOREGROUND: Color = Color::from_rgb(
    0xf6 as f32 / 255.0,
    0xf1 as f32 / 255.0,
    0xe3 as f32 / 255.0,
);

pub const ACCENT: Color = Color::from_rgb(
    0xf1 as f32 / 255.0,
    0xc3 as f32 / 255.0,
    0x5f as f32 / 255.0,
);

pub const ERROR: Color = Color::from_rgb(
    0xd1 as f32 / 255.0,
    0x4b as f32 / 255.0,
    0x64 as f32 / 255.0,
);

pub const TRANSPARENT: Color = Color::TRANSPARENT;

// Text input styling
pub fn text_input_style(_theme: &Theme, status: text_input::Status) -> text_input::Style {
    let base = text_input::Style {
        background: TRANSPARENT.into(),
        border: Border {
            color: FOREGROUND,
            width: 0.0,
            radius: 0.0.into(),
        },
        icon: FOREGROUND,
        placeholder: Color {
            a: 0.5,
            ..FOREGROUND
        },
        value: FOREGROUND,
        selection: ACCENT,
    };

    match status {
        text_input::Status::Active => text_input::Style {
            border: Border {
                color: FOREGROUND,
                width: 2.0,
                radius: 0.0.into(),
            },
            ..base
        },
        text_input::Status::Hovered => text_input::Style {
            border: Border {
                color: ACCENT,
                width: 2.0,
                radius: 0.0.into(),
            },
            ..base
        },
        text_input::Status::Focused => text_input::Style {
            border: Border {
                color: ACCENT,
                width: 2.0,
                radius: 0.0.into(),
            },
            ..base
        },
        text_input::Status::Disabled => text_input::Style {
            value: Color {
                a: 0.3,
                ..FOREGROUND
            },
            ..base
        },
    }
}

// Password input styling (same as text input but for secret fields)
pub fn password_input_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    text_input_style(theme, status)
}

// Container styling for the main background
pub fn background_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(BACKGROUND.into()),
        text_color: Some(FOREGROUND),
        border: Border::default(),
        ..Default::default()
    }
}

// Error text styling
pub fn error_text_style(_theme: &Theme) -> text::Style {
    text::Style { color: Some(ERROR) }
}

// Normal text styling
pub fn normal_text_style(_theme: &Theme) -> text::Style {
    text::Style {
        color: Some(FOREGROUND),
    }
}

// Power button styling (transparent background, accent on hover)
pub fn power_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: Some(TRANSPARENT.into()),
        text_color: match status {
            button::Status::Hovered | button::Status::Pressed => ACCENT,
            _ => FOREGROUND,
        },
        border: Border::default(),
        ..Default::default()
    }
}

// Clock text styling
pub fn clock_text_style(_theme: &Theme) -> text::Style {
    text::Style {
        color: Some(FOREGROUND),
    }
}

// Avatar container styling (subtle border)
pub fn avatar_container_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(TRANSPARENT.into()),
        text_color: Some(FOREGROUND),
        border: Border {
            color: FOREGROUND,
            width: 2.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}
