//! Dashboard widget for parallel eval execution.
//!
//! Provides a multi-pane grid layout showing real-time progress of all
//! parallel trials across different prompt modes.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crossterm::event::{Event as CrosstermEvent, EventStream, KeyCode, KeyModifiers};
use futures::StreamExt;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::error::RslphError;
use crate::eval::{TestResults, TrialEvent, TrialEventKind};
use crate::prompts::PromptMode;
use crate::tui::terminal::{init_terminal, restore_terminal};

/// State for the parallel eval dashboard.
///
/// Tracks progress of all trials organized by (mode, trial_num) keys.
#[derive(Debug, Clone)]
pub struct DashboardState {
    /// Trial progress keyed by (mode, trial_num)
    pub trials: HashMap<(PromptMode, u32), TrialProgress>,
    /// Start time for elapsed calculation
    pub start_time: Instant,
    /// Whether all trials are complete
    pub all_complete: bool,
}

/// Progress information for a single trial.
#[derive(Debug, Clone)]
pub struct TrialProgress {
    /// The prompt mode being used
    pub mode: PromptMode,
    /// Trial number within this mode (1-indexed)
    pub trial_num: u32,
    /// Current status of the trial
    pub status: TrialStatus,
    /// Current iteration number (for building phase)
    pub current_iteration: u32,
    /// Maximum iterations allowed
    pub max_iterations: u32,
    /// Elapsed time in seconds
    pub elapsed_secs: f64,
    /// Pass rate (0.0 to 1.0) when complete
    pub pass_rate: Option<f64>,
    /// Error message if failed
    pub error: Option<String>,
}

/// Status of a trial execution.
#[derive(Debug, Clone, PartialEq)]
pub enum TrialStatus {
    /// Waiting to start
    Pending,
    /// Running planning phase
    Planning,
    /// Running build phase
    Building,
    /// Running test phase
    Testing,
    /// Successfully completed
    Complete,
    /// Failed with error
    Failed,
}

impl DashboardState {
    /// Create a new dashboard state for the given modes and trial count.
    pub fn new(modes: &[PromptMode], trials_per_mode: u32) -> Self {
        let mut trials = HashMap::new();
        for mode in modes {
            for trial_num in 1..=trials_per_mode {
                trials.insert(
                    (*mode, trial_num),
                    TrialProgress {
                        mode: *mode,
                        trial_num,
                        status: TrialStatus::Pending,
                        current_iteration: 0,
                        max_iterations: 0,
                        elapsed_secs: 0.0,
                        pass_rate: None,
                        error: None,
                    },
                );
            }
        }
        Self {
            trials,
            start_time: Instant::now(),
            all_complete: false,
        }
    }

    /// Update state based on a trial event.
    pub fn update(&mut self, event: &TrialEvent) {
        if let Some(trial) = self.trials.get_mut(&(event.mode, event.trial_num)) {
            match &event.event {
                TrialEventKind::Started => trial.status = TrialStatus::Pending,
                TrialEventKind::Planning => trial.status = TrialStatus::Planning,
                TrialEventKind::Building {
                    iteration,
                    max_iterations,
                } => {
                    trial.status = TrialStatus::Building;
                    trial.current_iteration = *iteration;
                    trial.max_iterations = *max_iterations;
                }
                TrialEventKind::Testing => trial.status = TrialStatus::Testing,
                TrialEventKind::Complete { result } => {
                    trial.status = TrialStatus::Complete;
                    // Extract pass_rate from test_results if available (0-100 scale to 0-1)
                    trial.pass_rate = result
                        .eval_result
                        .test_results
                        .as_ref()
                        .map(|tr: &TestResults| tr.pass_rate() / 100.0);
                    trial.elapsed_secs = result.eval_result.elapsed_secs;
                }
                TrialEventKind::Failed { error } => {
                    trial.status = TrialStatus::Failed;
                    trial.error = Some(error.clone());
                }
            }
        }

        // Check if all complete
        self.all_complete = self
            .trials
            .values()
            .all(|t| matches!(t.status, TrialStatus::Complete | TrialStatus::Failed));
    }

