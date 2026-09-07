//! Document shell: header, tab bar, content panes, detail overlay, script boot.

use crate::data::DashboardData;

use super::js::JS;
use super::style::CSS;
use super::tabs;

pub struct TabSpec {
    pub id: &'static str,
    pub label: &'static str,
    /// Extra attributes on the tab button (hidden tabs, stable DOM ids).
    pub button_attrs: &'static str,
    pub render: fn() -> &'static str,
}

pub const TABS: &[TabSpec] = &[
    TabSpec {
        id: "overview",
        label: "Overview",
        button_attrs: "",
        render: tabs::overview::render,
    },
    TabSpec {
        id: "games",
        label: "Games",
        button_attrs: "",
        render: tabs::games::render,
    },
    TabSpec {
        id: "engines",
        label: "Engines",
        button_attrs: "",
        render: tabs::engines::render,
    },
    TabSpec {
        id: "libraries",
        label: "Libraries",
        button_attrs: "",
        render: tabs::libraries::render,
    },
    TabSpec {
        id: "nids",
        label: "NIDs",
        button_attrs: "",
        render: tabs::nids::render,
    },
    TabSpec {
        id: "segments",
        label: "Segments",
        button_attrs: "",
        render: tabs::segments::render,
    },
    TabSpec {
        id: "statistics",
        label: "Statistics",
        button_attrs: "",
        render: tabs::statistics::render,
    },
    TabSpec {
        id: "graph",
        label: "Graph",
        button_attrs: "",
        render: tabs::graph::render,
    },
    TabSpec {
        id: "loader",
        label: "Load Coverage",
        button_attrs: r#" id="loaderTab" style="display:none""#,
        render: tabs::loader::render,
    },
    TabSpec {
        id: "middleware",
        label: "Middleware",
        button_attrs: r#" id="middlewareTab" style="display:none""#,
        render: tabs::middleware::render,
    },
    TabSpec {
        id: "artifacts",
        label: "Artifacts",
        button_attrs: r#" id="artifactTab" style="display:none""#,
        render: tabs::artifacts::render,
    },
    TabSpec {
        id: "sdk",
        label: "SDK Timeline",
        button_attrs: "",
        render: tabs::sdk::render,
    },
    TabSpec {
        id: "shader",
        label: "Shaders",
        button_attrs: "",
        render: tabs::shader::render,
    },
    TabSpec {
        id: "firmware",
        label: "Firmware",
        button_attrs: "",
        render: tabs::firmware::render,
    },
];

fn dashboard_json(data: &DashboardData) -> String {
    serde_json::to_string(data).unwrap_or_else(|_| "{}".to_string())
}

pub fn render_page(data: &DashboardData, active_tab: &str) -> String {
    let active = if TABS.iter().any(|t| t.id == active_tab) {
        active_tab
    } else {
        "overview"
    };

    let json = dashboard_json(data);
    let mut html = String::with_capacity(96 * 1024);

    html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    html.push_str("<meta charset=\"UTF-8\">\n");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
    html.push_str("<title>PS5rs Dashboard</title>\n<style>");
    html.push_str(CSS);
    html.push_str("</style>\n</head>\n<body>\n\n");

    push_header(&mut html, data);
    html.push_str("\n<div class=\"layout\">\n");
    push_sidebar(&mut html, active);
    html.push_str("\n<div class=\"main\">\n<div class=\"container\">\n");
    push_tab_contents(&mut html, active);
    html.push_str("\n</div>\n</div>\n</div>\n\n");
    html.push_str(DETAIL_PANEL);
    html.push_str("\n<script>\nconst D = ");
    html.push_str(&json);
    html.push_str(";\n");
    html.push_str(JS);
    html.push_str("\n</script>\n</body>\n</html>");
    html
}

