use super::layout::TABS;
use super::{PAGES, generate_html, generate_html_with_active_tab};

#[test]
fn every_page_has_a_known_tab() {
    for page in PAGES {
        assert!(
            TABS.iter().any(|t| t.id == page.tab),
            "unknown tab {} for {}",
            page.tab,
            page.file
        );
    }
}

#[test]
fn tab_ids_are_unique() {
    let mut seen = std::collections::HashSet::new();
    for tab in TABS {
        assert!(seen.insert(tab.id), "duplicate tab id {}", tab.id);
        //assert!(!tab.render().is_empty(), "empty markup for {}", tab.id);
    }
}

// Full HTML tests live in the parent crate (they need a real DashboardData fixture).
#[allow(dead_code)]
fn _api_exists() {
    let _ = generate_html as fn(&crate::data::DashboardData) -> String;
    let _ = generate_html_with_active_tab as fn(&crate::data::DashboardData, &str) -> String;
}
