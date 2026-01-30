//! Status bar header widget.
//!
//! Renders the 2-line header showing:
//! - Line 1: "rslph" branding on left, "◆ model | HH:MM:SS" on right
//! - Line 2: Iteration/task counts, token usage, and context usage bar

use std::time::Instant;

use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    widgets::Paragraph,
    Frame,
};

use crate::build::tokens::format_tokens;
use crate::tui::app::App;
use crate::tui::theme::symbols::model_tier_indicator;
use crate::tui::widgets::progress_bar::render_context_bar;

/// Format session duration as HH:MM:SS or MM:SS.
fn format_session_time(start: Instant) -> String {
    let elapsed = start.elapsed();
    let secs = elapsed.as_secs();
    let mins = secs / 60;
    let hours = mins / 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, mins % 60, secs % 60)
    } else {
        format!("{:02}:{:02}", mins, secs % 60)
    }
}

/// Render the 2-line status bar header.
pub fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let [row1, row2] = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(area);

    render_branding_line(frame, row1, app);
    render_status_line(frame, row2, app);
}

/// Render the first line: "rslph" left, "◆ model | HH:MM:SS" right.
fn render_branding_line(frame: &mut Frame, area: Rect, app: &App) {
    let [left, right] = Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(area);

    frame.render_widget(
        Paragraph::new("rslph").style(Style::default().add_modifier(Modifier::BOLD)),
        left,
    );

    // Format: "◆ claude-opus-4 | 05:23"
    let tier_symbol = model_tier_indicator(&app.model_name);
    let session_time = format_session_time(app.session_start);
    let right_text = format!("{} {} | {}", tier_symbol, app.model_name, session_time);

    frame.render_widget(
        Paragraph::new(right_text).alignment(Alignment::Right),
        right,
    );
}

/// Render the second line: iteration/task counts, token usage, and context bar.
fn render_status_line(frame: &mut Frame, area: Rect, app: &App) {
    // Format the status text with token counts (per CONTEXT.md decision - abbreviated format)
    let status_text = format!(
        "Iter {}/{} | Task {}/{} | In: {} | Out: {} | CacheW: {} | CacheR: {} | ",
        app.current_iteration,
        app.max_iterations,
        app.current_task,
        app.total_tasks,
        format_tokens(app.total_tokens.input_tokens),
        format_tokens(app.total_tokens.output_tokens),
        format_tokens(app.total_tokens.cache_creation_input_tokens),
        format_tokens(app.total_tokens.cache_read_input_tokens),
    );

    // Calculate widths - status text on left, context bar fills remaining
    let text_width = status_text.len() as u16;
    let [text_area, bar_area] =
        Layout::horizontal([Constraint::Length(text_width), Constraint::Fill(1)]).areas(area);

    frame.render_widget(Paragraph::new(status_text), text_area);
    render_context_bar(frame, bar_area, app.context_usage);
}
