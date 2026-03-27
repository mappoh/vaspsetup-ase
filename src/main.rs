//! VASPSetup — VASP calculation setup and submission TUI.
//!
//! Entry point: detect directory, scan files, check Python, launch TUI.

mod app;
mod config;
mod python;
mod screens;
mod shell;
mod state;
mod widgets;

use std::env;
use std::io;
use std::path::PathBuf;
use std::process;

use crossterm::{
    cursor,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, DisableLineWrap, EnableLineWrap},
};
use ratatui::backend::CrosstermBackend;
use ratatui::{Terminal, TerminalOptions, Viewport};

use app::{App, ExitReason};
use config::Config;
use screens::result::JobResult;
use state::AppState;

/// File extensions recognized as VASP input structure files.
const STRUCTURE_EXTENSIONS: &[&str] = &["vasp", "cif", "xyz", "xsd", "gen"];

/// File prefixes recognized as VASP structure files.
const STRUCTURE_PREFIXES: &[&str] = &["POSCAR", "CONTCAR"];

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn run() -> io::Result<()> {
    // Determine working directory
    let work_dir = env::current_dir()?;

    // Check terminal dimensions
    let (cols, rows) = crossterm::terminal::size()?;
    if cols < 70 {
        eprintln!(
            "Terminal too narrow ({} cols). Resize to at least 70 columns.",
            cols
        );
        process::exit(1);
    }
    if rows < 10 {
        eprintln!(
            "Terminal too short ({} rows). Resize to at least 10 rows.",
            rows
        );
        process::exit(1);
    }

    // Scan for structure files
    let files = scan_structure_files(&work_dir);
    if files.is_empty() {
        eprintln!("No structure files found in: {}", work_dir.display());
        eprintln!(
            "Looking for: POSCAR, CONTCAR, or files with extensions: {}",
            STRUCTURE_EXTENSIONS.join(", ")
        );
        process::exit(1);
    }

    // Load configuration
    let (config, config_warning) = Config::load();
    if let Some(warning) = &config_warning {
        eprintln!("{}", warning);
    }

    // Check Python availability
    if let Err(e) = python::check_python() {
        eprintln!("{}", e);
        process::exit(1);
    }

    // Create application state
    let state = AppState::new(work_dir, files, &config);

    // Install panic hook to restore terminal on crash
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), cursor::Show, EnableLineWrap);
        default_hook(info);
    }));

    // Set up inline viewport (renders at current cursor position)
    let viewport_height = (rows / 2).max(12).min(rows);

    enable_raw_mode()?;
    execute!(io::stdout(), DisableLineWrap)?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::Inline(viewport_height),
        },
    )?;

    // Run the app
    let mut app = App::new(state, config);
    let exit_reason = app.run(&mut terminal);

    // Restore terminal: let ratatui clean up the inline viewport,
    // then disable raw mode and re-enable line wrap + cursor
    let _ = terminal.clear();
    drop(terminal);
    let _ = disable_raw_mode();
    let _ = execute!(
        io::stdout(),
        cursor::Show,
        EnableLineWrap
    );

    // Print summary after TUI cleanup
    match exit_reason? {
        ExitReason::Submitted(results) => print_summary(&results),
        ExitReason::Cancelled => {}
    }

    Ok(())
}

/// Print a one-line summary of submission results to stdout/stderr.
fn print_summary(results: &[JobResult]) {
    write_summary(results, &mut io::stdout(), &mut io::stderr());
}

