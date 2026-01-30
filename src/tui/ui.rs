//! Main UI rendering module.
//!
//! Provides the top-level render function that composes all TUI widgets.

use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::app::App;
use super::conversation::render_conversation;
use super::widgets::spinner::render_spinner;
use super::widgets::status_bar::render_header;
use super::widgets::thread_view::render_thread;

/// Render the entire TUI interface.
///
/// Divides the frame into three areas:
/// - Header (2 lines): Status bar with iteration/task count and context bar
/// - Body (fills remaining): Output area for Claude messages
/// - Footer (1 line): Key binding hints and log path
///
/// When show_conversation is enabled, the body area is split horizontally
/// with the conversation view on the left and main thread view on the right.
///
/// # Arguments
///
/// * `frame` - The frame to render to
/// * `app` - Application state (mutable for spinner animation state)
/// * `recent_count` - Number of recent messages to display
pub fn render(frame: &mut Frame, app: &mut App, recent_count: usize) {
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(2), // 2-line header
        Constraint::Fill(1),   // Main output area
        Constraint::Length(1), // Footer with key hints
    ])
    .areas(frame.area());

    render_header(frame, header, app);

    // Render spinner in header area when streaming
    if app.is_streaming {
        let spinner_area = Rect {
            x: header.x + header.width.saturating_sub(20),
            y: header.y,
            width: 20.min(header.width),
            height: 1,
        };
        render_spinner(frame, spinner_area, &mut app.spinner_state, "");
    }

    if app.show_conversation {
        // Split body: conversation on left, main view on right
        let [conv_area, main_area] =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .areas(body);

        render_conversation(
            frame,
            conv_area,
            app.conversation.items(),
            app.conversation_scroll,
            &app.thinking_collapsed,
        );
        render_body(frame, main_area, app, recent_count);
    } else {
        render_body(frame, body, app, recent_count);
    }

    render_footer(frame, footer, app);

    // Show pause overlay if paused
    if app.is_paused {
        render_pause_overlay(frame, body);
    }
}

/// Render the main body area with thread view.
fn render_body(frame: &mut Frame, area: Rect, app: &App, recent_count: usize) {
    // Add a subtle border at the top
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Use thread_view for styled message display
    render_thread(frame, inner, app, recent_count);
}

/// Render the footer with key binding hints and log path.
fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let key_hints = "j/k:scroll  Tab:select  Enter:toggle  {/}:iteration  c:conversation  t:thinking  p:pause  Ctrl+C:quit";

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

/// Render a centered pause overlay.
fn render_pause_overlay(frame: &mut Frame, area: Rect) {
    let message = "PAUSED - press p to resume";
    let width = message.len() as u16 + 4;
    let height = 3;

    let popup_area = Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(height)) / 2,
        width: width.min(area.width),
        height: height.min(area.height),
    };

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let text = Paragraph::new(message)
        .alignment(Alignment::Center)
        .block(block);

    frame.render_widget(text, popup_area);
}