    /// Get total elapsed time since dashboard started.
    pub fn total_elapsed_secs(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }
}

/// Run the dashboard TUI for parallel eval execution.
///
/// This function manages the TUI lifecycle for the dashboard:
/// 1. Initializes the terminal
/// 2. Renders the dashboard with updates from trial events
/// 3. Handles keyboard input (q/Esc/Ctrl+C to quit)
/// 4. Restores the terminal on exit
///
/// # Arguments
///
/// * `modes` - List of prompt modes being evaluated
/// * `trials_per_mode` - Number of trials per mode
/// * `event_rx` - Receiver for trial events from parallel eval execution
/// * `cancel_token` - Token to signal cancellation (user quit or completion)
///
/// # Returns
///
/// * `Ok(())` on successful completion or user quit
/// * `Err(RslphError)` on terminal or I/O error
pub async fn run_dashboard_tui(
    modes: Vec<PromptMode>,
    trials_per_mode: u32,
    mut event_rx: mpsc::UnboundedReceiver<TrialEvent>,
    cancel_token: CancellationToken,
) -> Result<(), RslphError> {
    let mut terminal = init_terminal()
        .map_err(|e| RslphError::Subprocess(format!("Terminal init failed: {}", e)))?;

    let mut state = DashboardState::new(&modes, trials_per_mode);
    let mut event_stream = EventStream::new();

    // Render interval for responsive updates (30 FPS)
    let mut render_interval = tokio::time::interval(Duration::from_millis(33));

    loop {
        // Render current state
        terminal
            .draw(|frame| {
                render_dashboard(frame, frame.area(), &state);
            })
            .map_err(|e| RslphError::Subprocess(format!("Render failed: {}", e)))?;

        // Poll for events with timeout for responsive rendering
        tokio::select! {
            biased;

            // Check for cancellation
            _ = cancel_token.cancelled() => {
                break;
            }

            // Handle keyboard events
            maybe_key = event_stream.next() => {
                match maybe_key {
                    Some(Ok(CrosstermEvent::Key(key))) => {
                        // Check for Ctrl+C
                        if key.modifiers.contains(KeyModifiers::CONTROL) {
                            if let KeyCode::Char('c') = key.code {
                                cancel_token.cancel();
                                break;
                            }
                        }
                        // Check for q or Esc to quit
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => {
                                cancel_token.cancel();
                                break;
                            }
                            _ => {}
                        }
                    }
                    Some(Ok(CrosstermEvent::Resize(_, _))) => {
                        // Resize triggers a re-render on next loop
                    }
                    Some(Err(_)) | None => {
                        // Event stream error or ended
                    }
                    _ => {}
                }
            }

            // Handle trial events
            event = event_rx.recv() => {
                match event {
                    Some(trial_event) => {
                        state.update(&trial_event);
                        if state.all_complete {
                            // Keep displayed for a moment to show final state
                            // Re-render first
                            let _ = terminal.draw(|frame| {
                                render_dashboard(frame, frame.area(), &state);
                            });
                            tokio::time::sleep(Duration::from_secs(2)).await;
                            break;
                        }
                    }
                    None => {
                        // Channel closed - all trials done
                        break;
                    }
                }
            }

            // Periodic render for elapsed time updates
            _ = render_interval.tick() => {
                // Loop continues and re-renders
            }
        }
    }

    restore_terminal()
        .map_err(|e| RslphError::Subprocess(format!("Terminal restore failed: {}", e)))?;

    Ok(())
}

