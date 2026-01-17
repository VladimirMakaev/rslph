//! Project stack detection for testing strategy.
//!
//! Detects programming language, test runner, type checker, and linter
//! by inspecting project manifest files.

use std::path::Path;

/// Detected project stack information.
#[derive(Debug, Clone, Default)]
pub struct DetectedStack {
    /// Primary programming language.
    pub language: Language,
    /// Framework (e.g., React, Next.js, Actix).
    pub framework: Option<String>,
    /// Test runner command (e.g., "cargo test", "jest").
    pub test_runner: Option<String>,
    /// Type checker (e.g., "rustc", "tsc", "mypy").
    pub type_checker: Option<String>,
    /// Linter (e.g., "clippy", "eslint", "ruff").
    pub linter: Option<String>,
}

/// Programming language of the project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    #[default]
    Unknown,
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Language::Rust => write!(f, "Rust"),
            Language::TypeScript => write!(f, "TypeScript"),
            Language::JavaScript => write!(f, "JavaScript"),
            Language::Python => write!(f, "Python"),
            Language::Go => write!(f, "Go"),
            Language::Unknown => write!(f, "Unknown"),
        }
    }
}

impl DetectedStack {
    /// Generate a human-readable summary for prompt injection.
    pub fn to_summary(&self) -> String {
        let mut parts = vec![format!("Language: {}", self.language)];

        if let Some(ref fw) = self.framework {
            parts.push(format!("Framework: {}", fw));
        }
        if let Some(ref tr) = self.test_runner {
            parts.push(format!("Test Runner: {}", tr));
        }
        if let Some(ref tc) = self.type_checker {
            parts.push(format!("Type Checker: {}", tc));
        }
        if let Some(ref l) = self.linter {
            parts.push(format!("Linter: {}", l));
        }

        parts.join("\n")
    }
}

/// Detect the project stack from manifest files.
///
/// Checks in priority order: Cargo.toml, package.json, pyproject.toml/setup.py, go.mod.
/// Returns `DetectedStack::default()` if no recognized project is found.
pub fn detect_stack(project_dir: &Path) -> DetectedStack {
    // Check in priority order
    if project_dir.join("Cargo.toml").exists() {
        return detect_rust_stack(project_dir);
    }
    if project_dir.join("package.json").exists() {
        return detect_node_stack(project_dir);
    }
    if project_dir.join("pyproject.toml").exists() || project_dir.join("setup.py").exists() {
        return detect_python_stack(project_dir);
    }
    if project_dir.join("go.mod").exists() {
        return detect_go_stack(project_dir);
    }

    DetectedStack::default()
}

fn detect_rust_stack(_dir: &Path) -> DetectedStack {
    DetectedStack {
        language: Language::Rust,
        framework: None, // Could parse Cargo.toml for framework detection
        test_runner: Some("cargo test".to_string()),
        type_checker: Some("rustc".to_string()),
        linter: Some("clippy".to_string()),
    }
}

fn detect_node_stack(dir: &Path) -> DetectedStack {
    let pkg_path = dir.join("package.json");
    let mut stack = DetectedStack {
        language: Language::JavaScript,
        framework: None,
        test_runner: None,
        type_checker: None,
        linter: None,
    };

    if let Ok(content) = std::fs::read_to_string(&pkg_path) {
        if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&content) {
            // Check for TypeScript
            if has_dep(&pkg, "devDependencies", "typescript") {
                stack.language = Language::TypeScript;
                stack.type_checker = Some("tsc".to_string());
            }

            // Detect test runner
            if has_dep(&pkg, "devDependencies", "vitest") {
                stack.test_runner = Some("vitest".to_string());
            } else if has_dep(&pkg, "devDependencies", "jest") {
                stack.test_runner = Some("jest".to_string());
            } else if has_dep(&pkg, "devDependencies", "mocha") {
                stack.test_runner = Some("mocha".to_string());
            }

            // Detect linter
            if has_dep(&pkg, "devDependencies", "eslint") {
                stack.linter = Some("eslint".to_string());
            } else if has_dep(&pkg, "devDependencies", "biome") {
                stack.linter = Some("biome".to_string());
            }

            // Detect framework
            if has_dep(&pkg, "dependencies", "next") {
                stack.framework = Some("Next.js".to_string());
            } else if has_dep(&pkg, "dependencies", "react") {
                stack.framework = Some("React".to_string());
            } else if has_dep(&pkg, "dependencies", "vue") {
                stack.framework = Some("Vue".to_string());
            } else if has_dep(&pkg, "dependencies", "express") {
                stack.framework = Some("Express".to_string());
            }
        }
    }

    stack
}

fn has_dep(pkg: &serde_json::Value, dep_type: &str, name: &str) -> bool {
    pkg.get(dep_type)
        .and_then(|deps| deps.get(name))
        .is_some()
}

fn detect_python_stack(dir: &Path) -> DetectedStack {
    let mut stack = DetectedStack {
        language: Language::Python,
        framework: None,
        test_runner: Some("pytest".to_string()), // Default to pytest
        type_checker: None,
        linter: None,
    };

    // Try to parse pyproject.toml for more specific info
    let pyproject_path = dir.join("pyproject.toml");
    if let Ok(content) = std::fs::read_to_string(pyproject_path) {
        let content_lower = content.to_lowercase();

        // Check for mypy
        if content_lower.contains("mypy") {
            stack.type_checker = Some("mypy".to_string());
        }

        // Check for linters
        if content_lower.contains("ruff") {
            stack.linter = Some("ruff".to_string());
        } else if content_lower.contains("flake8") {
            stack.linter = Some("flake8".to_string());
        } else if content_lower.contains("pylint") {
            stack.linter = Some("pylint".to_string());
        }

        // Check for frameworks
        if content_lower.contains("django") {
            stack.framework = Some("Django".to_string());
        } else if content_lower.contains("fastapi") {
            stack.framework = Some("FastAPI".to_string());
        } else if content_lower.contains("flask") {
            stack.framework = Some("Flask".to_string());
        }
    }

    stack
}

