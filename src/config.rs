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
    pub fn load() -> Self {
        let path = Self::config_path();

        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str::<Config>(&content) {
                    return config;
                }
            }
        }

        // Write default config for next time
        let config = Config::default();
        config.save_default(&path);
        config
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
}
