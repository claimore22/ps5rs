//! HTML dashboard generator.
//!
//! Split by concern:
//! - [`layout`] — document shell, tab bar, page assembly
//! - [`tabs`] — one module per tab’s markup
//! - [`style`] — CSS
//! - [`js`] — client script (receives `const D = <json>`)

use crate::data::DashboardData;

mod helpers;
mod js;
mod layout;
mod style;
pub mod tabs;

/// One output HTML file and the tab it should open on.
pub struct DashboardPage {
    pub file: &'static str,
    pub tab: &'static str,
}

/// All generated pages. `deps.html` is kept as an alias of the graph tab.
pub const PAGES: &[DashboardPage] = &[
    DashboardPage {
        file: "index.html",
        tab: "overview",
    },
    DashboardPage {
        file: "games.html",
        tab: "games",
    },
    DashboardPage {
        file: "engines.html",
        tab: "engines",
    },
    DashboardPage {
        file: "libraries.html",
        tab: "libraries",
    },
    DashboardPage {
        file: "nids.html",
        tab: "nids",
    },
    DashboardPage {
        file: "segments.html",
        tab: "segments",
    },
    DashboardPage {
        file: "statistics.html",
        tab: "statistics",
    },
    DashboardPage {
        file: "graph.html",
        tab: "graph",
    },
    DashboardPage {
        file: "deps.html",
        tab: "graph",
    },
    DashboardPage {
        file: "loader.html",
        tab: "loader",
    },
    DashboardPage {
        file: "middleware.html",
        tab: "middleware",
    },
    DashboardPage {
        file: "artifacts.html",
        tab: "artifacts",
    },
    DashboardPage {
        file: "sdk.html",
        tab: "sdk",
    },
    DashboardPage {
        file: "shader.html",
        tab: "shader",
    },
    DashboardPage {
        file: "firmware.html",
        tab: "firmware",
    },
];

pub fn generate_dashboard_pages(
    data: &DashboardData,
    out_dir: &std::path::Path,
) -> std::io::Result<()> {
    std::fs::create_dir_all(out_dir)?;
    for page in PAGES {
        let html = generate_html_with_active_tab(data, page.tab);
        std::fs::write(out_dir.join(page.file), html)?;
    }
    for game in &data.game_details {
        let sanitized = sanitize_filename(&game.name);
        let game_dir = out_dir.join("games").join(sanitized);
        std::fs::create_dir_all(&game_dir)?;
        let html = layout::render_game_page(data, game);
        std::fs::write(game_dir.join("index.html"), html)?;
    }
    Ok(())
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub fn generate_html_with_active_tab(data: &DashboardData, active_tab: &str) -> String {
    layout::render_page(data, active_tab)
}

/// Full dashboard HTML with the Overview tab active.
pub fn generate_html(data: &DashboardData) -> String {
    layout::render_page(data, "overview")
}

#[cfg(test)]
mod tests;
