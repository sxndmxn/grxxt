//! Greet - A brutalist greetd greeter

mod app;
mod greetd;
mod power;
mod theme;

use app::Greeter;
use iced::{Font, Settings, Size};

const HACK_FONT: &[u8] = include_bytes!("/usr/share/fonts/TTF/HackNerdFont-Regular.ttf");

fn main() -> iced::Result {
    iced::application(Greeter::title, Greeter::update, Greeter::view)
        .subscription(Greeter::subscription)
        .settings(Settings {
            default_font: Font::with_name("Hack Nerd Font"),
            antialiasing: true,
            ..Settings::default()
        })
        .window(iced::window::Settings {
            size: Size::new(1920.0, 1080.0),
            decorations: false,
            resizable: false,
            ..Default::default()
        })
        .font(HACK_FONT)
        .run_with(Greeter::new)
}
