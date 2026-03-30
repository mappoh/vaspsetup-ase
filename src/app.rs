//! Application event loop, state machine, and screen dispatch.
//!
//! Flow:
//!   Welcome → ChoosePath ─┬─→ QuickSubmit path: SubmitSetup → Confirm → Result
//!                          └─→ Calc path: CalcType → SpinMode → [Magmom]
//!                               → FilePick → EditIncar → EditKpoints
//!                               → SubmitSetup → Confirm → Result

use std::io;
use std::path::Path;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::Rect;
use ratatui::Frame;

use crate::config::Config;
use crate::python;
use crate::screens::result::JobResult;
use crate::screens::{self, ActiveScreen, ScreenAction};
use crate::shell;
use crate::state::{AppState, FlowPath, SpinMode, SpeciesInfo};

/// How the TUI exited — used by main to decide what summary to print.
pub enum ExitReason {
    Submitted(Vec<JobResult>),
    Cancelled,
}

pub struct App {
    pub state: AppState,
    pub config: Config,

    current_screen: ActiveScreen,
    screen_history: Vec<ActiveScreen>,

    // Screen instances
    welcome: screens::welcome::WelcomeScreen,
    choose_path: screens::choose_path::ChoosePathScreen,
    calc_type: screens::calc_type::CalcTypeScreen,
    tst_method: screens::tst_method::TstMethodScreen,
    spin_mode: screens::spin_mode::SpinModeScreen,
    magmom: Option<screens::magmom::MagmomScreen>,
    file_pick: Option<screens::file_pick::FilePickScreen>,
    edit_incar: screens::edit_incar::EditIncarScreen,
    edit_kpoints: screens::edit_kpoints::EditKpointsScreen,
    submit_setup: Option<screens::submit_setup::SubmitSetupScreen>,
    confirm: screens::confirm::ConfirmScreen,
    result: screens::result::ResultScreen,
}

impl App {
    pub fn new(state: AppState, config: Config) -> Self {
        Self {
            welcome: screens::welcome::WelcomeScreen::new(),
            choose_path: screens::choose_path::ChoosePathScreen::new(),
            calc_type: screens::calc_type::CalcTypeScreen::new(),
            tst_method: screens::tst_method::TstMethodScreen::new(),
            spin_mode: screens::spin_mode::SpinModeScreen::new(),
            magmom: None,
            file_pick: None,
            edit_incar: screens::edit_incar::EditIncarScreen::new(&state),
            edit_kpoints: screens::edit_kpoints::EditKpointsScreen::new(&state),
            submit_setup: None,
            confirm: screens::confirm::ConfirmScreen::new(),
            result: screens::result::ResultScreen::new(),
            state,
            config,
            current_screen: ActiveScreen::Welcome,
            screen_history: Vec::new(),
        }
    }

