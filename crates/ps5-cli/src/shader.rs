use std::path::Path;

use crate::cli::OutputFormat;
use crate::util::write_to_output_or_stdout;

pub(crate) fn cmd_shader(path: &Path, format: OutputFormat, output: &Option<std::path::PathBuf>) {
    let shaders = collect_shaders(path);
    match format {
        OutputFormat::Json => {
            write_to_output_or_stdout(output, &|w| {
                let json = serde_json::to_string_pretty(&shaders).unwrap();
                writeln!(w, "{json}")
            });
        }
        _ => {
            write_to_output_or_stdout(output, &|w| {
                writeln!(
                    w,
                    "Shader inventory: {} files from {}",
                    shaders.len(),
                    path.display()
                )?;
                let mut by_stage: std::collections::HashMap<String, usize> =
                    std::collections::HashMap::new();
                let mut total_bytes = 0u64;
                for s in &shaders {
                    *by_stage.entry(s.stage.clone()).or_insert(0) += 1;
                    total_bytes += s.size as u64;
                }
                writeln!(
                    w,
                    "Total bytes: {} ({:.1} MB)",
                    total_bytes,
                    total_bytes as f64 / 1048576.0
                )?;
                writeln!(w, "By stage:")?;
                for (stage, count) in &by_stage {
                    writeln!(w, "  {:<12} {}", stage, count)?;
                }
                writeln!(w, "\nFiles (first 20):")?;
                for s in shaders.iter().take(20) {
                    writeln!(
                        w,
                        "  {:<30} stage={:<8} size={} hash={}",
                        s.path,
                        s.stage,
                        s.size,
                        &s.hash[..16]
                    )?;
                }
                if shaders.len() > 20 {
                    writeln!(
                        w,
                        "  ... and {} more (full list in JSON)",
                        shaders.len() - 20
                    )?;
                }
                if shaders.is_empty() {
                    writeln!(
                        w,
                        "No shader files (.pssl/.sb/.ags) found — bare content may still contain shaders under Content/Shaders, run with correct game dir"
                    )?;
                }
                Ok(())
            });
        }
    }
}

#[derive(serde::Serialize)]
struct ShaderEntry {
    path: String,
    stage: String,
    size: usize,
    hash: String,
}

fn collect_shaders(root: &Path) -> Vec<ShaderEntry> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    let is_file = root.is_file();
    if is_file {
        if let Some(entry) = parse_shader_file(root, root) {
            out.push(entry);
        }
        return out;
    }
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if let Some(entry) = parse_shader_file(&path, root) {
                out.push(entry);
            }
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

fn parse_shader_file(path: &Path, root: &Path) -> Option<ShaderEntry> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !matches!(ext.as_str(), "pssl" | "sb" | "ags" | "agsd") {
        return None;
    }
    let data = std::fs::read(path).ok()?;
    let shader = ps5_shader::ShaderBinary::parse_with_path(&data, path).ok()?;
    let rel = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    Some(ShaderEntry {
        path: rel,
        stage: shader.stage.as_str().to_string(),
        size: shader.size,
        hash: shader.hash,
    })
}
