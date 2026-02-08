//! TUI rendering logic

use chrono::Local;
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::{App, Focus};

/// Render the entire UI
pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let theme = &app.theme;

    // Clear with background color
    let bg = Block::default().style(Style::default().bg(theme.background));
    frame.render_widget(bg, area);

    // Layout: header at top, form centered
    let chunks = Layout::vertical([
        Constraint::Length(3), // Header
        Constraint::Min(0),    // Main content
    ])
    .split(area);

    render_header(frame, app, chunks[0]);
    render_form(frame, app, chunks[1]);
}

/// Render the header with clock and power buttons
fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let time = Local::now();

    // Split header into left (clock) and right (power buttons)
    let chunks = Layout::horizontal([
        Constraint::Min(20),   // Clock
        Constraint::Length(30), // Power buttons
    ])
    .split(area);

    // Clock
    let clock_time = time.format("%H:%M").to_string();
    let clock_date = time.format("%a %d %b").to_string().to_uppercase();
    let clock = Paragraph::new(vec![
        Line::from(Span::styled(
            clock_time,
            Style::default().fg(theme.foreground).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            clock_date,
            Style::default().fg(theme.foreground),
        )),
    ])
    .alignment(Alignment::Left)
    .block(Block::default().style(Style::default().bg(theme.background)));

    frame.render_widget(clock, add_margin(chunks[0], 2, 1));

    // Power buttons
    let power = Paragraph::new(Line::from(vec![
        Span::styled("[F1] ", Style::default().fg(theme.foreground)),
        Span::styled("⏻ ", Style::default().fg(theme.accent)),
        Span::styled("[F2] ", Style::default().fg(theme.foreground)),
        Span::styled("󰜉 ", Style::default().fg(theme.accent)),
        Span::styled("[F3] ", Style::default().fg(theme.foreground)),
        Span::styled("󰤄", Style::default().fg(theme.accent)),
    ]))
    .alignment(Alignment::Right)
    .block(Block::default().style(Style::default().bg(theme.background)));

    frame.render_widget(power, add_margin(chunks[1], 2, 1));
}

/// Render the main form area
fn render_form(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;

    // Center the form vertically and horizontally
    let form_width = 30_u16;
    let form_height = 14_u16;

    let x = area.x + area.width.saturating_sub(form_width) / 2;
    let y = area.y + area.height.saturating_sub(form_height) / 2;
    let form_area = Rect::new(x, y, form_width, form_height);

    // Avatar (box with user icon)
    let avatar_area = Rect::new(form_area.x + 10, form_area.y, 10, 5);
    let avatar = Paragraph::new(Line::from(Span::styled(
        "󰀄",
        Style::default().fg(theme.foreground),
    )))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.foreground))
            .style(Style::default().bg(theme.background)),
    );
    frame.render_widget(avatar, avatar_area);

    // Username input
    let username_area = Rect::new(form_area.x, form_area.y + 6, form_width, 3);
    render_input(
        frame,
        &app.username,
        "username",
        app.focus == Focus::Username,
        theme.foreground,
        theme.accent,
        theme.background,
        username_area,
    );

    // Password input
    let password_area = Rect::new(form_area.x, form_area.y + 9, form_width, 3);
    let masked_password = "*".repeat(app.password.len());
    render_input(
        frame,
        &masked_password,
        "password",
        app.focus == Focus::Password,
        theme.foreground,
        theme.accent,
        theme.background,
        password_area,
    );

    // Error or status message
    let msg_area = Rect::new(form_area.x, form_area.y + 12, form_width, 1);
    if let Some(ref err) = app.error {
        let error = Paragraph::new(Line::from(Span::styled(
            err.to_uppercase(),
            Style::default().fg(theme.error),
        )))
        .alignment(Alignment::Center);
        frame.render_widget(error, msg_area);
    } else if app.authenticating {
        let status = Paragraph::new(Line::from(Span::styled(
            "authenticating...",
            Style::default().fg(theme.foreground),
        )))
        .alignment(Alignment::Center);
        frame.render_widget(status, msg_area);
    }
}

/// Render a single input field
#[allow(clippy::too_many_arguments, reason = "render helper takes individual style params")]
fn render_input(
    frame: &mut Frame,
    value: &str,
    placeholder: &str,
    focused: bool,
    fg: ratatui::style::Color,
    accent: ratatui::style::Color,
    bg: ratatui::style::Color,
    area: Rect,
) {
    let border_color = if focused { accent } else { fg };

    let display = if value.is_empty() {
        Span::styled(
            placeholder,
            Style::default().fg(fg).add_modifier(Modifier::DIM),
        )
    } else {
        Span::styled(value, Style::default().fg(fg))
    };

    let input = Paragraph::new(Line::from(display)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(bg)),
    );

    frame.render_widget(input, area);

    // Show cursor if focused
    if focused {
        #[allow(clippy::cast_possible_truncation, reason = "input limited to ~30 chars, fits u16")]
        let cursor_x = area.x + 1 + value.len() as u16;
        let cursor_y = area.y + 1;
        if cursor_x < area.x + area.width - 1 {
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }
}

/// Add margin to a rect
const fn add_margin(area: Rect, horizontal: u16, vertical: u16) -> Rect {
    Rect {
        x: area.x + horizontal,
        y: area.y + vertical,
        width: area.width.saturating_sub(horizontal * 2),
        height: area.height.saturating_sub(vertical * 2),
    }
}
