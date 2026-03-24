//! Python subprocess communication layer.
//!
//! All communication follows this JSON protocol:
//!
//!   Request (stdin):  {"command": "atoms", "args": {"file": "/path"}}
//!   Response (stdout): {"status": "ok", "data": {...}}
//!                  or: {"status": "error", "message": "..."}

use serde_json::Value;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Find the project root containing `vaspsetup_core/`.
/// Checks: executable's ancestor dirs, then current dir.
fn find_project_root() -> Option<PathBuf> {
    // Try relative to the executable (handles target/release/vaspsetup or target/debug/vaspsetup)
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent().map(|p| p.to_path_buf());
        while let Some(d) = dir {
            if d.join("vaspsetup_core").is_dir() {
                return Some(d);
            }
            dir = d.parent().map(|p| p.to_path_buf());
        }
    }
    // Fall back to current directory
    if let Ok(cwd) = std::env::current_dir() {
        if cwd.join("vaspsetup_core").is_dir() {
            return Some(cwd);
        }
    }
    None
}

/// Find a Python >= 3.9 executable. Tries "python" first, then "python3".
fn find_python() -> &'static str {
    // Cache the result after first call
    static PYTHON: std::sync::OnceLock<&str> = std::sync::OnceLock::new();
    PYTHON.get_or_init(|| {
        for candidate in &["python", "python3"] {
            if is_python_39_or_later(candidate) {
                return candidate;
            }
        }
        // Fall back to "python3" even if check failed — let the caller
        // surface a clear error via check_python() at startup.
        "python3"
    })
}

/// Check whether a Python binary exists and reports version >= 3.9.
fn is_python_39_or_later(bin: &str) -> bool {
    let output = match Command::new(bin)
        .arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return false,
    };

    // `python --version` prints "Python 3.12.2" to stdout (or stderr on old builds)
    let text = String::from_utf8_lossy(&output.stdout);
    let text = if text.trim().is_empty() {
        String::from_utf8_lossy(&output.stderr)
    } else {
        text
    };

    parse_python_version(&text)
        .map(|(major, minor)| major == 3 && minor >= 9)
        .unwrap_or(false)
}

/// Parse "Python X.Y.Z" into (major, minor).
fn parse_python_version(text: &str) -> Option<(u32, u32)> {
    let version_str = text.trim().strip_prefix("Python ")?;
    let mut parts = version_str.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    Some((major, minor))
}

/// Errors from the Python backend.
#[derive(Debug)]
pub enum PythonError {
    NotInstalled,
    ModuleNotFound,
    ProcessFailed(String),
    InvalidResponse(String),
    BackendError(String),
}

impl std::fmt::Display for PythonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PythonError::NotInstalled => {
                write!(f, "Python not found. Install Python 3.9+.")
            }
            PythonError::ModuleNotFound => {
                write!(f, "vaspsetup_core not found. Ensure the binary is run from the project directory or vaspsetup_core/ is alongside the executable.")
            }
            PythonError::ProcessFailed(msg) => {
                write!(f, "Python backend failed: {}", msg)
            }
            PythonError::InvalidResponse(msg) => {
                write!(f, "Unexpected response from backend: {}", msg)
            }
            PythonError::BackendError(msg) => write!(f, "{}", msg),
        }
    }
}

/// Verify Python and vaspsetup_core are available. Call at startup.
pub fn check_python() -> Result<(), PythonError> {
    call_python("version", serde_json::json!({}))?;
    Ok(())
}

/// Call the Python backend with a command and arguments.
///
/// Spawns `python -m vaspsetup_core`, writes JSON to stdin,
/// reads JSON from stdout, and returns the `data` field.
pub fn call_python(command: &str, args: Value) -> Result<Value, PythonError> {
    let request = serde_json::json!({
        "command": command,
        "args": args,
    });

    let mut cmd = Command::new(find_python());
    cmd.args(["-m", "vaspsetup_core"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Set PYTHONPATH so vaspsetup_core can be found without pip install
    if let Some(root) = find_project_root() {
        cmd.env("PYTHONPATH", root);
    }

    let mut child = cmd.spawn()
        .map_err(|_| PythonError::NotInstalled)?;

    // Write request to stdin and close it (triggers EOF for Python's stdin.read())
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(request.to_string().as_bytes())
            .map_err(|e| PythonError::ProcessFailed(format!("stdin write failed: {}", e)))?;
        // stdin is dropped here, closing the pipe
    }

    let output = child
        .wait_with_output()
        .map_err(|e| PythonError::ProcessFailed(format!("process failed: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // No output = process crashed or module not found
    if stdout.trim().is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("No module named") {
            return Err(PythonError::ModuleNotFound);
        }
        return Err(PythonError::ProcessFailed(format!(
            "No output. stderr: {}",
            stderr.trim()
        )));
    }

    // Parse JSON response
    let response: Value = serde_json::from_str(stdout.trim()).map_err(|e| {
        PythonError::InvalidResponse(format!("Invalid JSON: {}. Raw: {}", e, stdout.trim()))
    })?;

    // Check status field
    match response.get("status").and_then(|s| s.as_str()) {
        Some("ok") => response
            .get("data")
            .cloned()
            .ok_or_else(|| PythonError::InvalidResponse("Missing 'data' field".into())),
        Some("error") => {
            let msg = response
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown backend error");
            Err(PythonError::BackendError(msg.into()))
        }
        _ => Err(PythonError::InvalidResponse(format!(
            "Invalid 'status' field: {}",
            stdout.trim()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_error_display() {
        let err = PythonError::NotInstalled;
        assert!(err.to_string().contains("Python not found"));

        let err = PythonError::BackendError("test error".into());
        assert_eq!(err.to_string(), "test error");
    }

    #[test]
    fn test_parse_python_version() {
        assert_eq!(parse_python_version("Python 3.12.2"), Some((3, 12)));
        assert_eq!(parse_python_version("Python 3.9.19"), Some((3, 9)));
        assert_eq!(parse_python_version("Python 2.7.18"), Some((2, 7)));
        assert_eq!(parse_python_version("Python 3.14.0"), Some((3, 14)));
        assert_eq!(parse_python_version("  Python 3.12.2\n"), Some((3, 12)));
        assert_eq!(parse_python_version("not python"), None);
        assert_eq!(parse_python_version(""), None);
    }

    #[test]
    fn test_python_version_check() {
        // 3.9+ should pass
        assert!(parse_python_version("Python 3.12.2")
            .map(|(maj, min)| maj == 3 && min >= 9)
            .unwrap_or(false));
        assert!(parse_python_version("Python 3.9.0")
            .map(|(maj, min)| maj == 3 && min >= 9)
            .unwrap_or(false));

        // Python 2 should fail
        assert!(!parse_python_version("Python 2.7.18")
            .map(|(maj, min)| maj == 3 && min >= 9)
            .unwrap_or(false));

        // Python 3.8 should fail
        assert!(!parse_python_version("Python 3.8.10")
            .map(|(maj, min)| maj == 3 && min >= 9)
            .unwrap_or(false));
    }
}
