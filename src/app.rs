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
        let avatar = config.avatar.as_deref().and_then(crate::avatar::load);

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

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        App::new(&Config::default())
    }

    #[test]
    fn input_and_backspace_follow_focus() {
        let mut app = app();

        app.input_char('a');
        app.input_char('é');
        app.backspace();
        assert_eq!(app.username, "a");
        assert!(app.password.is_empty());

        app.next_field();
        app.input_char('p');
        app.backspace();
        assert!(app.password.is_empty());
    }

    #[test]
    fn submit_validates_username_then_password() {
        let mut app = app();

        assert!(!app.submit());
        assert_eq!(app.error.as_deref(), Some("Username required"));

        app.input_char('a');
        assert!(!app.submit());
        assert_eq!(app.focus, Focus::Password);
        assert!(app.error.is_none());

        assert!(!app.submit());
        assert_eq!(app.error.as_deref(), Some("Password required"));

        app.input_char('p');
        assert!(app.submit());
        assert!(app.authenticating);
        assert!(!app.submit());
    }

    #[test]
    fn navigation_wraps_and_quit_sets_flag() {
        let mut app = app();

        app.prev_field();
        assert_eq!(app.focus, Focus::Password);
        app.next_field();
        assert_eq!(app.focus, Focus::Username);

        app.quit();
        assert!(app.should_quit);
    }
}
