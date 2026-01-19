//! Status bar header widget.
//!
//! Renders the 2-line header showing:
//! - Line 1: "rslph" branding on left, "project (model)" on right
//! - Line 2: Iteration/task counts and context usage bar

use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    widgets::Paragraph,
    Frame,
};

use crate::tui::app::App;
use crate::tui::widgets::progress_bar::render_context_bar;

/// Render the 2-line status bar header.
pub fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let [row1, row2] = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(area);

    render_branding_line(frame, row1, app);
    render_status_line(frame, row2, app);
}

/// Render the first line: "rslph" left, "project (model)" right.
fn render_branding_line(frame: &mut Frame, area: Rect, app: &App) {
    let [left, right] =
        Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(area);

    frame.render_widget(
        Paragraph::new("rslph").style(Style::default().add_modifier(Modifier::BOLD)),
        left,
    );

    let project_model = format!("{} ({})", app.project_name, app.model_name);
    frame.render_widget(
        Paragraph::new(project_model).alignment(Alignment::Right),
        right,
    );
}

/// Render the second line: iteration/task counts and context bar.
fn render_status_line(frame: &mut Frame, area: Rect, app: &App) {
    // Format the status text
    let status_text = format!(
        "Iter {}/{} | Task {}/{} | ",
        app.current_iteration, app.max_iterations, app.current_task, app.total_tasks
    );

    // Calculate widths - status text on left, context bar fills remaining
    let text_width = status_text.len() as u16;
    let [text_area, bar_area] =
        Layout::horizontal([Constraint::Length(text_width), Constraint::Fill(1)]).areas(area);

    frame.render_widget(Paragraph::new(status_text), text_area);
    render_context_bar(frame, bar_area, app.context_usage);
}
