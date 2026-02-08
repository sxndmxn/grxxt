//! Application state management for the TUI greeter

use crate::avatar::Avatar;
use crate::config::Config;
use crate::greetd;
use crate::theme::Theme;

/// Which input field is currently focused
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Username,
    Password,
}

/// Application state
pub struct App {
    pub username: String,
    pub password: String,
    pub error: Option<String>,
    pub authenticating: bool,
    pub focus: Focus,
    pub session_cmd: String,
    pub theme: Theme,
    pub avatar: Option<Avatar>,
    pub should_quit: bool,
}

impl App {
    /// Create a new application with the given configuration
    pub fn new(config: &Config) -> Self {
        let avatar = config
            .avatar
            .as_deref()
            .and_then(crate::avatar::load);

        Self {
            username: String::new(),
            password: String::new(),
            error: None,
            authenticating: false,
            focus: Focus::Username,
            session_cmd: config.session.clone(),
            theme: Theme::from(&config.theme),
            avatar,
            should_quit: false,
        }
    }

    /// Handle character input for the focused field
    pub fn input_char(&mut self, c: char) {
        self.error = None;
        match self.focus {
            Focus::Username => self.username.push(c),
            Focus::Password => self.password.push(c),
        }
    }

    /// Handle backspace for the focused field
    pub fn backspace(&mut self) {
        match self.focus {
            Focus::Username => {
                self.username.pop();
            }
            Focus::Password => {
                self.password.pop();
            }
        }
    }

    /// Switch focus to the next field
    pub const fn next_field(&mut self) {
        self.focus = match self.focus {
            Focus::Username => Focus::Password,
            Focus::Password => Focus::Username,
        };
    }

    /// Switch focus to the previous field
    pub const fn prev_field(&mut self) {
        // Only two fields, so same as next
        self.focus = match self.focus {
            Focus::Username => Focus::Password,
            Focus::Password => Focus::Username,
        };
    }

    /// Handle submit action (Enter key).
    /// Returns `true` if credentials are valid and authentication should proceed.
    /// Caller must render before calling `authenticate()` (which blocks on IPC).
    pub fn submit(&mut self) -> bool {
        if self.authenticating {
            return false;
        }

        if self.focus == Focus::Username {
            if self.username.is_empty() {
                self.error = Some("Username required".to_string());
                return false;
            }
            self.focus = Focus::Password;
            return false;
        }

        if self.username.is_empty() {
            self.error = Some("Username required".to_string());
            self.focus = Focus::Username;
            return false;
        }

        if self.password.is_empty() {
            self.error = Some("Password required".to_string());
            return false;
        }

        self.authenticating = true;
        self.error = None;
        true
    }

    /// Perform authentication against greetd (blocking IPC).
    /// Returns `true` on success (session started).
    pub fn authenticate(&mut self) -> bool {
        match greetd::authenticate(&self.username, &self.password, &self.session_cmd) {
            Ok(()) => true,
            Err(e) => {
                self.authenticating = false;
                self.error = Some(e.to_string());
                self.password.clear();
                self.focus = Focus::Password;
                false
            }
        }
    }

    /// Request application quit
    pub const fn quit(&mut self) {
        self.should_quit = true;
    }
}
