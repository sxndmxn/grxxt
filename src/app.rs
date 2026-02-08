//! Application state management for the TUI greeter

use crate::config::Config;
use crate::greetd::{authenticate, AuthError};
use crate::power;
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
    pub should_quit: bool,
}

impl App {
    /// Create a new application with the given configuration
    pub fn new(config: &Config) -> Self {
        Self {
            username: String::new(),
            password: String::new(),
            error: None,
            authenticating: false,
            focus: Focus::Username,
            session_cmd: config.session.clone(),
            theme: Theme::from(&config.theme),
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

    /// Handle submit action (Enter key)
    pub fn submit(&mut self) -> Option<AuthResult> {
        if self.authenticating {
            return None;
        }

        // If on username, move to password
        if self.focus == Focus::Username {
            if self.username.is_empty() {
                self.error = Some("Username required".to_string());
                return None;
            }
            self.focus = Focus::Password;
            return None;
        }

        // Validate
        if self.username.is_empty() {
            self.error = Some("Username required".to_string());
            self.focus = Focus::Username;
            return None;
        }

        if self.password.is_empty() {
            self.error = Some("Password required".to_string());
            return None;
        }

        // Start authentication
        self.authenticating = true;
        self.error = None;

        Some(AuthResult::Pending)
    }

    /// Perform the actual authentication (blocking)
    pub fn do_authenticate(&mut self) -> AuthResult {
        match authenticate(&self.username, &self.password, &self.session_cmd) {
            Ok(()) => AuthResult::Success,
            Err(e) => {
                self.authenticating = false;
                self.error = Some(format_auth_error(&e));
                self.password.clear();
                self.focus = Focus::Password;
                AuthResult::Failed
            }
        }
    }

    /// Trigger shutdown
    pub fn shutdown() {
        power::shutdown();
    }

    /// Trigger reboot
    pub fn reboot() {
        power::reboot();
    }

    /// Trigger suspend
    pub fn suspend() {
        power::suspend();
    }

    /// Request application quit
    pub const fn quit(&mut self) {
        self.should_quit = true;
    }
}

/// Result of authentication attempt
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthResult {
    Pending,
    Success,
    Failed,
}

fn format_auth_error(e: &AuthError) -> String {
    e.to_string()
}
