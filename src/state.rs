//! Application state — single source of truth for the entire TUI session.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::config::Config;

/// Calculation types available in the TUI.
#[derive(Debug, Clone, PartialEq)]
pub enum CalcType {
    SinglePoint,
    GeometryOpt,
    Frequency,
    Bader,
    Pdos,
    ChargeDensity,
    Orbital,
    Tst,
}

impl CalcType {
    /// Preset file name used by the Python backend.
    /// For TST, the preset depends on the TstMethod.
    pub fn preset_name(&self) -> &str {
        match self {
            CalcType::SinglePoint => "single_point",
            CalcType::GeometryOpt => "geometry_opt",
            CalcType::Frequency => "frequency",
            CalcType::Bader => "bader",
            CalcType::Pdos => "pdos",
            CalcType::ChargeDensity => "charge_density",
            CalcType::Orbital => "orbital",
            CalcType::Tst => "neb", // default; overridden by TstMethod
        }
    }

    /// Human-readable label for the TUI menu.
    pub fn display_name(&self) -> &str {
        match self {
            CalcType::SinglePoint => "Single Point",
            CalcType::GeometryOpt => "Geometry Optimization",
            CalcType::Frequency => "Frequency Calculation",
            CalcType::Bader => "Bader Charge",
            CalcType::Pdos => "PDOS",
            CalcType::ChargeDensity => "Charge Density",
            CalcType::Orbital => "Orbital",
            CalcType::Tst => "Transition State",
        }
    }

    /// VASP binary for this calculation type.
    pub fn vasp_binary(&self) -> &str {
        match self {
            CalcType::Tst => "vasp_neb",
            _ => "vasp_std",
        }
    }

    /// All calculation types in display order.
    pub fn all() -> &'static [CalcType] {
        &[
            CalcType::SinglePoint,
            CalcType::GeometryOpt,
            CalcType::Frequency,
            CalcType::Bader,
            CalcType::Pdos,
            CalcType::ChargeDensity,
            CalcType::Orbital,
            CalcType::Tst,
        ]
    }
}

/// Transition state method sub-type.
#[derive(Debug, Clone, PartialEq)]
pub enum TstMethod {
    Neb,
    CiNeb,
    Dimer,
}

impl TstMethod {
    /// Preset file name for this TST method.
    pub fn preset_name(&self) -> &str {
        match self {
            TstMethod::Neb => "neb",
            TstMethod::CiNeb => "ci_neb",
            TstMethod::Dimer => "dimer",
        }
    }

    /// Human-readable label.
    pub fn display_name(&self) -> &str {
        match self {
            TstMethod::Neb => "NEB (Nudged Elastic Band)",
            TstMethod::CiNeb => "CI-NEB (Climbing Image NEB)",
            TstMethod::Dimer => "Dimer",
        }
    }

    /// Whether this method uses IMAGES (multi-directory NEB).
    pub fn uses_images(&self) -> bool {
        match self {
            TstMethod::Neb | TstMethod::CiNeb => true,
            TstMethod::Dimer => false,
        }
    }

    /// All TST methods in display order.
    pub fn all() -> &'static [TstMethod] {
        &[TstMethod::Neb, TstMethod::CiNeb, TstMethod::Dimer]
    }
}

/// Spin polarization mode.
#[derive(Debug, Clone, PartialEq)]
pub enum SpinMode {
    Restricted,
    Unrestricted,
}

/// Which path the user chose at step 2.
#[derive(Debug, Clone, PartialEq)]
pub enum FlowPath {
    QuickSubmit,
    PerformCalculation,
}

/// Species information from the Python backend (for MAGMOM display).
#[derive(Debug, Clone)]
pub struct SpeciesInfo {
    pub symbol: String,
    pub count: usize,
}

/// All session state. Every screen reads from and writes to this struct.
#[derive(Debug)]
pub struct AppState {
    // Directory and files
    pub work_dir: PathBuf,
    pub files: Vec<String>,

    // User choices
    pub flow_path: Option<FlowPath>,
    pub calc_type: Option<CalcType>,
    pub tst_method: Option<TstMethod>,
    pub spin_mode: Option<SpinMode>,
    pub selected_files: Vec<usize>, // indices into `files`

    // MAGMOM (spin unrestricted only)
    pub species: Vec<SpeciesInfo>,
    pub magmom_per_species: Vec<f64>, // one value per species entry

    // VASP parameters
    pub incar_params: HashMap<String, serde_json::Value>,
    pub kpoints: [u32; 3],

    // Navigation
    pub current_step: u32,

