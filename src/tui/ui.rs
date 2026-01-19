//! Main UI rendering module.
//!
//! Provides the top-level render function that composes all TUI widgets.

use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::Paragraph,
    Frame,
};

use super::app::App;
use super::widgets::status_bar::render_header;

/// Render the entire TUI interface.
///
/// Divides the frame into three areas:
/// - Header (2 lines): Status bar with iteration/task count and context bar
/// - Body (fills remaining): Output area for Claude messages
/// - Footer (1 line): Key binding hints and log path
pub fn render(frame: &mut Frame, app: &App) {
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(2),  // 2-line header
        Constraint::Fill(1),    // Main output area
        Constraint::Length(1),  // Footer with key hints
    ])
    .areas(frame.area());

    render_header(frame, header, app);
    render_body(frame, body, app);
    render_footer(frame, footer, app);
}

/// Render the main body area (placeholder for Plan 03).
fn render_body(frame: &mut Frame, area: Rect, _app: &App) {
    let paragraph = Paragraph::new("Output area")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    frame.render_widget(paragraph, area);
}

/// Render the footer with key binding hints and log path.
fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let key_hints = "j/k:scroll  {/}:iteration  p:pause  Ctrl+C:quit";

    // If log_path exists, show it on the right
    let log_display = app
        .log_path
        .as_ref()
        .map(|p| format!("Log: {}", p.display()))
        .unwrap_or_default();

    if log_display.is_empty() {
        // Just key hints, left-aligned
        let paragraph = Paragraph::new(key_hints).style(Style::default().fg(Color::DarkGray));
        frame.render_widget(paragraph, area);
    } else {
        // Split: key hints left, log path right
        let [left, right] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(area);

        frame.render_widget(
            Paragraph::new(key_hints).style(Style::default().fg(Color::DarkGray)),
            left,
        );
        frame.render_widget(
            Paragraph::new(log_display)
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Right),
            right,
        );
    }
}