/// Write submission summary to the given outputs (testable).
fn write_summary(
    results: &[JobResult],
    out: &mut impl io::Write,
    err: &mut impl io::Write,
) {
    if results.is_empty() {
        return;
    }

    let succeeded = results.iter().filter(|r| r.success).count();
    let failed = results.len() - succeeded;

    match succeeded {
        0 => {}
        1 => {
            let r = results.iter().find(|r| r.success).unwrap();
            if let Some(ref id) = r.job_id {
                let _ = writeln!(out, "vaspsetup: submitted {} (Job ID: {})", r.job_name, id);
            } else {
                let _ = writeln!(out, "vaspsetup: submitted {}", r.job_name);
            }
        }
        n => {
            let mut ids = String::new();
            for r in results.iter().filter(|r| r.success) {
                if let Some(ref id) = r.job_id {
                    if !ids.is_empty() {
                        ids.push_str(", ");
                    }
                    ids.push_str(id);
                }
            }
            if ids.is_empty() {
                let _ = writeln!(out, "vaspsetup: submitted {} job(s)", n);
            } else {
                let _ = writeln!(out, "vaspsetup: submitted {} job(s) — IDs: {}", n, ids);
            }
        }
    }

    if failed > 0 {
        let _ = writeln!(err, "vaspsetup: {} job(s) failed", failed);
    }
}

/// Scan the working directory for structure files.
fn scan_structure_files(dir: &PathBuf) -> Vec<String> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    let mut files: Vec<String> = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden files and directories
            if name.starts_with('.') || entry.file_type().ok()?.is_dir() {
                return None;
            }

            // Check by prefix (POSCAR, CONTCAR — with any suffix or standalone)
            for prefix in STRUCTURE_PREFIXES {
                if name == *prefix || name.starts_with(prefix) {
                    return Some(name);
                }
            }

            // Check by extension
            if let Some(ext) = name.rsplit('.').next() {
                if STRUCTURE_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
                    return Some(name);
                }
            }

            None
        })
        .collect();

    files.sort();
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(name: &str, success: bool, job_id: Option<&str>) -> JobResult {
        JobResult {
            job_name: name.to_string(),
            success,
            job_id: job_id.map(|s| s.to_string()),
            message: String::new(),
        }
    }

    /// Run write_summary and return (stdout, stderr) as strings.
    fn run_summary(results: &[JobResult]) -> (String, String) {
        let mut out = Vec::new();
        let mut err = Vec::new();
        write_summary(results, &mut out, &mut err);
        (
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    #[test]
    fn test_summary_empty() {
        let (out, err) = run_summary(&[]);
        assert!(out.is_empty());
        assert!(err.is_empty());
    }

    #[test]
    fn test_summary_single_with_id() {
        let (out, err) = run_summary(&[job("sp_Fe2O3", true, Some("123456"))]);
        assert_eq!(out, "vaspsetup: submitted sp_Fe2O3 (Job ID: 123456)\n");
        assert!(err.is_empty());
    }

    #[test]
    fn test_summary_single_no_id() {
        let (out, err) = run_summary(&[job("sp_Fe2O3", true, None)]);
        assert_eq!(out, "vaspsetup: submitted sp_Fe2O3\n");
        assert!(err.is_empty());
    }

    #[test]
    fn test_summary_multiple_with_ids() {
        let results = vec![
            job("sp_01", true, Some("123")),
            job("sp_02", true, Some("456")),
            job("sp_03", true, Some("789")),
        ];
        let (out, err) = run_summary(&results);
        assert!(out.contains("submitted 3 job(s)"));
        assert!(out.contains("123, 456, 789"));
        assert!(err.is_empty());
    }

    #[test]
    fn test_summary_multiple_no_ids() {
        let results = vec![job("sp_01", true, None), job("sp_02", true, None)];
        let (out, err) = run_summary(&results);
        assert_eq!(out, "vaspsetup: submitted 2 job(s)\n");
        assert!(err.is_empty());
    }

    #[test]
    fn test_summary_all_failed() {
        let (out, err) = run_summary(&[job("sp_01", false, None)]);
        assert!(out.is_empty());
        assert_eq!(err, "vaspsetup: 1 job(s) failed\n");
    }

    #[test]
    fn test_summary_mixed_success_and_failure() {
        let results = vec![
            job("sp_01", true, Some("123")),
            job("sp_02", false, None),
        ];
        let (out, err) = run_summary(&results);
        assert!(out.contains("submitted sp_01 (Job ID: 123)"));
        assert!(err.contains("1 job(s) failed"));
    }
}