    // Submission
    pub output_dir: String,
    pub job_names: Vec<String>,
    pub queue: String,
    pub cores: u32,
    /// User-chosen VASP binary (Quick Submit only; None = derive from calc_type).
    pub vasp_binary_override: Option<String>,
}

impl AppState {
    /// Create a new AppState, initializing defaults from the loaded Config.
    pub fn new(work_dir: PathBuf, files: Vec<String>, config: &Config) -> Self {
        Self {
            work_dir,
            files,
            flow_path: None,
            calc_type: None,
            tst_method: None,
            spin_mode: None,
            selected_files: Vec::new(),
            species: Vec::new(),
            magmom_per_species: Vec::new(),
            incar_params: HashMap::new(),
            kpoints: [1, 1, 1],
            current_step: 0,
            output_dir: String::new(),
            job_names: Vec::new(),
            queue: config.cluster.default_queue.clone(),
            cores: config.cluster.default_cores,
            vasp_binary_override: None,
        }
    }

    /// The effective preset name — uses TstMethod if TST, otherwise CalcType.
    pub fn effective_preset_name(&self) -> &str {
        if self.calc_type == Some(CalcType::Tst) {
            if let Some(ref method) = self.tst_method {
                return method.preset_name();
            }
        }
        self.calc_type
            .as_ref()
            .map_or("single_point", |ct| ct.preset_name())
    }

    /// VASP binary: user override if set, otherwise derived from calc type.
    pub fn vasp_binary(&self) -> &str {
        if let Some(ref bin) = self.vasp_binary_override {
            bin
        } else {
            self.calc_type
                .as_ref()
                .map_or("vasp_std", |ct| ct.vasp_binary())
        }
    }

    /// Build MAGMOM string in VASP format (e.g., "2*5.0 3*0.6").
    pub fn magmom_string(&self) -> Option<String> {
        if self.magmom_per_species.is_empty() || self.species.is_empty() {
            return None;
        }
        let parts: Vec<String> = self
            .species
            .iter()
            .zip(self.magmom_per_species.iter())
            .map(|(sp, mag)| format!("{}*{}", sp.count, mag))
            .collect();
        Some(parts.join(" "))
    }

    /// Total steps for the current flow path.
    /// Adjusts dynamically based on whether MAGMOM, TstMethod, FilePick, and KPOINTS screens are needed.
    pub fn total_steps(&self) -> u32 {
        match self.flow_path {
            Some(FlowPath::QuickSubmit) => 1,
            Some(FlowPath::PerformCalculation) => {
                let base = 6; // CalcType + SpinMode + FilePick + EditIncar + EditKpoints + SubmitSetup
                let magmom = if self.spin_mode == Some(SpinMode::Unrestricted) { 1 } else { 0 };
                let tst = if self.calc_type == Some(CalcType::Tst) { 1 } else { 0 };
                // All TST methods skip FilePick and KPOINTS (-2)
                let tst_skip = if self.calc_type == Some(CalcType::Tst) { 2 } else { 0 };
                base + magmom + tst - tst_skip
            }
            None => 0,
        }
    }

