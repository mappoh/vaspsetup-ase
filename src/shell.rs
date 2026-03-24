//! Shell operations: SGE template rendering, qsub execution, name sanitization.

use std::fs;
use std::path::Path;
use std::process::Command;

/// Result of a qsub submission.
#[derive(Debug)]
pub struct QsubResult {
    pub success: bool,
    pub job_id: Option<String>,
    pub message: String,
}

/// Reject script parameters that contain newlines or other injection vectors.
fn validate_script_param(value: &str, name: &str) -> Result<(), String> {
    if value.contains('\n') || value.contains('\r') {
        return Err(format!("{} contains newline characters", name));
    }
    Ok(())
}

/// Render an SGE submission script from parameters.
/// Returns Err if any parameter contains injection-risk characters.
pub fn render_sge_script(
    job_name: &str,
    queue: &str,
    cores: u32,
    parallel_env: &str,
    vasp_module: &str,
    work_dir: &str,
    vasp_cmd: &str,
) -> Result<String, String> {
    validate_script_param(job_name, "job_name")?;
    validate_script_param(queue, "queue")?;
    validate_script_param(parallel_env, "parallel_env")?;
    validate_script_param(vasp_module, "vasp_module")?;
    validate_script_param(work_dir, "work_dir")?;
    validate_script_param(vasp_cmd, "vasp_cmd")?;

    Ok(format!(
        "#!/bin/bash\n\
         #$ -N {job_name}\n\
         #$ -q {queue}\n\
         #$ -pe {parallel_env} {cores}\n\
         #$ -cwd\n\
         module load {vasp_module}\n\
         cd {work_dir}\n\
         mpirun -np $NSLOTS {vasp_cmd}\n"
    ))
}

/// Write the submission script to a directory. Returns the script path.
pub fn write_submission_script(dir: &Path, content: &str) -> std::io::Result<std::path::PathBuf> {
    let script_path = dir.join("qscript.sh");
    fs::write(&script_path, content)?;
    Ok(script_path)
}

/// Execute qsub on a submission script. Parse the result.
pub fn run_qsub(script_path: &Path) -> QsubResult {
    let result = Command::new("qsub").arg(script_path).output();

    match result {
        Err(e) => QsubResult {
            success: false,
            job_id: None,
            message: format!("qsub not available: {}", e),
        },
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let job_id = parse_job_id(&stdout);
                QsubResult {
                    success: true,
                    job_id,
                    message: stdout.trim().to_string(),
                }
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                QsubResult {
                    success: false,
                    job_id: None,
                    message: format!("Submission failed: {}", stderr.trim()),
                }
            }
        }
    }
}

/// Extract job ID from qsub success output.
/// Expected format: "Your job 123456 ("name") has been submitted"
/// Returns None if the third token is not a numeric string.
fn parse_job_id(stdout: &str) -> Option<String> {
    stdout
        .split_whitespace()
        .nth(2)
        .filter(|s| s.chars().all(|c| c.is_ascii_digit()) && !s.is_empty())
        .map(|s| s.to_string())
}

/// Sanitize a job name: only alphanumeric, hyphens, underscores allowed.
/// Falls back to "vaspsetup_job" if result would be empty.
pub fn sanitize_job_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if sanitized.is_empty() {
        "vaspsetup_job".into()
    } else {
        sanitized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_job_name_clean() {
        assert_eq!(sanitize_job_name("sp_Fe2O3_01"), "sp_Fe2O3_01");
    }

    #[test]
    fn test_sanitize_job_name_dirty() {
        assert_eq!(sanitize_job_name("calc; rm -rf /"), "calcrm-rf");
    }

    #[test]
    fn test_sanitize_job_name_spaces() {
        assert_eq!(sanitize_job_name("my job name"), "myjobname");
    }

    #[test]
    fn test_sanitize_job_name_empty_fallback() {
        assert_eq!(sanitize_job_name(""), "vaspsetup_job");
        assert_eq!(sanitize_job_name("!!!"), "vaspsetup_job");
    }

    #[test]
    fn test_parse_job_id() {
        let output = "Your job 123456 (\"sp_Fe2O3\") has been submitted";
        assert_eq!(parse_job_id(output), Some("123456".to_string()));
    }

    #[test]
    fn test_parse_job_id_non_numeric() {
        // If third token isn't digits, return None
        let output = "Some weird output format here";
        assert_eq!(parse_job_id(output), None);
    }

    #[test]
    fn test_parse_job_id_no_match() {
        assert_eq!(parse_job_id(""), None);
        assert_eq!(parse_job_id("error"), None);
    }

    #[test]
    fn test_render_sge_script() {
        let script = render_sge_script(
            "test_job", "long", 64, "mpi-*", "vasp/6.4.0/", "/work/calc", "vasp_std",
        )
        .unwrap();
        assert!(script.contains("#$ -N test_job"));
        assert!(script.contains("#$ -q long"));
        assert!(script.contains("#$ -pe mpi-* 64"));
        assert!(script.contains("module load vasp/6.4.0/"));
        assert!(script.contains("cd /work/calc"));
        assert!(script.contains("mpirun -np $NSLOTS vasp_std"));
    }

    #[test]
    fn test_render_sge_script_injection_blocked() {
        let result = render_sge_script(
            "test", "long", 64, "mpi-*", "vasp/6.4.0/",
            "/work\nrm -rf /", "vasp_std",
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("newline"));
    }

    #[test]
    fn test_render_sge_script_neb() {
        let script = render_sge_script(
            "neb_job", "short", 32, "mpi-*", "vasp/6.4.0/", "/work/neb", "vasp_neb",
        )
        .unwrap();
        assert!(script.contains("vasp_neb"));
    }

    #[test]
    fn test_validate_script_param() {
        assert!(validate_script_param("normal_value", "test").is_ok());
        assert!(validate_script_param("has\nnewline", "test").is_err());
        assert!(validate_script_param("has\rreturn", "test").is_err());
    }
}
