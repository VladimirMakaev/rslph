//! Eval command handler.
//!
//! Orchestrates plan+build execution in isolated temporary directories
//! for controlled benchmarking.

use color_eyre::eyre::eyre;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

use crate::build::run_build_command;
use crate::build::tokens::{format_tokens, TokenUsage};
use crate::config::Config;
use crate::eval::{load_test_cases, TestRunner, TestResults};
use crate::planning::run_plan_command;
use crate::progress::ProgressFile;

use super::EvalResult;

/// Run the eval command (EVAL-01, EVAL-05).
///
/// Executes plan and build in an isolated temporary directory,
/// collecting metrics for tokens and timing.
///
/// # Arguments
///
/// * `project` - Path to project directory to evaluate
/// * `keep` - If true, preserve temp directory after completion
/// * `no_tui` - If true, disable TUI output
/// * `config` - Application configuration
/// * `cancel_token` - Token for graceful cancellation
///
/// # Returns
///
/// * `Ok(EvalResult)` - Eval completed with metrics
/// * `Err(e)` - Eval failed
pub async fn run_eval_command(
    project: String,
    keep: bool,
    no_tui: bool,
    config: &Config,
    cancel_token: CancellationToken,
) -> color_eyre::Result<EvalResult> {
    let start = Instant::now();

    // Step 1: Resolve project - check if built-in or external path
    let (is_builtin_project, project_source) = if crate::eval::is_builtin(&project) {
        (true, None)
    } else {
        let path = PathBuf::from(&project);
        if !path.exists() {
            return Err(eyre!(
                "Project '{}' is neither a built-in project nor a valid path",
                project
            ));
        }
        (false, Some(path))
    };

    // Step 2: Create isolated temp directory
    let project_name = if is_builtin_project {
        project.clone()
    } else {
        project_source
            .as_ref()
            .unwrap()
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("project")
            .to_string()
    };
    let workspace = TempDir::with_prefix(&format!("rslph-eval-{}-", project_name))?;
    let working_dir = workspace.path().to_path_buf();

    println!("Eval workspace: {}", working_dir.display());

    // Step 3: Copy/extract project files to temp directory
    if is_builtin_project {
        let proj = crate::eval::get_project(&project)
            .ok_or_else(|| eyre!("Built-in project not found: {}", project))?;
        crate::eval::extract_project_files(proj, &working_dir)?;
        println!("Extracted built-in project: {}", project);
    } else {
        copy_dir_recursive(project_source.as_ref().unwrap(), &working_dir)?;
        println!("Copied project files to workspace");
    }

    // Step 4: Initialize git in workspace (required for VCS tracking)
    init_git_repo(&working_dir)?;

    // Step 5: Detect starting prompt
    let prompt = detect_eval_prompt(&working_dir)?;
    println!("Detected prompt: {} chars", prompt.len());

    // Step 6: Run plan command and capture tokens
    println!("\n=== PLANNING PHASE ===\n");
    let timeout = Duration::from_secs(config.max_iterations as u64 * 600);
    let (progress_path, plan_tokens) = run_plan_command(
        &prompt,
        false, // not adaptive
        config,
        &working_dir,
        cancel_token.clone(),
        timeout,
    )
    .await?;

    println!(
        "Planning tokens: In: {} | Out: {} | CacheW: {} | CacheR: {}",
        format_tokens(plan_tokens.input_tokens),
        format_tokens(plan_tokens.output_tokens),
        format_tokens(plan_tokens.cache_creation_input_tokens),
        format_tokens(plan_tokens.cache_read_input_tokens),
    );

    // Step 7: Run build command and capture tokens
    println!("\n=== BUILD PHASE ===\n");
    let build_tokens = run_build_command(
        progress_path.clone(),
        false,          // not once
        false,          // not dry-run
        no_tui || true, // force no-tui for eval to get clean output
        config,
        cancel_token.clone(),
    )
    .await?;

    println!(
        "Build tokens: In: {} | Out: {} | CacheW: {} | CacheR: {}",
        format_tokens(build_tokens.input_tokens),
        format_tokens(build_tokens.output_tokens),
        format_tokens(build_tokens.cache_creation_input_tokens),
        format_tokens(build_tokens.cache_read_input_tokens),
    );

    // Step 8: Aggregate tokens from plan + build
    let total_tokens = TokenUsage {
        input_tokens: plan_tokens.input_tokens + build_tokens.input_tokens,
        output_tokens: plan_tokens.output_tokens + build_tokens.output_tokens,
        cache_creation_input_tokens: plan_tokens.cache_creation_input_tokens
            + build_tokens.cache_creation_input_tokens,
        cache_read_input_tokens: plan_tokens.cache_read_input_tokens
            + build_tokens.cache_read_input_tokens,
    };

    // Step 9: Collect metrics from progress file
    let progress = ProgressFile::load(&progress_path)?;
    let iterations = progress.iteration_log.len() as u32;

    let elapsed_secs = start.elapsed().as_secs_f64();

    // Step 10: Execute hidden tests for built-in projects (EVAL-02, EVAL-03)
    let test_results = if is_builtin_project {
        run_project_tests(&project, &working_dir)
    } else {
        None // External projects don't have hidden tests
    };

    // Step 11: Handle workspace cleanup
    let workspace_path = if keep {
        let preserved = workspace.keep();
        println!("\nWorkspace preserved at: {}", preserved.display());
        Some(preserved)
    } else {
        // TempDir will be dropped and cleaned up automatically
        drop(workspace);
        None
    };

    Ok(EvalResult {
        project,
        elapsed_secs,
        total_tokens,
        iterations,
        workspace_path,
        test_results,
    })
}

