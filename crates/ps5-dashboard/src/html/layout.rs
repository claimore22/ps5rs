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

const DETAIL_PANEL: &str = r#"<div class="detail-overlay" id="detailPanel">
<div class="detail-header"><h2 id="detailTitle">Detail</h2><button class="detail-close" id="detailClose">&times; Close</button></div>
<div class="detail-body" id="detailBody"></div>
</div>"#;