/// Render the dashboard to the frame.
///
/// Creates a grid layout with columns for each mode and rows for each trial.
pub fn render_dashboard(frame: &mut Frame, area: Rect, state: &DashboardState) {
    // Collect unique modes and sort for consistent ordering
    let mut modes: Vec<PromptMode> = state
        .trials
        .keys()
        .map(|(m, _)| *m)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    modes.sort_by_key(|m| format!("{:?}", m));

    let mode_count = modes.len();
    if mode_count == 0 {
        // Empty state - nothing to render
        let msg =
            Paragraph::new("No trials configured").style(Style::default().fg(Color::DarkGray));
        frame.render_widget(msg, area);
        return;
    }

    // Split into header and main content
    let [header_area, content_area] =
        Layout::vertical([Constraint::Length(2), Constraint::Min(0)]).areas(area);

    // Render header with elapsed time
    render_header(frame, header_area, state);

    // Create columns for each mode
    let col_constraints: Vec<Constraint> = (0..mode_count)
        .map(|_| Constraint::Percentage(100 / mode_count as u16))
        .collect();

    let columns = Layout::horizontal(col_constraints).split(content_area);

    for (col_idx, mode) in modes.iter().enumerate() {
        render_mode_column(frame, columns[col_idx], *mode, state);
    }
}

