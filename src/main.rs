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
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::*;
use signal_hook::{
    consts::signal::{SIGHUP, SIGINT, SIGQUIT, SIGTERM},
    flag,
};

use app::App;
use config::Config;
use power::PowerAction;

fn main() -> Result<()> {
    let config = Config::load()?;
    let terminate = termination_flag()?;

    terminal::enable_raw_mode().context("failed to enable terminal raw mode")?;
    let _terminal_guard = TerminalGuard;
    stdout()
        .execute(EnterAlternateScreen)
        .context("failed to enter alternate screen")?;
    stdout()
        .execute(cursor::Hide)
        .context("failed to hide terminal cursor")?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend).context("failed to initialize terminal backend")?;

    run(&mut terminal, &config, &terminate).context("terminal event loop failed")
}

fn termination_flag() -> Result<Arc<AtomicBool>> {
    let terminate = Arc::new(AtomicBool::new(false));
    for signal in [SIGHUP, SIGINT, SIGQUIT, SIGTERM] {
        flag::register(signal, Arc::clone(&terminate))
            .with_context(|| format!("failed to register signal handler for {signal}"))?;
    }
    Ok(terminate)
}

/// Restore the TTY even when setup or the event loop exits with an error.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        stdout().execute(cursor::Show).ok();
        stdout().execute(LeaveAlternateScreen).ok();
        terminal::disable_raw_mode().ok();
    }
}

fn run<B: Backend>(
    terminal: &mut Terminal<B>,
    config: &Config,
    terminate: &AtomicBool,
) -> Result<()>
where
    B::Error: Send + Sync + 'static,
{
    let mut app = App::new(config);

    while !terminate.load(Ordering::Relaxed) {
        // Render
        terminal
            .draw(|frame| ui::render(frame, &mut app))
            .context("failed to render terminal frame")?;

        if terminate.load(Ordering::Relaxed) {
            break;
        }

        // Handle events with 500ms timeout for clock updates
        if event::poll(Duration::from_millis(500)).context("failed to poll terminal input")? {
            if let Event::Key(key) = event::read().context("failed to read terminal input")? {
                // Handle initial and repeated presses, never key-release events.
                if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                    continue;
                }

                #[allow(
                    clippy::wildcard_enum_match_arm,
                    reason = "KeyCode has 20+ variants from external crate"
                )]
                match key.code {
                    // Power controls
                    KeyCode::F(1) => run_power_action(&mut app, PowerAction::PowerOff),
                    KeyCode::F(2) => run_power_action(&mut app, PowerAction::Reboot),
                    KeyCode::F(3) => run_power_action(&mut app, PowerAction::Suspend),

                    // Quit (development only)
                    KeyCode::Esc if cfg!(debug_assertions) => app.quit(),

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
                    KeyCode::Char(c)
                        if !key.modifiers.intersects(
                            KeyModifiers::CONTROL
                                | KeyModifiers::ALT
                                | KeyModifiers::SUPER
                                | KeyModifiers::HYPER
                                | KeyModifiers::META,
                        ) =>
                    {
                        app.input_char(c);
                    }
                    KeyCode::Backspace => app.backspace(),

                    // Submit
                    KeyCode::Enter if app.submit() => {
                        terminal
                            .draw(|frame| ui::render(frame, &mut app))
                            .context("failed to render authentication status")?;
                        if app.authenticate() {
                            break;
                        }
                    }

                    _ => {}
                }
            }
        }

        if app.should_quit() {
            break;
        }
    }

    Ok(())
}

fn run_power_action(app: &mut App, action: PowerAction) {
    if let Err(error) = power::execute(action) {
        app.show_error(&error.to_string());
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "tests can unwrap")]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;

    #[test]
    fn termination_flag_stops_before_waiting_for_input() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let terminate = AtomicBool::new(true);

        assert!(run(&mut terminal, &Config::default(), &terminate).is_ok());
    }
}