    /// Main event loop. Runs until the user quits or submission completes.
    pub fn run(&mut self, terminal: &mut ratatui::Terminal<impl ratatui::backend::Backend>) -> io::Result<ExitReason> {
        loop {
            terminal.draw(|frame| self.render(frame))?;

            match event::read()? {
                Event::Key(key) => {
                    // Only handle key press events (ignore release/repeat)
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }

                    let action = self.handle_input(key.code);
                    match action {
                        ScreenAction::Continue => {}
                        ScreenAction::Advance => self.advance(),
                        ScreenAction::Back => {
                            if let Some(reason) = self.go_back() {
                                return Ok(reason);
                            }
                        }
                        ScreenAction::Quit => {
                            if self.current_screen == ActiveScreen::Result {
                                let results = std::mem::take(&mut self.result.results);
                                return Ok(ExitReason::Submitted(results));
                            }
                            return Ok(ExitReason::Cancelled);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Render the current screen.
    fn render(&self, frame: &mut Frame) {
        let full = frame.area();
        let area = Rect {
            width: 80,
            height: full.height,
            ..full
        };

        match self.current_screen {
            ActiveScreen::Welcome => self.welcome.render(frame, area, &self.state),
            ActiveScreen::ChoosePath => self.choose_path.render(frame, area, &self.state),
            ActiveScreen::CalcType => self.calc_type.render(frame, area, &self.state),
            ActiveScreen::TstMethod => self.tst_method.render(frame, area, &self.state),
            ActiveScreen::SpinMode => self.spin_mode.render(frame, area, &self.state),
            ActiveScreen::Magmom => {
                if let Some(ref screen) = self.magmom {
                    screen.render(frame, area, &self.state);
                }
            }
            ActiveScreen::FilePick => {
                if let Some(ref screen) = self.file_pick {
                    screen.render(frame, area, &self.state);
                }
            }
            ActiveScreen::EditIncar => self.edit_incar.render(frame, area, &self.state),
            ActiveScreen::EditKpoints => self.edit_kpoints.render(frame, area, &self.state),
            ActiveScreen::SubmitSetup => {
                if let Some(ref screen) = self.submit_setup {
                    screen.render(frame, area, &self.state);
                }
            }
            ActiveScreen::Confirm => self.confirm.render(frame, area, &self.state),
            ActiveScreen::Result => self.result.render(frame, area, &self.state),
        }
    }

    /// Dispatch input to the current screen.
    fn handle_input(&mut self, code: KeyCode) -> ScreenAction {
        match self.current_screen {
            ActiveScreen::Welcome => self.welcome.handle_input(code, &mut self.state),
            ActiveScreen::ChoosePath => self.choose_path.handle_input(code, &mut self.state),
            ActiveScreen::CalcType => self.calc_type.handle_input(code, &mut self.state),
            ActiveScreen::TstMethod => self.tst_method.handle_input(code, &mut self.state),
            ActiveScreen::SpinMode => self.spin_mode.handle_input(code, &mut self.state),
            ActiveScreen::Magmom => {
                if let Some(ref mut screen) = self.magmom {
                    screen.handle_input(code, &mut self.state)
                } else {
                    ScreenAction::Continue
                }
            }
            ActiveScreen::FilePick => {
                if let Some(ref mut screen) = self.file_pick {
                    screen.handle_input(code, &mut self.state)
                } else {
                    ScreenAction::Continue
                }
            }
            ActiveScreen::EditIncar => self.edit_incar.handle_input(code, &mut self.state),
            ActiveScreen::EditKpoints => self.edit_kpoints.handle_input(code, &mut self.state),
            ActiveScreen::SubmitSetup => {
                if let Some(ref mut screen) = self.submit_setup {
                    screen.handle_input(code, &mut self.state)
                } else {
                    ScreenAction::Continue
                }
            }
            ActiveScreen::Confirm => self.confirm.handle_input(code, &mut self.state),
            ActiveScreen::Result => self.result.handle_input(code, &mut self.state),
        }
    }

    /// Advance to the next screen in the flow.
    fn advance(&mut self) {
        if let Some(next) = self.next_screen() {
            // Run transition actions before switching
            self.run_transition(&next);

            // Push current screen onto history
            self.screen_history.push(self.current_screen.clone());
            self.current_screen = next.clone();

            // Update step counter
            self.update_step_counter();

            // Post-transition: execute submissions when arriving at Result
            if next == ActiveScreen::Result {
                self.execute_submissions();
            }
        }
    }

    /// Go back to the previous screen. Returns `Some(ExitReason)` if we should exit.
    fn go_back(&mut self) -> Option<ExitReason> {
        if let Some(prev) = self.screen_history.pop() {
            // Clear stale state when backing out of certain screens
            match self.current_screen {
                ActiveScreen::TstMethod => {
                    self.state.tst_method = None;
                }
                _ => {}
            }
            self.current_screen = prev;
            self.update_step_counter();
            None
        } else {
            // Back on first screen = quit
            Some(ExitReason::Cancelled)
        }
    }

    /// Determine the next screen based on current screen and state.
    fn next_screen(&self) -> Option<ActiveScreen> {
        match self.current_screen {
            ActiveScreen::Welcome => Some(ActiveScreen::ChoosePath),
            ActiveScreen::ChoosePath => match self.state.flow_path {
                Some(FlowPath::QuickSubmit) => Some(ActiveScreen::SubmitSetup),
                Some(FlowPath::PerformCalculation) => Some(ActiveScreen::CalcType),
                None => None,
            },
            ActiveScreen::CalcType => {
                if self.state.calc_type == Some(crate::state::CalcType::Tst) {
                    Some(ActiveScreen::TstMethod)
                } else {
                    Some(ActiveScreen::SpinMode)
                }
            }
            ActiveScreen::TstMethod => Some(ActiveScreen::SpinMode),
            ActiveScreen::SpinMode => match self.state.spin_mode {
                Some(SpinMode::Unrestricted) => Some(ActiveScreen::Magmom),
                _ => {
                    // TST methods skip file selection (operates on prepared directory)
                    if self.is_tst_flow() {
                        Some(ActiveScreen::EditIncar)
                    } else {
                        Some(ActiveScreen::FilePick)
                    }
                }
            },
            ActiveScreen::Magmom => {
                if self.is_tst_flow() {
                    Some(ActiveScreen::EditIncar)
                } else {
                    Some(ActiveScreen::FilePick)
                }
            }
            ActiveScreen::FilePick => Some(ActiveScreen::EditIncar),
            ActiveScreen::EditIncar => {
                // TST methods skip KPOINTS (comes from optimized structures)
                if self.is_tst_flow() {
                    Some(ActiveScreen::SubmitSetup)
                } else {
                    Some(ActiveScreen::EditKpoints)
                }
            }
            ActiveScreen::EditKpoints => Some(ActiveScreen::SubmitSetup),
            ActiveScreen::SubmitSetup => {
                if self.state.flow_path == Some(FlowPath::QuickSubmit) {
                    Some(ActiveScreen::Result)
                } else {
                    Some(ActiveScreen::Confirm)
                }
            }
            ActiveScreen::Confirm => Some(ActiveScreen::Result),
            ActiveScreen::Result => {
                None // Result screen handles quit directly
            }
        }
    }

    /// Run transition actions when moving to a new screen.
    fn run_transition(&mut self, next: &ActiveScreen) {
        match next {
            ActiveScreen::TstMethod => {
                // Don't load preset yet — wait until TstMethod is chosen
            }
            ActiveScreen::SpinMode => {
                // Load preset for the selected calc type (or TST method)
                self.load_preset();
                // For TST with NEB/CI-NEB: auto-detect IMAGES from subdirectories
                if self.is_tst_flow() {
                    if let Some(ref method) = self.state.tst_method {
                        if method.uses_images() {
                            if let Some(images) = self.state.detect_images() {
                                self.state
                                    .incar_params
                                    .insert("IMAGES".to_string(), serde_json::json!(images));
                            }
                        }
                    }
                }
            }
            ActiveScreen::Magmom => {
                // Load atom info from the first file (for species/MAGMOM setup)
                // This happens before FilePick — user will select files next
                self.load_atom_info();
                let count = self.state.species.len();
                self.magmom = Some(screens::magmom::MagmomScreen::new(count));
            }
            ActiveScreen::FilePick => {
                let count = self.state.files.len();
                self.file_pick = Some(screens::file_pick::FilePickScreen::new(count, &self.state.files));
            }
            ActiveScreen::EditIncar => {
                // Always reload atom info from the selected file to ensure
                // species data matches what the user actually selected in FilePick
                self.load_atom_info_for_selected();
                // Set ISPIN=2 for unrestricted spin (after preset loaded, before showing INCAR)
                if self.state.spin_mode == Some(SpinMode::Unrestricted) {
                    self.state.incar_params.insert("ISPIN".to_string(), serde_json::json!(2));
                }
                self.edit_incar = screens::edit_incar::EditIncarScreen::new(&self.state);
                self.state.error = None; // Clear after screen picks it up
            }
            ActiveScreen::EditKpoints => {
                self.edit_kpoints = screens::edit_kpoints::EditKpointsScreen::new(&self.state);
            }
            ActiveScreen::SubmitSetup => {
                self.submit_setup =
                    Some(screens::submit_setup::SubmitSetupScreen::new(&self.state));
            }
            _ => {}
        }
    }

    /// Whether the current flow is a TST method (NEB/CI-NEB/Dimer — skip FilePick and KPOINTS).
    fn is_tst_flow(&self) -> bool {
        self.state.calc_type == Some(crate::state::CalcType::Tst)
    }

    /// Load VASP preset parameters into state.
    fn load_preset(&mut self) {
        let preset_name = self.state.effective_preset_name().to_string();

        match python::call_python(
            "preset",
            serde_json::json!({"name": preset_name}),
        ) {
            Ok(data) => {
                // Convert JSON object to HashMap
                if let Some(obj) = data.as_object() {
                    self.state.incar_params.clear();
                    for (k, v) in obj {
                        self.state.incar_params.insert(k.clone(), v.clone());
                    }
                }
            }
            Err(e) => {
                self.state.error = Some(format!("Failed to load preset '{}': {}", preset_name, e));
            }
        }
    }

    /// Load atom info from the first selected file, or first available file.
    fn load_atom_info_for_selected(&mut self) {
        let file_idx = self.state.selected_files.first().copied().unwrap_or(0);
        if let Some(file) = self.state.files.get(file_idx) {
            let path = self.state.work_dir.join(file).to_string_lossy().to_string();
            self.load_atom_info_from_path(&path);
        }
    }

    /// Load atom info from the first available structure file.
    fn load_atom_info(&mut self) {
        if let Some(file) = self.state.files.first() {
            let path = self.state.work_dir.join(file).to_string_lossy().to_string();
            self.load_atom_info_from_path(&path);
        }
    }

    /// Load atom info from a specific file path.
    fn load_atom_info_from_path(&mut self, file_path: &str) {
        match python::call_python("atoms", serde_json::json!({"file": file_path})) {
            Ok(data) => {
                if let Some(species_arr) = data.get("species").and_then(|s| s.as_array()) {
                    self.state.species = species_arr
                        .iter()
                        .filter_map(|sp| {
                            let symbol = sp.get("symbol")?.as_str()?.to_string();
                            let count = sp.get("count")?.as_u64()? as usize;
                            Some(SpeciesInfo { symbol, count })
                        })
                        .collect();
                }
            }
            Err(e) => {
                eprintln!("Failed to load atom info: {}", e);
            }
        }
    }

    /// Execute the full submission pipeline. Called after advancing to Result screen.
    pub fn execute_submissions(&mut self) {
        let is_quick_submit = self.state.flow_path == Some(FlowPath::QuickSubmit);

        // For QuickSubmit: submit from the output directory (user already has input files)
        // For PerformCalculation: process each selected file
        if is_quick_submit {
            let mut results = Vec::new();
            self.execute_quick_submit(&mut results);
            self.result.results = results;
            return;
        }

        let file_indices = self.state.selected_files.clone();

        // Pre-compute shared state for all threads
        let base_output_dir = &self.state.output_dir;
        let queue = &self.state.queue;
        let cores = self.state.cores;
        let parallel_env = &self.config.cluster.parallel_env;
        let vasp_module = &self.config.cluster.vasp_module;
        let vasp_binary = self.state.vasp_binary();
        let kpoints = self.state.kpoints.to_vec();
        let mut params = self.state.incar_params.clone();
        if let Some(magmom) = self.state.magmom_string() {
            params.insert("MAGMOM".to_string(), serde_json::Value::String(magmom));
        }


        // Prepare per-job data (job_name, file_path, output_dir)
        let jobs: Vec<_> = file_indices
            .iter()
            .enumerate()
            .map(|(i, &file_idx)| {
                let job_name = self
                    .state
                    .job_names
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| format!("vaspsetup_{:02}", i + 1));

                // Always create a subdirectory per job (e.g., tn-lp-09/)
                let output_dir = format!("{}/{}", base_output_dir, job_name);

                let file_path = self
                    .state
                    .files
                    .get(file_idx)
                    .map(|f| {
                        self.state
                            .work_dir
                            .join(f)
                            .to_string_lossy()
                            .to_string()
                    })
                    .unwrap_or_default();

                (job_name, file_path, output_dir)
            })
            .collect();

        // Submit all jobs in parallel using scoped threads
        let mut results = Vec::with_capacity(jobs.len());

        std::thread::scope(|s| {
            let handles: Vec<_> = jobs
                .iter()
                .map(|(job_name, file_path, output_dir)| {
                    let params = params.clone();
                    let kpoints = kpoints.clone();
                    s.spawn(move || {
                        // Write VASP input files via Python
                        if let Err(e) = python::call_python(
                            "write",
                            serde_json::json!({
                                "file": file_path,
                                "output_dir": output_dir,
                                "params": params,
                                "kpts": kpoints,
                            }),
                        ) {
                            return JobResult::error(
                                job_name.clone(),
                                format!("Failed to write input files: {}", e),
                            );
                        }

                        Self::submit_job_script(
                            job_name, queue, cores, parallel_env, vasp_module,
                            output_dir, vasp_binary,
                        )
                    })
                })
                .collect();

            // Collect results in order (catch panics gracefully)
            for (i, handle) in handles.into_iter().enumerate() {
                match handle.join() {
                    Ok(result) => results.push(result),
                    Err(_) => results.push(JobResult::error(
                        jobs[i].0.clone(),
                        "Internal error: submission thread panicked".to_string(),
                    )),
                }
            }
        });

        self.result.results = results;
    }

    /// Quick submit: render script and qsub from the output directory.
    fn execute_quick_submit(&self, results: &mut Vec<JobResult>) {
        let job_name = self
            .state
            .job_names
            .first()
            .cloned()
            .unwrap_or_else(|| "vaspsetup_job".to_string());

        let output_dir = if self.state.output_dir.is_empty() {
            self.state.work_dir.to_string_lossy().to_string()
        } else {
            self.state.output_dir.clone()
        };

        results.push(Self::submit_job_script(
            &job_name,
            &self.state.queue,
            self.state.cores,
            &self.config.cluster.parallel_env,
            &self.config.cluster.vasp_module,
            &output_dir,
            self.state.vasp_binary(),
        ));
    }

    /// Render SGE script, write it to disk, and submit via qsub.
    fn submit_job_script(
        job_name: &str,
        queue: &str,
        cores: u32,
        parallel_env: &str,
        vasp_module: &str,
        output_dir: &str,
        vasp_binary: &str,
    ) -> JobResult {
        let script_content = match shell::render_sge_script(
            job_name, queue, cores, parallel_env, vasp_module, vasp_binary,
        ) {
            Ok(content) => content,
            Err(e) => {
                return JobResult::error(
                    job_name.to_string(),
                    format!("Script rendering failed: {}", e),
                );
            }
        };

        let output_path = Path::new(output_dir);
        if let Err(e) = std::fs::create_dir_all(output_path) {
            return JobResult::error(
                job_name.to_string(),
                format!("Failed to create output directory: {}", e),
            );
        }

        let script_path = match shell::write_submission_script(output_path, &script_content) {
            Ok(path) => path,
            Err(e) => {
                return JobResult::error(
                    job_name.to_string(),
                    format!("Failed to write script: {}", e),
                );
            }
        };

        let qsub_result = shell::run_qsub(&script_path, output_path);
        JobResult {
            job_name: job_name.to_string(),
            success: qsub_result.success,
            job_id: qsub_result.job_id,
            message: qsub_result.message,
        }
    }

    /// Update the step counter based on current screen position.
    fn update_step_counter(&mut self) {
        let step = match self.current_screen {
            ActiveScreen::Welcome | ActiveScreen::ChoosePath => 0,
            ActiveScreen::Confirm | ActiveScreen::Result => 0, // use labels instead
            _ => {
                // Count steps from after ChoosePath
                self.screen_history
                    .iter()
                    .filter(|s| {
                        !matches!(s, ActiveScreen::Welcome | ActiveScreen::ChoosePath)
                    })
                    .count() as u32
                    + 1
            }
        };
        self.state.current_step = step;
    }
}
