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
const MIN_FULL_WIDTH: u16 = 28;
const MIN_FULL_HEIGHT: u16 = 17;

/// Render the entire UI
pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let theme = &app.theme;

    // Clear with background color
    let bg = Block::default().style(Style::default().bg(theme.background));
    frame.render_widget(bg, area);

    if area.width < MIN_FULL_WIDTH || area.height < MIN_FULL_HEIGHT {
        let compact = Paragraph::new("grxxt")
            .style(Style::default().fg(theme.accent).bg(theme.background))
            .alignment(Alignment::Center);
        frame.render_widget(compact, area);
        return;
    }

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
            Style::default()
                .fg(theme.foreground)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            clock_date,
            Style::default().fg(theme.foreground),
        )),
    ])
    .alignment(Alignment::Left)
    .block(Block::default().style(Style::default().bg(theme.background)));

    frame.render_widget(clock, add_margin(chunks[0], 2, 0));

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

    frame.render_widget(power, add_margin(chunks[1], 2, 0));
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
    let form_width = (f32::from(area.width) * PHI_COMP).round().clamp(28.0, 50.0) as u16;

    // Avatar height adapts: 10 with image, 5 for icon; shrinks to fit terminal
    // Non-avatar portion: gap(2) + user(3) + gap(1) + pass(3) + gap(1) + msg(1) = 11
    let base_height: u16 = 11;
    let desired_avatar: u16 = if app.avatar.is_some() { 10 } else { 5 };
    let avatar_height = desired_avatar.min(area.height.saturating_sub(base_height).max(3));
    let form_height = avatar_height.saturating_add(base_height);

    // Golden section vertical placement: form center at 38.2% from top, clamped to fit
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "area dimensions are small u16 values, product fits u16"
    )]
    let y = area.y.saturating_add(
        f32::from(area.height)
            .mul_add(PHI_COMP, -(f32::from(form_height) / 2.0))
            .round()
            .clamp(0.0, f32::from(area.height.saturating_sub(form_height))) as u16,
    );
    let x = area
        .x
        .saturating_add(area.width.saturating_sub(form_width) / 2);
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

        let resize = Resize::Fit(None);
        let image_size = avatar.protocol.size_for(resize.clone(), inner.into());
        let centered = centered_rect(inner, image_size.width, image_size.height);
        let image = StatefulImage::default().resize(resize);
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
    let user_y = form_area.y.saturating_add(avatar_height).saturating_add(2);
    let pass_y = user_y.saturating_add(4);
    let msg_y = pass_y.saturating_add(4);

    let username_area = Rect::new(form_area.x, user_y, form_width, 3);
    render_input(
        frame,
        app.username(),
        "username",
        app.focus() == Focus::Username,
        theme.foreground,
        theme.accent,
        theme.background,
        username_area,
    );

    let password_area = Rect::new(form_area.x, pass_y, form_width, 3);
    let masked_password = "*".repeat(app.password_character_count());
    render_input(
        frame,
        &masked_password,
        "password",
        app.focus() == Focus::Password,
        theme.foreground,
        theme.accent,
        theme.background,
        password_area,
    );

    let msg_area = Rect::new(form_area.x, msg_y, form_width, 1);
    if let Some(err) = app.error() {
        let error = Paragraph::new(Line::from(Span::styled(
            err.to_uppercase(),
            Style::default().fg(theme.error),
        )))
        .alignment(Alignment::Center);
        frame.render_widget(error, msg_area);
    } else if app.is_authenticating() {
        let status = Paragraph::new(Line::from(Span::styled(
            "authenticating...",
            Style::default().fg(theme.foreground),
        )))
        .alignment(Alignment::Center);
        frame.render_widget(status, msg_area);
    }
}

/// Render a single input field
#[allow(
    clippy::too_many_arguments,
    reason = "render helper takes individual style params"
)]
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

    let (scroll, cursor_offset) = input_view(value, area.width);
    let input = Paragraph::new(Line::from(display))
        .scroll((0, scroll))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .style(Style::default().bg(bg)),
        );

    frame.render_widget(input, area);

    // Show cursor if focused
    if focused {
        let cursor_x = area.x.saturating_add(1).saturating_add(cursor_offset);
        let cursor_y = area.y.saturating_add(1);
        let right_border = area.x.saturating_add(area.width).saturating_sub(1);
        if cursor_x < right_border {
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }
}

