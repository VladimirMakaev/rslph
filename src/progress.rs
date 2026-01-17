use serde::{Deserialize, Serialize};

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
}

#[cfg(test)]
mod tests {
    use super::*;

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
                        Task { description: "Task 1".to_string(), completed: true },
                        Task { description: "Task 2".to_string(), completed: false },
                    ],
                },
                TaskPhase {
                    name: "Phase 2".to_string(),
                    tasks: vec![
                        Task { description: "Task 3".to_string(), completed: false },
                    ],
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
            tasks: vec![
                TaskPhase {
                    name: "Phase 1".to_string(),
                    tasks: vec![
                        Task { description: "Task 1".to_string(), completed: true },
                        Task { description: "Task 2".to_string(), completed: false },
                    ],
                },
            ],
            ..Default::default()
        };

        let (phase, task) = pf.next_task().expect("Should have next task");
        assert_eq!(phase, "Phase 1");
        assert_eq!(task.description, "Task 2");
    }
}
