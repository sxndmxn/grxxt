//! Application state management for the TUI greeter

use crate::avatar::Avatar;
use crate::config::Config;
use crate::greetd;
use crate::theme::Theme;
use zeroize::Zeroize;

const MAX_USERNAME_BYTES: usize = 256;
const MAX_PASSWORD_BYTES: usize = 4096;
const MAX_ERROR_CHARS: usize = 512;

/// Which input field is currently focused
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Username,
    Password,
}

/// Application state
pub struct App {
    username: String,
    password: String,
    error: Option<String>,
    authenticating: bool,
    focus: Focus,
    session_cmd: String,
    pub theme: Theme,
    pub avatar: Option<Avatar>,
    should_quit: bool,
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
        if c.is_control() {
            return;
        }

        self.error = None;
        match self.focus {
            Focus::Username
                if self.username.len().saturating_add(c.len_utf8()) <= MAX_USERNAME_BYTES =>
            {
                self.username.push(c);
            }
            Focus::Password
                if self.password.len().saturating_add(c.len_utf8()) <= MAX_PASSWORD_BYTES =>
            {
                self.password.push(c);
            }
            Focus::Username => self.error = Some("Username too long".into()),
            Focus::Password => self.error = Some("Password too long".into()),
        }
    }

    /// Handle backspace for the focused field
    pub fn backspace(&mut self) {
        self.error = None;
        match self.focus {
            Focus::Username => {
                self.username.pop();
            }
            Focus::Password => {
                self.password.pop();
            }
        }
    }

    /// Number of scalar values to mask in the password field.
    pub fn password_character_count(&self) -> usize {
        self.password.chars().count()
    }

    /// Username currently entered in the form.
    pub fn username(&self) -> &str {
        &self.username
    }

    /// Current validation or operational error, if any.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Field currently receiving input.
    pub const fn focus(&self) -> Focus {
        self.focus
    }

    /// Whether an authentication request is in progress.
    pub const fn is_authenticating(&self) -> bool {
        self.authenticating
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
        self.authenticate_with(greetd::authenticate)
    }

    fn authenticate_with(
        &mut self,
        authenticate: impl FnOnce(&str, &str, &str) -> Result<(), greetd::AuthError>,
    ) -> bool {
        let result = authenticate(&self.username, &self.password, &self.session_cmd);
        self.password.zeroize();

        match result {
            Ok(()) => true,
            Err(e) => {
                self.authenticating = false;
                self.show_error(&e.to_string());
                self.focus = Focus::Password;
                false
            }
        }
    }

    /// Request application quit
    pub const fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Whether the event loop should exit.
    pub const fn should_quit(&self) -> bool {
        self.should_quit
    }

    /// Show an operational error in the form.
    pub fn show_error(&mut self, message: &str) {
        let mut characters = message.chars();
        let mut visible: String = characters
            .by_ref()
            .take(MAX_ERROR_CHARS)
            .map(|character| {
                if character.is_control() {
                    ' '
                } else {
                    character
                }
            })
            .collect();
        if characters.next().is_some() {
            visible.push('…');
        }
        self.error = Some(visible);
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.password.zeroize();
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

    #[test]
    fn operational_errors_are_visible() {
        let mut app = app();

        app.show_error("systemctl failed");

        assert_eq!(app.error.as_deref(), Some("systemctl failed"));
    }

    #[test]
    fn operational_errors_are_safe_and_bounded_for_terminal_display() {
        let mut app = app();
        app.show_error("failed\x1b[2J\nnext");
        assert_eq!(app.error.as_deref(), Some("failed [2J next"));

        app.show_error(&"x".repeat(MAX_ERROR_CHARS + 1));
        assert_eq!(
            app.error.as_deref().map(|visible| visible.chars().count()),
            Some(MAX_ERROR_CHARS + 1)
        );
        assert!(app
            .error
            .as_deref()
            .is_some_and(|visible| visible.ends_with('…')));
    }

    #[test]
    fn credential_inputs_are_bounded() {
        let mut app = app();
        app.username = "a".repeat(MAX_USERNAME_BYTES);
        app.input_char('b');
        assert_eq!(app.username.len(), MAX_USERNAME_BYTES);
        assert_eq!(app.error.as_deref(), Some("Username too long"));

        app.focus = Focus::Password;
        app.password = "p".repeat(MAX_PASSWORD_BYTES);
        app.input_char('q');
        assert_eq!(app.password.len(), MAX_PASSWORD_BYTES);
        assert_eq!(app.error.as_deref(), Some("Password too long"));
    }

    #[test]
    fn control_characters_are_ignored() {
        let mut app = app();
        app.input_char('\n');
        app.input_char('\0');
        assert!(app.username.is_empty());
    }

    #[test]
    fn failed_authentication_clears_password_and_allows_retry() {
        let mut app = app();
        app.username = "alice".into();
        app.password = "test-secret".into();
        app.focus = Focus::Password;
        app.authenticating = true;

        let succeeded = app.authenticate_with(|username, password, command| {
            assert_eq!(username, "alice");
            assert_eq!(password, "test-secret");
            assert_eq!(command, "/usr/bin/Hyprland");
            Err(greetd::AuthError::AuthFailed)
        });

        assert!(!succeeded);
        assert!(app.password.is_empty());
        assert!(!app.authenticating);
        assert_eq!(app.focus, Focus::Password);
        assert_eq!(app.error.as_deref(), Some("Authentication failed"));
    }

    #[test]
    fn successful_authentication_also_clears_password() {
        let mut app = app();
        app.password = "test-secret".into();

        assert!(app.authenticate_with(|_, _, _| Ok(())));
        assert!(app.password.is_empty());
    }
}
