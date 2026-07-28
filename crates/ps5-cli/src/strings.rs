use std::path::PathBuf;

use crate::util::{load_file, write_to_output_or_stdout};

pub(crate) fn cmd_strings(
    path: &PathBuf,
    min_length: u8,
    offsets: bool,
    detect: bool,
    output: &Option<PathBuf>,
) {
    let data = load_file(path);

    if detect {
        let analysis = ps5_analysis::string_patterns::analyze_strings(&data);
        write_to_output_or_stdout(output, &|w| {
            writeln!(w, "String analysis of {}", path.display())?;
            writeln!(w)?;

            if !analysis.sce_libraries.is_empty() {
                writeln!(w, "SCE Libraries:")?;
                for lib in &analysis.sce_libraries {
                    writeln!(w, "  {lib}")?;
                }
                writeln!(w)?;
            }

            if let Some(ref engine) = analysis.engine {
                writeln!(
                    w,
                    "Engine: {} (confidence: {}%)",
                    engine.value, engine.confidence
                )?;
                for e in &engine.evidence {
                    writeln!(w, "  {e}")?;
                }
                writeln!(w)?;
            }

            if let Some(ref bs) = analysis.build_system {
                writeln!(w, "Build System: {}", bs.value)?;
                for e in &bs.evidence {
                    writeln!(w, "  {e}")?;
                }
                writeln!(w)?;
            }

            if let Some(ref depot) = analysis.source_depot {
                writeln!(w, "Source Depot: {}", depot.value)?;
                for e in &depot.evidence {
                    writeln!(w, "  {e}")?;
                }
                writeln!(w)?;
            }

            if !analysis.third_party_libs.is_empty() {
                writeln!(w, "Third-party Libraries:")?;
                for lib in &analysis.third_party_libs {
                    writeln!(w, "  {}: {:?}", lib.value, lib.evidence)?;
                }
                writeln!(w)?;
            }

            if !analysis.sdk_hints.is_empty() {
                writeln!(w, "SDK Hints:")?;
                for sdk in &analysis.sdk_hints {
                    writeln!(w, "  {}: {:?}", sdk.value, sdk.evidence)?;
                }
                writeln!(w)?;
            }

            if !analysis.detected_versions.is_empty() {
                writeln!(w, "Detected Versions:")?;
                for ver in &analysis.detected_versions {
                    writeln!(w, "  {}: {:?}", ver.value, ver.evidence)?;
                }
                writeln!(w)?;
            }

            if !analysis.source_paths.is_empty() {
                writeln!(w, "Source Paths ({}):", analysis.source_paths.len())?;
                for p in &analysis.source_paths {
                    writeln!(w, "  {p}")?;
                }
                writeln!(w)?;
            }

            if !analysis.project_paths.is_empty() {
                writeln!(w, "Project Paths:")?;
                for pp in &analysis.project_paths {
                    writeln!(w, "  {}: {:?}", pp.value, pp.evidence)?;
                }
                writeln!(w)?;
            }

            if !analysis.custom_forks.is_empty() {
                writeln!(w, "Custom Forks:")?;
                for cf in &analysis.custom_forks {
                    writeln!(w, "  {} (confidence: {}%)", cf.value, cf.confidence)?;
                    for e in &cf.evidence {
                        writeln!(w, "    {e}")?;
                    }
                }
            }

            Ok(())
        });
        return;
    }

    let min = min_length as usize;

    write_to_output_or_stdout(output, &|w| {
        let mut start: Option<usize> = None;

        for (i, &byte) in data.iter().enumerate() {
            let printable = byte.is_ascii_graphic() || byte == b' ';
            if printable {
                if start.is_none() {
                    start = Some(i);
                }
            } else if let Some(s) = start {
                if i - s >= min
                    && let Ok(string) = std::str::from_utf8(&data[s..i])
                {
                    if offsets {
                        writeln!(w, "{:08x} {string}", s)?;
                    } else {
                        writeln!(w, "{string}")?;
                    }
                }
                start = None;
            }
        }

        if let Some(s) = start
            && data.len() - s >= min
            && let Ok(string) = std::str::from_utf8(&data[s..])
        {
            if offsets {
                writeln!(w, "{:08x} {string}", s)?;
            } else {
                writeln!(w, "{string}")?;
            }
        }

        Ok(())
    });
}
