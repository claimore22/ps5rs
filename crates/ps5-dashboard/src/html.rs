use crate::data::DashboardData;

pub fn generate_html(data: &DashboardData) -> String {
    let json = serde_json::to_string(data).unwrap_or_default();
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>PS5rs Dashboard</title>
<style>
*{{margin:0;padding:0;box-sizing:border-box;}}
body{{background:#0d1117;color:#c9d1d9;font-family:'Segoe UI',system-ui,-apple-system,sans-serif;line-height:1.5;}}
a{{color:#58a6ff;text-decoration:none;}}
a:hover{{text-decoration:underline;}}
.header{{background:#161b22;border-bottom:1px solid #30363d;padding:12px 24px;display:flex;align-items:center;gap:16px;position:sticky;top:0;z-index:100;}}
.header h1{{font-size:1.2rem;color:#58a6ff;white-space:nowrap;}}
.header .subtitle{{color:#8b949e;font-size:0.78rem;}}
.search-box{{background:#0d1117;border:1px solid #30363d;color:#c9d1d9;padding:6px 12px;border-radius:6px;width:280px;font-size:0.85rem;margin-left:auto;}}
.search-box:focus{{outline:none;border-color:#58a6ff;}}
.tabs{{background:#161b22;border-bottom:1px solid #30363d;display:flex;gap:0;padding:0 24px;overflow-x:auto;position:sticky;top:49px;z-index:99;}}
.tab{{padding:10px 16px;cursor:pointer;color:#8b949e;font-size:0.85rem;border-bottom:2px solid transparent;white-space:nowrap;transition:all 0.15s;}}
.tab:hover{{color:#c9d1d9;}}
.tab.active{{color:#58a6ff;border-bottom-color:#58a6ff;}}
.container{{max-width:1400px;margin:0 auto;padding:24px;}}
.tab-content{{display:none;}}
.tab-content.active{{display:block;}}
.cards{{display:grid;grid-template-columns:repeat(auto-fit,minmax(180px,1fr));gap:12px;margin-bottom:24px;}}
.card{{background:#161b22;border:1px solid #30363d;border-radius:8px;padding:16px;}}
.card-label{{color:#8b949e;font-size:0.75rem;text-transform:uppercase;letter-spacing:0.05em;}}
.card-value{{font-size:1.5rem;font-weight:600;color:#e6edf3;margin-top:4px;}}
.card-value.green{{color:#3fb950;}}
.card-value.blue{{color:#58a6ff;}}
.card-value.yellow{{color:#d29922;}}
.section{{background:#161b22;border:1px solid #30363d;border-radius:8px;padding:20px;margin-bottom:20px;}}
.section h2{{font-size:1.05rem;color:#e6edf3;margin-bottom:12px;border-bottom:1px solid #21262d;padding-bottom:8px;}}
.table-wrap{{max-height:500px;overflow-y:auto;}}
table{{width:100%;border-collapse:collapse;font-size:0.82rem;}}
th{{background:#0d1117;color:#8b949e;text-align:left;padding:8px 10px;cursor:pointer;user-select:none;position:sticky;top:0;}}
th:hover{{color:#58a6ff;}}
th .arrow{{font-size:0.65rem;margin-left:4px;color:#484f58;}}
th.sorted .arrow{{color:#58a6ff;}}
td{{padding:6px 10px;border-top:1px solid #21262d;}}
tr:hover{{background:#1c2128;}}
tr.clickable{{cursor:pointer;}}
tr.clickable:hover{{background:#1c2128;outline:1px solid #30363d;}}
.pct{{font-variant-numeric:tabular-nums;}}
.pct-high{{color:#3fb950;}}.pct-med{{color:#d29922;}}.pct-low{{color:#f85149;}}
.pill{{display:inline-block;padding:1px 8px;border-radius:10px;font-size:0.72rem;font-weight:500;}}
.pill-self{{background:#1f6feb22;color:#58a6ff;border:1px solid #1f6feb44;}}
.pill-elf{{background:#23863622;color:#3fb950;border:1px solid #23863644;}}
.pill-eng{{background:#d2992222;color:#d29922;border:1px solid #d2992244;}}
.filter-bar{{display:flex;gap:8px;margin-bottom:12px;flex-wrap:wrap;align-items:center;}}
.filter-input,.filter-select{{background:#0d1117;border:1px solid #30363d;color:#c9d1d9;padding:6px 12px;border-radius:6px;font-size:0.82rem;}}
.filter-input:focus,.filter-select:focus{{outline:none;border-color:#58a6ff;}}
.filter-select{{cursor:pointer;}}
.hbar{{display:flex;align-items:center;margin-bottom:4px;}}
.hbar-label{{width:260px;font-size:0.78rem;color:#c9d1d9;text-overflow:ellipsis;overflow:hidden;white-space:nowrap;display:flex;align-items:center;}}
.hbar-track{{flex:1;height:14px;background:#21262d;border-radius:3px;overflow:hidden;}}
.hbar-fill{{height:100%;border-radius:3px;min-width:1px;transition:width 0.3s;}}
.hbar-count{{width:60px;text-align:right;font-size:0.75rem;color:#8b949e;margin-left:6px;}}
.fill-blue{{background:linear-gradient(90deg,#1f6feb,#58a6ff);}}
.fill-green{{background:linear-gradient(90deg,#238636,#3fb950);}}
.fill-red{{background:linear-gradient(90deg,#da3633,#f85149);}}
.fill-yellow{{background:linear-gradient(90deg,#9e6a03,#d29922);}}
.fill-purple{{background:linear-gradient(90deg,#8957e5,#bc8cff);}}
.heatmap-wrap{{overflow-x:auto;}}
.heatmap{{border-collapse:collapse;font-size:0.7rem;}}
.heatmap th{{position:static;padding:4px 6px;white-space:nowrap;writing-mode:horizontal-tb;cursor:default;}}
.heatmap th:hover{{color:#8b949e;}}
.heatmap td{{width:28px;height:28px;padding:0;text-align:center;font-size:0;cursor:default;}}
.heatmap td:hover{{outline:2px solid #58a6ff;position:relative;z-index:1;}}
.heatmap .lib-name{{text-align:right;padding-right:8px;white-space:nowrap;color:#c9d1d9;}}
.seg-bar{{display:flex;height:18px;border-radius:3px;overflow:hidden;min-width:100px;}}
.seg-rx{{background:#238636;}}.seg-r{{background:#1f6feb;}}.seg-rw{{background:#d29922;}}.seg-other{{background:#484f58;}}
.legend{{display:flex;gap:16px;margin-top:8px;font-size:0.75rem;color:#8b949e;flex-wrap:wrap;}}
.legend span::before{{content:'';display:inline-block;width:10px;height:10px;border-radius:2px;margin-right:4px;vertical-align:middle;}}
.legend .l-rx::before{{background:#238636;}}.legend .l-r::before{{background:#1f6feb;}}
.legend .l-rw::before{{background:#d29922;}}.legend .l-other::before{{background:#484f58;}}
.detail-overlay{{position:fixed;top:0;right:0;width:600px;max-width:90vw;height:100vh;background:#161b22;border-left:1px solid #30363d;z-index:200;transform:translateX(100%);transition:transform 0.25s ease;overflow-y:auto;box-shadow:-4px 0 24px rgba(0,0,0,0.5);}}
.detail-overlay.open{{transform:translateX(0);}}
.detail-header{{position:sticky;top:0;background:#161b22;border-bottom:1px solid #30363d;padding:16px 20px;display:flex;justify-content:space-between;align-items:center;z-index:1;}}
.detail-header h2{{font-size:1rem;color:#e6edf3;}}
.detail-close{{background:none;border:1px solid #30363d;color:#8b949e;cursor:pointer;border-radius:4px;padding:4px 10px;font-size:0.85rem;}}
.detail-close:hover{{color:#c9d1d9;border-color:#8b949e;}}
.detail-body{{padding:20px;}}
.detail-section{{margin-bottom:20px;}}
.detail-section h3{{font-size:0.9rem;color:#58a6ff;margin-bottom:8px;}}
.detail-kv{{display:grid;grid-template-columns:140px 1fr;gap:4px 12px;font-size:0.82rem;}}
.detail-kv .k{{color:#8b949e;}}.detail-kv .v{{color:#c9d1d9;word-break:break-all;}}
.detail-table{{font-size:0.78rem;width:100%;}}
.detail-table td{{padding:4px 8px;border-top:1px solid #21262d;}}
.lib-pri td:nth-child(3),.lib-pri td:nth-child(4){{font-variant-numeric:tabular-nums;}}
.stat-grid{{display:grid;grid-template-columns:repeat(auto-fit,minmax(300px,1fr));gap:16px;}}
.stat-card{{background:#0d1117;border:1px solid #21262d;border-radius:6px;padding:14px;}}
.stat-card h3{{font-size:0.85rem;color:#58a6ff;margin-bottom:8px;}}
.stat-row{{display:flex;justify-content:space-between;font-size:0.8rem;padding:3px 0;border-bottom:1px solid #21262d22;}}
.stat-row .sv{{color:#e6edf3;font-variant-numeric:tabular-nums;}}
.graph-wrap{{overflow:auto;background:#0d1117;border:1px solid #21262d;border-radius:6px;padding:16px;}}
.search-results{{position:absolute;top:100%;left:0;right:0;background:#161b22;border:1px solid #30363d;border-radius:0 0 6px 6px;max-height:400px;overflow-y:auto;z-index:201;display:none;}}
.search-results.show{{display:block;}}
.sr-item{{padding:8px 12px;cursor:pointer;font-size:0.82rem;border-bottom:1px solid #21262d;}}
.sr-item:hover{{background:#1c2128;}}
.sr-item .sr-type{{color:#8b949e;font-size:0.72rem;}}
.sr-item .sr-name{{color:#e6edf3;}}
.sr-item .sr-detail{{color:#8b949e;font-size:0.78rem;}}
.search-wrap{{position:relative;}}
</style>
</head>
<body>

<div class="header">
<h1>PS5rs</h1>
<span class="subtitle">Generated {generated_at} &middot; {game_count} games &middot; v{tool_version}</span>
<div class="search-wrap">
<input type="text" class="search-box" id="globalSearch" placeholder="Search NIDs, libraries, games..." autocomplete="off">
<div class="search-results" id="searchResults"></div>
</div>
</div>

<div class="tabs" id="tabBar">
<div class="tab active" data-tab="overview">Overview</div>
<div class="tab" data-tab="games">Games</div>
<div class="tab" data-tab="engines">Engines</div>
<div class="tab" data-tab="libraries">Libraries</div>
<div class="tab" data-tab="nids">NIDs</div>
<div class="tab" data-tab="segments">Segments</div>
<div class="tab" data-tab="statistics">Statistics</div>
<div class="tab" data-tab="graph">Graph</div>
</div>

<div class="container">

<div class="tab-content active" id="tab-overview">
<div class="cards" id="overviewCards"></div>
<div class="section"><h2>NID Resolution</h2><div id="nidResBar"></div></div>
<div class="section"><h2>Platform Distribution</h2><div id="platformBar"></div></div>
<div class="section"><h2>Engine Distribution</h2><div id="engineBar"></div></div>
</div>

<div class="tab-content" id="tab-games">
<div class="filter-bar" id="gameFilters">
<input type="text" class="filter-input" id="gameFilterText" placeholder="Filter games...">
<select class="filter-select" id="filterPlatform"><option value="">All Platforms</option></select>
<select class="filter-select" id="filterEngine"><option value="">All Engines</option></select>
<select class="filter-select" id="filterSelf"><option value="">SELF + ELF</option><option value="self">SELF only</option><option value="elf">ELF only</option></select>
<select class="filter-select" id="filterHasUnknown"><option value="">All</option><option value="yes">Has Unknown NIDs</option><option value="no">No Unknown NIDs</option></select>
</div>
<div class="table-wrap"><table id="gamesTable">
<thead><tr>
<th data-col="0">Game <span class="arrow">&#9650;</span></th>
<th data-col="1">Engine <span class="arrow">&#9650;</span></th>
<th data-col="2">Confidence <span class="arrow">&#9650;</span></th>
<th data-col="3">Libraries <span class="arrow">&#9650;</span></th>
<th data-col="4">Unknown NIDs <span class="arrow">&#9650;</span></th>
<th data-col="5">Size MB <span class="arrow">&#9650;</span></th>
</tr></thead>
<tbody id="gamesBody"></tbody>
</table></div>
</div>

<div class="tab-content" id="tab-engines">
<div class="cards" id="engineOverviewCards"></div>
<div class="section"><h2>Engine Distribution</h2><div id="engineDistBars"></div></div>
<div class="section"><h2>Per-Game Engine Forensics</h2><div class="table-wrap"><table id="engineTable">
<thead><tr>
<th data-col="0">Game <span class="arrow">&#9650;</span></th>
<th data-col="1">Engine <span class="arrow">&#9650;</span></th>
<th data-col="2">Score <span class="arrow">&#9650;</span></th>
<th data-col="3">Confidence <span class="arrow">&#9650;</span></th>
<th data-col="4">Third-Party Libs <span class="arrow">&#9650;</span></th>
<th data-col="5">Custom Forks <span class="arrow">&#9650;</span></th>
<th data-col="6">Build System <span class="arrow">&#9650;</span></th>
</tr></thead>
<tbody id="engineBody"></tbody>
</table></div></div>
</div>

<div class="tab-content" id="tab-libraries">
<div class="table-wrap"><table id="libTable">
<thead><tr>
<th data-col="0">Library <span class="arrow">&#9650;</span></th>
<th data-col="1">Games <span class="arrow">&#9650;</span></th>
<th data-col="2">Imports <span class="arrow">&#9650;</span></th>
<th data-col="3">Unique NIDs <span class="arrow">&#9650;</span></th>
</tr></thead>
<tbody id="libBody"></tbody>
</table></div>
<div class="section" style="margin-top:20px"><h2>Library Heatmap (log2)</h2><div class="heatmap-wrap" id="heatmapWrap"></div></div>
</div>

<div class="tab-content" id="tab-nids">
<div class="section"><h2>NID Resolution</h2><div id="nidResDetail"></div></div>
<div class="section"><h2>Top 20 NIDs by Frequency</h2><div id="nidBars"></div></div>
<div class="section"><h2>Library NID Breakdown</h2><p style="color:#8b949e;font-size:0.82rem;margin-bottom:12px">Top 10 per library</p><div id="libNidBreakdown"></div></div>
</div>

<div class="tab-content" id="tab-segments">
<div class="section"><h2>Segment Sizes by Game</h2><div id="segBars"></div>
<div class="legend"><span class="l-rx">RX (code)</span><span class="l-r">R (rodata)</span><span class="l-rw">RW (data)</span><span class="l-other">Other</span></div></div>
</div>

<div class="tab-content" id="tab-statistics">
<div class="stat-grid" id="statGrid"></div>
</div>

<div class="tab-content" id="tab-graph">
<div class="section"><h2>Library Dependency Graph</h2><p style="color:#8b949e;font-size:0.82rem;margin-bottom:12px">Click a node to view details</p><div class="graph-wrap" id="graphWrap"></div></div>
</div>

</div>

<div class="detail-overlay" id="detailPanel">
<div class="detail-header"><h2 id="detailTitle">Detail</h2><button class="detail-close" id="detailClose">&times; Close</button></div>
<div class="detail-body" id="detailBody"></div>
</div>

<script>
const D = {json};
const $ = s => document.querySelector(s);
const $$ = s => document.querySelectorAll(s);
const pctCls = v => v >= 80 ? 'pct-high' : v >= 50 ? 'pct-med' : 'pct-low';
const fmt = v => typeof v === 'number' ? v.toLocaleString() : v;
const trunc = (s, n) => s && s.length > n ? s.slice(0, n-2) + '..' : s || '';

function showGameDetail(gameId) {{
  const d = (D.game_details || []).find(x => x.name === gameId);
  if (!d) return;
  const segsHtml = d.segments.map(s => `<tr><td>${{s.index}}</td><td>${{s.seg_type}}</td><td>${{s.vaddr}}</td><td>${{(s.filesz/1048576).toFixed(2)}} MB</td><td>${{s.flags}}</td></tr>`).join('');
  const libsHtml = d.import_summary.slice(0, 15).map(l => `<tr><td>${{l.library}}</td><td>${{fmt(l.count)}}</td></tr>`).join('');
  const importsHtml = d.imports.slice(0, 200).map(i => `<tr><td style="font-family:monospace;font-size:0.72rem">${{i.nid_hash}}</td><td>${{i.resolved_name||'<span style="color:#f85149">unknown</span>'}}</td><td style="color:#8b949e">${{i.library_name}}</td></tr>`).join('');
  const unresolvedHtml = d.unresolved_nids.slice(0, 100).map(i => `<tr><td style="font-family:monospace;font-size:0.72rem">${{i.nid_hash}}</td><td style="color:#8b949e">${{i.library_name}}</td></tr>`).join('');

  const engineHtml = `
    <div class="detail-section"><h3>Engine Forensics</h3><div class="detail-kv">
      <div class="k">Engine</div><div class="v">${{d.engine||'Unknown'}}</div>
      <div class="k">Score</div><div class="v" style="font-family:monospace">${{d.engine_score}}</div>
      <div class="k">Confidence</div><div class="v">${{d.engine_confidence}}%</div>
      ${{d.build_system ? `<div class="k">Build System</div><div class="v">${{d.build_system}}</div>` : ''}}
      ${{d.source_depot ? `<div class="k">Source Depot</div><div class="v">${{d.source_depot}}</div>` : ''}}
      ${{(d.sce_libraries||[]).length ? `<div class="k">SCE Libraries</div><div class="v">${{(d.sce_libraries||[]).length}} detected</div>` : ''}}
      ${{(d.third_party_libs||[]).length ? `<div class="k">Third-Party Libs</div><div class="v">${{(d.third_party_libs||[]).join(', ')}}</div>` : ''}}
      ${{(d.custom_forks||[]).length ? `<div class="k">Custom Forks</div><div class="v" style="color:#f85149">${{(d.custom_forks||[]).join(', ')}}</div>` : ''}}
      ${{(d.sdk_hints||[]).length ? `<div class="k">SDK Hints</div><div class="v">${{(d.sdk_hints||[]).join(', ')}}</div>` : ''}}
      ${{(d.detected_versions||[]).length ? `<div class="k">Versions</div><div class="v">${{(d.detected_versions||[]).join(', ')}}</div>` : ''}}
    </div></div>
    ${{(d.engine_evidence||[]).length ? `<div class="detail-section"><h3>Engine Evidence</h3><div class="table-wrap"><table class="detail-table"><thead><tr><th>String</th></tr></thead><tbody>${{(d.engine_evidence||[]).map(e => `<tr><td style="font-family:monospace;font-size:0.72rem">${{e}}</td></tr>`).join('')}}</tbody></table></div></div>` : ''}}
    ${{(d.lib_versions||[]).length ? `<div class="detail-section"><h3>SDK Library Versions</h3><div class="table-wrap"><table class="detail-table"><thead><tr><th>Library</th><th>Version</th><th>Raw</th></tr></thead><tbody>${{(d.lib_versions||[]).map(lv => `<tr><td style="font-family:monospace;font-size:0.78rem">${{lv.name}}</td><td style="font-variant-numeric:tabular-nums">${{lv.version_string}}</td><td style="font-family:monospace;font-size:0.72rem;color:#8b949e">0x${{lv.version_raw.toString(16).padStart(8,'0')}}</td></tr>`).join('')}}</tbody></table></div></div>` : ''}}`;

  openDetail(d.title_name || d.name, `
    <div class="detail-section"><h3>General</h3><div class="detail-kv">
      <div class="k">Name</div><div class="v">${{d.title_name || d.name}}</div>
      <div class="k">Platform</div><div class="v">${{d.platform}}</div>
      <div class="k">Type</div><div class="v">${{d.is_self?'SELF':'Raw ELF'}}</div>
      <div class="k">File Size</div><div class="v">${{d.file_size_mb.toFixed(1)}} MB</div>
      <div class="k">Entry Point</div><div class="v" style="font-family:monospace">${{d.entry_point}}</div>
      <div class="k">SHA-256</div><div class="v" style="font-family:monospace;font-size:0.72rem">${{d.sha256.slice(0,32)}}...</div>
    </div></div>
    ${{engineHtml}}
    <div class="detail-section"><h3>ELF Header</h3><div class="detail-kv">
      <div class="k">ELF Type</div><div class="v">0x${{d.elf_type.toString(16)}}</div>
      <div class="k">OS/ABI</div><div class="v">0x${{d.osabi.toString(16)}}</div>
      <div class="k">ABI Version</div><div class="v">${{d.abi_version}}</div>
      <div class="k">ELF Version</div><div class="v">${{d.elf_version}}</div>
      <div class="k">Build ID</div><div class="v" style="font-family:monospace;font-size:0.72rem">${{d.build_id||'N/A'}}</div>
      <div class="k">Relocations</div><div class="v">${{fmt(d.relocations)}}</div>
      <div class="k">TLS</div><div class="v">${{d.has_tls?'Yes':'No'}}</div>
    </div></div>
    <div class="detail-section"><h3>Segments (${{d.segments.length}})</h3>
      <div class="table-wrap"><table class="detail-table"><thead><tr><th>#</th><th>Type</th><th>VAddr</th><th>Size</th><th>Flags</th></tr></thead><tbody>${{segsHtml}}</tbody></table></div></div>
    <div class="detail-section"><h3>Libraries (${{d.import_summary.length}})</h3>
      <div class="table-wrap"><table class="detail-table"><thead><tr><th>Library</th><th>Imports</th></tr></thead><tbody>${{libsHtml}}</tbody></table></div></div>
    <div class="detail-section"><h3>Imports (${{d.imports.length}})</h3>
      <div class="table-wrap"><table class="detail-table"><thead><tr><th>NID Hash</th><th>Resolved Name</th><th>Library</th></tr></thead><tbody>${{importsHtml}}</tbody></table></div></div>
    ${{d.unresolved_nids.length > 0 ? `<div class="detail-section"><h3>Unknown NIDs (${{d.unresolved_nids.length}})</h3><div class="table-wrap"><table class="detail-table"><thead><tr><th>NID Hash</th><th>Library</th></tr></thead><tbody>${{unresolvedHtml}}</tbody></table></div></div>` : ''}}
  `);
}}

// --- TABS ---
$$('.tab').forEach(tab => tab.addEventListener('click', () => {{
  $$('.tab').forEach(t => t.classList.remove('active'));
  $$('.tab-content').forEach(t => t.classList.remove('active'));
  tab.classList.add('active');
  $(`#tab-${{tab.dataset.tab}}`).classList.add('active');
}}));

// --- DETAIL PANEL ---
function openDetail(title, html) {{
  $('#detailTitle').textContent = title;
  $('#detailBody').innerHTML = html;
  $('#detailPanel').classList.add('open');
}}
$('#detailClose').addEventListener('click', () => $('#detailPanel').classList.remove('open'));
document.addEventListener('keydown', e => {{ if (e.key === 'Escape') $('#detailPanel').classList.remove('open'); }});

// --- OVERVIEW ---
(function() {{
  const o = D.overview;
  $('#overviewCards').innerHTML = [
    ['Games', o.total_games, 'blue'],
    ['ELF Valid', o.elf_valid, 'green'],
    ['Total Imports', fmt(o.total_imports), ''],
    ['Unique NIDs', fmt(o.unique_nids), 'yellow'],
    ['Unique Libraries', o.unique_libs, ''],
    ['Resolution', o.resolution_rate.toFixed(1) + '%', 'green'],
    ['Avg Imports/Game', Math.round(o.avg_imports_per_game), ''],
  ].map(([l, v, c]) => `<div class="card"><div class="card-label">${{l}}</div><div class="card-value ${{c}}">${{v}}</div></div>`).join('');

  const ns = D.nid_stats;
  const total = ns.resolved_count + ns.unknown_count;
  const rPct = total > 0 ? (ns.resolved_count / total * 100) : 0;
  const uPct = 100 - rPct;
  $('#nidResBar').innerHTML = `
    <div class="hbar"><div class="hbar-label">Resolved</div><div class="hbar-track"><div class="hbar-fill fill-green" style="width:${{rPct.toFixed(1)}}%"></div></div><div class="hbar-count">${{ns.resolved_count.toLocaleString()}} (${{rPct.toFixed(1)}}%)</div></div>
    <div class="hbar"><div class="hbar-label">Unknown</div><div class="hbar-track"><div class="hbar-fill fill-red" style="width:${{uPct.toFixed(1)}}%"></div></div><div class="hbar-count">${{ns.unknown_count.toLocaleString()}} (${{uPct.toFixed(1)}}%)</div></div>`;

  const platforms = {{}};
  D.games.forEach(g => {{ platforms[g.platform] = (platforms[g.platform]||0) + 1; }});
  const pMax = Math.max(...Object.values(platforms));
  const pColors = {{ PS4: 'fill-blue', PS5: 'fill-green', RawELF: 'fill-yellow' }};
  $('#platformBar').innerHTML = Object.entries(platforms).sort((a,b) => b[1]-a[1]).map(([k,v]) =>
    `<div class="hbar"><div class="hbar-label">${{k}}</div><div class="hbar-track"><div class="hbar-fill ${{pColors[k]||'fill-blue'}}" style="width:${{pMax>0?(v/pMax*100).toFixed(1):'0'}}%"></div></div><div class="hbar-count">${{v}}</div></div>`
  ).join('');

  const engines = {{}};
  D.games.forEach(g => {{ engines[g.engine] = (engines[g.engine]||0) + 1; }});
  const eMax = Math.max(...Object.values(engines));
  const eColors = {{ 'Native': 'fill-green', 'Native/SCE': 'fill-blue', 'Unity': 'fill-purple', 'Unreal Engine 4': 'fill-yellow', 'Unreal Engine 5': 'fill-red' }};
  $('#engineBar').innerHTML = Object.entries(engines).sort((a,b) => b[1]-a[1]).map(([k,v]) =>
    `<div class="hbar"><div class="hbar-label">${{k}}</div><div class="hbar-track"><div class="hbar-fill ${{eColors[k]||'fill-yellow'}}" style="width:${{eMax>0?(v/eMax*100).toFixed(1):'0'}}%"></div></div><div class="hbar-count">${{v}}</div></div>`
  ).join('');
}})();

// --- GAMES ---
(function() {{
  let allRows = D.games.map(g => [g.name, g.engine||'', g.engine_confidence, g.library_count, g.unknown_nid_count, g.file_size_mb, g.title_name||'', g.platform, g.is_self]);
  let filteredRows = [...allRows];

  const platforms = [...new Set(D.games.map(g=>g.platform))].sort();
  const engines = [...new Set(D.games.map(g=>g.engine||'Unknown'))].sort();
  platforms.forEach(p => {{ const o = document.createElement('option'); o.value = p; o.textContent = p; $('#filterPlatform').appendChild(o); }});
  engines.forEach(e => {{ const o = document.createElement('option'); o.value = e; o.textContent = e; $('#filterEngine').appendChild(o); }});

  function applyFilters() {{
    const q = $('#gameFilterText').value.toLowerCase();
    const fp = $('#filterPlatform').value;
    const fe = $('#filterEngine').value;
    const fs = $('#filterSelf').value;
    const fu = $('#filterHasUnknown').value;
    filteredRows = allRows.filter(r => {{
      if (q && !r[0].toLowerCase().includes(q) && !r[6].toLowerCase().includes(q)) return false;
      if (fp && r[7] !== fp) return false;
      if (fe && r[1] !== fe) return false;
      if (fs === 'self' && !r[8]) return false;
      if (fs === 'elf' && r[8]) return false;
      if (fu === 'yes' && r[4] === 0) return false;
      if (fu === 'no' && r[4] > 0) return false;
      return true;
    }});
    renderGames();
  }}

  function confPill(c) {{
    if (c >= 90) return `<span class="pill pill-self">${{c}}</span>`;
    if (c >= 50) return `<span class="pill pill-eng">${{c}}</span>`;
    if (c > 0) return `<span class="pill pill-elf">${{c}}</span>`;
    return '<span style="color:#8b949e">-</span>';
  }}

  function renderGames() {{
    $('#gamesBody').innerHTML = filteredRows.map(r => `<tr class="clickable" data-game="${{r[0]}}">
      <td title="${{r[6]}}">${{trunc(r[6] || r[0],32)}}</td>
      <td>${{r[1]?`<span class="pill pill-eng">${{r[1]}}</span>`:'-'}}</td>
      <td>${{confPill(r[2])}}</td>
      <td>${{r[3]}}</td>
      <td class="pct ${{r[4]>0?'pct-low':'pct-high'}}">${{r[4]}}</td>
      <td>${{r[5].toFixed(1)}}</td>
    </tr>`).join('');

    $$('#gamesBody tr.clickable').forEach(tr => tr.addEventListener('click', () => {{
      showGameDetail(tr.dataset.game);
    }}));
  }}

  let sortState = {{ col: -1, asc: true }};
  $('#gamesTable thead').addEventListener('click', e => {{
    const th = e.target.closest('th');
    if (!th) return;
    const col = +th.dataset.col;
    if (sortState.col === col) sortState.asc = !sortState.asc;
    else {{ sortState.col = col; sortState.asc = true; }}
    $$('#gamesTable th').forEach(h => h.classList.remove('sorted'));
    th.classList.add('sorted');
    th.querySelector('.arrow').innerHTML = sortState.asc ? '&#9650;' : '&#9660;';
    filteredRows.sort((a, b) => {{
      let va = a[col], vb = b[col];
      if (typeof va === 'number') return sortState.asc ? va - vb : vb - va;
      return sortState.asc ? String(va).localeCompare(String(vb)) : String(vb).localeCompare(String(va));
    }});
    renderGames();
  }});

  ['gameFilterText','filterPlatform','filterEngine','filterSelf','filterHasUnknown'].forEach(id => {{
    const el = $('#'+id);
    el.addEventListener('input', applyFilters);
    el.addEventListener('change', applyFilters);
  }});
  renderGames();
}})();

// --- ENGINES ---
(function() {{
  const hints = D.engine_hints || [];
  const summary = D.engine_summary || [];

  const totalGames = hints.length;
  const avgScore = hints.length > 0 ? hints.reduce((s,h) => s + h.score, 0) / hints.length : 0;
  const avgConf = hints.length > 0 ? hints.reduce((s,h) => s + h.confidence, 0) / hints.length : 0;
  const withThirdParty = hints.filter(h => (h.third_party_libs||[]).length > 0).length;
  const withForks = hints.filter(h => (h.custom_forks||[]).length > 0).length;

  $('#engineOverviewCards').innerHTML = [
    ['Total Games', totalGames, 'blue'],
    ['Avg Score', avgScore.toFixed(0), 'yellow'],
    ['Avg Confidence', avgConf.toFixed(1) + '%', 'green'],
    ['With Third-Party', withThirdParty, ''],
    ['With Custom Forks', withForks, ''],
  ].map(([l, v, c]) => `<div class="card"><div class="card-label">${{l}}</div><div class="card-value ${{c}}">${{v}}</div></div>`).join('');

  const maxGameCount = summary.length > 0 ? summary[0].game_count : 1;
  const eColors = {{ 'Native/SCE': 'fill-green', 'Native': 'fill-blue', 'Unity': 'fill-purple', 'Unreal Engine 4': 'fill-yellow', 'Unreal Engine 5': 'fill-red' }};
  $('#engineDistBars').innerHTML = summary.map(s =>
    `<div class="hbar"><div class="hbar-label">${{s.engine}}</div><div class="hbar-track"><div class="hbar-fill ${{eColors[s.engine]||'fill-blue'}}" style="width:${{maxGameCount>0?(s.game_count/maxGameCount*100).toFixed(1):'0'}}%"></div></div><div class="hbar-count">${{s.game_count}} (${{s.avg_confidence.toFixed(0)}}% avg)</div></div>`
  ).join('');

  let rows = hints.map(h => [h.display_name||h.name, h.engine, h.score, h.confidence, (h.third_party_libs||[]).join(', '), (h.custom_forks||[]).join(', '), h.build_system||'']);
  let filtered = [...rows];

  function renderEngineTable() {{
    $('#engineBody').innerHTML = filtered.map(r => `<tr class="clickable" data-game="${{r[0]}}">
      <td title="${{r[0]}}">${{trunc(r[0],32)}}</td>
      <td>${{r[1]?`<span class="pill pill-eng">${{r[1]}}</span>`:'-'}}</td>
      <td style="font-family:monospace">${{r[2]}}</td>
      <td class="pct ${{pctCls(r[3])}}">${{r[3]}}%</td>
      <td style="font-size:0.72rem">${{r[4]?trunc(r[4],40):'-'}}</td>
      <td style="font-size:0.72rem;color:${{r[5]?'#f85149':'#8b949e'}}">${{r[5]?trunc(r[5],40):'-'}}</td>
      <td>${{r[6]||'-'}}</td>
    </tr>`).join('');

    $$('#engineBody tr.clickable').forEach(tr => tr.addEventListener('click', () => {{
      const gameName = tr.dataset.game;
      const h = hints.find(x => (x.display_name||x.name) === gameName);
      if (!h) return;
      openDetail(h.display_name || h.name, `
        <div class="detail-section"><h3>Engine Overview</h3><div class="detail-kv">
          <div class="k">Engine</div><div class="v">${{h.engine}}</div>
          <div class="k">Score</div><div class="v" style="font-family:monospace">${{h.score}}</div>
          <div class="k">Confidence</div><div class="v">${{h.confidence}}%</div>
          ${{h.build_system ? `<div class="k">Build System</div><div class="v">${{h.build_system}}</div>` : ''}}
          ${{h.source_depot ? `<div class="k">Source Depot</div><div class="v">${{h.source_depot}}</div>` : ''}}
        </div></div>
        ${{(h.sce_libraries||[]).length ? `<div class="detail-section"><h3>SCE Libraries (${{(h.sce_libraries||[]).length}})</h3><p style="color:#8b949e;font-size:0.82rem">${{(h.sce_libraries||[]).join(', ')}}</p></div>` : ''}}
        ${{(h.third_party_libs||[]).length ? `<div class="detail-section"><h3>Third-Party Libraries</h3><p style="color:#8b949e;font-size:0.82rem">${{(h.third_party_libs||[]).join(', ')}}</p></div>` : ''}}
        ${{(h.custom_forks||[]).length ? `<div class="detail-section"><h3 style="color:#f85149">Custom Forks</h3><p style="color:#f85149;font-size:0.82rem">${{(h.custom_forks||[]).join(', ')}}</p></div>` : ''}}
        ${{(h.sdk_hints||[]).length ? `<div class="detail-section"><h3>SDK Hints</h3><p style="color:#8b949e;font-size:0.82rem">${{(h.sdk_hints||[]).join(', ')}}</p></div>` : ''}}
        ${{(h.detected_versions||[]).length ? `<div class="detail-section"><h3>Detected Versions</h3><p style="color:#8b949e;font-size:0.82rem">${{(h.detected_versions||[]).join(', ')}}</p></div>` : ''}}
        ${{(h.evidence||[]).length ? `<div class="detail-section"><h3>Evidence Strings (${{(h.evidence||[]).length}})</h3><div class="table-wrap"><table class="detail-table"><thead><tr><th>String</th></tr></thead><tbody>${{(h.evidence||[]).slice(0,30).map(e => `<tr><td style="font-family:monospace;font-size:0.72rem">${{e}}</td></tr>`).join('')}}</tbody></table></div></div>` : ''}}
        ${{(h.lib_versions||[]).length ? `<div class="detail-section"><h3>SDK Library Versions</h3><div class="table-wrap"><table class="detail-table"><thead><tr><th>Library</th><th>Version</th><th>Raw</th></tr></thead><tbody>${{(h.lib_versions||[]).map(lv => `<tr><td style="font-family:monospace;font-size:0.78rem">${{lv.name}}</td><td style="font-variant-numeric:tabular-nums">${{lv.version_string}}</td><td style="font-family:monospace;font-size:0.72rem;color:#8b949e">0x${{lv.version_raw.toString(16).padStart(8,'0')}}</td></tr>`).join('')}}</tbody></table></div></div>` : ''}}
      `);
    }}));
  }}

  let sortState = {{ col: -1, asc: true }};
  $('#engineTable thead').addEventListener('click', e => {{
    const th = e.target.closest('th');
    if (!th) return;
    const col = +th.dataset.col;
    if (sortState.col === col) sortState.asc = !sortState.asc;
    else {{ sortState.col = col; sortState.asc = true; }}
    $$('#engineTable th').forEach(h => h.classList.remove('sorted'));
    th.classList.add('sorted');
    th.querySelector('.arrow').innerHTML = sortState.asc ? '&#9650;' : '&#9660;';
    filtered.sort((a, b) => {{
      let va = a[col], vb = b[col];
      if (typeof va === 'number') return sortState.asc ? va - vb : vb - va;
      return sortState.asc ? String(va).localeCompare(String(vb)) : String(vb).localeCompare(String(va));
    }});
    renderEngineTable();
  }});
  renderEngineTable();
}})();

// --- LIBRARIES ---
(function() {{
  const libDetailMap = {{}};
  (D.library_details || []).forEach(d => {{ libDetailMap[d.name] = d; }});

  let rows = D.library_priority.map(l => [l.name, l.game_count, l.import_count, l.unique_nid_count]);
  const tbody = $('#libBody');
  function render(data) {{
    tbody.innerHTML = data.map(r => `<tr class="clickable" data-lib="${{r[0]}}">
      <td>${{r[0]}}</td><td>${{r[1]}}</td><td>${{fmt(r[2])}}</td><td>${{fmt(r[3])}}</td>
    </tr>`).join('');
    $$('#libBody tr.clickable').forEach(tr => tr.addEventListener('click', () => {{
      const d = libDetailMap[tr.dataset.lib];
      if (!d) return;
      const gamesHtml = d.games.map(g => `<tr><td>${{trunc(g.title_name||g.game,30)}}</td><td>${{fmt(g.import_count)}}</td><td>${{fmt(g.unique_nid_count)}}</td></tr>`).join('');
      const nidsHtml = d.top_nids.map(n => `<tr><td style="font-family:monospace;font-size:0.72rem">${{n.nid_hash}}</td><td>${{n.resolved_name||'-'}}</td><td>${{fmt(n.count)}}</td></tr>`).join('');
      const unkHtml = d.unknown_nids.map(n => `<tr><td style="font-family:monospace;font-size:0.72rem">${{n.nid_hash}}</td><td>${{fmt(n.count)}}</td></tr>`).join('');

      openDetail(d.name, `
        <div class="detail-section"><h3>Overview</h3><div class="detail-kv">
          <div class="k">Games</div><div class="v">${{d.game_count}}</div>
          <div class="k">Total Imports</div><div class="v">${{fmt(d.total_imports)}}</div>
          <div class="k">Unique NIDs</div><div class="v">${{fmt(d.unique_nid_count)}}</div>
        </div></div>
        <div class="detail-section"><h3>Games (${{d.games.length}})</h3><div class="table-wrap"><table class="detail-table">
          <thead><tr><th>Game</th><th>Imports</th><th>Unique NIDs</th></tr></thead><tbody>${{gamesHtml}}</tbody></table></div></div>
        <div class="detail-section"><h3>Top NIDs</h3><div class="table-wrap"><table class="detail-table">
          <thead><tr><th>NID Hash</th><th>Resolved Name</th><th>Count</th></tr></thead><tbody>${{nidsHtml}}</tbody></table></div></div>
        ${{d.unknown_nids.length > 0 ? `<div class="detail-section"><h3>Unknown NIDs (${{d.unknown_nids.length}})</h3><div class="table-wrap"><table class="detail-table">
          <thead><tr><th>NID Hash</th><th>Count</th></tr></thead><tbody>${{unkHtml}}</tbody></table></div></div>` : ''}}
      `);
    }}));
  }}
  render(rows);
  let sortState = {{ col: -1, asc: true }};
  $('#libTable thead').addEventListener('click', e => {{
    const th = e.target.closest('th');
    if (!th) return;
    const col = +th.dataset.col;
    if (sortState.col === col) sortState.asc = !sortState.asc;
    else {{ sortState.col = col; sortState.asc = true; }}
    $$('#libTable th').forEach(h => h.classList.remove('sorted'));
    th.classList.add('sorted');
    th.querySelector('.arrow').innerHTML = sortState.asc ? '&#9650;' : '&#9660;';
    rows.sort((a, b) => {{
      if (typeof a[col] === 'number') return sortState.asc ? a[col] - b[col] : b[col] - a[col];
      return sortState.asc ? a[col].localeCompare(b[col]) : b[col].localeCompare(a[col]);
    }});
    render(rows);
  }});

  // Heatmap
  const hm = D.heatmap;
  if (hm.libraries.length) {{
    const maxLog = Math.max(...hm.log_matrix.flat());
    let html = '<table class="heatmap"><thead><tr><th></th>';
    hm.games.forEach(g => {{ html += `<th title="${{g}}">${{trunc(g,10)}}</th>`; }});
    html += '</tr></thead><tbody>';
    hm.libraries.forEach((lib, i) => {{
      html += `<tr><td class="lib-name" title="${{lib}}">${{lib}}</td>`;
      hm.log_matrix[i].forEach((v, j) => {{
        const raw = hm.raw_matrix[i][j];
        const intensity = maxLog > 0 ? v / maxLog : 0;
        const r = Math.round(35 + intensity * 198);
        const g = Math.round(134 + intensity * 11);
        const b = Math.round(54 - intensity * 15);
        html += `<td style="background:rgba(${{r}},${{g}},${{b}},0.85)" title="${{lib}}: ${{raw}} in ${{hm.games[j]}}"></td>`;
      }});
      html += '</tr>';
    }});
    html += '</tbody></table>';
    $('#heatmapWrap').innerHTML = html;
  }}
}})();

// --- NIDS ---
(function() {{
  const ns = D.nid_stats;
  const total = ns.resolved_count + ns.unknown_count;
  const rPct = total > 0 ? (ns.resolved_count / total * 100) : 0;
  const uPct = 100 - rPct;
  $('#nidResDetail').innerHTML = `
    <div class="hbar"><div class="hbar-label">Resolved</div><div class="hbar-track"><div class="hbar-fill fill-green" style="width:${{rPct.toFixed(1)}}%"></div></div><div class="hbar-count">${{ns.resolved_count.toLocaleString()}} (${{rPct.toFixed(1)}}%)</div></div>
    <div class="hbar"><div class="hbar-label">Unknown</div><div class="hbar-track"><div class="hbar-fill fill-red" style="width:${{uPct.toFixed(1)}}%"></div></div><div class="hbar-count">${{ns.unknown_count.toLocaleString()}} (${{uPct.toFixed(1)}}%)</div></div>`;

  const maxCount = ns.top_nids.length > 0 ? ns.top_nids[0].count : 1;
  $('#nidBars').innerHTML = ns.top_nids.slice(0, 20).map(n => {{
    const w = (n.count / maxCount * 100).toFixed(1);
    const label = n.resolved_name || n.nid_hash;
    return `<div class="hbar"><div class="hbar-label" title="${{n.nid_hash}} &rarr; ${{label}}"><code style="font-size:0.7rem;color:#8b949e;margin-right:6px">${{n.nid_hash}}</code>${{trunc(label,24)}}</div><div class="hbar-track"><div class="hbar-fill fill-blue" style="width:${{w}}%"></div></div><div class="hbar-count">${{fmt(n.count)}}</div></div>`;
  }}).join('');

  const groups = D.library_nid_breakdown;
  $('#libNidBreakdown').innerHTML = groups.map((g, i) => {{
      const nidRows = g.top_nids.map(n => {{
      const label = n.resolved_name || n.nid_hash;
      const mc = g.top_nids[0].count;
      const w = mc > 0 ? (n.count / mc * 100).toFixed(1) : '0';
      return `<div class="hbar"><div class="hbar-label" title="${{n.nid_hash}}"><code style="font-size:0.7rem;color:#8b949e;margin-right:6px">${{n.nid_hash}}</code>${{trunc(label,24)}}</div><div class="hbar-track"><div class="hbar-fill fill-blue" style="width:${{w}}%"></div></div><div class="hbar-count">${{fmt(n.count)}}</div></div>`;
    }}).join('');
    return `<details style="margin-bottom:8px" ${{i < 5 ? 'open' : ''}}>
      <summary style="cursor:pointer;padding:8px 12px;background:#0d1117;border:1px solid #30363d;border-radius:6px;font-size:0.88rem;color:#c9d1d9">
        <strong style="color:#58a6ff">${{g.library}}</strong> &mdash; ${{g.game_count}} games, ${{fmt(g.total_imports)}} imports, ${{fmt(g.unique_nid_count)}} unique NIDs
      </summary>
      <div style="padding:12px 12px 4px;border:1px solid #30363d;border-top:0;border-radius:0 0 6px 6px">${{nidRows}}</div>
    </details>`;
  }}).join('');
}})();

// --- SEGMENTS ---
(function() {{
  const segs = D.segments.sort((a, b) => b.total_mb - a.total_mb);
  const maxTotal = segs.length > 0 ? segs[0].total_mb : 1;
  $('#segBars').innerHTML = segs.map(s => {{
    const total = s.rx_mb + s.r_mb + s.rw_mb + s.other_mb;
    const w = total > 0 ? (total / maxTotal * 100).toFixed(1) : '0';
    return `<div style="display:flex;align-items:center;margin-bottom:3px">
      <div style="width:160px;font-size:0.78rem;color:#c9d1d9;text-overflow:ellipsis;overflow:hidden;white-space:nowrap" title="${{s.game}}">${{trunc(s.game,22)}}</div>
      <div class="seg-bar" style="width:${{w}}%">
        <div class="seg-rx" style="width:${{total>0?(s.rx_mb/total*100).toFixed(1):'0'}}%" title="RX: ${{s.rx_mb.toFixed(1)}} MB"></div>
        <div class="seg-r" style="width:${{total>0?(s.r_mb/total*100).toFixed(1):'0'}}%" title="R: ${{s.r_mb.toFixed(1)}} MB"></div>
        <div class="seg-rw" style="width:${{total>0?(s.rw_mb/total*100).toFixed(1):'0'}}%" title="RW: ${{s.rw_mb.toFixed(1)}} MB"></div>
        <div class="seg-other" style="width:${{total>0?(s.other_mb/total*100).toFixed(1):'0'}}%" title="Other: ${{s.other_mb.toFixed(1)}} MB"></div>
      </div>
      <div style="width:70px;text-align:right;font-size:0.75rem;color:#8b949e;margin-left:8px">${{total.toFixed(1)}} MB</div>
    </div>`;
  }}).join('');
}})();

// --- STATISTICS ---
(function() {{
  const s = D.statistics;
  if (!s) return;
  const entry = (label, game, value) => `<div class="stat-row"><span>${{label}}: ${{trunc(game,20)}}</span><span class="sv">${{value}}</span></div>`;
  const section = (title, entries) => `<div class="stat-card"><h3>${{title}}</h3>${{entries}}</div>`;
  let html = '';
  html += section('Largest Binaries', s.top_5_largest.map(e => entry('', e.game, e.value.toFixed(1) + ' MB')).join(''));
  html += section('Smallest Binaries', s.top_5_smallest.map(e => entry('', e.game, e.value.toFixed(1) + ' MB')).join(''));
  html += section('Most Imports', s.top_5_most_imports.map(e => entry('', e.game, fmt(Math.round(e.value)))).join(''));
  html += section('Most Libraries', s.top_5_most_libs.map(e => entry('', e.game, fmt(Math.round(e.value)))).join(''));
  html += section('Highest Unknown %', s.top_5_highest_unknown.map(e => entry('', e.game, e.value.toFixed(1) + '%')).join(''));
  html += section('Averages', [
    `<div class="stat-row"><span>Code (RX)</span><span class="sv">${{s.avg_code_size_mb.toFixed(1)}} MB</span></div>`,
    `<div class="stat-row"><span>Data (RW)</span><span class="sv">${{s.avg_data_size_mb.toFixed(1)}} MB</span></div>`,
    `<div class="stat-row"><span>Read-only (R)</span><span class="sv">${{s.avg_rodata_size_mb.toFixed(1)}} MB</span></div>`,
    `<div class="stat-row"><span>Other</span><span class="sv">${{s.avg_other_size_mb.toFixed(1)}} MB</span></div>`,
    `<div class="stat-row"><span>Total Code</span><span class="sv">${{s.total_code_mb.toFixed(1)}} MB</span></div>`,
    `<div class="stat-row"><span>Total Data</span><span class="sv">${{s.total_data_mb.toFixed(1)}} MB</span></div>`,
  ].join(''));
  $('#statGrid').innerHTML = html;

  const lv = D.library_versions || [];
  if (lv.length) {{
    const totalGames = D.games.length;
    const uniqueLibs = [...new Set(lv.map(v => v.library))].length;
    const gamesWithVersions = new Set();
    lv.forEach(v => v.game_ids.forEach(g => gamesWithVersions.add(g)));
    let lvHtml = `<div class="section"><h2>SDK Library Version Distribution</h2>
      <div class="cards" style="margin-bottom:16px">
        <div class="card"><div class="card-label">Games with Version Info</div><div class="card-value blue">${{gamesWithVersions.size}}/${{totalGames}}</div></div>
        <div class="card"><div class="card-label">Unique Libraries</div><div class="card-value yellow">${{uniqueLibs}}</div></div>
        <div class="card"><div class="card-label">Library-Version Pairs</div><div class="card-value">${{lv.length}}</div></div>
      </div>`;
    const grouped = {{}};
    lv.forEach(v => {{
      if (!grouped[v.library]) grouped[v.library] = [];
      grouped[v.library].push(v);
    }});
    Object.keys(grouped).sort().forEach(lib => {{
      const versions = grouped[lib].sort((a, b) => b.version_raw - a.version_raw);
      lvHtml += `<details style="margin-bottom:6px" open>
        <summary style="cursor:pointer;padding:8px 12px;background:#0d1117;border:1px solid #30363d;border-radius:6px;font-size:0.85rem;color:#c9d1d9">
          <strong style="color:#58a6ff">${{lib}}</strong> &mdash; ${{versions.length}} version(s), ${{versions[0].game_count}} game(s)
        </summary>
        <div style="padding:8px 12px;border:1px solid #30363d;border-top:0;border-radius:0 0 6px 6px">
          <table style="width:100%;font-size:0.82rem;border-collapse:collapse">
            <thead><tr style="color:#8b949e"><th style="text-align:left;padding:4px 8px">Version</th><th style="text-align:left;padding:4px 8px">Raw</th><th style="text-align:left;padding:4px 8px">Games</th></tr></thead>
            <tbody>${{versions.map(v => `<tr><td style="padding:4px 8px;font-variant-numeric:tabular-nums">${{v.version_string}}</td><td style="padding:4px 8px;font-family:monospace;font-size:0.72rem;color:#8b949e">0x${{v.version_raw.toString(16).padStart(8,'0')}}</td><td style="padding:4px 8px;font-size:0.78rem">${{v.games.map((g, i) => `<a href="#" class="game-link" data-game-id="${{v.game_ids[i]}}" style="color:#58a6ff">${{trunc(g,24)}}</a>`).join(', ')}}</td></tr>`).join('')}}</tbody>
          </table>
        </div>
      </details>`;
    }});
    lvHtml += '</div>';
    const el = document.createElement('div');
    el.innerHTML = lvHtml;
    $('#statGrid').parentNode.appendChild(el);
  }}

  $('#statGrid').addEventListener('click', e => {{
    const link = e.target.closest('.game-link');
    if (!link) return;
    e.preventDefault();
    const gameId = link.dataset.gameId;
    showGameDetail(gameId);
  }});
}})();

// --- GRAPH ---
(function() {{
  const libs = D.library_priority;
  const games = D.games;
  if (!libs.length || !games.length) return;

  const W = 1200, H = Math.max(500, libs.length * 50 + 100);
  const libX = 200, gameX = W - 200;
  const libSpacing = H / (libs.length + 1);
  const gameSpacing = H / (games.length + 1);

  const libPositions = {{}};
  libs.forEach((l, i) => {{ libPositions[l.name] = {{ x: libX, y: libSpacing * (i + 1) }}; }});

  const gamePositions = {{}};
  games.forEach((g, i) => {{ gamePositions[g.name] = {{ x: gameX, y: gameSpacing * (i + 1) }}; }});

  let svg = `<svg width="${{W}}" height="${{H}}" xmlns="http://www.w3.org/2000/svg" style="font-family:system-ui,sans-serif;">`;

  for (const [game, doc] of Object.entries(D.game_details || {{}})) {{
    const gp = gamePositions[game];
    if (!gp) continue;
    for (const imp of doc.imports || []) {{
      const lp = libPositions[imp.library_name];
      if (!lp) continue;
      svg += `<line x1="${{lp.x+80}}" y1="${{lp.y}}" x2="${{gp.x-60}}" y2="${{gp.y}}" stroke="#30363d" stroke-width="0.5" opacity="0.3"/>`;
    }}
  }}

  libs.forEach(l => {{
    const p = libPositions[l.name];
    svg += `<rect x="${{p.x-10}}" y="${{p.y-8}}" width="160" height="16" rx="3" fill="#1f6feb" opacity="0.8" class="graph-node" data-type="lib" data-name="${{l.name}}" style="cursor:pointer"/>`;
    svg += `<text x="${{p.x+70}}" y="${{p.y+4}}" text-anchor="middle" fill="#e6edf3" font-size="9">${{trunc(l.name,20)}} (${{l.game_count}})</text>`;
  }});

  games.forEach(g => {{
    const p = gamePositions[g.name];
    svg += `<rect x="${{p.x-60}}" y="${{p.y-6}}" width="120" height="12" rx="3" fill="#238636" opacity="0.7" class="graph-node" data-type="game" data-name="${{g.name}}" style="cursor:pointer"/>`;
    svg += `<text x="${{p.x}}" y="${{p.y+3}}" text-anchor="middle" fill="#e6edf3" font-size="7">${{trunc(g.title_name || g.name,16)}}</text>`;
  }});

  svg += '</svg>';
  $('#graphWrap').innerHTML = svg;

  $$('.graph-node').forEach(node => node.addEventListener('click', () => {{
    const type = node.dataset.type;
    const name = node.dataset.name;
    if (type === 'lib') {{
      const d = (D.library_details || []).find(x => x.name === name);
      if (d) {{
        const gamesHtml = d.games.map(g => `<tr><td>${{trunc(g.title_name||g.game,30)}}</td><td>${{fmt(g.import_count)}}</td></tr>`).join('');
        openDetail(d.name, `<div class="detail-section"><div class="detail-kv"><div class="k">Games</div><div class="v">${{d.game_count}}</div><div class="k">Imports</div><div class="v">${{fmt(d.total_imports)}}</div></div></div><div class="detail-section"><h3>Games</h3><div class="table-wrap"><table class="detail-table"><thead><tr><th>Game</th><th>Imports</th></tr></thead><tbody>${{gamesHtml}}</tbody></table></div></div>`);
      }}
    }} else {{
      const d = (D.game_details || []).find(x => x.name === name);
      if (d) openDetail(d.title_name||d.name, `<div class="detail-kv"><div class="k">Imports</div><div class="v">${{fmt(d.imports.length)}}</div><div class="k">Libraries</div><div class="v">${{d.import_summary.length}}</div></div>`);
    }}
  }}));
}})();

// --- GLOBAL SEARCH ---
(function() {{
  const searchIndex = [];
  (D.game_details || []).forEach(d => {{
    d.imports.forEach(imp => {{
      searchIndex.push({{
        type: 'import',
        nid: imp.nid_hash,
        name: imp.resolved_name || '',
        library: imp.library_name,
        game: d.name,
        gameTitle: d.title_name || d.name,
      }});
    }});
  }});
  (D.library_details || []).forEach(d => {{
    d.top_nids.forEach(n => {{
      searchIndex.push({{
        type: 'library-nid',
        nid: n.nid_hash,
        name: n.resolved_name,
        library: d.name,
        game: '',
        gameTitle: '',
      }});
    }});
  }});
  (D.game_details || []).forEach(d => {{
    (d.lib_versions || []).forEach(lv => {{
      searchIndex.push({{
        type: 'lib-version',
        name: lv.name,
        version: lv.version_string,
        versionRaw: '0x' + lv.version_raw.toString(16).padStart(8,'0'),
        game: d.name,
        gameTitle: d.title_name || d.name,
      }});
    }});
  }});

  const input = $('#globalSearch');
  const results = $('#searchResults');

  input.addEventListener('input', () => {{
    const q = input.value.trim().toLowerCase();
    if (q.length < 2) {{ results.classList.remove('show'); return; }}
    const matches = searchIndex.filter(e => {{
      if (e.type === 'lib-version') {{
        return (e.name||'').toLowerCase().includes(q) ||
               (e.version||'').includes(q) ||
               (e.versionRaw||'').includes(q) ||
               (e.game||'').toLowerCase().includes(q) ||
               (e.gameTitle||'').toLowerCase().includes(q);
      }}
      return (e.nid||'').toLowerCase().includes(q) ||
             (e.name||'').toLowerCase().includes(q) ||
             (e.library||'').toLowerCase().includes(q) ||
             (e.game||'').toLowerCase().includes(q) ||
             (e.gameTitle||'').toLowerCase().includes(q);
    }}).slice(0, 30);

    if (matches.length === 0) {{ results.classList.remove('show'); return; }}

    const grouped = {{}};
    matches.forEach(m => {{
      if (m.type === 'lib-version') {{
        const key = 'lv:' + m.name + ':' + m.version + ':' + m.game;
        if (!grouped[key]) grouped[key] = {{ ...m, games: new Set(), type: 'lib-version' }};
        if (m.game) grouped[key].games.add(m.gameTitle || m.game);
        return;
      }}
      const key = m.nid + m.library;
      if (!grouped[key]) grouped[key] = {{ ...m, games: new Set() }};
      if (m.game) grouped[key].games.add(m.gameTitle || m.game);
    }});

    results.innerHTML = Object.values(grouped).map(m => `
      <div class="sr-item" data-nid="${{m.nid}}" data-lib="${{m.library}}" data-game="${{m.game}}" data-type="${{m.type}}">
        <div class="sr-type">${{m.type}}${{m.type !== 'lib-version' ? ` &middot; ${{m.library||''}}` : ''}}</div>
        <div class="sr-name">${{m.type === 'lib-version' ? m.name + ' ' + m.version : (m.name || m.nid)}}</div>
        <div class="sr-detail">${{[...m.games].slice(0,3).join(', ')}}${{m.games.size > 3 ? ` +${{m.games.size-3}} more` : ''}}</div>
      </div>
    `).join('');
    results.classList.add('show');
  }});

  results.addEventListener('click', e => {{
    const item = e.target.closest('.sr-item');
    if (!item) return;
    const type = item.dataset.type;
    if (type === 'lib-version') {{
      const gameId = item.dataset.game;
      if (gameId) showGameDetail(gameId);
    }} else {{
      const lib = item.dataset.lib;
      const d = (D.library_details || []).find(x => x.name === lib);
      if (d) {{
        const gamesHtml = d.games.map(g => `<tr><td>${{trunc(g.title_name||g.game,30)}}</td><td>${{fmt(g.import_count)}}</td></tr>`).join('');
        openDetail(d.name, `<div class="detail-section"><div class="detail-kv"><div class="k">Games</div><div class="v">${{d.game_count}}</div></div></div><div class="detail-section"><h3>Games</h3><div class="table-wrap"><table class="detail-table"><thead><tr><th>Game</th><th>Imports</th></tr></thead><tbody>${{gamesHtml}}</tbody></table></div></div>`);
      }}
    }}
    results.classList.remove('show');
    input.value = '';
  }});

  input.addEventListener('blur', () => {{ setTimeout(() => results.classList.remove('show'), 200); }});
}})();
</script>
</body>
</html>"##,
        generated_at = data.meta.generated_at,
        game_count = data.meta.game_count,
        tool_version = data.meta.tool_version,
        json = json,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::*;

    fn sample_data() -> DashboardData {
        DashboardData {
            meta: DashboardMeta {
                generated_at: "2026-07-26".to_string(),
                game_count: 2,
                tool_version: "0.1.0".to_string(),
            },
            overview: Overview {
                total_games: 2,
                elf_valid: 2,
                total_imports: 100,
                unique_nids: 50,
                unique_libs: 10,
                resolution_rate: 95.0,
                avg_imports_per_game: 50.0,
            },
            games: vec![GameRow {
                name: "game1".to_string(),
                title_name: Some("Game One".to_string()),
                platform: "PS5".to_string(),
                is_self: true,
                engine: "Native".to_string(),
                engine_confidence: 0,
                library_count: 5,
                unknown_nid_count: 3,
                file_size_mb: 45.2,
            }],
            game_details: vec![GameDetail {
                name: "game1".to_string(),
                title_name: Some("Game One".to_string()),
                platform: "PS5".to_string(),
                is_self: true,
                file_size_mb: 45.2,
                sha256: "a".repeat(64),
                entry_point: "0x80000000".to_string(),
                elf_type: 3,
                osabi: 0x9,
                abi_version: 2,
                elf_version: 1,
                build_id: None,
                segments: vec![SegmentDetail {
                    index: 0,
                    seg_type: "Load".to_string(),
                    vaddr: "0x80000000".to_string(),
                    filesz: 30 * 1024 * 1024,
                    memsz: 30 * 1024 * 1024,
                    flags: "RX".to_string(),
                }],
                imports: vec![ImportDetail {
                    nid_hash: "ABCDEF0123456789".to_string(),
                    resolved_name: Some("sceKernelOpen".to_string()),
                    library_name: "libkernel".to_string(),
                }],
                unresolved_nids: vec![],
                import_summary: vec![LibImportCount {
                    library: "libkernel".to_string(),
                    count: 1,
                }],
                relocations: 0,
                has_tls: false,
                engine: "Native".to_string(),
                engine_score: 0,
                engine_confidence: 0,
                engine_evidence: vec![],
                sce_libraries: vec![],
                third_party_libs: vec![],
                custom_forks: vec![],
                build_system: None,
                source_depot: None,
                sdk_hints: vec![],
                detected_versions: vec![],
                lib_versions: vec![],
            }],
            heatmap: HeatmapData {
                libraries: vec!["libkernel".to_string()],
                games: vec!["game1".to_string()],
                log_matrix: vec![vec![1.0]],
                raw_matrix: vec![vec![1]],
            },
            nid_stats: NidStats {
                top_nids: vec![TopNid {
                    nid_hash: "ABCDEF0123456789".to_string(),
                    resolved_name: "sceKernelOpen".to_string(),
                    count: 42,
                    game_count: 1,
                }],
                resolved_count: 95,
                unknown_count: 5,
            },
            segments: vec![SegmentRow {
                game: "game1".to_string(),
                rx_mb: 30.0,
                r_mb: 5.0,
                rw_mb: 10.0,
                other_mb: 2.0,
                total_mb: 47.0,
            }],
            library_priority: vec![LibraryPriority {
                name: "libkernel".to_string(),
                game_count: 2,
                import_count: 30,
                unique_nid_count: 25,
            }],
            library_details: vec![LibraryDetail {
                name: "libkernel".to_string(),
                game_count: 2,
                total_imports: 30,
                unique_nid_count: 25,
                games: vec![LibGameEntry {
                    game: "game1".to_string(),
                    title_name: Some("Game One".to_string()),
                    import_count: 30,
                    unique_nid_count: 25,
                }],
                top_nids: vec![TopNid {
                    nid_hash: "ABCDEF0123456789".to_string(),
                    resolved_name: "sceKernelOpen".to_string(),
                    count: 15,
                    game_count: 2,
                }],
                unknown_nids: vec![],
            }],
            library_nid_breakdown: vec![LibraryNidGroup {
                library: "libkernel".to_string(),
                game_count: 2,
                total_imports: 30,
                unique_nid_count: 25,
                top_nids: vec![TopNid {
                    nid_hash: "ABCDEF0123456789".to_string(),
                    resolved_name: "sceKernelOpen".to_string(),
                    count: 15,
                    game_count: 2,
                }],
            }],
            statistics: Some(DashboardStatistics {
                top_5_largest: vec![StatEntry {
                    game: "game1".to_string(),
                    value: 47.0,
                }],
                top_5_smallest: vec![StatEntry {
                    game: "game1".to_string(),
                    value: 47.0,
                }],
                top_5_most_imports: vec![StatEntry {
                    game: "game1".to_string(),
                    value: 60.0,
                }],
                top_5_most_libs: vec![StatEntry {
                    game: "game1".to_string(),
                    value: 10.0,
                }],
                top_5_highest_unknown: vec![StatEntry {
                    game: "game1".to_string(),
                    value: 8.3,
                }],
                avg_code_size_mb: 30.0,
                avg_data_size_mb: 10.0,
                avg_rodata_size_mb: 5.0,
                avg_other_size_mb: 2.0,
                total_code_mb: 30.0,
                total_data_mb: 10.0,
            }),
            engine_hints: vec![],
            engine_summary: vec![],
            library_versions: vec![],
        }
    }

    #[test]
    fn html_contains_data() {
        let html = generate_html(&sample_data());
        assert!(html.contains("PS5rs"));
        assert!(html.contains("sceKernelOpen"));
        assert!(html.contains("libkernel"));
    }

    #[test]
    fn html_has_all_tabs() {
        let html = generate_html(&sample_data());
        assert!(html.contains("tab-overview"));
        assert!(html.contains("tab-games"));
        assert!(html.contains("tab-engines"));
        assert!(html.contains("tab-libraries"));
        assert!(html.contains("tab-nids"));
        assert!(html.contains("tab-segments"));
        assert!(html.contains("tab-statistics"));
        assert!(html.contains("tab-graph"));
    }

    #[test]
    fn html_has_detail_panel() {
        let html = generate_html(&sample_data());
        assert!(html.contains("detailPanel"));
        assert!(html.contains("detail-overlay"));
    }

    #[test]
    fn html_has_search() {
        let html = generate_html(&sample_data());
        assert!(html.contains("globalSearch"));
        assert!(html.contains("searchResults"));
    }

    #[test]
    fn html_is_self_contained() {
        let html = generate_html(&sample_data());
        assert!(!html.contains("cdn."));
        assert!(!html.contains("https://"));
        assert!(html.contains("<style>"));
        assert!(html.contains("<script>"));
    }

    #[test]
    fn html_js_has_balanced_parens() {
        let html = generate_html(&sample_data());
        let script_start = html.find("<script>").unwrap() + 8;
        let script_end = html.find("</script>").unwrap();
        let js = &html[script_start..script_end];
        let open = js.matches('(').count();
        let close = js.matches(')').count();
        assert_eq!(
            open, close,
            "unbalanced parens: {} open vs {} close",
            open, close
        );
    }
}
