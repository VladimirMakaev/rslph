use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader, Lines};
use tokio::process::{Child, ChildStderr, ChildStdout, Command};

use super::OutputLine;

pub struct ClaudeRunner {
    child: Child,
    stdout: Lines<BufReader<ChildStdout>>,
    stderr: Lines<BufReader<ChildStderr>>,
    stdout_done: bool,
    stderr_done: bool,
}

impl ClaudeRunner {
    /// Spawn Claude CLI (or any command) with piped stdout/stderr.
    ///
    /// The process is spawned in its own process group to prevent
    /// terminal signal inheritance (Ctrl+C won't kill it directly).
    pub async fn spawn(
        command_path: &str,
        args: &[String],
        working_dir: &Path,
    ) -> std::io::Result<Self> {
        let mut child = Command::new(command_path)
            .args(args)
            .current_dir(working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .process_group(0) // CRITICAL: Isolate from terminal signals
            .kill_on_drop(true) // Safety net
            .spawn()?;

        let stdout = child.stdout.take().expect("stdout configured as piped");
        let stderr = child.stderr.take().expect("stderr configured as piped");

        Ok(Self {
            child,
            stdout: BufReader::new(stdout).lines(),
            stderr: BufReader::new(stderr).lines(),
            stdout_done: false,
            stderr_done: false,
        })
    }

    /// Read next line from either stdout or stderr (cancellation-safe).
    ///
    /// Returns `Some(OutputLine)` for each line, or `None` when both streams
    /// are exhausted (process has finished and closed both streams).
    pub async fn next_output(&mut self) -> Option<OutputLine> {
        loop {
            if self.stdout_done && self.stderr_done {
                return None;
            }

            tokio::select! {
                result = self.stdout.next_line(), if !self.stdout_done => {
                    match result {
                        Ok(Some(line)) => return Some(OutputLine::Stdout(line)),
                        Ok(None) => {
                            self.stdout_done = true;
                            // Continue to try stderr
                        }
                        Err(_) => {
                            self.stdout_done = true;
                            // Continue to try stderr
                        }
                    }
                }
                result = self.stderr.next_line(), if !self.stderr_done => {
                    match result {
                        Ok(Some(line)) => return Some(OutputLine::Stderr(line)),
                        Ok(None) => {
                            self.stderr_done = true;
                            // Continue to try stdout
                        }
                        Err(_) => {
                            self.stderr_done = true;
                            // Continue to try stdout
                        }
                    }
                }
            }
        }
    }

    /// Get the process ID (for external monitoring).
    pub fn id(&self) -> Option<u32> {
        self.child.id()
    }

    /// Wait for the process to complete.
    pub async fn wait(&mut self) -> std::io::Result<std::process::ExitStatus> {
        self.child.wait().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_spawn_echo_command() {
        let mut runner = ClaudeRunner::spawn(
            "/bin/echo",
            &["hello".to_string(), "world".to_string()],
            &PathBuf::from("/tmp"),
        )
        .await
        .expect("spawn should succeed");

        let mut output = Vec::new();
        while let Some(line) = runner.next_output().await {
            output.push(line);
        }

        assert_eq!(output.len(), 1);
        assert_eq!(output[0], OutputLine::Stdout("hello world".to_string()));

        let status = runner.wait().await.expect("wait should succeed");
        assert!(status.success());
    }

    #[tokio::test]
    async fn test_spawn_stderr_output() {
        let mut runner = ClaudeRunner::spawn(
            "/bin/sh",
            &["-c".to_string(), "echo error >&2".to_string()],
            &PathBuf::from("/tmp"),
        )
        .await
        .expect("spawn should succeed");

        let mut output = Vec::new();
        while let Some(line) = runner.next_output().await {
            output.push(line);
        }

        assert_eq!(output.len(), 1);
        assert_eq!(output[0], OutputLine::Stderr("error".to_string()));
    }

    #[tokio::test]
    async fn test_concurrent_stdout_stderr() {
        let mut runner = ClaudeRunner::spawn(
            "/bin/sh",
            &[
                "-c".to_string(),
                "echo out; echo err >&2; echo out2".to_string(),
            ],
            &PathBuf::from("/tmp"),
        )
        .await
        .expect("spawn should succeed");

        let mut output = Vec::new();
        while let Some(line) = runner.next_output().await {
            output.push(line);
        }

        // Should receive all 3 lines (order may vary due to concurrency)
        assert_eq!(output.len(), 3);

        // Check we got both stdout and stderr lines
        let stdout_count = output
            .iter()
            .filter(|l| matches!(l, OutputLine::Stdout(_)))
            .count();
        let stderr_count = output
            .iter()
            .filter(|l| matches!(l, OutputLine::Stderr(_)))
            .count();

        assert_eq!(stdout_count, 2, "Should have 2 stdout lines");
        assert_eq!(stderr_count, 1, "Should have 1 stderr line");

        // Verify content (order-independent)
        let stdout_lines: Vec<_> = output
            .iter()
            .filter_map(|l| match l {
                OutputLine::Stdout(s) => Some(s.as_str()),
                _ => None,
            })
            .collect();
        assert!(stdout_lines.contains(&"out"));
        assert!(stdout_lines.contains(&"out2"));

        let stderr_lines: Vec<_> = output
            .iter()
            .filter_map(|l| match l {
                OutputLine::Stderr(s) => Some(s.as_str()),
                _ => None,
            })
            .collect();
        assert!(stderr_lines.contains(&"err"));
    }

    #[tokio::test]
    async fn test_process_id_available() {
        let mut runner = ClaudeRunner::spawn(
            "/bin/sleep",
            &["0.1".to_string()],
            &PathBuf::from("/tmp"),
        )
        .await
        .expect("spawn should succeed");

        // Process ID should be available
        assert!(runner.id().is_some());

        // Drain output and wait
        while runner.next_output().await.is_some() {}
        let status = runner.wait().await.expect("wait should succeed");
        assert!(status.success());
    }

    #[tokio::test]
    async fn test_nonexistent_command_fails() {
        let result = ClaudeRunner::spawn(
            "/nonexistent/command",
            &[],
            &PathBuf::from("/tmp"),
        )
        .await;

        assert!(result.is_err());
    }
}
