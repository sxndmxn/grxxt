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
use std::time::{Duration, Instant};

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

const FULL_REDRAW_INTERVAL: Duration = Duration::from_secs(5);

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
    let mut last_full_redraw = None;

    while !terminate.load(Ordering::Relaxed) {
        // Kernel messages and other privileged console writers can bypass the
        // alternate screen. Periodically invalidate Ratatui's diff buffer so
        // those external writes cannot leave the login form corrupted.
        let full_redraw =
            last_full_redraw.is_none_or(|last: Instant| last.elapsed() >= FULL_REDRAW_INTERVAL);
        draw_ui(terminal, &mut app, full_redraw)?;
        if full_redraw {
            last_full_redraw = Some(Instant::now());
        }

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
                // Repair any external console output immediately on the next
                // frame after the user interacts.
                last_full_redraw = None;

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
                        draw_ui(terminal, &mut app, true)
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

fn draw_ui<B: Backend>(terminal: &mut Terminal<B>, app: &mut App, full_redraw: bool) -> Result<()>
where
    B::Error: Send + Sync + 'static,
{
    if full_redraw {
        terminal
            .clear()
            .context("failed to clear terminal for a full redraw")?;
    }
    terminal
        .draw(|frame| ui::render(frame, app))
        .context("failed to render terminal frame")?;
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

    #[test]
    fn full_redraw_repairs_external_terminal_writes() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(&Config::default());

        draw_ui(&mut terminal, &mut app, false).unwrap();
        let external = Buffer::with_lines(["external console output"]);
        terminal
            .backend_mut()
            .draw(
                external
                    .content
                    .iter()
                    .zip(0_u16..)
                    .map(|(cell, x)| (x, 0, cell)),
            )
            .unwrap();

        draw_ui(&mut terminal, &mut app, true).unwrap();

        let first_row = terminal.backend().buffer().content[..80]
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();
        assert!(!first_row.contains("external console output"));
        assert!(first_row.contains("[F1]"));
    }
}