/// Render the header showing overall elapsed time.
fn render_header(frame: &mut Frame, area: Rect, state: &DashboardState) {
    let elapsed = state.total_elapsed_secs();
    let completed = state
        .trials
        .values()
        .filter(|t| matches!(t.status, TrialStatus::Complete | TrialStatus::Failed))
        .count();
    let total = state.trials.len();

    let status_char = if state.all_complete {
        "Done"
    } else {
        "Running"
    };
    let header_text = format!(
        "Parallel Eval [{status_char}] | {completed}/{total} trials | Elapsed: {elapsed:.1}s"
    );

    let header = Paragraph::new(header_text)
        .style(Style::default().add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(header, area);
}

/// Render a column for a single mode with all its trials.
fn render_mode_column(frame: &mut Frame, area: Rect, mode: PromptMode, state: &DashboardState) {
    // Get trials for this mode
    let mut trials: Vec<&TrialProgress> =
        state.trials.values().filter(|t| t.mode == mode).collect();
    trials.sort_by_key(|t| t.trial_num);

    // Header + trial rows
    let row_constraints: Vec<Constraint> = std::iter::once(Constraint::Length(2))
        .chain(trials.iter().map(|_| Constraint::Length(4)))
        .chain(std::iter::once(Constraint::Min(0))) // Filler
        .collect();

    let rows = Layout::vertical(row_constraints).split(area);

    // Render mode header
    let header = Paragraph::new(format!("{:?}", mode))
        .style(Style::default().add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(header, rows[0]);

    // Render each trial
    for (idx, trial) in trials.iter().enumerate() {
        render_trial_cell(frame, rows[idx + 1], trial);
    }
}

/// Render a single trial cell showing status and progress.
fn render_trial_cell(frame: &mut Frame, area: Rect, trial: &TrialProgress) {
    let status_color = match trial.status {
        TrialStatus::Pending => Color::DarkGray,
        TrialStatus::Planning => Color::Yellow,
        TrialStatus::Building => Color::Blue,
        TrialStatus::Testing => Color::Cyan,
        TrialStatus::Complete => Color::Green,
        TrialStatus::Failed => Color::Red,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(status_color))
        .title(format!("Trial {}", trial.trial_num));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Need at least 2 lines for content
    if inner.height < 2 {
        return;
    }

    let [status_area, progress_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(inner);

    // Status line
    let status_text = match &trial.status {
        TrialStatus::Pending => "Pending...".to_string(),
        TrialStatus::Planning => "Planning...".to_string(),
        TrialStatus::Building => {
            format!("Build {}/{}", trial.current_iteration, trial.max_iterations)
        }
        TrialStatus::Testing => "Testing...".to_string(),
        TrialStatus::Complete => format!("Done: {:.0}%", trial.pass_rate.unwrap_or(0.0) * 100.0),
        TrialStatus::Failed => "FAILED".to_string(),
    };
    let status = Paragraph::new(status_text).style(Style::default().fg(status_color));
    frame.render_widget(status, status_area);

    // Progress bar (only for building status)
    if trial.status == TrialStatus::Building && trial.max_iterations > 0 {
        let progress = trial.current_iteration as f64 / trial.max_iterations as f64;
        let gauge = Gauge::default()
            .ratio(progress.min(1.0))
            .gauge_style(Style::default().fg(Color::Blue));
        frame.render_widget(gauge, progress_area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_state_new() {
        let modes = vec![PromptMode::Basic, PromptMode::Gsd];
        let state = DashboardState::new(&modes, 2);

        assert_eq!(state.trials.len(), 4); // 2 modes x 2 trials
        assert!(!state.all_complete);

        // Check that all trials are pending
        for trial in state.trials.values() {
            assert_eq!(trial.status, TrialStatus::Pending);
        }
    }

    #[test]
    fn test_dashboard_state_update_started() {
        let modes = vec![PromptMode::Basic];
        let mut state = DashboardState::new(&modes, 1);

        let event = TrialEvent {
            mode: PromptMode::Basic,
            trial_num: 1,
            event: TrialEventKind::Started,
        };
        state.update(&event);

        let trial = state.trials.get(&(PromptMode::Basic, 1)).unwrap();
        assert_eq!(trial.status, TrialStatus::Pending);
    }

    #[test]
    fn test_dashboard_state_update_planning() {
        let modes = vec![PromptMode::Basic];
        let mut state = DashboardState::new(&modes, 1);

        let event = TrialEvent {
            mode: PromptMode::Basic,
            trial_num: 1,
            event: TrialEventKind::Planning,
        };
        state.update(&event);

        let trial = state.trials.get(&(PromptMode::Basic, 1)).unwrap();
        assert_eq!(trial.status, TrialStatus::Planning);
    }

    #[test]
    fn test_dashboard_state_update_building() {
        let modes = vec![PromptMode::Basic];
        let mut state = DashboardState::new(&modes, 1);

        let event = TrialEvent {
            mode: PromptMode::Basic,
            trial_num: 1,
            event: TrialEventKind::Building {
                iteration: 3,
                max_iterations: 10,
            },
        };
        state.update(&event);

        let trial = state.trials.get(&(PromptMode::Basic, 1)).unwrap();
        assert_eq!(trial.status, TrialStatus::Building);
        assert_eq!(trial.current_iteration, 3);
        assert_eq!(trial.max_iterations, 10);
    }

    #[test]
    fn test_dashboard_state_update_failed() {
        let modes = vec![PromptMode::Basic];
        let mut state = DashboardState::new(&modes, 1);

        let event = TrialEvent {
            mode: PromptMode::Basic,
            trial_num: 1,
            event: TrialEventKind::Failed {
                error: "Test error".to_string(),
            },
        };
        state.update(&event);

        let trial = state.trials.get(&(PromptMode::Basic, 1)).unwrap();
        assert_eq!(trial.status, TrialStatus::Failed);
        assert_eq!(trial.error, Some("Test error".to_string()));
        assert!(state.all_complete); // Single trial, now complete
    }

    #[test]
    fn test_dashboard_state_all_complete() {
        let modes = vec![PromptMode::Basic, PromptMode::Gsd];
        let mut state = DashboardState::new(&modes, 1);

        assert!(!state.all_complete);

        // Complete first trial
        let event1 = TrialEvent {
            mode: PromptMode::Basic,
            trial_num: 1,
            event: TrialEventKind::Failed {
                error: "Error".to_string(),
            },
        };
        state.update(&event1);
        assert!(!state.all_complete); // Still have Gsd pending

        // Complete second trial
        let event2 = TrialEvent {
            mode: PromptMode::Gsd,
            trial_num: 1,
            event: TrialEventKind::Failed {
                error: "Error".to_string(),
            },
        };
        state.update(&event2);
        assert!(state.all_complete); // All done
    }

    #[test]
    fn test_trial_status_variants() {
        // Ensure all variants are constructible
        let _ = TrialStatus::Pending;
        let _ = TrialStatus::Planning;
        let _ = TrialStatus::Building;
        let _ = TrialStatus::Testing;
        let _ = TrialStatus::Complete;
        let _ = TrialStatus::Failed;
    }
}
