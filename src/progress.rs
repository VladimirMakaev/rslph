#![allow(dead_code)] // TODO: Progress parsing will be used in a future plan

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::error::RslphError;

/// Complete progress file structure (PROG-01 through PROG-07)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProgressFile {
    /// Plan name/title
    pub name: String,

    /// Current status - "In Progress" or contains "RALPH_DONE" (PROG-01)
    pub status: String,

    /// Analysis/research section content (PROG-02)
    pub analysis: String,

    /// Task list organized by phases (PROG-03)
    pub tasks: Vec<TaskPhase>,

    /// Testing strategy section (PROG-04)
    pub testing_strategy: String,

    /// Tasks completed in current iteration (PROG-05)
    pub completed_this_iteration: Vec<String>,

    /// Recent attempts for failure memory (PROG-06)
    pub recent_attempts: Vec<Attempt>,

    /// Full iteration log history (PROG-07)
    pub iteration_log: Vec<IterationEntry>,
}

/// A phase containing related tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPhase {
    pub name: String,
    pub tasks: Vec<Task>,
}

/// Individual task with completion state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub description: String,
    pub completed: bool,
}

/// Record of an iteration attempt (PROG-06)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attempt {
    pub iteration: u32,
    pub tried: String,
    pub result: String,
    pub next: Option<String>,
}

/// Log entry for iteration history (PROG-07)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IterationEntry {
    pub iteration: u32,
    pub started: String,
    pub duration: String,
    pub tasks_completed: u32,
    pub notes: String,
}

impl ProgressFile {
    /// Check if progress file indicates completion (PROG-01)
    pub fn is_done(&self) -> bool {
        self.status.contains("RALPH_DONE")
    }

    /// Count total tasks
    pub fn total_tasks(&self) -> usize {
        self.tasks.iter().map(|p| p.tasks.len()).sum()
    }

    /// Count completed tasks
    pub fn completed_tasks(&self) -> usize {
        self.tasks.iter()
            .flat_map(|p| &p.tasks)
            .filter(|t| t.completed)
            .count()
    }

    /// Get next incomplete task
    pub fn next_task(&self) -> Option<(&str, &Task)> {
        for phase in &self.tasks {
            for task in &phase.tasks {
                if !task.completed {
                    return Some((&phase.name, task));
                }
            }
        }
        None
    }

    /// Parse markdown content into ProgressFile
    pub fn parse(content: &str) -> Result<Self, RslphError> {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_TASKLISTS);
        options.insert(Options::ENABLE_TABLES);

        let parser = Parser::new_ext(content, options);

        let mut pf = ProgressFile::default();
        let mut current_h2 = String::new();
        let mut current_h3 = String::new();
        let mut heading_level: Option<HeadingLevel> = None;
        let mut heading_text = String::new();
        let mut section_text = String::new();
        let mut current_task_checked: Option<bool> = None;
        let mut current_phase_tasks: Vec<Task> = Vec::new();
        let mut in_table_cell = false;
        let mut table_row: Vec<String> = Vec::new();
        let mut cell_text = String::new();
        let mut in_list_item = false;
        let mut list_item_text = String::new();

