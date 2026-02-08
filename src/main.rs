//! grxxt - A brutalist greetd greeter
//!
//! A TUI-based greeter that runs directly on the TTY.

mod app;
mod config;
mod greetd;
mod power;
mod theme;
mod ui;

use std::io::stdout;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::*;

use app::{App, AuthResult};
use config::Config;

fn main() -> Result<()> {
    // Load configuration
    let config = Config::load();

    // Setup terminal
    terminal::enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    stdout().execute(cursor::Hide)?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    // Run the application
    let result = run(&mut terminal, &config);

    // Restore terminal
    stdout().execute(cursor::Show)?;
    stdout().execute(LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;

    result
}

fn run<B: Backend>(terminal: &mut Terminal<B>, config: &Config) -> Result<()> {
    let mut app = App::new(config);

    loop {
        // Render
        terminal.draw(|frame| ui::render(frame, &app))?;

        // Handle events with 500ms timeout for clock updates
        if event::poll(Duration::from_millis(500))? {
            if let Event::Key(key) = event::read()? {
                // Only handle key press events, not release
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match key.code {
                    // Power controls
                    KeyCode::F(1) => App::shutdown(),
                    KeyCode::F(2) => App::reboot(),
                    KeyCode::F(3) => App::suspend(),

                    // Quit (development only)
                    KeyCode::Esc => app.quit(),

                    // Navigation
                    KeyCode::Tab => {
                        if key.modifiers.contains(KeyModifiers::SHIFT) {
                            app.prev_field();
                        } else {
                            app.next_field();
                        }
                    }
                    KeyCode::BackTab => app.prev_field(),

                    // Input
                    KeyCode::Char(c) => app.input_char(c),
                    KeyCode::Backspace => app.backspace(),

                    // Submit
                    KeyCode::Enter => {
                        if app.submit() == Some(AuthResult::Pending) {
                            // Need to render "authenticating..." before blocking
                            terminal.draw(|frame| ui::render(frame, &app))?;

                            // Perform blocking authentication
                            if app.do_authenticate() == AuthResult::Success {
                                // Successful auth - greetd starts the session
                                break;
                            }
                        }
                    }

                    _ => {}
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
