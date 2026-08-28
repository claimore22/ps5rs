//! Dashboard CSS (GitHub-dark forensic console).

pub const CSS: &str = r##"
:root {
  --bg: #0d1117;
  --surface: #161b22;
  --elev: #21262d;
  --border: #30363d;
  --text: #e6edf3;
  --muted: #8b949e;
  --accent: #58a6ff;
  --green: #3fb950;
  --red: #f85149;
  --yellow: #d29922;
  --purple: #8957e5;
  --blue: #1f6feb;
}
* { box-sizing: border-box; }
html, body {
  margin: 0; padding: 0;
  background: var(--bg); color: var(--text);
  font-family: ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif;
  font-size: 14px; line-height: 1.5;
}
.header {
  display: flex; align-items: center; gap: 16px; flex-wrap: wrap;
  padding: 16px 24px; border-bottom: 1px solid var(--border);
  background: var(--surface); position: sticky; top: 0; z-index: 20;
}
.header h1 { margin: 0; font-size: 1.35rem; letter-spacing: -0.03em; }
.subtitle { color: var(--muted); font-size: 0.82rem; }
.search-wrap { margin-left: auto; position: relative; min-width: 240px; flex: 1; max-width: 420px; }
.search-box {
  width: 100%; background: var(--bg); border: 1px solid var(--border); color: var(--text);
  border-radius: 8px; padding: 8px 12px; font-size: 0.88rem;
}
.search-box:focus { outline: 1px solid var(--accent); border-color: var(--accent); }
.search-results {
  display: none; position: absolute; top: 110%; left: 0; right: 0;
  background: var(--surface); border: 1px solid var(--border); border-radius: 8px;
  max-height: 360px; overflow: auto; z-index: 30;
}
.search-results.show { display: block; }
.sr-item { padding: 8px 12px; cursor: pointer; border-bottom: 1px solid var(--border); }
.sr-item:hover { background: var(--elev); }
.sr-type { font-size: 0.68rem; color: var(--muted); text-transform: uppercase; letter-spacing: 0.04em; }
.sr-name { font-size: 0.88rem; }
.sr-detail { font-size: 0.75rem; color: var(--muted); }
.tabs {
  display: flex; gap: 2px; overflow-x: auto; padding: 0 16px;
  border-bottom: 1px solid var(--border); background: var(--surface);
}
.tab {
  padding: 10px 14px; cursor: pointer; color: var(--muted); white-space: nowrap;
  border-bottom: 2px solid transparent; font-size: 0.85rem;
}
.tab:hover { color: var(--text); }
.tab.active { color: var(--text); border-bottom-color: var(--accent); }
.container { padding: 20px 24px 64px; max-width: 1400px; margin: 0 auto; }
.tab-content { display: none; }
.tab-content.active { display: block; }
.cards { display: grid; grid-template-columns: repeat(auto-fill, minmax(160px, 1fr)); gap: 12px; margin-bottom: 20px; }
.card {
  background: var(--surface); border: 1px solid var(--border); border-radius: 10px; padding: 14px 16px;
}
.card-label { font-size: 0.72rem; color: var(--muted); text-transform: uppercase; letter-spacing: 0.04em; }
.card-value { font-size: 1.35rem; font-weight: 650; margin-top: 4px; font-variant-numeric: tabular-nums; }
.card-value.blue { color: var(--accent); }
.card-value.green { color: var(--green); }
.card-value.yellow { color: var(--yellow); }
.section { margin: 22px 0; }
.section h2 { font-size: 1rem; margin: 0 0 12px; }
.filter-bar { display: flex; gap: 8px; flex-wrap: wrap; margin-bottom: 12px; }
.filter-input, .filter-select {
  background: var(--surface); border: 1px solid var(--border); color: var(--text);
  border-radius: 8px; padding: 8px 10px; font-size: 0.85rem;
}
.table-wrap { overflow: auto; border: 1px solid var(--border); border-radius: 10px; }
table { width: 100%; border-collapse: collapse; }
th, td { padding: 8px 12px; text-align: left; border-bottom: 1px solid var(--border); }
th { color: var(--muted); font-size: 0.75rem; cursor: pointer; user-select: none; background: var(--surface); }
th.sorted { color: var(--accent); }
tr.clickable { cursor: pointer; }
tr.clickable:hover { background: var(--elev); }
.arrow { font-size: 0.65rem; opacity: 0.6; }
.pill {
  display: inline-block; padding: 1px 8px; border-radius: 999px; font-size: 0.72rem; font-weight: 600;
  border: 1px solid var(--border);
}
.pill-eng { background: #388bfd22; color: var(--accent); border-color: #388bfd44; }
.pill-self { background: #23863622; color: var(--green); border-color: #23863644; }
.pill-elf { background: #d2992222; color: var(--yellow); border-color: #d2992244; }
.pct-high { color: var(--green); }
.pct-med { color: var(--yellow); }
.pct-low { color: var(--red); }
.hbar { display: flex; align-items: center; gap: 10px; margin-bottom: 6px; }
.hbar-label { width: 180px; flex-shrink: 0; font-size: 0.78rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.hbar-track { flex: 1; height: 14px; background: var(--elev); border-radius: 4px; overflow: hidden; }
.hbar-fill { height: 100%; }
.hbar-count { width: 140px; text-align: right; font-size: 0.75rem; color: var(--muted); font-variant-numeric: tabular-nums; }
.fill-green { background: var(--green); }
.fill-red { background: var(--red); }
.fill-blue { background: var(--blue); }
.fill-yellow { background: var(--yellow); }
.fill-purple { background: var(--purple); }
.heatmap-wrap { overflow: auto; }
.heatmap { border-collapse: collapse; font-size: 0.7rem; }
.heatmap th, .heatmap td { padding: 2px; min-width: 18px; height: 18px; }
.heatmap .lib-name { color: var(--muted); padding-right: 8px; white-space: nowrap; }
.legend { display: flex; gap: 16px; font-size: 0.75rem; color: var(--muted); margin-top: 8px; }
.l-rx::before, .l-r::before, .l-rw::before, .l-other::before {
  content: ""; display: inline-block; width: 10px; height: 10px; border-radius: 2px; margin-right: 6px; vertical-align: -1px;
}
.l-rx::before { background: #1f6feb; }
.l-r::before { background: #3fb950; }
.l-rw::before { background: #d29922; }
.l-other::before { background: #6e7681; }
.seg-bar { display: flex; height: 14px; border-radius: 4px; overflow: hidden; flex: 1; }
.seg-rx { background: #1f6feb; }
.seg-r { background: #3fb950; }
.seg-rw { background: #d29922; }
.seg-other { background: #6e7681; }
.stat-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(240px, 1fr)); gap: 12px; }
.stat-card { background: var(--surface); border: 1px solid var(--border); border-radius: 10px; padding: 14px; }
.stat-card h3 { margin: 0 0 8px; font-size: 0.85rem; }
.stat-row { display: flex; justify-content: space-between; gap: 8px; font-size: 0.82rem; padding: 3px 0; color: var(--muted); }
.stat-row .sv { color: var(--text); font-variant-numeric: tabular-nums; }
.graph-wrap { overflow: auto; background: var(--surface); border: 1px solid var(--border); border-radius: 10px; }
.detail-overlay {
  position: fixed; top: 0; right: 0; width: min(560px, 100%); height: 100%;
  background: var(--surface); border-left: 1px solid var(--border);
  transform: translateX(100%); transition: transform 180ms ease; z-index: 40;
  display: flex; flex-direction: column;
}
.detail-overlay.open { transform: translateX(0); }
.detail-header {
  display: flex; align-items: center; justify-content: space-between;
  padding: 14px 16px; border-bottom: 1px solid var(--border);
}
.detail-header h2 { margin: 0; font-size: 1rem; }
.detail-close {
  background: transparent; border: 1px solid var(--border); color: var(--text);
  border-radius: 8px; padding: 6px 10px;
}
.detail-body { overflow: auto; padding: 16px; }
.detail-section { margin-bottom: 18px; }
.detail-section h3 { margin: 0 0 8px; font-size: 0.85rem; }
.detail-kv { display: grid; grid-template-columns: 140px 1fr; gap: 6px 10px; font-size: 0.85rem; }
.detail-kv .k { color: var(--muted); }
.detail-table { font-size: 0.82rem; }
code { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; }
@media (max-width: 640px) {
  .header { padding: 12px; }
  .container { padding: 12px 12px 48px; }
  .hbar-label { width: 110px; }
  .search-wrap { max-width: none; margin-left: 0; width: 100%; }
  .detail-overlay { width: 100%; }
}
"##;