fn detect_go_stack(_dir: &Path) -> DetectedStack {
    DetectedStack {
        language: Language::Go,
        framework: None,
        test_runner: Some("go test".to_string()),
        type_checker: Some("go build".to_string()), // Go's type checking is in the compiler
        linter: Some("golangci-lint".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_detect_rust_stack() {
        let dir = TempDir::new().expect("temp dir");
        fs::write(dir.path().join("Cargo.toml"), "[package]").expect("write");

        let stack = detect_stack(dir.path());
        assert_eq!(stack.language, Language::Rust);
        assert_eq!(stack.test_runner, Some("cargo test".to_string()));
        assert_eq!(stack.type_checker, Some("rustc".to_string()));
        assert_eq!(stack.linter, Some("clippy".to_string()));
    }

    #[test]
    fn test_detect_node_typescript_stack() {
        let dir = TempDir::new().expect("temp dir");
        let pkg_json = r#"{
            "name": "test",
            "devDependencies": {
                "typescript": "^5.0.0",
                "vitest": "^1.0.0",
                "eslint": "^8.0.0"
            },
            "dependencies": {
                "react": "^18.0.0"
            }
        }"#;
        fs::write(dir.path().join("package.json"), pkg_json).expect("write");

        let stack = detect_stack(dir.path());
        assert_eq!(stack.language, Language::TypeScript);
        assert_eq!(stack.test_runner, Some("vitest".to_string()));
        assert_eq!(stack.type_checker, Some("tsc".to_string()));
        assert_eq!(stack.linter, Some("eslint".to_string()));
        assert_eq!(stack.framework, Some("React".to_string()));
    }

    #[test]
    fn test_detect_node_javascript_stack() {
        let dir = TempDir::new().expect("temp dir");
        let pkg_json = r#"{
            "name": "test",
            "devDependencies": {
                "jest": "^29.0.0"
            },
            "dependencies": {
                "express": "^4.0.0"
            }
        }"#;
        fs::write(dir.path().join("package.json"), pkg_json).expect("write");

        let stack = detect_stack(dir.path());
        assert_eq!(stack.language, Language::JavaScript);
        assert_eq!(stack.test_runner, Some("jest".to_string()));
        assert_eq!(stack.framework, Some("Express".to_string()));
    }

    #[test]
    fn test_detect_python_stack() {
        let dir = TempDir::new().expect("temp dir");
        let pyproject = r#"
[tool.pytest.ini_options]
testpaths = ["tests"]

[tool.mypy]
strict = true

[tool.ruff]
line-length = 88

[project]
dependencies = ["fastapi"]
"#;
        fs::write(dir.path().join("pyproject.toml"), pyproject).expect("write");

        let stack = detect_stack(dir.path());
        assert_eq!(stack.language, Language::Python);
        assert_eq!(stack.test_runner, Some("pytest".to_string()));
        assert_eq!(stack.type_checker, Some("mypy".to_string()));
        assert_eq!(stack.linter, Some("ruff".to_string()));
        assert_eq!(stack.framework, Some("FastAPI".to_string()));
    }

    #[test]
    fn test_detect_go_stack() {
        let dir = TempDir::new().expect("temp dir");
        fs::write(dir.path().join("go.mod"), "module example.com/test").expect("write");

        let stack = detect_stack(dir.path());
        assert_eq!(stack.language, Language::Go);
        assert_eq!(stack.test_runner, Some("go test".to_string()));
        assert_eq!(stack.linter, Some("golangci-lint".to_string()));
    }

    #[test]
    fn test_detect_unknown_stack() {
        let dir = TempDir::new().expect("temp dir");
        // Empty directory - no manifest files

        let stack = detect_stack(dir.path());
        assert_eq!(stack.language, Language::Unknown);
        assert!(stack.test_runner.is_none());
    }

    #[test]
    fn test_to_summary() {
        let stack = DetectedStack {
            language: Language::Rust,
            framework: Some("Actix".to_string()),
            test_runner: Some("cargo test".to_string()),
            type_checker: Some("rustc".to_string()),
            linter: Some("clippy".to_string()),
        };

        let summary = stack.to_summary();
        assert!(summary.contains("Language: Rust"));
        assert!(summary.contains("Framework: Actix"));
        assert!(summary.contains("Test Runner: cargo test"));
        assert!(summary.contains("Type Checker: rustc"));
        assert!(summary.contains("Linter: clippy"));
    }

    #[test]
    fn test_to_summary_minimal() {
        let stack = DetectedStack::default();

        let summary = stack.to_summary();
        assert!(summary.contains("Language: Unknown"));
        // Should NOT contain Framework, Test Runner, etc. since they're None
        assert!(!summary.contains("Framework:"));
    }

    #[test]
    fn test_language_display() {
        assert_eq!(format!("{}", Language::Rust), "Rust");
        assert_eq!(format!("{}", Language::TypeScript), "TypeScript");
        assert_eq!(format!("{}", Language::JavaScript), "JavaScript");
        assert_eq!(format!("{}", Language::Python), "Python");
        assert_eq!(format!("{}", Language::Go), "Go");
        assert_eq!(format!("{}", Language::Unknown), "Unknown");
    }
}