        for event in parser {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    // Flush previous section
                    Self::flush_section(
                        &mut pf,
                        &current_h2,
                        &section_text,
                        &current_h3,
                        &mut current_phase_tasks,
                    );
                    section_text.clear();
                    heading_level = Some(level);
                    heading_text.clear();
                }
                Event::End(TagEnd::Heading(_)) => {
                    match heading_level {
                        Some(HeadingLevel::H1) => {
                            pf.name = heading_text
                                .trim()
                                .trim_start_matches("Progress:")
                                .trim()
                                .to_string();
                        }
                        Some(HeadingLevel::H2) => {
                            current_h2 = heading_text.trim().to_string();
                            current_h3.clear();
                        }
                        Some(HeadingLevel::H3) => {
                            // Save previous phase if we're in Tasks section
                            if current_h2 == "Tasks"
                                && !current_h3.is_empty()
                                && !current_phase_tasks.is_empty()
                            {
                                pf.tasks.push(TaskPhase {
                                    name: current_h3.clone(),
                                    tasks: std::mem::take(&mut current_phase_tasks),
                                });
                            }
                            current_h3 = heading_text.trim().to_string();
                        }
                        _ => {}
                    }
                    heading_level = None;
                }
                Event::Text(text) => {
                    if heading_level.is_some() {
                        heading_text.push_str(&text);
                    } else if current_task_checked.is_some() {
                        // This is task description following TaskListMarker
                        // Check this BEFORE in_list_item since task items are also list items
                        let checked = current_task_checked.take().unwrap();
                        let task = Task {
                            description: text.trim().to_string(),
                            completed: checked,
                        };
                        if current_h2 == "Tasks" {
                            current_phase_tasks.push(task);
                        } else if current_h2 == "Completed This Iteration" {
                            pf.completed_this_iteration.push(text.trim().to_string());
                        }
                    } else if in_list_item {
                        list_item_text.push_str(&text);
                    } else if in_table_cell {
                        cell_text.push_str(&text);
                    } else {
                        section_text.push_str(&text);
                        section_text.push('\n');
                    }
                }
                Event::TaskListMarker(checked) => {
                    current_task_checked = Some(checked);
                }
                Event::Start(Tag::Item) => {
                    in_list_item = true;
                    list_item_text.clear();
                }
                Event::End(TagEnd::Item) => {
                    in_list_item = false;
                    // Handle list items in Recent Attempts section
                    if current_h2 == "Recent Attempts" && !list_item_text.is_empty() {
                        let text = list_item_text.trim();
                        if let Some(iteration_num) = current_h3.strip_prefix("Iteration ") {
                            if let Ok(iteration) = iteration_num.trim().parse::<u32>() {
                                // Find or create the attempt for this iteration
                                let attempt = pf
                                    .recent_attempts
                                    .iter_mut()
                                    .find(|a| a.iteration == iteration);
                                if let Some(attempt) = attempt {
                                    if let Some(tried) = text.strip_prefix("Tried:") {
                                        attempt.tried = tried.trim().to_string();
                                    } else if let Some(result) = text.strip_prefix("Result:") {
                                        attempt.result = result.trim().to_string();
                                    } else if let Some(next) = text.strip_prefix("Next:") {
                                        attempt.next = Some(next.trim().to_string());
                                    }
                                } else {
                                    // Create new attempt
                                    let mut new_attempt = Attempt {
                                        iteration,
                                        tried: String::new(),
                                        result: String::new(),
                                        next: None,
                                    };
                                    if let Some(tried) = text.strip_prefix("Tried:") {
                                        new_attempt.tried = tried.trim().to_string();
                                    } else if let Some(result) = text.strip_prefix("Result:") {
                                        new_attempt.result = result.trim().to_string();
                                    } else if let Some(next) = text.strip_prefix("Next:") {
                                        new_attempt.next = Some(next.trim().to_string());
                                    }
                                    pf.recent_attempts.push(new_attempt);
                                }
                            }
                        }
                    } else if !list_item_text.is_empty() {
                        // For other sections (like Testing Strategy), append list items to section text
                        section_text.push_str("- ");
                        section_text.push_str(&list_item_text);
                        section_text.push('\n');
                    }
                    list_item_text.clear();
                }
                Event::Start(Tag::Table(_)) | Event::End(TagEnd::Table) => {
                    // Table boundaries handled via cell/row events
                }
                Event::End(TagEnd::TableHead) => {
                    // Clear header row, we don't need it
                    table_row.clear();
                }
                Event::Start(Tag::TableCell) => {
                    in_table_cell = true;
                    cell_text.clear();
                }
                Event::End(TagEnd::TableCell) => {
                    in_table_cell = false;
                    table_row.push(cell_text.trim().to_string());
                }
                Event::End(TagEnd::TableRow) => {
                    // Parse iteration log row
                    if current_h2 == "Iteration Log" && table_row.len() >= 5 {
                        if let Ok(iteration) = table_row[0].parse::<u32>() {
                            pf.iteration_log.push(IterationEntry {
                                iteration,
                                started: table_row.get(1).cloned().unwrap_or_default(),
                                duration: table_row.get(2).cloned().unwrap_or_default(),
                                tasks_completed: table_row
                                    .get(3)
                                    .and_then(|s| s.parse().ok())
                                    .unwrap_or(0),
                                notes: table_row.get(4).cloned().unwrap_or_default(),
                            });
                        }
                    }
                    table_row.clear();
                }
                Event::SoftBreak | Event::HardBreak => {
                    if in_list_item {
                        list_item_text.push('\n');
                    } else {
                        section_text.push('\n');
                    }
                }
                _ => {}
            }
        }

        // Flush final section
        Self::flush_section(
            &mut pf,
            &current_h2,
            &section_text,
            &current_h3,
            &mut current_phase_tasks,
        );

        Ok(pf)
    }

    fn flush_section(
        pf: &mut ProgressFile,
        h2: &str,
        text: &str,
        h3: &str,
        phase_tasks: &mut Vec<Task>,
    ) {
        let text = text.trim();
        match h2 {
            "Status" => pf.status = text.to_string(),
            "Analysis" => pf.analysis = text.to_string(),
            "Testing Strategy" => pf.testing_strategy = text.to_string(),
            "Tasks" => {
                if !h3.is_empty() && !phase_tasks.is_empty() {
                    pf.tasks.push(TaskPhase {
                        name: h3.to_string(),
                        tasks: std::mem::take(phase_tasks),
                    });
                }
            }
            _ => {}
        }
    }

    /// Generate markdown representation
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        // Title
        md.push_str(&format!("# Progress: {}\n\n", self.name));

        // Status (PROG-01)
        md.push_str("## Status\n\n");
        md.push_str(&self.status);
        md.push_str("\n\n");

        // Analysis (PROG-02)
        md.push_str("## Analysis\n\n");
        md.push_str(&self.analysis);
        md.push_str("\n\n");

        // Tasks (PROG-03)
        md.push_str("## Tasks\n\n");
        for phase in &self.tasks {
            md.push_str(&format!("### {}\n\n", phase.name));
            for task in &phase.tasks {
                let checkbox = if task.completed { "[x]" } else { "[ ]" };
                md.push_str(&format!("- {} {}\n", checkbox, task.description));
            }
            md.push_str("\n");
        }

        // Testing Strategy (PROG-04)
        md.push_str("## Testing Strategy\n\n");
        md.push_str(&self.testing_strategy);
        md.push_str("\n\n");

        // Completed This Iteration (PROG-05)
        md.push_str("## Completed This Iteration\n\n");
        for item in &self.completed_this_iteration {
            md.push_str(&format!("- [x] {}\n", item));
        }
        md.push_str("\n");

        // Recent Attempts (PROG-06)
        md.push_str("## Recent Attempts\n\n");
        for attempt in &self.recent_attempts {
            md.push_str(&format!("### Iteration {}\n\n", attempt.iteration));
            md.push_str(&format!("- Tried: {}\n", attempt.tried));
            md.push_str(&format!("- Result: {}\n", attempt.result));
            if let Some(next) = &attempt.next {
                md.push_str(&format!("- Next: {}\n", next));
            }
            md.push_str("\n");
        }

        // Iteration Log (PROG-07)
        md.push_str("## Iteration Log\n\n");
        md.push_str("| Iteration | Started | Duration | Tasks Completed | Notes |\n");
        md.push_str("|-----------|---------|----------|-----------------|-------|\n");
        for entry in &self.iteration_log {
            md.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                entry.iteration, entry.started, entry.duration, entry.tasks_completed, entry.notes
            ));
        }

        md
    }

    /// Write progress file atomically (crash-safe)
    /// Uses temp file + rename pattern for durability
    pub fn write(&self, path: &std::path::Path) -> Result<(), RslphError> {
        use atomicwrites::{AllowOverwrite, AtomicFile};
        use std::io::Write;

        let content = self.to_markdown();
        let af = AtomicFile::new(path, AllowOverwrite);

        af.write(|f| f.write_all(content.as_bytes()))
            .map_err(|e| RslphError::Io(e.into()))?;

        Ok(())
    }

    /// Load progress file from disk
    pub fn load(path: &std::path::Path) -> Result<Self, RslphError> {
        let content = std::fs::read_to_string(path)?;
        Self::parse(&content)
    }

    /// Mark a task as completed
    pub fn complete_task(&mut self, phase_name: &str, task_description: &str) -> bool {
        for phase in &mut self.tasks {
            if phase.name == phase_name {
                for task in &mut phase.tasks {
                    if task.description == task_description && !task.completed {
                        task.completed = true;
                        self.completed_this_iteration
                            .push(task_description.to_string());
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Add an attempt record
    pub fn add_attempt(&mut self, iteration: u32, tried: &str, result: &str, next: Option<&str>) {
        self.recent_attempts.push(Attempt {
            iteration,
            tried: tried.to_string(),
            result: result.to_string(),
            next: next.map(String::from),
        });
    }

    /// Add iteration log entry
    pub fn log_iteration(
        &mut self,
        iteration: u32,
        started: &str,
        duration: &str,
        tasks_completed: u32,
        notes: &str,
    ) {
        self.iteration_log.push(IterationEntry {
            iteration,
            started: started.to_string(),
            duration: duration.to_string(),
            tasks_completed,
            notes: notes.to_string(),
        });
    }

    /// Clear completed this iteration (for next iteration)
    pub fn clear_iteration_completed(&mut self) {
        self.completed_this_iteration.clear();
    }

    /// Mark as done
    pub fn mark_done(&mut self, message: &str) {
        self.status = format!("RALPH_DONE - {}", message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_PROGRESS: &str = r#"# Progress: Test Plan

## Status

In Progress

## Analysis

Some analysis notes here.

## Tasks

### Phase 1: Foundation

- [x] Task 1 completed
- [ ] Task 2 pending

### Phase 2: Core

- [ ] Task 3 pending

## Testing Strategy

- Unit tests for core logic
- Integration tests for API

## Completed This Iteration

- [x] Task 1 completed

## Recent Attempts

### Iteration 1

- Tried: Initial setup
- Result: Success

## Iteration Log

| Iteration | Started | Duration | Tasks Completed | Notes |
|-----------|---------|----------|-----------------|-------|
| 1 | 2026-01-17 10:00 | 5m 30s | 1 | Initial run |
"#;

    #[test]
    fn test_is_done() {
        let mut pf = ProgressFile::default();
        assert!(!pf.is_done());

        pf.status = "RALPH_DONE - completed successfully".to_string();
        assert!(pf.is_done());
    }

    #[test]
    fn test_task_counting() {
        let pf = ProgressFile {
            tasks: vec![
                TaskPhase {
                    name: "Phase 1".to_string(),
                    tasks: vec![
                        Task {
                            description: "Task 1".to_string(),
                            completed: true,
                        },
                        Task {
                            description: "Task 2".to_string(),
                            completed: false,
                        },
                    ],
                },
                TaskPhase {
                    name: "Phase 2".to_string(),
                    tasks: vec![Task {
                        description: "Task 3".to_string(),
                        completed: false,
                    }],
                },
            ],
            ..Default::default()
        };

        assert_eq!(pf.total_tasks(), 3);
        assert_eq!(pf.completed_tasks(), 1);
    }

    #[test]
    fn test_next_task() {
        let pf = ProgressFile {
            tasks: vec![TaskPhase {
                name: "Phase 1".to_string(),
                tasks: vec![
                    Task {
                        description: "Task 1".to_string(),
                        completed: true,
                    },
                    Task {
                        description: "Task 2".to_string(),
                        completed: false,
                    },
                ],
            }],
            ..Default::default()
        };

        let (phase, task) = pf.next_task().expect("Should have next task");
        assert_eq!(phase, "Phase 1");
        assert_eq!(task.description, "Task 2");
    }

    #[test]
    fn test_parse_basic_sections() {
        let pf = ProgressFile::parse(SAMPLE_PROGRESS).expect("Should parse");

        assert_eq!(pf.name, "Test Plan");
        assert_eq!(pf.status, "In Progress");
        assert!(pf.analysis.contains("Some analysis"));
        assert!(pf.testing_strategy.contains("Unit tests"));
    }

    #[test]
    fn test_parse_tasks() {
        let pf = ProgressFile::parse(SAMPLE_PROGRESS).expect("Should parse");

        assert_eq!(pf.tasks.len(), 2);
        assert_eq!(pf.tasks[0].name, "Phase 1: Foundation");
        assert_eq!(pf.tasks[0].tasks.len(), 2);
        assert!(pf.tasks[0].tasks[0].completed);
        assert!(!pf.tasks[0].tasks[1].completed);
    }

    #[test]
    fn test_parse_iteration_log() {
        let pf = ProgressFile::parse(SAMPLE_PROGRESS).expect("Should parse");

        assert_eq!(pf.iteration_log.len(), 1);
        assert_eq!(pf.iteration_log[0].iteration, 1);
        assert_eq!(pf.iteration_log[0].tasks_completed, 1);
    }

    #[test]
    fn test_roundtrip() {
        let original = ProgressFile::parse(SAMPLE_PROGRESS).expect("Should parse");
        let markdown = original.to_markdown();
        let reparsed = ProgressFile::parse(&markdown).expect("Should reparse");

        assert_eq!(original.name, reparsed.name);
        assert_eq!(original.status, reparsed.status);
        assert_eq!(original.tasks.len(), reparsed.tasks.len());
    }

    #[test]
    fn test_atomic_write() {
        let dir = tempfile::tempdir().expect("Should create temp dir");
        let path = dir.path().join("progress.md");

        let mut pf = ProgressFile {
            name: "Test".to_string(),
            status: "In Progress".to_string(),
            tasks: vec![TaskPhase {
                name: "Phase 1".to_string(),
                tasks: vec![Task {
                    description: "Task 1".to_string(),
                    completed: false,
                }],
            }],
            ..Default::default()
        };

        // Write
        pf.write(&path).expect("Should write");

        // Read back
        let loaded = ProgressFile::load(&path).expect("Should load");
        assert_eq!(loaded.name, "Test");
        assert_eq!(loaded.tasks.len(), 1);

        // Modify and write again
        pf.complete_task("Phase 1", "Task 1");
        pf.write(&path).expect("Should write again");

        let reloaded = ProgressFile::load(&path).expect("Should reload");
        assert!(reloaded.tasks[0].tasks[0].completed);
    }

    #[test]
    fn test_complete_task() {
        let mut pf = ProgressFile {
            tasks: vec![TaskPhase {
                name: "Phase 1".to_string(),
                tasks: vec![
                    Task {
                        description: "Task A".to_string(),
                        completed: false,
                    },
                    Task {
                        description: "Task B".to_string(),
                        completed: false,
                    },
                ],
            }],
            ..Default::default()
        };

        assert!(pf.complete_task("Phase 1", "Task A"));
        assert!(pf.tasks[0].tasks[0].completed);
        assert!(!pf.tasks[0].tasks[1].completed);
        assert_eq!(pf.completed_this_iteration.len(), 1);
    }

    #[test]
    fn test_mark_done() {
        let mut pf = ProgressFile::default();
        assert!(!pf.is_done());

        pf.mark_done("All tasks complete");
        assert!(pf.is_done());
        assert!(pf.status.contains("RALPH_DONE"));
    }
}
