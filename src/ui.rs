//! TUI rendering logic

use chrono::Local;
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use ratatui_image::{Resize, StatefulImage};

use crate::app::{App, Focus};

/// Complement of the golden ratio (1 - 1/φ ≈ 0.382)
const PHI_COMP: f32 = 0.382;

/// Render the entire UI
pub fn render(frame: &mut Frame, app: &mut App) {
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
fn render_form(frame: &mut Frame, app: &mut App, area: Rect) {
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

    // Avatar height adapts: 10 with image, 5 for icon; shrinks to fit terminal
    // Non-avatar portion: gap(2) + user(3) + gap(1) + pass(3) + gap(1) + msg(1) = 11
    let base_height: u16 = 11;
    let desired_avatar: u16 = if app.avatar.is_some() { 10 } else { 5 };
    let avatar_height = desired_avatar.min(area.height.saturating_sub(base_height).max(3));
    let form_height = avatar_height + base_height;

    // Golden section vertical placement: form center at 38.2% from top, clamped to fit
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "area dimensions are small u16 values, product fits u16"
    )]
    let y = area.y
        + f32::from(area.height)
            .mul_add(PHI_COMP, -(f32::from(form_height) / 2.0))
            .round()
            .clamp(0.0, f32::from(area.height.saturating_sub(form_height))) as u16;
    let x = area.x + area.width.saturating_sub(form_width) / 2;
    let form_area = Rect::new(x, y, form_width, form_height);

    // Avatar: full form width, adaptive height
    let avatar_area = Rect::new(form_area.x, form_area.y, form_width, avatar_height);

    let avatar_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.foreground))
        .style(Style::default().bg(theme.background));

    if let Some(ref mut avatar) = app.avatar {
        let inner = avatar_block.inner(avatar_area);
        frame.render_widget(avatar_block, avatar_area);

        // Center image: with halfblocks + font(4,8), cells are effectively square.
        // Compute how many columns the fit image occupies, then offset.
        let img_cols = (f32::from(inner.height) * avatar.aspect_ratio).round();
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "result is small positive u16"
        )]
        let x_offset = (f32::from(inner.width).max(img_cols) - img_cols) as u16 / 2;
        let centered = Rect::new(inner.x + x_offset, inner.y, inner.width.saturating_sub(x_offset * 2), inner.height);

        let image = StatefulImage::default().resize(Resize::Fit(None));
        frame.render_stateful_widget(image, centered, &mut avatar.protocol);
    } else {
        let icon = Paragraph::new(Line::from(Span::styled(
            "󰀄",
            Style::default().fg(theme.foreground),
        )))
        .alignment(Alignment::Center)
        .block(avatar_block);
        frame.render_widget(icon, avatar_area);
    }

    // Offsets derived from avatar height
    let user_y = form_area.y + avatar_height + 2;
    let pass_y = user_y + 4;
    let msg_y = pass_y + 4;

    let username_area = Rect::new(form_area.x, user_y, form_width, 3);
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

    let password_area = Rect::new(form_area.x, pass_y, form_width, 3);
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

    let msg_area = Rect::new(form_area.x, msg_y, form_width, 1);
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