/// Return horizontal scroll and cursor cell offset for an input's inner width.
fn input_view(value: &str, area_width: u16) -> (u16, u16) {
    let display_width = u16::try_from(Line::from(value).width()).unwrap_or(u16::MAX);
    let max_cursor_offset = area_width.saturating_sub(3);
    let scroll = display_width.saturating_sub(max_cursor_offset);
    (scroll, display_width.saturating_sub(scroll))
}

fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x.saturating_add(area.width.saturating_sub(width) / 2),
        area.y
            .saturating_add(area.height.saturating_sub(height) / 2),
        width,
        height,
    )
}

/// Add margin to a rect
const fn add_margin(area: Rect, horizontal: u16, vertical: u16) -> Rect {
    Rect {
        x: area.x.saturating_add(horizontal),
        y: area.y.saturating_add(vertical),
        width: area.width.saturating_sub(horizontal.saturating_mul(2)),
        height: area.height.saturating_sub(vertical.saturating_mul(2)),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "tests can unwrap")]
mod tests {
    use super::*;
    use crate::avatar::Avatar;
    use crate::config::Config;
    use image::DynamicImage;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ratatui::Terminal;
    use ratatui_image::picker::Picker;

    fn render_to_text(app: &mut App, width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, app)).unwrap();
        buffer_text(terminal.backend().buffer())
    }

    fn buffer_text(buffer: &Buffer) -> String {
        let width = usize::from(buffer.area.width);
        let mut output = String::new();

        for row in buffer.content.chunks(width) {
            for cell in row {
                output.push_str(cell.symbol());
            }
            output.push('\n');
        }

        output
    }

    #[test]
    fn renders_login_form_and_power_shortcuts() {
        let mut app = App::new(&Config::default());
        let date_before = Local::now().format("%a %d %b").to_string().to_uppercase();
        let output = render_to_text(&mut app, 100, 30);
        let date_after = Local::now().format("%a %d %b").to_string().to_uppercase();

        assert!(output.contains("username"));
        assert!(output.contains("password"));
        assert!(output.contains(&date_before) || output.contains(&date_after));
        assert!(output.contains("[F1]"));
        assert!(output.contains("[F2]"));
        assert!(output.contains("[F3]"));
    }

    #[test]
    fn masks_password_by_character_not_utf8_byte() {
        let mut app = App::new(&Config::default());
        app.next_field();
        app.input_char('é');
        app.input_char('a');
        let output = render_to_text(&mut app, 80, 24);

        assert!(output.contains("**"));
        assert!(!output.contains("***"));
    }

    #[test]
    fn renders_validation_error() {
        let mut app = App::new(&Config::default());
        app.show_error("Password required");
        let output = render_to_text(&mut app, 80, 24);

        assert!(output.contains("PASSWORD REQUIRED"));
    }

    #[test]
    fn tiny_terminal_does_not_panic() {
        let mut app = App::new(&Config::default());

        let _output = render_to_text(&mut app, 10, 5);
    }

    #[test]
    fn input_view_uses_display_width_and_scrolls_long_values() {
        assert_eq!(input_view("界a", 10), (0, 3));
        assert_eq!(input_view("界界界界", 8), (3, 5));

        let mut app = App::new(&Config::default());
        for _ in 0..40 {
            app.input_char('界');
        }
        let _output = render_to_text(&mut app, 28, 17);
    }

    #[test]
    fn centered_rect_accounts_for_both_axes_and_clamps() {
        let area = Rect::new(10, 20, 12, 8);
        assert_eq!(centered_rect(area, 6, 4), Rect::new(13, 22, 6, 4));
        assert_eq!(centered_rect(area, 20, 10), area);
    }

    #[test]
    fn renders_avatar_protocol_on_test_backend() {
        for (width, height) in [(28, 17), (80, 24)] {
            let mut app = App::new(&Config::default());
            app.avatar = Some(Avatar {
                protocol: Picker::halfblocks().new_resize_protocol(DynamicImage::new_rgba8(4, 2)),
            });

            let output = render_to_text(&mut app, width, height);
            assert!(!output.contains("󰀄"));
            assert!(output.contains("username"));
        }
    }
}
