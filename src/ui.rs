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

/// Complement of the golden ratio (1 - 1/φ ≈ 0.382)
const PHI_COMP: f32 = 0.382;

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
        Constraint::Ratio(618, 1000), // Clock (φ⁻¹)
        Constraint::Ratio(382, 1000), // Power buttons (1 - φ⁻¹)
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

    // Golden ratio form width: area.width * PHI_COMP, clamped [28, 50]
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "area dimensions are small u16 values, product fits u16"
    )]
    let form_width = (f32::from(area.width) * PHI_COMP)
        .round()
        .clamp(28.0, 50.0) as u16;

    // Fibonacci-based form height: avatar(5) + gap(2) + user(3) + gap(1) + pass(3) + gap(1) + msg(1) = 16
    let form_height = 16_u16;

    // Golden section vertical placement: form center at 38.2% from top
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "area dimensions are small u16 values, product fits u16"
    )]
    let y = area.y
        + f32::from(area.height)
            .mul_add(PHI_COMP, -(f32::from(form_height) / 2.0))
            .round()
            .max(0.0) as u16;
    let x = area.x + area.width.saturating_sub(form_width) / 2;
    let form_area = Rect::new(x, y, form_width, form_height);

    // Avatar: width = form_width * PHI_COMP, clamped min 8, centered in form
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "form_width is small u16, product fits u16"
    )]
    let avatar_width = (f32::from(form_width) * PHI_COMP)
        .round()
        .max(8.0) as u16;
    let avatar_x = form_area.x + (form_width.saturating_sub(avatar_width)) / 2;
    let avatar_area = Rect::new(avatar_x, form_area.y, avatar_width, 5);
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

    // Fibonacci spacing: avatar(5) + gap(2) = offset 7
    let username_area = Rect::new(form_area.x, form_area.y + 7, form_width, 3);
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

    // Fibonacci spacing: username(3) + gap(1) = offset 11
    let password_area = Rect::new(form_area.x, form_area.y + 11, form_width, 3);
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

    // Fibonacci spacing: password(3) + gap(1) = offset 15
    let msg_area = Rect::new(form_area.x, form_area.y + 15, form_width, 1);
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
