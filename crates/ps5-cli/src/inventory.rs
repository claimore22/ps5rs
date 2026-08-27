use std::path::Path;

use ps5_analysis::artifacts::{inventory_corpus, inventory_game};

use crate::cli::OutputFormat;
use crate::util::write_to_output_or_stdout;

pub(crate) fn cmd_inventory(
    path: &Path,
    format: OutputFormat,
    output: &Option<std::path::PathBuf>,
) {
    let report = if path.is_file() {
        // single file not meaningful for inventory; treat parent as game dir
        let dir = path.parent().unwrap_or(path);
        let game = inventory_game(dir);
        ps5_analysis::artifacts::ArtifactReport {
            games: vec![game],
            total_games: 1,
            total_files: 0,
            by_extension: std::collections::HashMap::new(),
            by_category: std::collections::HashMap::new(),
        }
    } else if path.join("eboot.bin").exists() {
        let game = inventory_game(path);
        ps5_analysis::artifacts::ArtifactReport {
            games: vec![game.clone()],
            total_games: 1,
            total_files: game.total_files,
            by_extension: game.by_extension.clone(),
            by_category: game.by_category.clone(),
        }
    } else {
        inventory_corpus(path)
    };

    match format {
        OutputFormat::Json => {
            write_to_output_or_stdout(output, &|w| {
                let json = serde_json::to_string_pretty(&report).unwrap();
                writeln!(w, "{json}")
            });
        }
        _ => {
            write_to_output_or_stdout(output, &|w| {
                writeln!(
                    w,
                    "Artifact inventory: {} games, {} files",
                    report.total_games, report.total_files
                )?;
                writeln!(w, "\nBy extension (top):")?;
                let mut exts: Vec<_> = report.by_extension.iter().collect();
                exts.sort_by_key(|(_, c)| std::cmp::Reverse(**c));
                for (ext, count) in exts.iter().take(20) {
                    writeln!(w, "  {:<12} {}", ext, count)?;
                }
                writeln!(w, "\nBy category:")?;
                let mut cats: Vec<_> = report.by_category.iter().collect();
                cats.sort_by_key(|(_, c)| std::cmp::Reverse(**c));
                for (cat, count) in cats {
                    writeln!(w, "  {:<12} {}", cat, count)?;
                }
                for game in &report.games {
                    writeln!(
                        w,
                        "\n{} ({} files, {} bytes)",
                        game.game, game.total_files, game.total_bytes
                    )?;
                    let mut exts: Vec<_> = game.by_extension.iter().collect();
                    exts.sort_by_key(|(_, c)| std::cmp::Reverse(**c));
                    for (ext, count) in exts.iter().take(15) {
                        writeln!(w, "  {:<12} {}", ext, count)?;
                    }
                    // sample relative paths
                    writeln!(w, "  sample paths:")?;
                    for art in game.artifacts.iter().take(10) {
                        writeln!(
                            w,
                            "    {} [{}:{}]",
                            art.relative_path,
                            art.category.as_str(),
                            art.extension
                        )?;
                    }
                    if game.artifacts.len() > 10 {
                        writeln!(w, "    ... and {} more", game.artifacts.len() - 10)?;
                    }
                    // cross-artifact hint: count prx/pssl/sb/gnf/at9 per game
                    let prx = game.by_extension.get("prx").copied().unwrap_or(0)
                        + game.by_extension.get("sprx").copied().unwrap_or(0);
                    let shader = game.by_extension.get("pssl").copied().unwrap_or(0)
                        + game.by_extension.get("sb").copied().unwrap_or(0);
                    let gnf = game.by_extension.get("gnf").copied().unwrap_or(0);
                    let at9 = game.by_extension.get("at9").copied().unwrap_or(0);
                    writeln!(
                        w,
                        "  cross-artifact: prx/sprx={} pssl/sb={} gnf={} at9={}",
                        prx, shader, gnf, at9
                    )?;
                }
                Ok(())
            });
        }
    }
}
