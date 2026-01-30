use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader, Lines};
use tokio::process::{Child, ChildStderr, ChildStdout, Command};
use tokio::sync::mpsc;
use tokio::time::{sleep, timeout};
use tokio_util::sync::CancellationToken;

use super::OutputLine;
use crate::error::RslphError;

/// Build complete args for Claude invocation.
///
/// Combines base_args, optional --dangerously-skip-permissions, and additional_args.
///
/// # Arguments
///
/// * `base_args` - Base arguments from config (e.g., ["--internet"])
/// * `additional_args` - Command-specific arguments (e.g., ["-p", "--verbose"])
/// * `no_dsp` - Whether to append --dangerously-skip-permissions
///
/// # Returns
///
/// Combined argument vector in order: base_args, dsp (if enabled), additional_args
pub fn build_claude_args(
    base_args: &[String],
    additional_args: &[String],
    no_dsp: bool,
) -> Vec<String> {
    let mut args = base_args.to_vec();
    if no_dsp {
        args.push("--dangerously-skip-permissions".to_string());
    }
    args.extend_from_slice(additional_args);
    args
}

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

    /// Gracefully terminate the process with SIGTERM, then SIGKILL after grace period.
    ///
    /// Sends SIGTERM to the process group (to catch any children), waits for the
    /// grace period, then sends SIGKILL if still running. Always reaps the child
    /// to prevent zombie processes.
    pub async fn terminate_gracefully(&mut self, grace_period: Duration) -> std::io::Result<()> {
        #[cfg(unix)]
        if let Some(id) = self.child.id() {
            use nix::sys::signal::{kill, Signal};
            use nix::unistd::Pid;

            // Send SIGTERM to process group (negative PID)
            let _ = kill(Pid::from_raw(-(id as i32)), Signal::SIGTERM);

            // Wait for graceful exit or timeout
            tokio::select! {
                result = self.child.wait() => {
                    return result.map(|_| ());
                }
                _ = sleep(grace_period) => {
                    // Grace period expired, force kill
                }
            }
        }

        // Force kill if still running
        self.child.kill().await?;
        // Reap to prevent zombie
        let _ = self.child.wait().await;
        Ok(())
    }

    /// Immediately kill the process and reap it.
    pub async fn kill(&mut self) -> std::io::Result<()> {
        self.child.kill().await?;
        // Reap to prevent zombie
        let _ = self.child.wait().await;
        Ok(())
    }

    /// Run to completion, respecting the cancellation token.
    ///
    /// Collects all output until the process finishes or cancellation is requested.
    /// On cancellation, gracefully terminates the process and returns Cancelled error.
    pub async fn run_to_completion(
        &mut self,
        cancel_token: CancellationToken,
    ) -> Result<Vec<OutputLine>, RslphError> {
        let mut output = Vec::new();

        loop {
            tokio::select! {
                biased;

                _ = cancel_token.cancelled() => {
                    // User requested cancellation (Ctrl+C)
                    self.terminate_gracefully(Duration::from_secs(5)).await
                        .map_err(|e| RslphError::Subprocess(e.to_string()))?;
                    return Err(RslphError::Cancelled);
                }

                line = self.next_output() => {
                    match line {
                        Some(l) => output.push(l),
                        None => break, // Process finished
                    }
                }
            }
        }

        // Reap child and check exit status
        let status = self.child.wait().await
            .map_err(|e| RslphError::Subprocess(e.to_string()))?;

        if !status.success() {
            let exit_code = status.code().unwrap_or(-1);
            let stderr_content: String = output
                .iter()
                .filter_map(|l| match l {
                    OutputLine::Stderr(s) => Some(s.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            return Err(RslphError::Subprocess(format!(
                "Process exited with code {}: {}",
                exit_code, stderr_content
            )));
        }

        Ok(output)
    }

    /// Run with a maximum duration, terminating if the timeout is exceeded.
    ///
    /// Wraps `run_to_completion` with a timeout. If the process takes longer than
    /// `max_duration`, it is gracefully terminated and a Timeout error is returned.
    pub async fn run_with_timeout(
        &mut self,
        max_duration: Duration,
        cancel_token: CancellationToken,
    ) -> Result<Vec<OutputLine>, RslphError> {
        match timeout(max_duration, self.run_to_completion(cancel_token)).await {
            Ok(result) => result,
            Err(_elapsed) => {
                // Timeout - terminate the process
                self.terminate_gracefully(Duration::from_secs(5))
                    .await
                    .map_err(|e| RslphError::Subprocess(e.to_string()))?;
                Err(RslphError::Timeout(max_duration.as_secs()))
            }
        }
    }

    /// Run subprocess while sending output lines to a channel.
    ///
    /// Unlike run_to_completion which collects all output, this streams
    /// each line to the provided sender for real-time display.
    ///
    /// # Arguments
    /// * `tx` - Unbounded sender for output lines
    /// * `cancel_token` - Token to request graceful cancellation
    ///
    /// # Returns
    /// * `Ok(())` - Process completed normally
    /// * `Err(RslphError::Cancelled)` - User cancelled
    /// * `Err(RslphError::Subprocess)` - Process error
    pub async fn run_with_channel(
        &mut self,
        tx: mpsc::UnboundedSender<OutputLine>,
        cancel_token: CancellationToken,
    ) -> Result<(), RslphError> {
        loop {
            tokio::select! {
                biased;

                _ = cancel_token.cancelled() => {
                    self.terminate_gracefully(Duration::from_secs(5)).await
                        .map_err(|e| RslphError::Subprocess(e.to_string()))?;
                    return Err(RslphError::Cancelled);
                }

                line = self.next_output() => {
                    match line {
                        Some(l) => {
                            // If receiver dropped, stop gracefully
                            if tx.send(l).is_err() {
                                break;
                            }
                        }
                        None => break, // Process finished
                    }
                }
            }
        }

        // Reap child and check exit status
        let status = self.child.wait().await
            .map_err(|e| RslphError::Subprocess(e.to_string()))?;

        if !status.success() {
            let exit_code = status.code().unwrap_or(-1);
            return Err(RslphError::Subprocess(format!(
                "Process exited with code {}",
                exit_code
            )));
        }

        Ok(())
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
        let mut runner =
            ClaudeRunner::spawn("/bin/sleep", &["0.1".to_string()], &PathBuf::from("/tmp"))
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
        let result = ClaudeRunner::spawn("/nonexistent/command", &[], &PathBuf::from("/tmp")).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_terminate_gracefully_on_sleeping_process() {
        let mut runner =
            ClaudeRunner::spawn("/bin/sleep", &["60".to_string()], &PathBuf::from("/tmp"))
                .await
                .expect("spawn should succeed");

        let pid = runner.id().expect("should have PID");

        // Terminate with 1 second grace period - should complete quickly
        let start = std::time::Instant::now();
        runner
            .terminate_gracefully(Duration::from_secs(1))
            .await
            .expect("terminate should succeed");
        let elapsed = start.elapsed();

        // Should complete within 2 seconds (grace period + some buffer)
        assert!(
            elapsed < Duration::from_secs(2),
            "Should terminate within 2 seconds, took {:?}",
            elapsed
        );

        // Verify process is no longer running by checking if we can signal it
        #[cfg(unix)]
        {
            use nix::sys::signal::{kill, Signal};
            use nix::unistd::Pid;

            let result = kill(Pid::from_raw(pid as i32), Signal::SIGCONT);
            assert!(
                result.is_err(),
                "Process should no longer be running after termination"
            );
        }
    }

    #[tokio::test]
    async fn test_run_with_timeout_expires() {
        let mut runner =
            ClaudeRunner::spawn("/bin/sleep", &["60".to_string()], &PathBuf::from("/tmp"))
                .await
                .expect("spawn should succeed");

        let token = CancellationToken::new();
        let result = runner
            .run_with_timeout(Duration::from_millis(100), token)
            .await;

        assert!(matches!(result, Err(RslphError::Timeout(_))));
    }

    #[tokio::test]
    async fn test_run_with_timeout_completes_before_timeout() {
        let mut runner =
            ClaudeRunner::spawn("/bin/echo", &["test".to_string()], &PathBuf::from("/tmp"))
                .await
                .expect("spawn should succeed");

        let token = CancellationToken::new();
        let result = runner
            .run_with_timeout(Duration::from_secs(10), token)
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.len(), 1);
        assert_eq!(output[0], OutputLine::Stdout("test".to_string()));
    }

    #[tokio::test]
    async fn test_cancellation_token_stops_execution() {
        let mut runner =
            ClaudeRunner::spawn("/bin/sleep", &["60".to_string()], &PathBuf::from("/tmp"))
                .await
                .expect("spawn should succeed");

        let token = CancellationToken::new();
        let token_clone = token.clone();

        // Cancel after 100ms
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            token_clone.cancel();
        });

        let result = runner.run_to_completion(token).await;
        assert!(matches!(result, Err(RslphError::Cancelled)));
    }

    #[tokio::test]
    async fn test_no_zombie_after_termination() {
        let mut runner = ClaudeRunner::spawn(
            "/bin/sh",
            &["-c".to_string(), "sleep 60".to_string()],
            &PathBuf::from("/tmp"),
        )
        .await
        .expect("spawn should succeed");

        let pid = runner.id().expect("should have PID");

        // Terminate the process
        runner
            .terminate_gracefully(Duration::from_secs(1))
            .await
            .expect("terminate should succeed");

        // Give system a moment to clean up
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Verify no zombie - process should not exist at all
        #[cfg(unix)]
        {
            use nix::sys::signal::{kill, Signal};
            use nix::unistd::Pid;

            // SIGCONT (0 on some systems) is used to check process existence without affecting it
            let result = kill(Pid::from_raw(pid as i32), Signal::SIGCONT);
            assert!(
                result.is_err(),
                "Process {} should not exist after termination (was properly reaped)",
                pid
            );
        }
    }

    #[tokio::test]
    async fn test_kill_immediately_terminates() {
        let mut runner =
            ClaudeRunner::spawn("/bin/sleep", &["60".to_string()], &PathBuf::from("/tmp"))
                .await
                .expect("spawn should succeed");

        let pid = runner.id().expect("should have PID");

        // Kill immediately
        let start = std::time::Instant::now();
        runner.kill().await.expect("kill should succeed");
        let elapsed = start.elapsed();

        // Should be very fast (no grace period)
        assert!(
            elapsed < Duration::from_millis(500),
            "Kill should be immediate, took {:?}",
            elapsed
        );

        // Verify process is gone
        #[cfg(unix)]
        {
            use nix::sys::signal::{kill, Signal};
            use nix::unistd::Pid;

            let result = kill(Pid::from_raw(pid as i32), Signal::SIGCONT);
            assert!(result.is_err(), "Process should not exist after kill");
        }
    }

    #[test]
    fn test_build_args_no_dsp_false() {
        let base = vec!["--internet".to_string()];
        let additional = vec!["--verbose".to_string()];
        let result = build_claude_args(&base, &additional, false);
        assert_eq!(result, vec!["--internet", "--verbose"]);
    }

    #[test]
    fn test_build_args_no_dsp_true() {
        let base = vec!["--internet".to_string()];
        let additional = vec!["--verbose".to_string()];
        let result = build_claude_args(&base, &additional, true);
        assert_eq!(
            result,
            vec!["--internet", "--dangerously-skip-permissions", "--verbose"]
        );
    }

    #[test]
    fn test_build_args_empty_base() {
        let base = vec![];
        let additional = vec!["--verbose".to_string()];
        let result = build_claude_args(&base, &additional, true);
        assert_eq!(result, vec!["--dangerously-skip-permissions", "--verbose"]);
    }

    #[test]
    fn test_build_args_empty_additional() {
        let base = vec!["--internet".to_string()];
        let additional = vec![];
        let result = build_claude_args(&base, &additional, true);
        assert_eq!(result, vec!["--internet", "--dangerously-skip-permissions"]);
    }
}
