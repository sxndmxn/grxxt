//! grxxt - A brutalist greetd greeter
//!
//! A TUI-based greeter that runs directly on the TTY.

mod app;
mod avatar;
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

use app::App;
use config::Config;
use power::{reboot, shutdown, suspend};

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
        terminal.draw(|frame| ui::render(frame, &mut app))?;

        // Handle events with 500ms timeout for clock updates
        if event::poll(Duration::from_millis(500))? {
            if let Event::Key(key) = event::read()? {
                // Only handle key press events, not release
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                #[allow(clippy::wildcard_enum_match_arm, reason = "KeyCode has 20+ variants from external crate")]
                match key.code {
                    // Power controls
                    KeyCode::F(1) => shutdown(),
                    KeyCode::F(2) => reboot(),
                    KeyCode::F(3) => suspend(),

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
                        if app.submit() {
                            terminal.draw(|frame| ui::render(frame, &mut app))?;
                            if app.authenticate() {
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
