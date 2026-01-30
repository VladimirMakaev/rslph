//! Built-in eval project registry.
//!
//! Embeds eval projects from the `evals/` directory at compile time using include_dir.
//! Projects include prompts (visible to agents) and hidden test data.

use include_dir::{include_dir, Dir};
use std::path::Path;

/// Embedded evals directory containing all built-in projects.
static EVALS_DIR: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/evals");

/// Get a project by name.
///
/// Returns the project directory if it exists.
pub fn get_project(name: &str) -> Option<&'static Dir<'static>> {
    EVALS_DIR.get_dir(name)
}

/// List all available built-in projects.
///
/// Returns a vector of project names.
pub fn list_projects() -> Vec<&'static str> {
    EVALS_DIR
        .dirs()
        .filter_map(|d| d.path().file_name())
        .filter_map(|name| name.to_str())
        .collect()
}

/// Check if a project name refers to a built-in project.
pub fn is_builtin(name: &str) -> bool {
    get_project(name).is_some()
}

/// Extract project files to a destination directory.
///
/// Extracts all files EXCEPT tests.jsonl, which contains hidden test cases.
/// This ensures agents can see the prompt but not the test data.
pub fn extract_project_files(project: &Dir<'static>, dest: &Path) -> std::io::Result<()> {
    extract_dir_recursive(project, dest)
}

/// Recursively extract directory contents, excluding tests.jsonl.
fn extract_dir_recursive(dir: &Dir<'static>, dest: &Path) -> std::io::Result<()> {
    // Create destination directory if it doesn't exist
    std::fs::create_dir_all(dest)?;

    // Extract files (excluding tests.jsonl)
    for file in dir.files() {
        if let Some(name) = file.path().file_name() {
            if name == "tests.jsonl" {
                continue; // Skip test data file
            }
        }
        let file_dest = dest.join(file.path().file_name().unwrap_or_default());
        std::fs::write(&file_dest, file.contents())?;
    }

    // Recursively extract subdirectories
    for subdir in dir.dirs() {
        let subdir_name = subdir.path().file_name().unwrap_or_default();
        let subdir_dest = dest.join(subdir_name);
        extract_dir_recursive(subdir, &subdir_dest)?;
    }

    Ok(())
}

/// Get the test data (tests.jsonl) for a project.
///
/// Returns the contents of tests.jsonl if it exists.
pub fn get_test_data(project: &Dir<'static>) -> Option<&'static str> {
    // File paths in include_dir include the project directory prefix
    let test_path = project.path().join("tests.jsonl");
    project.get_file(test_path).and_then(|f| f.contents_utf8())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_projects_contains_calculator() {
        let projects = list_projects();
        assert!(
            projects.contains(&"calculator"),
            "Expected 'calculator' in project list, got: {:?}",
            projects
        );
    }

    #[test]
    fn test_get_project_calculator() {
        let project = get_project("calculator");
        assert!(project.is_some(), "Calculator project should exist");
    }

    #[test]
    fn test_get_test_data_has_lines() {
        let project = get_project("calculator").expect("calculator project");
        let test_data = get_test_data(project);
        assert!(test_data.is_some(), "Test data should exist");

        let lines: Vec<_> = test_data.unwrap().lines().collect();
        assert!(
            lines.len() >= 10,
            "Expected at least 10 test lines, got {}",
            lines.len()
        );
    }

    #[test]
    fn test_is_builtin() {
        assert!(is_builtin("calculator"));
        assert!(is_builtin("fizzbuzz"));
        assert!(!is_builtin("nonexistent"));
    }

    #[test]
    fn test_list_projects_contains_fizzbuzz() {
        let projects = list_projects();
        assert!(
            projects.contains(&"fizzbuzz"),
            "Expected 'fizzbuzz' in project list, got: {:?}",
            projects
        );
    }

    #[test]
    fn test_list_projects_contains_both() {
        let projects = list_projects();
        assert!(
            projects.contains(&"calculator") && projects.contains(&"fizzbuzz"),
            "Expected both 'calculator' and 'fizzbuzz' in project list, got: {:?}",
            projects
        );
    }

    #[test]
    fn test_get_fizzbuzz_test_data() {
        let project = get_project("fizzbuzz").expect("fizzbuzz project");
        let test_data = get_test_data(project);
        assert!(test_data.is_some(), "Fizzbuzz test data should exist");

        let lines: Vec<_> = test_data.unwrap().lines().collect();
        assert!(
            lines.len() >= 8,
            "Expected at least 8 test lines for fizzbuzz, got {}",
            lines.len()
        );
    }

    #[test]
    fn test_extract_excludes_tests() {
        let project = get_project("calculator").expect("calculator project");
        let temp = tempfile::TempDir::new().expect("temp dir");

        extract_project_files(project, temp.path()).expect("extract");

        // prompt.txt should exist
        assert!(temp.path().join("prompt.txt").exists());

        // tests.jsonl should NOT exist
        assert!(
            !temp.path().join("tests.jsonl").exists(),
            "tests.jsonl should be excluded"
        );
    }
}
