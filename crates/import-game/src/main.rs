use eframe::{NativeOptions, egui};
use rfd::FileDialog;
use std::path::PathBuf;
use std::sync::mpsc;

fn ps5rs_command() -> std::process::Command {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            #[cfg(windows)]
            let name = "ps5rs.exe";
            #[cfg(not(windows))]
            let name = "ps5rs";
            let cand = dir.join(name);
            if cand.is_file() {
                return std::process::Command::new(cand);
            }
        }
    }
    let mut probe = std::process::Command::new("ps5rs");
    probe.arg("--version");
    if probe.output().map(|o| o.status.success()).unwrap_or(false) {
        return std::process::Command::new("ps5rs");
    }
    let mut fallback = std::process::Command::new("cargo");
    fallback.args(["run", "--release", "-p", "ps5-cli", "--"]);
    fallback
}
fn main() -> eframe::Result<()> {
    let options = NativeOptions::default();
    // Force a light theme via egui visuals (set later in UI)

    eframe::run_native(
        "ps5rs – Import Game",
        options,
        Box::new(|_cc| Ok(Box::new(App::default()))),
    )
}

struct App {
    game_path: Option<PathBuf>,
    dataset_path: Option<PathBuf>,
    offline_dir: Option<PathBuf>,
    status: String,
    importing: bool,
    progress_rx: Option<mpsc::Receiver<String>>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            game_path: None,
            dataset_path: Some(PathBuf::from("analysis_with_modules")),
            offline_dir: Some(PathBuf::from("analysis_with_modules")),
            status: String::new(),
            importing: false,
            progress_rx: None,
        }
    }
}
impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        // white background for the window
        [1.0, 1.0, 1.0, 1.0]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Use a light theme with explicit black text
        let mut visuals = egui::Visuals::light();
        visuals.override_text_color = Some(egui::Color32::BLACK);
        ui.ctx().set_visuals(visuals);

        // Add padding around the whole UI
        egui::Frame::default()
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.heading("Import a PS5 Game");
                ui.add_space(4.0);
                ui.label("Choose the required directories and click **Import**.");
                ui.add_space(8.0);
                egui::Grid::new("import_grid")
                    .spacing([12.0, 8.0])
                    .num_columns(3)
                    .show(ui, |ui| {
                        // Game folder (must contain eboot.bin)
                        ui.label("Game folder");
                        if ui.button("Browse…").clicked() {
                            if let Some(path) = FileDialog::new().pick_folder() {
                                self.game_path = Some(path);
                            }
                        }
                        ui.label(self.game_path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "<none>".to_owned()));
                        ui.end_row();

                        // Dataset root (analysis/dataset)
                        ui.label("Dataset root");
                        if ui.button("Browse…").clicked() {
                            if let Some(path) = FileDialog::new().pick_folder() {
                                self.dataset_path = Some(path);
                            }
                        }
                        ui.label(self.dataset_path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "<none>".to_owned()));
                        ui.end_row();

                        // Offline exports directory (system_modules)
                        ui.label("Offline exports");
                        if ui.button("Browse…").clicked() {
                            if let Some(path) = FileDialog::new().pick_folder() {
                                self.offline_dir = Some(path);
                            }
                        }
                        ui.label(self.offline_dir.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "<none>".to_owned()));

                    });
                    // Explanations for each field (shown after the grid)
                    ui.add_space(4.0);
                    ui.label("Game folder – directory containing the game’s eboot.bin");
                    ui.label("Dataset root – typically the ‘analysis/dataset’ directory under the repo root (e.g., C:/Users/claimoar/Documents/Rust/ps5rs/analysis/dataset)");
                    ui.label("Offline exports – directory with pre‑generated .exports.json files (system_modules), usually C:/Users/claimoar/Documents/Rust/ps5rs/system_modules");
                    ui.add_space(8.0);
                    // Progress UI will be inserted here later
                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(6.0);

                if ui.button("Import").clicked() {
                    if let (Some(game), Some(dataset), Some(offline)) = (
                        &self.game_path,
                        &self.dataset_path,
                        &self.offline_dir,
                    ) {
                            let game = game.clone();
                            let dataset = dataset.clone();
                            let _offline = offline.clone();
// Scan the picked folder's parent so the game directory itself is
// discovered (scan roots never match their own eboot.bin). `--append`
// keeps every already-ingested game untouched.
                            let scan_root = game
                                .parent()
                                .map(|p| p.to_path_buf())
                                .unwrap_or_else(|| game.clone());
                            let mut cmd = ps5rs_command();
                            let scan_out = cmd
                                .args([
                                    "scan",
                                    scan_root.to_string_lossy().as_ref(),
                                    "--output",
                                    dataset.to_string_lossy().as_ref(),
                                    "--append",
                                ])
                                .output();
                         match scan_out {
                             Ok(res) => {
                                 // Print command output for debugging
                                 println!("{}", String::from_utf8_lossy(&res.stdout));
                                 eprintln!("{}", String::from_utf8_lossy(&res.stderr));
                                 if res.status.success() {
                                     self.status = "Import completed successfully".to_string();
                                 } else {
                                     self.status = format!("Scan failed for {}", game.display());
                                 }
                             }
                             Err(e) => {
                                 self.status = format!("Failed to start scan command: {}", e);
                             }
                         }

                    } else {
                        self.status = "Please fill all fields".to_string();
                    }
                }
                ui.add_space(4.0);
                ui.label(&self.status);
            });
    }
}