fn push_header(html: &mut String, data: &DashboardData) {
    html.push_str(
        r#"<div class="header">
<h1>PS5rs</h1>
<span class="subtitle">Generated "#,
    );
    html.push_str(&data.meta.generated_at.to_string());
    html.push_str(" &middot; ");
    html.push_str(&data.meta.game_count.to_string());
    html.push_str(" games &middot; v");
    html.push_str(&data.meta.tool_version.to_string());
    html.push_str(r#"</span>
<div class="search-wrap">
<input type="text" class="search-box" id="globalSearch" placeholder="Search NIDs, libraries, games..." autocomplete="off">
<div class="search-results" id="searchResults"></div>
</div>
</div>
"#);
}

fn push_sidebar(html: &mut String, active: &str) {
    html.push_str(r#"<nav class="sidebar" id="sidebar">"#);
    html.push_str(r#"<div class="sidebar-title">PS5rs</div>"#);
    for tab in TABS {
        // strip display:none so sidebar shows all items docs.rs style; JS will still toggle active
        let attrs = tab
            .button_attrs
            .replace(r#" style="display:none""#, "")
            .replace(r#" style='display:none'"#, "");
        html.push_str(r#"<div class="sidebar-item tab"#);
        if tab.id == active {
            html.push_str(" active");
        }
        html.push_str(r#"" data-tab=""#);
        html.push_str(tab.id);
        html.push('"');
        html.push_str(&attrs);
        html.push('>');
        html.push_str(tab.label);
        html.push_str("</div>\n");
    }
    html.push_str("</nav>");
}

#[allow(dead_code)]
fn push_tab_bar(html: &mut String, active: &str) {
    html.push_str(
        r#"<div class="tabs" id="tabBar">
"#,
    );
    for tab in TABS {
        html.push_str(r#"<div class="tab"#);
        if tab.id == active {
            html.push_str(" active");
        }
        html.push_str(r#"" data-tab=""#);
        html.push_str(tab.id);
        html.push('"');
        html.push_str(tab.button_attrs);
        html.push('>');
        html.push_str(tab.label);
        html.push_str("</div>\n");
    }
    html.push_str("</div>\n");
}

fn push_tab_contents(html: &mut String, active: &str) {
    for tab in TABS {
        html.push_str(r#"<div class="tab-content"#);
        if tab.id == active {
            html.push_str(" active");
        }
        html.push_str(r#"" id="tab-"#);
        html.push_str(tab.id);
        html.push_str(
            r#"">
"#,
        );
        html.push_str((tab.render)());
        html.push_str("\n</div>\n");
    }
}

pub fn render_game_page(data: &DashboardData, game: &crate::data::GameDetail) -> String {
    let json = dashboard_json(data);
    let mut html = String::with_capacity(96 * 1024);
    html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    html.push_str("<meta charset=\"UTF-8\">\n");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
    html.push_str("<title>");
    html.push_str(&game.title_name.clone().unwrap_or_else(|| game.name.clone()));
    html.push_str(" — PS5rs</title>\n<style>");
    html.push_str(CSS);
    html.push_str("</style>\n</head>\n<body>\n\n");
    push_header(&mut html, data);
    html.push_str("\n<div class=\"layout\">\n");
    push_sidebar(&mut html, "games");
    html.push_str("\n<div class=\"main\">\n<div class=\"container\">\n");
    html.push_str(&render_game_detail_html(game, data));
    html.push_str("\n</div>\n</div>\n</div>\n\n");
    html.push_str(DETAIL_PANEL);
    html.push_str("\n<script>\nconst D = ");
    html.push_str(&json);
    html.push_str(";\n");
    html.push_str(JS);
    html.push_str("\n</script>\n</body>\n</html>");
    html
}

fn render_game_detail_html(game: &crate::data::GameDetail, data: &DashboardData) -> String {
    let mut out = String::new();
    out.push_str(r#"<a href="../../index.html" style="color:#58a6ff;font-size:0.85rem">&larr; Back to Games</a>"#);
    out.push_str(&format!(
        r#"<h1 style="margin:12px 0 4px">{}</h1><p style="color:#8b949e;font-size:0.85rem">{} &middot; {} &middot; {:.1} MB</p>"#,
        game.title_name.clone().unwrap_or_else(|| game.name.clone()),
        game.name,
        game.platform,
        game.file_size_mb
    ));
    out.push_str(&format!(
        r#"<div class="cards"><div class="card"><div class="card-label">Engine</div><div class="card-value">{}</div></div><div class="card"><div class="card-label">Confidence</div><div class="card-value">{}</div></div><div class="card"><div class="card-label">SCE Libs</div><div class="card-value">{}</div></div><div class="card"><div class="card-label">Unknown NIDs</div><div class="card-value">{}</div></div></div>"#,
        game.engine,
        game.engine_confidence,
        game.sce_libraries.len(),
        game.unresolved_nids.len()
    ));
    out.push_str(r#"<div class="section"><h2>Identity</h2><div class="detail-kv">"#);
    out.push_str(&format!(r#"<div class="k">SHA-256</div><div class="v" style="font-family:monospace;font-size:0.72rem">{}</div>"#, game.sha256));
    out.push_str(&format!(r#"<div class="k">Entry Point</div><div class="v" style="font-family:monospace">{}</div>"#, game.entry_point));
    out.push_str(&format!(r#"<div class="k">Build ID</div><div class="v" style="font-family:monospace;font-size:0.72rem">{}</div>"#, game.build_id.clone().unwrap_or_else(|| "N/A".to_string())));
    out.push_str(&format!(r#"<div class="k">ELF Type</div><div class="v">0x{:x}</div>"#, game.elf_type));
    out.push_str("</div></div>");
    out.push_str(r#"<div class="section"><h2>Engine Forensics</h2><div class="detail-kv">"#);
    out.push_str(&format!(r#"<div class="k">Engine</div><div class="v">{}</div><div class="k">Score</div><div class="v">{}</div><div class="k">Confidence</div><div class="v">{}%</div>"#, game.engine, game.engine_score, game.engine_confidence));
    if let Some(bs) = &game.build_system {
        out.push_str(&format!(r#"<div class="k">Build System</div><div class="v">{}</div>"#, bs));
    }
    if let Some(sd) = &game.source_depot {
        out.push_str(&format!(r#"<div class="k">Source Depot</div><div class="v">{}</div>"#, sd));
    }
    out.push_str("</div>");
    if !game.engine_evidence.is_empty() {
        out.push_str(r#"<h3 style="margin-top:12px">Evidence</h3><div class="table-wrap"><table class="detail-table"><thead><tr><th>String</th></tr></thead><tbody>"#);
        for ev in &game.engine_evidence {
            out.push_str(&format!(r#"<tr><td style="font-family:monospace;font-size:0.72rem">{}</td></tr>"#, ev));
        }
        out.push_str("</tbody></table></div>");
    }
    out.push_str("</div>");
    if !game.lib_versions.is_empty() {
        out.push_str(r#"<div class="section"><h2>SDK Library Versions</h2><div class="table-wrap"><table class="detail-table"><thead><tr><th>Library</th><th>Version</th></tr></thead><tbody>"#);
        for lv in &game.lib_versions {
            out.push_str(&format!(r#"<tr><td style="font-family:monospace">{}</td><td>{}</td></tr>"#, lv.name, lv.version_string));
        }
        out.push_str("</tbody></table></div></div>");
    }
    out.push_str(r#"<div class="section"><h2>Segments</h2><div class="table-wrap"><table class="detail-table"><thead><tr><th>#</th><th>Type</th><th>VAddr</th><th>Size</th></tr></thead><tbody>"#);
    for seg in &game.segments {
        out.push_str(&format!(r#"<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>"#, seg.index, seg.seg_type, seg.vaddr, seg.filesz));
    }
    out.push_str("</tbody></table></div></div>");
    out.push_str(r#"<div class="section"><h2>Libraries</h2><div class="table-wrap"><table class="detail-table"><thead><tr><th>Library</th><th>Imports</th></tr></thead><tbody>"#);
    for lib in &game.import_summary {
        out.push_str(&format!(r#"<tr><td>{}</td><td>{}</td></tr>"#, lib.library, lib.count));
    }
    out.push_str("</tbody></table></div></div>");
    let shader_count = data.shaders.iter().filter(|s| s.game == game.name).count();
    if shader_count > 0 {
        out.push_str(&format!(r#"<div class="section"><h2>Shaders ({} records)</h2><p style="color:#8b949e;font-size:0.82rem">From ShaderArchive/GlobalShaderCache/sb and Unity assets</p><div class="table-wrap"><table class="detail-table"><thead><tr><th>Stage</th><th>Format</th><th>Size</th></tr></thead><tbody>"#, shader_count));
        for s in data.shaders.iter().filter(|s| s.game == game.name).take(20) {
            out.push_str(&format!(r#"<tr><td>{}</td><td>{}</td><td>{}</td></tr>"#, s.stage, s.format, s.size));
        }
        out.push_str("</tbody></table></div></div>");
    }
    if let Some(fw) = data.firmware_checks.iter().find(|f| f.game == game.name) {
        out.push_str(r#"<div class="section"><h2>Firmware Compatibility</h2><div class="table-wrap"><table class="detail-table"><thead><tr><th>Library</th><th>Required</th><th>Status</th></tr></thead><tbody>"#);
        for c in &fw.checks {
            out.push_str(&format!(r#"<tr><td>{}</td><td>{}</td><td>{}</td></tr>"#, c.library, c.required, c.status));
        }
        out.push_str("</tbody></table></div></div>");
    }
    if !game.unresolved_nids.is_empty() {
        out.push_str(&format!(r#"<div class="section"><h2>Unknown NIDs ({} shown)</h2><div class="table-wrap"><table class="detail-table"><thead><tr><th>NID</th><th>Library</th></tr></thead><tbody>"#, game.unresolved_nids.len().min(50)));
        for nid in game.unresolved_nids.iter().take(50) {
            out.push_str(&format!(r#"<tr><td style="font-family:monospace">{}</td><td>{}</td></tr>"#, nid.nid_hash, nid.library_name));
        }
        out.push_str("</tbody></table></div></div>");
    }
    out
}

const DETAIL_PANEL: &str = r#"<div class="detail-overlay" id="detailPanel">
<div class="detail-header"><h2 id="detailTitle">Detail</h2><button class="detail-close" id="detailClose">&times; Close</button></div>
<div class="detail-body" id="detailBody"></div>
</div>"#;
