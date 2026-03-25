//! Configuration management for cluster settings (~/.vaspsetup/config.json).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct ClusterConfig {
    pub scheduler: String,
    pub default_queue: String,
    pub default_cores: u32,
    pub parallel_env: String,
    pub vasp_module: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VaspConfig {
    pub default_executable: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub cluster: ClusterConfig,
    pub vasp: VaspConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cluster: ClusterConfig {
                scheduler: "sge".into(),
                default_queue: "long".into(),
                default_cores: 64,
                parallel_env: "mpi-*".into(),
                vasp_module: "vasp/6.4.0/".into(),
            },
            vasp: VaspConfig {
                default_executable: "vasp_std".into(),
            },
        }
    }
}

impl Config {
    /// Load config from ~/.vaspsetup/config.json, or create default.
    /// Returns (config, warning) — warning is set if the file existed but had invalid JSON.
    pub fn load() -> (Self, Option<String>) {
        let path = Self::config_path();
        Self::load_from(&path)
    }

    /// Load config from a specific path. Testable version of `load()`.
    fn load_from(path: &PathBuf) -> (Self, Option<String>) {
        if path.exists() {
            match fs::read_to_string(path) {
                Ok(content) => match serde_json::from_str::<Config>(&content) {
                    Ok(config) => return (config, None),
                    Err(e) => {
                        // File exists but has invalid JSON — use defaults, do NOT overwrite
                        let warning = format!(
                            "Warning: {} has invalid JSON ({}). Using defaults.",
                            path.display(),
                            e
                        );
                        return (Config::default(), Some(warning));
                    }
                },
                Err(e) => {
                    let warning = format!(
                        "Warning: Could not read {} ({}). Using defaults.",
                        path.display(),
                        e
                    );
                    return (Config::default(), Some(warning));
                }
            }
        }

        // File doesn't exist — write default config for next time
        let config = Config::default();
        config.save_default(path);
        (config, None)
    }

    /// Path to the config file.
    fn config_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".vaspsetup")
            .join("config.json")
    }

    /// Write default config to disk (best-effort, ignore errors).
    fn save_default(&self, path: &PathBuf) {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, json);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_default_values() {
        let config = Config::default();
        assert_eq!(config.cluster.scheduler, "sge");
        assert_eq!(config.cluster.default_queue, "long");
        assert_eq!(config.cluster.default_cores, 64);
        assert_eq!(config.vasp.default_executable, "vasp_std");
    }

    #[test]
    fn test_serialize_roundtrip() {
        let config = Config::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.cluster.default_queue, "long");
        assert_eq!(parsed.cluster.default_cores, 64);
    }

    #[test]
    fn test_load_valid_config_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");

        // Write a valid config with custom values
        let mut custom = Config::default();
        custom.cluster.default_queue = "short".into();
        custom.cluster.default_cores = 128;
        let json = serde_json::to_string_pretty(&custom).unwrap();
        fs::write(&path, json).unwrap();

        let (config, warning) = Config::load_from(&path.to_path_buf());
        assert!(warning.is_none());
        assert_eq!(config.cluster.default_queue, "short");
        assert_eq!(config.cluster.default_cores, 128);
    }

    #[test]
    fn test_load_invalid_json_warns_and_uses_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");

        // Write invalid JSON
        let mut f = fs::File::create(&path).unwrap();
        f.write_all(b"{ this is not valid json }").unwrap();

        let (config, warning) = Config::load_from(&path.to_path_buf());

        // Should warn
        assert!(warning.is_some());
        assert!(warning.unwrap().contains("invalid JSON"));

        // Should use defaults
        assert_eq!(config.cluster.default_queue, "long");
        assert_eq!(config.cluster.default_cores, 64);

        // Should NOT have overwritten the file
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "{ this is not valid json }");
    }

    #[test]
    fn test_load_missing_file_creates_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        assert!(!path.exists());

        let (config, warning) = Config::load_from(&path.to_path_buf());

        // No warning for missing file
        assert!(warning.is_none());

        // Uses defaults
        assert_eq!(config.cluster.default_queue, "long");

        // Creates the file
        assert!(path.exists());
        let content = fs::read_to_string(&path).unwrap();
        let parsed: Config = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed.cluster.default_queue, "long");
    }
}