    /// Auto-detect IMAGES count from numbered subdirectories (00/, 01/, ...).
    /// Returns the count of intermediate images (excluding endpoints 00/ and last/).
    /// Validates that directories form a contiguous sequence starting at 00.
    pub fn detect_images(&self) -> Option<u32> {
        let mut numbered_dirs: Vec<u32> = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.work_dir) {
            for entry in entries.flatten() {
                if entry.file_type().map_or(false, |ft| ft.is_dir()) {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if let Ok(n) = name.parse::<u32>() {
                        numbered_dirs.push(n);
                    }
                }
            }
        }
        numbered_dirs.sort_unstable();
        // Must start at 0, be contiguous, and have at least 3 dirs (00, ≥1 image, last)
        if numbered_dirs.len() >= 3
            && numbered_dirs[0] == 0
            && numbered_dirs.windows(2).all(|w| w[1] == w[0] + 1)
        {
            Some(numbered_dirs.len() as u32 - 2)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> Config {
        Config::default()
    }

    #[test]
    fn test_calc_type_preset_names() {
        assert_eq!(CalcType::SinglePoint.preset_name(), "single_point");
        assert_eq!(CalcType::GeometryOpt.preset_name(), "geometry_opt");
        assert_eq!(CalcType::Tst.preset_name(), "neb");
    }

    #[test]
    fn test_calc_type_vasp_binary() {
        assert_eq!(CalcType::SinglePoint.vasp_binary(), "vasp_std");
        assert_eq!(CalcType::GeometryOpt.vasp_binary(), "vasp_std");
        assert_eq!(CalcType::Tst.vasp_binary(), "vasp_neb");
    }

    #[test]
    fn test_calc_type_all_count() {
        assert_eq!(CalcType::all().len(), 8);
    }

    #[test]
    fn test_tst_method_preset_names() {
        assert_eq!(TstMethod::Neb.preset_name(), "neb");
        assert_eq!(TstMethod::CiNeb.preset_name(), "ci_neb");
        assert_eq!(TstMethod::Dimer.preset_name(), "dimer");
    }

    #[test]
    fn test_tst_method_uses_images() {
        assert!(TstMethod::Neb.uses_images());
        assert!(TstMethod::CiNeb.uses_images());
        assert!(!TstMethod::Dimer.uses_images());
    }

    #[test]
    fn test_tst_method_all_count() {
        assert_eq!(TstMethod::all().len(), 3);
    }

    #[test]
    fn test_effective_preset_name() {
        let cfg = test_config();
        let mut state = AppState::new(PathBuf::from("/tmp"), vec![], &cfg);

        state.calc_type = Some(CalcType::SinglePoint);
        assert_eq!(state.effective_preset_name(), "single_point");

        state.calc_type = Some(CalcType::Tst);
        state.tst_method = Some(TstMethod::CiNeb);
        assert_eq!(state.effective_preset_name(), "ci_neb");

        state.tst_method = Some(TstMethod::Dimer);
        assert_eq!(state.effective_preset_name(), "dimer");
    }

    #[test]
    fn test_total_steps_with_tst() {
        let cfg = test_config();
        let mut state = AppState::new(PathBuf::from("/tmp"), vec![], &cfg);

        state.flow_path = Some(FlowPath::PerformCalculation);
        state.spin_mode = Some(SpinMode::Restricted);
        assert_eq!(state.total_steps(), 6);

        // All TST: +1 (TstMethod) -2 (no FilePick, no KPOINTS) = 5
        state.calc_type = Some(CalcType::Tst);
        state.tst_method = Some(TstMethod::Dimer);
        assert_eq!(state.total_steps(), 5);

        state.tst_method = Some(TstMethod::Neb);
        assert_eq!(state.total_steps(), 5);

        // TST + unrestricted: 5 + 1 (magmom) = 6
        state.tst_method = Some(TstMethod::CiNeb);
        state.spin_mode = Some(SpinMode::Unrestricted);
        assert_eq!(state.total_steps(), 6);
    }

    #[test]
    fn test_magmom_string() {
        let cfg = test_config();
        let mut state = AppState::new(PathBuf::from("/tmp"), vec![], &cfg);
        state.species = vec![
            SpeciesInfo { symbol: "Fe".into(), count: 2 },
            SpeciesInfo { symbol: "O".into(), count: 3 },
        ];
        state.magmom_per_species = vec![5.0, 0.6];
        assert_eq!(state.magmom_string(), Some("2*5 3*0.6".to_string()));
    }

    #[test]
    fn test_magmom_string_empty() {
        let cfg = test_config();
        let state = AppState::new(PathBuf::from("/tmp"), vec![], &cfg);
        assert_eq!(state.magmom_string(), None);
    }

    #[test]
    fn test_total_steps() {
        let cfg = test_config();
        let mut state = AppState::new(PathBuf::from("/tmp"), vec![], &cfg);
        assert_eq!(state.total_steps(), 0);

        state.flow_path = Some(FlowPath::QuickSubmit);
        assert_eq!(state.total_steps(), 1);

        state.flow_path = Some(FlowPath::PerformCalculation);
        state.spin_mode = Some(SpinMode::Restricted);
        assert_eq!(state.total_steps(), 6);

        state.spin_mode = Some(SpinMode::Unrestricted);
        assert_eq!(state.total_steps(), 7);
    }

    #[test]
    fn test_new_defaults_from_config() {
        let cfg = test_config();
        let state = AppState::new(PathBuf::from("/work"), vec!["POSCAR".into()], &cfg);
        assert_eq!(state.queue, "long");
        assert_eq!(state.cores, 64);
        assert_eq!(state.kpoints, [1, 1, 1]);
        assert_eq!(state.files.len(), 1);
    }

    #[test]
    fn test_vasp_binary_derived() {
        let cfg = test_config();
        let mut state = AppState::new(PathBuf::from("/tmp"), vec![], &cfg);
        assert_eq!(state.vasp_binary(), "vasp_std");

        state.calc_type = Some(CalcType::Tst);
        assert_eq!(state.vasp_binary(), "vasp_neb");

        state.calc_type = Some(CalcType::SinglePoint);
        assert_eq!(state.vasp_binary(), "vasp_std");
    }
}
