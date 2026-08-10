use crate::catalog::load_catalog;
use crate::cli::OutputFormat;
use crate::util::write_to_output_or_stdout;
use ps5_analysis::{GameMiddlewareReport, MiddlewareModule, MiddlewareReport};
use std::path::PathBuf;

pub(crate) fn cmd_middleware(
    path: &std::path::Path,
    format: OutputFormat,
    output: &Option<PathBuf>,
) {
    let catalog = load_catalog(&[]);
    eprintln!("Scanning {} for third-party middleware...", path.display());
    let report = ps5_analysis::build_middleware_report(path, &catalog);
    eprintln!(
        "Scanned {} games, {} modules ({} third-party, {} Sony, {} unknown)",
        report.games.len(),
        report.total_prx,
        report.third_party_modules,
        report.sony_modules,
        report.unknown_modules
    );

    match format {
        OutputFormat::Terminal => {
            let text = render_middleware_terminal(&report);
            write_to_output_or_stdout(output, &|w| w.write_all(text.as_bytes()))
        }
        OutputFormat::Json => write_to_output_or_stdout(output, &|w| {
            let json = serde_json::to_string_pretty(&report).unwrap();
            writeln!(w, "{json}")
        }),
        _ => {
            eprintln!(
                "error: unsupported format for middleware (use --format terminal or --format json)"
            );
            std::process::exit(1);
        }
    }
}

pub(crate) fn render_middleware_terminal(report: &MiddlewareReport) -> String {
    let mut out = String::new();

    for game in &report.games {
        render_game(&mut out, game);
    }

    out.push_str(&format!(
        "Summary: {} games, {} modules — {} third-party, {} Sony, {} unknown\n",
        report.games.len(),
        report.total_prx,
        report.third_party_modules,
        report.sony_modules,
        report.unknown_modules
    ));
    out
}

fn render_game(out: &mut String, game: &GameMiddlewareReport) {
    let title = game
        .title_id
        .as_deref()
        .map(|t| format!(" [{t}]"))
        .unwrap_or_default();
    let engine = game
        .engine
        .as_deref()
        .map(|e| format!(" — {e}"))
        .unwrap_or_default();
    out.push_str(&format!("{}{}{}\n", game.game, title, engine));

    out.push_str(&format!("  Third-party ({})\n", game.third_party.len()));
    for module in &game.third_party {
        render_module(out, module, "3RD");
    }

    out.push_str(&format!("  Sony system ({})\n", game.sony.len()));
    for module in &game.sony {
        render_module(out, module, "SCE");
    }

    if !game.unknown.is_empty() {
        out.push_str(&format!("  Unidentified ({})\n", game.unknown.len()));
        for module in &game.unknown {
            render_module(out, module, "?");
        }
    }
    out.push('\n');
}

fn render_module(out: &mut String, module: &MiddlewareModule, label: &str) {
    let mut line = format!("    [{label}] {:<36}", module.file_name);
    if let (Some(vendor), Some(product)) = (module.vendor.as_deref(), module.product.as_deref()) {
        line.push_str(&format!(" {vendor} — {product}"));
        if let Some(description) = module.description.as_deref() {
            line.push_str(&format!(" ({description})"));
        }
    } else if module.imports == 0 && !module.parseable {
        line.push_str(" — could not parse");
    }
    if module.imports > 0 {
        line.push_str(&format!(" — {} imports", module.imports));
    }
    line.push('\n');
    out.push_str(&line);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ps5_analysis::ModuleKind;

    #[test]
    fn render_groups_and_vendors() {
        let report = MiddlewareReport {
            games: vec![GameMiddlewareReport {
                game: "TestGame-PPSA12345-PS5".to_string(),
                title_id: Some("PPSA12345".to_string()),
                engine: Some("Unreal Engine".to_string()),
                third_party: vec![MiddlewareModule {
                    file_name: "libfmod.prx".to_string(),
                    module_name: "libfmod".to_string(),
                    kind: ModuleKind::ThirdParty,
                    sha256: None,
                    vendor: Some("Firelight Technologies".to_string()),
                    product: Some("FMOD".to_string()),
                    description: Some("FMOD low-level audio engine".to_string()),
                    parseable: true,
                    imports: 144,
                    exports: 1091,
                    import_libs: vec!["libkernel".to_string()],
                    needed_files: vec![],
                }],
                sony: vec![MiddlewareModule {
                    file_name: "libSceFace.prx".to_string(),
                    module_name: "libSceFace".to_string(),
                    kind: ModuleKind::Sony,
                    sha256: None,
                    vendor: None,
                    product: None,
                    description: None,
                    parseable: true,
                    imports: 12,
                    exports: 30,
                    import_libs: vec![],
                    needed_files: vec![],
                }],
                unknown: vec![MiddlewareModule {
                    file_name: "mystery.prx".to_string(),
                    module_name: "mystery".to_string(),
                    kind: ModuleKind::Unknown,
                    sha256: None,
                    vendor: None,
                    product: None,
                    description: None,
                    parseable: false,
                    imports: 0,
                    exports: 0,
                    import_libs: vec![],
                    needed_files: vec![],
                }],
            }],
            total_prx: 3,
            third_party_modules: 1,
            sony_modules: 1,
            unknown_modules: 1,
        };

        let text = render_middleware_terminal(&report);
        assert!(text.contains("TestGame-PPSA12345-PS5 [PPSA12345] — Unreal Engine"));
        assert!(text.contains("Firelight Technologies — FMOD"));
        assert!(text.contains("[SCE]"));
        assert!(text.contains("[3RD]"));
        assert!(text.contains("could not parse"));
        assert!(text.contains("Summary: 1 games, 3 modules — 1 third-party, 1 Sony, 1 unknown"));
    }
}
