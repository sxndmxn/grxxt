//! Main iced Application for the greeter

use crate::greetd::{authenticate, AuthError};
use crate::power;
use crate::theme;
use chrono::Local;
use iced::widget::image::Handle;
use iced::widget::{
    button, center, column, container, horizontal_space, image, row, text, text_input,
    vertical_space,
};
use iced::{Alignment, Element, Font, Length, Subscription, Task};
use std::time::Duration;

const DEFAULT_SESSION: &str = "Hyprland";

#[derive(Debug, Clone)]
pub enum Message {
    UsernameChanged(String),
    PasswordChanged(String),
    Submit,
    AuthResult(Result<(), String>),
    Tick,
    Shutdown,
    Reboot,
    Suspend,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputFocus {
    Username,
    Password,
}

pub struct Greeter {
    username: String,
    password: String,
    error: Option<String>,
    authenticating: bool,
    focus: InputFocus,
    avatar: Option<Handle>,
}

impl Default for Greeter {
    fn default() -> Self {
        // Try to load avatar from ~/.grxxt/avatar.png
        let avatar = std::env::var("HOME")
            .ok()
            .and_then(|home| std::fs::read(format!("{}/.grxxt/avatar.png", home)).ok())
            .map(Handle::from_bytes);

        Self {
            username: String::new(),
            password: String::new(),
            error: None,
            authenticating: false,
            focus: InputFocus::Username,
            avatar,
        }
    }
}

impl Greeter {
    pub fn new() -> (Self, Task<Message>) {
        (
            Self::default(),
            text_input::focus(text_input::Id::new("username")),
        )
    }

    pub fn title(&self) -> String {
        String::from("grxxt")
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::UsernameChanged(value) => {
                self.username = value;
                self.error = None;
                Task::none()
            }
            Message::PasswordChanged(value) => {
                self.password = value;
                self.error = None;
                Task::none()
            }
            Message::Submit => {
                if self.authenticating {
                    return Task::none();
                }

                if self.focus == InputFocus::Username {
                    // Move to password field
                    self.focus = InputFocus::Password;
                    return text_input::focus(text_input::Id::new("password"));
                }

                if self.username.is_empty() {
                    self.error = Some("Username required".to_string());
                    self.focus = InputFocus::Username;
                    return text_input::focus(text_input::Id::new("username"));
                }

                if self.password.is_empty() {
                    self.error = Some("Password required".to_string());
                    return Task::none();
                }

                self.authenticating = true;
                self.error = None;

                let username = self.username.clone();
                let password = self.password.clone();

                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            authenticate(&username, &password, DEFAULT_SESSION)
                        })
                        .await
                        .map_err(|e| e.to_string())?
                        .map_err(|e: AuthError| e.to_string())
                    },
                    Message::AuthResult,
                )
            }
            Message::AuthResult(result) => {
                self.authenticating = false;
                match result {
                    Ok(()) => {
                        // Authentication successful, greetd will start the session
                        // We should exit cleanly
                        std::process::exit(0);
                    }
                    Err(msg) => {
                        self.error = Some(msg);
                        self.password.clear();
                        self.focus = InputFocus::Password;
                        text_input::focus(text_input::Id::new("password"))
                    }
                }
            }
            Message::Tick => Task::none(),
            Message::Shutdown => {
                power::shutdown();
                Task::none()
            }
            Message::Reboot => {
                power::reboot();
                Task::none()
            }
            Message::Suspend => {
                power::suspend();
                Task::none()
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_secs(1)).map(|_| Message::Tick)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let time = Local::now();

        // Top-left: clock and date
        let clock_col = column![
            text(time.format("%H:%M").to_string())
                .size(48)
                .font(Font::MONOSPACE)
                .style(theme::clock_text_style),
            text(time.format("%a %d %b").to_string().to_uppercase())
                .size(16)
                .style(theme::clock_text_style),
        ]
        .align_x(Alignment::Start);

        // Top-right: power buttons (Nerd Font icons)
        let power_row = row![
            button(text("⏻").size(20))
                .style(theme::power_button_style)
                .on_press(Message::Shutdown),
            button(text("󰜉").size(20))
                .style(theme::power_button_style)
                .on_press(Message::Reboot),
            button(text("󰤄").size(20))
                .style(theme::power_button_style)
                .on_press(Message::Suspend),
        ]
        .spacing(16);

        // Header row
        let header = row![clock_col, horizontal_space(), power_row,].padding(32);

        // Avatar widget
        let avatar_widget: Element<Message> = if let Some(ref handle) = self.avatar {
            container(image(handle.clone()).width(100).height(100))
                .style(theme::avatar_container_style)
                .into()
        } else {
            // Placeholder with Nerd Font user icon
            container(text("").size(64).style(theme::normal_text_style))
                .width(100)
                .height(100)
                .center_x(100)
                .center_y(100)
                .style(theme::avatar_container_style)
                .into()
        };

        // Input fields
        let username_input = text_input("username", &self.username)
            .id(text_input::Id::new("username"))
            .on_input(Message::UsernameChanged)
            .on_submit(Message::Submit)
            .padding(12)
            .size(20)
            .width(280)
            .style(theme::text_input_style);

        let password_input = text_input("password", &self.password)
            .id(text_input::Id::new("password"))
            .on_input(Message::PasswordChanged)
            .on_submit(Message::Submit)
            .secure(true)
            .padding(12)
            .size(20)
            .width(280)
            .style(theme::password_input_style);

        let error_text: Element<Message> = if let Some(ref err) = self.error {
            text(err.to_uppercase())
                .size(16)
                .style(theme::error_text_style)
                .into()
        } else {
            text("").size(16).into()
        };

        let status_text: Element<Message> = if self.authenticating {
            text("authenticating...")
                .size(14)
                .style(theme::normal_text_style)
                .into()
        } else {
            text("").size(14).into()
        };

        // Center: avatar + form
        let form = column![
            avatar_widget,
            username_input,
            password_input,
            error_text,
            status_text,
        ]
        .spacing(16)
        .align_x(Alignment::Center);

        // Full layout
        let content = column![header, vertical_space(), center(form), vertical_space(),]
            .width(Length::Fill)
            .height(Length::Fill);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::background_style)
            .into()
    }
}