/// Copy directory contents recursively.
fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf) -> std::io::Result<()> {
    if !dst.exists() {
        std::fs::create_dir_all(dst)?;
    }

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            // Skip .git directories
            if entry.file_name() == ".git" {
                continue;
            }
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

/// Initialize a git repository in the workspace.
fn init_git_repo(working_dir: &PathBuf) -> std::io::Result<()> {
    use std::process::Command;

    Command::new("git")
        .args(["init"])
        .current_dir(working_dir)
        .output()?;

    Command::new("git")
        .args(["config", "user.email", "eval@rslph.local"])
        .current_dir(working_dir)
        .output()?;

    Command::new("git")
        .args(["config", "user.name", "Eval"])
        .current_dir(working_dir)
        .output()?;

    // Initial commit so we have a clean baseline
    Command::new("git")
        .args(["add", "."])
        .current_dir(working_dir)
        .output()?;

    Command::new("git")
        .args(["commit", "-m", "Initial eval state", "--allow-empty"])
        .current_dir(working_dir)
        .output()?;

    Ok(())
}

/// Detect the eval prompt from the project directory.
///
/// Looks for prompt.txt or README.md in the project root.
fn detect_eval_prompt(working_dir: &PathBuf) -> color_eyre::Result<String> {
    // Priority 1: prompt.txt
    let prompt_file = working_dir.join("prompt.txt");
    if prompt_file.exists() {
        return Ok(std::fs::read_to_string(prompt_file)?);
    }

    // Priority 2: README.md
    let readme_file = working_dir.join("README.md");
    if readme_file.exists() {
        return Ok(std::fs::read_to_string(readme_file)?);
    }

    // Priority 3: PROMPT.md
    let prompt_md = working_dir.join("PROMPT.md");
    if prompt_md.exists() {
        return Ok(std::fs::read_to_string(prompt_md)?);
    }

    Err(color_eyre::eyre::eyre!(
        "No prompt file found. Expected prompt.txt, README.md, or PROMPT.md in project root"
    ))
}

/// Run hidden tests for a built-in project.
///
/// Loads test cases from the embedded project and runs them against
/// the built program, displaying results.
fn run_project_tests(project: &str, working_dir: &PathBuf) -> Option<TestResults> {
    println!("\n=== TEST PHASE ===\n");

    // Get test data from embedded project
    let proj = crate::eval::get_project(project)?;
    let test_content = crate::eval::get_test_data(proj)?;
    let test_cases = load_test_cases(test_content);

    if test_cases.is_empty() {
        println!("Warning: No test cases found in project");
        return None;
    }

    // Find the built program
    let program_path = match find_built_program(working_dir) {
        Some(path) => path,
        None => {
            println!("Warning: Could not find built program to test");
            return None;
        }
    };

    println!("Testing program: {}", program_path.display());

    // Run tests
    let runner = TestRunner::new(program_path);
    let results = runner.run_tests(&test_cases);

    // Print summary
    println!(
        "Tests: {}/{} passed ({:.1}%)",
        results.passed,
        results.total,
        results.pass_rate()
    );

    // Print failed tests for debugging
    for case in &results.cases {
        if !case.passed {
            println!(
                "  FAIL: input='{}' expected='{}' got='{}'",
                case.input, case.expected, case.actual
            );
        }
    }

    Some(results)
}

/// Attempt to find a runnable program in the workspace.
///
/// Looks for common patterns: Rust target, Python script, shell script.
fn find_built_program(working_dir: &PathBuf) -> Option<PathBuf> {
    // Check for Rust binary in target/debug or target/release
    let cargo_toml = working_dir.join("Cargo.toml");
    if cargo_toml.exists() {
        // Parse Cargo.toml to find package name
        if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
            for line in content.lines() {
                if line.trim().starts_with("name = ") {
                    let name = line.split('"').nth(1)?;
                    let debug_path = working_dir.join("target/debug").join(name);
                    let release_path = working_dir.join("target/release").join(name);
                    if debug_path.exists() {
                        return Some(debug_path);
                    }
                    if release_path.exists() {
                        return Some(release_path);
                    }
                }
            }
        }
    }

    // Check for executable scripts
    for script_name in &["main.py", "main.sh", "calculator", "calc"] {
        let script_path = working_dir.join(script_name);
        if script_path.exists() {
            return Some(script_path);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_copy_dir_recursive() {
        let src_dir = TempDir::new().expect("src temp dir");
        let dst_dir = TempDir::new().expect("dst temp dir");

        // Create source structure
        std::fs::write(src_dir.path().join("file.txt"), "content").expect("write file");
        std::fs::create_dir(src_dir.path().join("subdir")).expect("create subdir");
        std::fs::write(src_dir.path().join("subdir/nested.txt"), "nested").expect("write nested");

        // Create .git directory that should be skipped
        std::fs::create_dir(src_dir.path().join(".git")).expect("create .git");
        std::fs::write(src_dir.path().join(".git/config"), "git stuff").expect("write git config");

        // Copy
        copy_dir_recursive(
            &src_dir.path().to_path_buf(),
            &dst_dir.path().to_path_buf(),
        )
        .expect("copy");

        // Verify
        assert!(dst_dir.path().join("file.txt").exists());
        assert!(dst_dir.path().join("subdir/nested.txt").exists());
        assert!(
            !dst_dir.path().join(".git").exists(),
            ".git should be skipped"
        );
    }

    #[test]
    fn test_detect_eval_prompt_priority() {
        let dir = TempDir::new().expect("temp dir");

        // No prompt file
        let result = detect_eval_prompt(&dir.path().to_path_buf());
        assert!(result.is_err());

        // Add README.md
        std::fs::write(dir.path().join("README.md"), "readme content").expect("write readme");
        let result = detect_eval_prompt(&dir.path().to_path_buf());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "readme content");

        // Add prompt.txt (should take priority)
        std::fs::write(dir.path().join("prompt.txt"), "prompt content").expect("write prompt");
        let result = detect_eval_prompt(&dir.path().to_path_buf());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "prompt content");
    }

    #[test]
    fn test_init_git_repo() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().to_path_buf();

        init_git_repo(&path).expect("init git");

        assert!(path.join(".git").exists(), ".git directory should exist");
    }

    #[test]
    fn test_detect_eval_prompt_with_prompt_md() {
        let dir = TempDir::new().expect("temp dir");

        // Add PROMPT.md (priority 3)
        std::fs::write(dir.path().join("PROMPT.md"), "prompt md content").expect("write prompt md");
        let result = detect_eval_prompt(&dir.path().to_path_buf());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "prompt md content");

        // Add README.md (should take priority over PROMPT.md)
        std::fs::write(dir.path().join("README.md"), "readme content").expect("write readme");
        let result = detect_eval_prompt(&dir.path().to_path_buf());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "readme content");
    }

    #[test]
    fn test_copy_dir_recursive_empty_src() {
        let src_dir = TempDir::new().expect("src temp dir");
        let dst_dir = TempDir::new().expect("dst temp dir");

        // Copy empty directory
        copy_dir_recursive(
            &src_dir.path().to_path_buf(),
            &dst_dir.path().to_path_buf(),
        )
        .expect("copy");

        // Verify destination exists and is empty
        assert!(dst_dir.path().exists());
    }

    #[test]
    fn test_find_built_program_cargo_project() {
        let dir = TempDir::new().expect("temp dir");

        // Create Cargo.toml
        std::fs::write(
            dir.path().join("Cargo.toml"),
            r#"[package]
name = "myapp"
version = "0.1.0"
"#,
        )
        .expect("write Cargo.toml");

        // Create fake binary
        std::fs::create_dir_all(dir.path().join("target/debug")).expect("create target/debug");
        std::fs::write(dir.path().join("target/debug/myapp"), "binary").expect("write binary");

        let result = find_built_program(&dir.path().to_path_buf());
        assert!(result.is_some(), "Should find Cargo binary");
        assert!(
            result.unwrap().ends_with("myapp"),
            "Path should end with binary name"
        );
    }

    #[test]
    fn test_find_built_program_release_build() {
        let dir = TempDir::new().expect("temp dir");

        // Create Cargo.toml
        std::fs::write(
            dir.path().join("Cargo.toml"),
            r#"[package]
name = "myrelease"
version = "0.1.0"
"#,
        )
        .expect("write Cargo.toml");

        // Create only release binary (no debug)
        std::fs::create_dir_all(dir.path().join("target/release")).expect("create target/release");
        std::fs::write(dir.path().join("target/release/myrelease"), "binary").expect("write binary");

        let result = find_built_program(&dir.path().to_path_buf());
        assert!(result.is_some(), "Should find release binary");
        assert!(
            result.unwrap().to_str().unwrap().contains("release"),
            "Path should contain 'release'"
        );
    }

    #[test]
    fn test_find_built_program_script() {
        let dir = TempDir::new().expect("temp dir");

        // Create main.py
        std::fs::write(dir.path().join("main.py"), "print('hello')").expect("write main.py");

        let result = find_built_program(&dir.path().to_path_buf());
        assert!(result.is_some(), "Should find Python script");
        assert!(
            result.unwrap().ends_with("main.py"),
            "Path should end with main.py"
        );
    }

    #[test]
    fn test_find_built_program_shell_script() {
        let dir = TempDir::new().expect("temp dir");

        // Create main.sh
        std::fs::write(dir.path().join("main.sh"), "#!/bin/bash\necho hello").expect("write main.sh");

        let result = find_built_program(&dir.path().to_path_buf());
        assert!(result.is_some(), "Should find shell script");
        assert!(
            result.unwrap().ends_with("main.sh"),
            "Path should end with main.sh"
        );
    }

    #[test]
    fn test_find_built_program_calculator_name() {
        let dir = TempDir::new().expect("temp dir");

        // Create "calculator" executable
        std::fs::write(dir.path().join("calculator"), "#!/bin/bash").expect("write calculator");

        let result = find_built_program(&dir.path().to_path_buf());
        assert!(result.is_some(), "Should find calculator");
        assert!(
            result.unwrap().ends_with("calculator"),
            "Path should end with calculator"
        );
    }

    #[test]
    fn test_find_built_program_no_match() {
        let dir = TempDir::new().expect("temp dir");

        // Create a random file that doesn't match any pattern
        std::fs::write(dir.path().join("random.txt"), "content").expect("write");

        let result = find_built_program(&dir.path().to_path_buf());
        assert!(result.is_none(), "Should not find any program");
    }

    #[test]
    fn test_find_built_program_cargo_debug_over_release() {
        let dir = TempDir::new().expect("temp dir");

        // Create Cargo.toml
        std::fs::write(
            dir.path().join("Cargo.toml"),
            r#"[package]
name = "myapp"
version = "0.1.0"
"#,
        )
        .expect("write Cargo.toml");

        // Create both debug and release binaries
        std::fs::create_dir_all(dir.path().join("target/debug")).expect("create target/debug");
        std::fs::create_dir_all(dir.path().join("target/release")).expect("create target/release");
        std::fs::write(dir.path().join("target/debug/myapp"), "debug").expect("write debug");
        std::fs::write(dir.path().join("target/release/myapp"), "release").expect("write release");

        let result = find_built_program(&dir.path().to_path_buf());
        assert!(result.is_some(), "Should find binary");
        // Debug should be preferred over release
        assert!(
            result.unwrap().to_str().unwrap().contains("debug"),
            "Debug build should be preferred"
        );
    }

    #[test]
    fn test_builtin_project_detection() {
        // Test that calculator is detected as built-in
        assert!(crate::eval::is_builtin("calculator"));
        assert!(!crate::eval::is_builtin("nonexistent"));
        assert!(!crate::eval::is_builtin("/some/path"));
    }
}
