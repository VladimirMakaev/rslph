use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader, Lines};
use tokio::process::{Child, ChildStderr, ChildStdout, Command};

use super::OutputLine;

pub struct ClaudeRunner {
    child: Child,
    stdout: Lines<BufReader<ChildStdout>>,
    stderr: Lines<BufReader<ChildStderr>>,
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
        })
    }

    /// Read next line from either stdout or stderr (cancellation-safe).
    ///
    /// Returns `Some(OutputLine)` for each line, or `None` when both streams
    /// are exhausted (process has finished and closed both streams).
    pub async fn next_output(&mut self) -> Option<OutputLine> {
        // Track which streams have reached EOF
        let mut stdout_done = false;
        let mut stderr_done = false;

        loop {
            if stdout_done && stderr_done {
                return None;
            }

            tokio::select! {
                result = self.stdout.next_line(), if !stdout_done => {
                    match result {
                        Ok(Some(line)) => return Some(OutputLine::Stdout(line)),
                        Ok(None) => {
                            stdout_done = true;
                            // Continue to try stderr
                        }
                        Err(_) => {
                            stdout_done = true;
                            // Continue to try stderr
                        }
                    }
                }
                result = self.stderr.next_line(), if !stderr_done => {
                    match result {
                        Ok(Some(line)) => return Some(OutputLine::Stderr(line)),
                        Ok(None) => {
                            stderr_done = true;
                            // Continue to try stdout
                        }
                        Err(_) => {
                            stderr_done = true;
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
