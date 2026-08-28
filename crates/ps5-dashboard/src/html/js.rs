//! Dashboard client script. Layout prepends `const D = <json>;`.

pub const JS: &str = r##"const $ = s => document.querySelector(s);
const $$ = s => document.querySelectorAll(s);
const pctCls = v => v >= 80 ? 'pct-high' : v >= 50 ? 'pct-med' : 'pct-low';
const fmt = v => typeof v === 'number' ? v.toLocaleString() : v;
const trunc = (s, n) => s && s.length > n ? s.slice(0, n-2) + '..' : s || '';
const gameLabel = x => {
  if (!x) return '';
  if (typeof x === 'string') {
    const g = (D.games||[]).find(y=>y.name===x);
    return g ? (g.title_name || g.name) : x;
  }
  return x.title_name || x.name || x.game || '';
};
const gameDisplay = (x,n=32) => trunc(gameLabel(x), n);

  function showGameDetail(gameId) {
  const d = (D.game_details || []).find(x => x.name === gameId);
  if (!d) return;
  const segsHtml = d.segments.map(s => `<tr><td>${s.index}</td><td>${s.seg_type}</td><td>${s.vaddr}</td><td>${(s.filesz/1048576).toFixed(2)} MB</td><td>${s.flags}</td></tr>`).join('');
  const libsHtml = d.import_summary.slice(0, 15).map(l => `<tr><td>${l.library}</td><td>${fmt(l.count)}</td></tr>`).join('');
  const importsHtml = d.imports.slice(0, 200).map(i => `<tr><td style="font-family:monospace;font-size:0.72rem">${i.nid_hash}</td><td>${i.resolved_name||'<span style="color:#f85149">unknown</span>'}</td><td style="color:#8b949e">${i.library_name}</td></tr>`).join('');
  const unresolvedHtml = d.unresolved_nids.slice(0, 100).map(i => `<tr><td style="font-family:monospace;font-size:0.72rem">${i.nid_hash}</td><td style="color:#8b949e">${i.library_name}</td></tr>`).join('');

  const engineHtml = `
    <div class="detail-section"><h3>Engine Forensics</h3><div class="detail-kv">
      <div class="k">Engine</div><div class="v">${d.engine||'Unknown'}</div>
      <div class="k">Score</div><div class="v" style="font-family:monospace">${d.engine_score}</div>
      <div class="k">Confidence</div><div class="v">${d.engine_confidence}%</div>
      ${d.build_system ? `<div class="k">Build System</div><div class="v">${d.build_system}</div>` : ''}
      ${d.source_depot ? `<div class="k">Source Depot</div><div class="v">${d.source_depot}</div>` : ''}
      ${(d.sce_libraries||[]).length ? `<div class="k">SCE Libraries</div><div class="v">${(d.sce_libraries||[]).length} detected</div>` : ''}
      ${(d.third_party_libs||[]).length ? `<div class="k">Third-Party Libs</div><div class="v">${(d.third_party_libs||[]).join(', ')}</div>` : ''}
      ${(d.custom_forks||[]).length ? `<div class="k">Custom Forks</div><div class="v" style="color:#f85149">${(d.custom_forks||[]).join(', ')}</div>` : ''}
      ${(d.sdk_hints||[]).length ? `<div class="k">SDK Hints</div><div class="v">${(d.sdk_hints||[]).join(', ')}</div>` : ''}
      ${(d.detected_versions||[]).length ? `<div class="k">Versions</div><div class="v">${(d.detected_versions||[]).join(', ')}</div>` : ''}
    </div></div>
    ${(d.engine_evidence||[]).length ? `<div class="detail-section"><h3>Engine Evidence</h3><div class="table-wrap"><table class="detail-table"><thead><tr><th>String</th></tr></thead><tbody>${(d.engine_evidence||[]).map(e => `<tr><td style="font-family:monospace;font-size:0.72rem">${e}</td></tr>`).join('')}</tbody></table></div></div>` : ''}
    ${(d.lib_versions||[]).length ? `<div class="detail-section"><h3>SDK Library Versions</h3><div class="table-wrap"><table class="detail-table"><thead><tr><th>Library</th><th>Version</th><th>Raw</th></tr></thead><tbody>${(d.lib_versions||[]).map(lv => `<tr><td style="font-family:monospace;font-size:0.78rem">${lv.name}</td><td style="font-variant-numeric:tabular-nums">${lv.version_string}</td><td style="font-family:monospace;font-size:0.72rem;color:#8b949e">0x${lv.version_raw.toString(16).padStart(8,'0')}</td></tr>`).join('')}</tbody></table></div></div>` : ''}`;

  openDetail(gameLabel(d), `
    <div class="detail-section"><h3>General</h3><div class="detail-kv">
      <div class="k">Name</div><div class="v">${gameLabel(d)}</div>
      <div class="k">Platform</div><div class="v">${d.platform}</div>
      <div class="k">Type</div><div class="v">${d.is_self?'SELF':'Raw ELF'}</div>
      <div class="k">File Size</div><div class="v">${d.file_size_mb.toFixed(1)} MB</div>
      <div class="k">Entry Point</div><div class="v" style="font-family:monospace">${d.entry_point}</div>
      <div class="k">SHA-256</div><div class="v" style="font-family:monospace;font-size:0.72rem">${d.sha256.slice(0,32)}...</div>
    </div></div>
    ${engineHtml}
    ${d.imports_resolved != null ? `<div class="detail-section"><h3>Loader</h3><div class="detail-kv">
      <div class="k">State</div><div class="v">${d.load_state||'N/A'}</div>
      <div class="k">Resolved</div><div class="v" style="font-family:monospace">${fmt(d.imports_resolved)}</div>
      <div class="k">Known (offline)</div><div class="v" style="font-family:monospace">${fmt(d.imports_known)}</div>
      <div class="k">Stubbed</div><div class="v" style="font-family:monospace">${fmt(d.imports_stubbed)}</div>
      <div class="k">Resolution Rate</div><div class="v">${((d.imports_resolved + d.imports_known) / (d.imports_resolved + d.imports_known + d.imports_stubbed) * 100).toFixed(1)}%</div>
      ${d.loader_tls ? `<div class="k">TLS</div><div class="v">Yes</div>` : ''}
      ${d.init_array_count ? `<div class="k">Init Array</div><div class="v">${d.init_array_count} entries</div>` : ''}
      ${d.fini_array_count ? `<div class="k">Fini Array</div><div class="v">${d.fini_array_count} entries</div>` : ''}
    </div></div>` : ''}
    ${(d.unavailable_modules||[]).length ? `<div class="detail-section"><h3>Unavailable Modules (${d.unavailable_modules.length})</h3><div style="font-size:0.82rem;color:#f85149;word-break:break-all">${d.unavailable_modules.join(', ')}</div></div>` : ''}
    <div class="detail-section"><h3>ELF Header</h3><div class="detail-kv">
      <div class="k">ELF Type</div><div class="v">0x${d.elf_type.toString(16)}</div>
      <div class="k">OS/ABI</div><div class="v">0x${d.osabi.toString(16)}</div>
      <div class="k">ABI Version</div><div class="v">${d.abi_version}</div>
      <div class="k">ELF Version</div><div class="v">${d.elf_version}</div>
      <div class="k">Build ID</div><div class="v" style="font-family:monospace;font-size:0.72rem">${d.build_id||'N/A'}</div>
      <div class="k">Relocations</div><div class="v">${fmt(d.relocations)}</div>
      <div class="k">TLS</div><div class="v">${d.has_tls?'Yes':'No'}</div>
    </div></div>
    <div class="detail-section"><h3>Segments (${d.segments.length})</h3>
      <div class="table-wrap"><table class="detail-table"><thead><tr><th>#</th><th>Type</th><th>VAddr</th><th>Size</th><th>Flags</th></tr></thead><tbody>${segsHtml}</tbody></table></div></div>
    <div class="detail-section"><h3>Libraries (${d.import_summary.length})</h3>
      <div class="table-wrap"><table class="detail-table"><thead><tr><th>Library</th><th>Imports</th></tr></thead><tbody>${libsHtml}</tbody></table></div></div>
    <div class="detail-section"><h3>Imports (${d.imports.length})</h3>
      <div class="table-wrap"><table class="detail-table"><thead><tr><th>NID Hash</th><th>Resolved Name</th><th>Library</th></tr></thead><tbody>${importsHtml}</tbody></table></div></div>
    ${d.unresolved_nids.length > 0 ? `<div class="detail-section"><h3>Unknown NIDs (${d.unresolved_nids.length})</h3><div class="table-wrap"><table class="detail-table"><thead><tr><th>NID Hash</th><th>Library</th></tr></thead><tbody>${unresolvedHtml}</tbody></table></div></div>` : ''}
  `);
}

// --- TABS ---
$$('.tab').forEach(tab => tab.addEventListener('click', () => {
  $$('.tab').forEach(t => t.classList.remove('active'));
  $$('.tab-content').forEach(t => t.classList.remove('active'));
  tab.classList.add('active');
  $(`#tab-${tab.dataset.tab}`).classList.add('active');
}));

// --- DETAIL PANEL ---
function openDetail(title, html) {
  $('#detailTitle').textContent = title;
  $('#detailBody').innerHTML = html;
  $('#detailPanel').classList.add('open');
}
$('#detailClose').addEventListener('click', () => $('#detailPanel').classList.remove('open'));
document.addEventListener('keydown', e => { if (e.key === 'Escape') $('#detailPanel').classList.remove('open'); });

// --- OVERVIEW ---
(function() {
  const o = D.overview;
  $('#overviewCards').innerHTML = [
    ['Games', o.total_games, 'blue'],
    ['ELF Valid', o.elf_valid, 'green'],
    ['Total Imports', fmt(o.total_imports), ''],
    ['Unique NIDs', fmt(o.unique_nids), 'yellow'],
    ['Unique Libraries', o.unique_libs, ''],
    ['Resolution', o.resolution_rate.toFixed(1) + '%', 'green'],
    ['Avg Imports/Game', Math.round(o.avg_imports_per_game), ''],
    ['Artifacts', o.total_artifacts ? fmt(o.total_artifacts) : '—', ''],
    ['Shader Files', o.shader_files ? o.shader_files : '—', 'yellow'],
  ].map(([l, v, c]) => `<div class="card"><div class="card-label">${l}</div><div class="card-value ${c}">${v}</div></div>`).join('');

  const ns = D.nid_stats;
  const total = ns.resolved_count + ns.unknown_count;
  const rPct = total > 0 ? (ns.resolved_count / total * 100) : 0;
  const uPct = 100 - rPct;
  $('#nidResBar').innerHTML = `
    <div class="hbar"><div class="hbar-label">Resolved</div><div class="hbar-track"><div class="hbar-fill fill-green" style="width:${rPct.toFixed(1)}%"></div></div><div class="hbar-count">${ns.resolved_count.toLocaleString()} (${rPct.toFixed(1)}%)</div></div>
    <div class="hbar"><div class="hbar-label">Unknown</div><div class="hbar-track"><div class="hbar-fill fill-red" style="width:${uPct.toFixed(1)}%"></div></div><div class="hbar-count">${ns.unknown_count.toLocaleString()} (${uPct.toFixed(1)}%)</div></div>`;

  const platforms = {};
  D.games.forEach(g => { platforms[g.platform] = (platforms[g.platform]||0) + 1; });
  const pMax = Math.max(...Object.values(platforms));
  const pColors = { PS4: 'fill-blue', PS5: 'fill-green', RawELF: 'fill-yellow' };
  $('#platformBar').innerHTML = Object.entries(platforms).sort((a,b) => b[1]-a[1]).map(([k,v]) =>
    `<div class="hbar"><div class="hbar-label">${k}</div><div class="hbar-track"><div class="hbar-fill ${pColors[k]||'fill-blue'}" style="width:${pMax>0?(v/pMax*100).toFixed(1):'0'}%"></div></div><div class="hbar-count">${v}</div></div>`
  ).join('');

  const engines = {};
  D.games.forEach(g => { engines[g.engine] = (engines[g.engine]||0) + 1; });
  const eMax = Math.max(...Object.values(engines));
  const eColors = { 'Native': 'fill-green', 'Native/SCE': 'fill-blue', 'Unity': 'fill-purple', 'Unreal Engine 4': 'fill-yellow', 'Unreal Engine 5': 'fill-red' };
  $('#engineBar').innerHTML = Object.entries(engines).sort((a,b) => b[1]-a[1]).map(([k,v]) =>
    `<div class="hbar"><div class="hbar-label">${k}</div><div class="hbar-track"><div class="hbar-fill ${eColors[k]||'fill-yellow'}" style="width:${eMax>0?(v/eMax*100).toFixed(1):'0'}%"></div></div><div class="hbar-count">${v}</div></div>`
  ).join('');
})();

// --- GAMES ---
(function() {
  let allRows = D.games.map(g => [g.name, g.engine||'', g.engine_confidence, g.library_count, g.sce_library_count, g.unknown_nid_count, g.file_size_mb, g.title_name||'', g.platform, g.is_self]);
  let filteredRows = [...allRows];

  const platforms = [...new Set(D.games.map(g=>g.platform))].sort();
  const engines = [...new Set(D.games.map(g=>g.engine||'Unknown'))].sort();
  platforms.forEach(p => { const o = document.createElement('option'); o.value = p; o.textContent = p; $('#filterPlatform').appendChild(o); });
  engines.forEach(e => { const o = document.createElement('option'); o.value = e; o.textContent = e; $('#filterEngine').appendChild(o); });

  function applyFilters() {
    const q = $('#gameFilterText').value.toLowerCase();
    const fp = $('#filterPlatform').value;
    const fe = $('#filterEngine').value;
    const fs = $('#filterSelf').value;
    const fu = $('#filterHasUnknown').value;
    filteredRows = allRows.filter(r => {
      if (q && !r[0].toLowerCase().includes(q) && !r[7].toLowerCase().includes(q)) return false;
      if (fp && r[8] !== fp) return false;
      if (fe && r[1] !== fe) return false;
      if (fs === 'self' && !r[9]) return false;
      if (fs === 'elf' && r[9]) return false;
      if (fu === 'yes' && r[5] === 0) return false;
      if (fu === 'no' && r[5] > 0) return false;
      return true;
    });
    renderGames();
  }

  function confPill(c) {
    if (c >= 90) return `<span class="pill pill-self">${c}</span>`;
    if (c >= 50) return `<span class="pill pill-eng">${c}</span>`;
    if (c > 0) return `<span class="pill pill-elf">${c}</span>`;
    return '<span style="color:#8b949e">-</span>';
  }

  function renderGames() {
    $('#gamesBody').innerHTML = filteredRows.map(r => `<tr class="clickable" data-game="${r[0]}">
      <td title="${r[7]}">${trunc(r[7] || r[0],32)}</td>
      <td>${r[1]?`<span class="pill pill-eng">${r[1]}</span>`:'-'}</td>
      <td>${confPill(r[2])}</td>
      <td>${r[3]}</td>
      <td>${r[4]}</td>
      <td class="pct ${r[5]>0?'pct-low':'pct-high'}">${r[5]}</td>
      <td>${r[6].toFixed(1)}</td>
    </tr>`).join('');

    $$('#gamesBody tr.clickable').forEach(tr => tr.addEventListener('click', () => {
      showGameDetail(tr.dataset.game);
    }));
  }

  let sortState = { col: -1, asc: true };
  $('#gamesTable thead').addEventListener('click', e => {
    const th = e.target.closest('th');
    if (!th) return;
    const col = +th.dataset.col;
    if (sortState.col === col) sortState.asc = !sortState.asc;
    else { sortState.col = col; sortState.asc = true; }
    $$('#gamesTable th').forEach(h => h.classList.remove('sorted'));
    th.classList.add('sorted');
    th.querySelector('.arrow').innerHTML = sortState.asc ? '&#9650;' : '&#9660;';
    filteredRows.sort((a, b) => {
      let va = a[col], vb = b[col];
      if (typeof va === 'number') return sortState.asc ? va - vb : vb - va;
      return sortState.asc ? String(va).localeCompare(String(vb)) : String(vb).localeCompare(String(va));
    });
    renderGames();
  });

  ['gameFilterText','filterPlatform','filterEngine','filterSelf','filterHasUnknown'].forEach(id => {
    const el = $('#'+id);
    el.addEventListener('input', applyFilters);
    el.addEventListener('change', applyFilters);
  });
  renderGames();
})();

// --- ENGINES ---
(function() {
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
  ].map(([l, v, c]) => `<div class="card"><div class="card-label">${l}</div><div class="card-value ${c}">${v}</div></div>`).join('');

  const maxGameCount = summary.length > 0 ? summary[0].game_count : 1;
  const eColors = { 'Native/SCE': 'fill-green', 'Native': 'fill-blue', 'Unity': 'fill-purple', 'Unreal Engine 4': 'fill-yellow', 'Unreal Engine 5': 'fill-red' };
  $('#engineDistBars').innerHTML = summary.map(s =>
    `<div class="hbar"><div class="hbar-label">${s.engine}</div><div class="hbar-track"><div class="hbar-fill ${eColors[s.engine]||'fill-blue'}" style="width:${maxGameCount>0?(s.game_count/maxGameCount*100).toFixed(1):'0'}%"></div></div><div class="hbar-count">${s.game_count} (${s.avg_confidence.toFixed(0)}% avg)</div></div>`
  ).join('');

  let rows = hints.map(h => [h.display_name||h.name, h.engine, h.score, h.confidence, (h.third_party_libs||[]).join(', '), (h.custom_forks||[]).join(', '), h.build_system||'']);
  let filtered = [...rows];

  function renderEngineTable() {
    $('#engineBody').innerHTML = filtered.map(r => `<tr class="clickable" data-game="${r[0]}">
      <td title="${r[0]}">${trunc(r[0],32)}</td>
      <td>${r[1]?`<span class="pill pill-eng">${r[1]}</span>`:'-'}</td>
      <td style="font-family:monospace">${r[2]}</td>
      <td class="pct ${pctCls(r[3])}">${r[3]}%</td>
      <td style="font-size:0.72rem">${r[4]?trunc(r[4],40):'-'}</td>
      <td style="font-size:0.72rem;color:${r[5]?'#f85149':'#8b949e'}">${r[5]?trunc(r[5],40):'-'}</td>
      <td>${r[6]||'-'}</td>
    </tr>`).join('');

    $$('#engineBody tr.clickable').forEach(tr => tr.addEventListener('click', () => {
      const gameName = tr.dataset.game;
      const h = hints.find(x => (x.display_name||x.name) === gameName);
      if (!h) return;
      openDetail(h.display_name || h.name, `
        <div class="detail-section"><h3>Engine Overview</h3><div class="detail-kv">
          <div class="k">Engine</div><div class="v">${h.engine}</div>
          <div class="k">Score</div><div class="v" style="font-family:monospace">${h.score}</div>
          <div class="k">Confidence</div><div class="v">${h.confidence}%</div>
          ${h.build_system ? `<div class="k">Build System</div><div class="v">${h.build_system}</div>` : ''}
          ${h.source_depot ? `<div class="k">Source Depot</div><div class="v">${h.source_depot}</div>` : ''}
        </div></div>
        ${(h.sce_libraries||[]).length ? `<div class="detail-section"><h3>SCE Libraries (${(h.sce_libraries||[]).length})</h3><p style="color:#8b949e;font-size:0.82rem">${(h.sce_libraries||[]).join(', ')}</p></div>` : ''}
        ${(h.third_party_libs||[]).length ? `<div class="detail-section"><h3>Third-Party Libraries</h3><p style="color:#8b949e;font-size:0.82rem">${(h.third_party_libs||[]).join(', ')}</p></div>` : ''}
        ${(h.custom_forks||[]).length ? `<div class="detail-section"><h3 style="color:#f85149">Custom Forks</h3><p style="color:#f85149;font-size:0.82rem">${(h.custom_forks||[]).join(', ')}</p></div>` : ''}
        ${(h.sdk_hints||[]).length ? `<div class="detail-section"><h3>SDK Hints</h3><p style="color:#8b949e;font-size:0.82rem">${(h.sdk_hints||[]).join(', ')}</p></div>` : ''}
        ${(h.detected_versions||[]).length ? `<div class="detail-section"><h3>Detected Versions</h3><p style="color:#8b949e;font-size:0.82rem">${(h.detected_versions||[]).join(', ')}</p></div>` : ''}
        ${(h.evidence||[]).length ? `<div class="detail-section"><h3>Evidence Strings (${(h.evidence||[]).length})</h3><div class="table-wrap"><table class="detail-table"><thead><tr><th>String</th></tr></thead><tbody>${(h.evidence||[]).slice(0,30).map(e => `<tr><td style="font-family:monospace;font-size:0.72rem">${e}</td></tr>`).join('')}</tbody></table></div></div>` : ''}
        ${(h.lib_versions||[]).length ? `<div class="detail-section"><h3>SDK Library Versions</h3><div class="table-wrap"><table class="detail-table"><thead><tr><th>Library</th><th>Version</th><th>Raw</th></tr></thead><tbody>${(h.lib_versions||[]).map(lv => `<tr><td style="font-family:monospace;font-size:0.78rem">${lv.name}</td><td style="font-variant-numeric:tabular-nums">${lv.version_string}</td><td style="font-family:monospace;font-size:0.72rem;color:#8b949e">0x${lv.version_raw.toString(16).padStart(8,'0')}</td></tr>`).join('')}</tbody></table></div></div>` : ''}
      `);
    }));
  }

  let sortState = { col: -1, asc: true };
  $('#engineTable thead').addEventListener('click', e => {
    const th = e.target.closest('th');
    if (!th) return;
    const col = +th.dataset.col;
    if (sortState.col === col) sortState.asc = !sortState.asc;
    else { sortState.col = col; sortState.asc = true; }
    $$('#engineTable th').forEach(h => h.classList.remove('sorted'));
    th.classList.add('sorted');
    th.querySelector('.arrow').innerHTML = sortState.asc ? '&#9650;' : '&#9660;';
    filtered.sort((a, b) => {
      let va = a[col], vb = b[col];
      if (typeof va === 'number') return sortState.asc ? va - vb : vb - va;
      return sortState.asc ? String(va).localeCompare(String(vb)) : String(vb).localeCompare(String(va));
    });
    renderEngineTable();
  });
  renderEngineTable();
})();

// --- LIBRARIES ---
(function() {
  const libDetailMap = {};
  (D.library_details || []).forEach(d => { libDetailMap[d.name] = d; });

  let rows = D.library_priority.map(l => [l.name, l.game_count, l.import_count, l.unique_nid_count]);
  const tbody = $('#libBody');
  function render(data) {
    tbody.innerHTML = data.map(r => `<tr class="clickable" data-lib="${r[0]}">
      <td>${r[0]}</td><td>${r[1]}</td><td>${fmt(r[2])}</td><td>${fmt(r[3])}</td>
    </tr>`).join('');
    $$('#libBody tr.clickable').forEach(tr => tr.addEventListener('click', () => {
      const d = libDetailMap[tr.dataset.lib];
      if (!d) return;
      const gamesHtml = d.games.map(g => `<tr><td>${trunc(g.title_name||g.game,30)}</td><td>${fmt(g.import_count)}</td><td>${fmt(g.unique_nid_count)}</td></tr>`).join('');
      const nidsHtml = d.top_nids.map(n => `<tr><td style="font-family:monospace;font-size:0.72rem">${n.nid_hash}</td><td>${n.resolved_name||'-'}</td><td>${fmt(n.count)}</td></tr>`).join('');
      const unkHtml = d.unknown_nids.map(n => `<tr><td style="font-family:monospace;font-size:0.72rem">${n.nid_hash}</td><td>${fmt(n.count)}</td></tr>`).join('');

      openDetail(d.name, `
        <div class="detail-section"><h3>Overview</h3><div class="detail-kv">
          <div class="k">Games</div><div class="v">${d.game_count}</div>
          <div class="k">Total Imports</div><div class="v">${fmt(d.total_imports)}</div>
          <div class="k">Unique NIDs</div><div class="v">${fmt(d.unique_nid_count)}</div>
        </div></div>
        <div class="detail-section"><h3>Games (${d.games.length})</h3><div class="table-wrap"><table class="detail-table">
          <thead><tr><th>Game</th><th>Imports</th><th>Unique NIDs</th></tr></thead><tbody>${gamesHtml}</tbody></table></div></div>
        <div class="detail-section"><h3>Top NIDs</h3><div class="table-wrap"><table class="detail-table">
          <thead><tr><th>NID Hash</th><th>Resolved Name</th><th>Count</th></tr></thead><tbody>${nidsHtml}</tbody></table></div></div>
        ${d.unknown_nids.length > 0 ? `<div class="detail-section"><h3>Unknown NIDs (${d.unknown_nids.length})</h3><div class="table-wrap"><table class="detail-table">
          <thead><tr><th>NID Hash</th><th>Count</th></tr></thead><tbody>${unkHtml}</tbody></table></div></div>` : ''}
      `);
    }));
  }
  render(rows);
  let sortState = { col: -1, asc: true };
  $('#libTable thead').addEventListener('click', e => {
    const th = e.target.closest('th');
    if (!th) return;
    const col = +th.dataset.col;
    if (sortState.col === col) sortState.asc = !sortState.asc;
    else { sortState.col = col; sortState.asc = true; }
    $$('#libTable th').forEach(h => h.classList.remove('sorted'));
    th.classList.add('sorted');
    th.querySelector('.arrow').innerHTML = sortState.asc ? '&#9650;' : '&#9660;';
    rows.sort((a, b) => {
      if (typeof a[col] === 'number') return sortState.asc ? a[col] - b[col] : b[col] - a[col];
      return sortState.asc ? a[col].localeCompare(b[col]) : b[col].localeCompare(a[col]);
    });
    render(rows);
  });

  // Heatmap
  const hm = D.heatmap;
  if (hm.libraries.length) {
    const maxLog = Math.max(...hm.log_matrix.flat());
    let html = '<table class="heatmap"><thead><tr><th></th>';
    hm.games.forEach(g => { html += `<th title="${g}">${trunc(g,10)}</th>`; });
    html += '</tr></thead><tbody>';
    hm.libraries.forEach((lib, i) => {
      html += `<tr><td class="lib-name" title="${lib}">${lib}</td>`;
      hm.log_matrix[i].forEach((v, j) => {
        const raw = hm.raw_matrix[i][j];
        const intensity = maxLog > 0 ? v / maxLog : 0;
        const r = Math.round(35 + intensity * 198);
        const g = Math.round(134 + intensity * 11);
        const b = Math.round(54 - intensity * 15);
        html += `<td style="background:rgba(${r},${g},${b},0.85)" title="${lib}: ${raw} in ${hm.games[j]}"></td>`;
      });
      html += '</tr>';
    });
    html += '</tbody></table>';
    $('#heatmapWrap').innerHTML = html;
  }
})();

// --- SCE & THIRD PARTY LIBRARIES ---
(function() {
  const isScePlatform = n => n.startsWith('libSce') || n.startsWith('libkernel') || n.startsWith('libc');

  // Third Party bars
  const tpData = D.library_priority.filter(l => !isScePlatform(l.name));
  const tpMax = tpData.length ? Math.max(...tpData.map(l => l.game_count)) : 1;
  const tpHtml = tpData.slice(0, 30).map(l =>
    `<div class="hbar"><div class="hbar-label">${l.name}</div><div class="hbar-track"><div class="hbar-fill fill-blue" style="width:${(l.game_count/tpMax*100).toFixed(1)}%"></div></div><div class="hbar-count">${l.game_count} games</div></div>`
  ).join('');
  document.getElementById('thirdPartyBars').innerHTML = tpHtml || '<p style="color:#8b949e;font-size:0.82rem">No third-party library data.</p>';

  // SCE library stats grouped by category
  const stats = D.sce_library_stats || [];
  const catOrder = ['Graphics', 'Audio', 'Input', 'Network', 'User', 'Storage', 'System', 'Unknown'];
  const catLabels = { Graphics:'Graphics', Audio:'Audio', Input:'Input', Network:'Network', User:'User', Storage:'Storage', System:'System', Unknown:'Other' };
  const cats = {};
  stats.forEach(s => {
    if (!cats[s.category]) cats[s.category] = [];
    cats[s.category].push(s);
  });

  let barsHtml = '';
  catOrder.forEach(cat => {
    const group = cats[cat];
    if (!group || !group.length) return;
    barsHtml += `<h4 style="color:#e6edf3;font-size:0.82rem;margin:14px 0 6px;text-transform:uppercase;letter-spacing:0.05em">${catLabels[cat]||cat}</h4>`;
    const maxGc = Math.max(...group.map(s => s.game_count));
    group.sort((a, b) => b.game_count - a.game_count);
    group.forEach(s => {
      barsHtml += `<div class="hbar"><div class="hbar-label" style="cursor:pointer;color:#58a6ff" data-sce="${s.library}">${s.library}</div><div class="hbar-track"><div class="hbar-fill fill-blue" style="width:${(s.game_count/maxGc*100).toFixed(1)}%"></div></div><div class="hbar-count">${s.game_count} games, ${fmt(s.import_count)} imports</div></div>`;
    });
  });
  document.getElementById('sceCategoryBars').innerHTML = barsHtml || '<p style="color:#8b949e;font-size:0.82rem">No SCE library data available.</p>';

  // SCE heatmap — presence matrix
  const hm = D.sce_heatmap;
  if (hm && hm.libraries.length) {
    let hHtml = '<table class="heatmap"><thead><tr><th></th>';
    hm.games.forEach(g => { hHtml += `<th title="${g}">${trunc(g,10)}</th>`; });
    hHtml += '</tr></thead><tbody>';
    hm.libraries.forEach((lib, i) => {
      hHtml += `<tr><td class="lib-name" style="cursor:pointer;color:#58a6ff" data-sce="${lib}">${lib}</td>`;
      hm.raw_matrix[i].forEach(v => {
        hHtml += v > 0
          ? '<td class="presence-cell" style="background:#1f6feb;color:#e6edf3;border-radius:3px">&#9632;</td>'
          : '<td class="presence-cell"></td>';
      });
      hHtml += '</tr>';
    });
    hHtml += '</tbody></table>';
    document.getElementById('sceHeatmapWrap').innerHTML = hHtml;
  } else {
    document.getElementById('sceHeatmapWrap').innerHTML = '<p style="color:#8b949e;font-size:0.82rem">No SCE library heatmap available.</p>';
  }

  // SCE version distribution
  const versions = D.sce_library_versions || [];
  if (versions.length) {
    const grouped = {};
    versions.forEach(v => { if (!grouped[v.library]) grouped[v.library] = []; grouped[v.library].push(v); });
    let vHtml = '';
    Object.keys(grouped).sort().forEach(lib => {
      const vs = grouped[lib].sort((a, b) => b.version_raw - a.version_raw);
      vHtml += `<details style="margin-bottom:6px" open>
        <summary style="cursor:pointer;padding:8px 12px;background:#0d1117;border:1px solid #30363d;border-radius:6px;font-size:0.85rem;color:#c9d1d9">
          <strong style="color:#58a6ff">${lib}</strong> &mdash; ${vs.length} version(s), ${vs[0].game_count} game(s)
        </summary>
        <div style="padding:8px 12px;border:1px solid #30363d;border-top:0;border-radius:0 0 6px 6px">
          <table style="width:100%;font-size:0.82rem;border-collapse:collapse">
            <thead><tr style="color:#8b949e"><th style="text-align:left;padding:4px 8px">Version</th><th style="text-align:left;padding:4px 8px">Raw</th><th style="text-align:left;padding:4px 8px">Games</th></tr></thead>
            <tbody>${vs.map(v => `<tr><td style="padding:4px 8px;font-variant-numeric:tabular-nums">${v.version_string}</td><td style="padding:4px 8px;font-family:monospace;font-size:0.72rem;color:#8b949e">0x${v.version_raw.toString(16).padStart(8,'0')}</td><td style="padding:4px 8px;font-size:0.78rem">${v.games.map((g, i) => `<a href="#" class="game-link" data-game-id="${v.game_ids[i]}" style="color:#58a6ff">${trunc(g,24)}</a>`).join(', ')}</td></tr>`).join('')}</tbody>
          </table>
        </div>
      </details>`;
    });
    document.getElementById('sceVersionDist').innerHTML = vHtml;
  } else {
    document.getElementById('sceVersionDist').innerHTML = '<p style="color:#8b949e;font-size:0.82rem">No SDK version data for SCE libraries.</p>';
  }

  // Click: SCE bars -> detail
  document.getElementById('sceCategoryBars').addEventListener('click', e => {
    const el = e.target.closest('[data-sce]');
    if (el) showSceDetail(el.dataset.sce);
  });

  // Click: SCE heatmap -> detail
  document.getElementById('sceHeatmapWrap').addEventListener('click', e => {
    const el = e.target.closest('[data-sce]');
    if (el) showSceDetail(el.dataset.sce);
  });

  // Click: SCE version game links
  document.getElementById('sceVersionDist').addEventListener('click', e => {
    const link = e.target.closest('.game-link');
    if (!link) return;
    e.preventDefault();
    showGameDetail(link.dataset.gameId);
  });

  function showSceDetail(lib) {
    const s = stats.find(x => x.library === lib);
    if (!s) return;
    const gamesHtml = s.games.map((g, i) =>
      `<div style="padding:2px 0;font-size:0.82rem"><a href="#" class="game-link" data-game-id="${s.game_ids[i]}" style="color:#58a6ff">&#8226; ${trunc(g,40)}</a></div>`
    ).join('');
    const vHtml = s.versions.length
      ? s.versions.map(v =>
        `<tr><td style="font-variant-numeric:tabular-nums">${v.version_string}</td><td style="font-family:monospace;font-size:0.72rem;color:#8b949e">0x${v.version_raw.toString(16).padStart(8,'0')}</td><td>${v.game_count}</td></tr>`
      ).join('')
      : '';
    openDetail(lib, `
      <div class="detail-section"><div class="detail-kv">
        <div class="k">Category</div><div class="v">${catLabels[s.category]||s.category}</div>
        <div class="k">Games</div><div class="v">${s.game_count}</div>
        <div class="k">Imports</div><div class="v">${fmt(s.import_count)}</div>
        <div class="k">Modules</div><div class="v">${s.module_count > 0 ? s.module_count : 'Not available in current dataset'}</div>
      </div></div>
      <div class="detail-section"><h3>Games (${s.game_count})</h3>${gamesHtml}</div>
      ${vHtml ? `<div class="detail-section"><h3>Versions</h3><div class="table-wrap"><table class="detail-table"><thead><tr><th>Version</th><th>Raw</th><th>Game Count</th></tr></thead><tbody>${vHtml}</tbody></table></div></div>` : ''}
    `);
  }
})();

// --- NIDS ---
(function() {
  const ns = D.nid_stats;
  const total = ns.resolved_count + ns.unknown_count;
  const rPct = total > 0 ? (ns.resolved_count / total * 100) : 0;
  const uPct = 100 - rPct;
  $('#nidResDetail').innerHTML = `
    <div class="hbar"><div class="hbar-label">Resolved</div><div class="hbar-track"><div class="hbar-fill fill-green" style="width:${rPct.toFixed(1)}%"></div></div><div class="hbar-count">${ns.resolved_count.toLocaleString()} (${rPct.toFixed(1)}%)</div></div>
    <div class="hbar"><div class="hbar-label">Unknown</div><div class="hbar-track"><div class="hbar-fill fill-red" style="width:${uPct.toFixed(1)}%"></div></div><div class="hbar-count">${ns.unknown_count.toLocaleString()} (${uPct.toFixed(1)}%)</div></div>`;

  const maxCount = ns.top_nids.length > 0 ? ns.top_nids[0].count : 1;
  $('#nidBars').innerHTML = ns.top_nids.slice(0, 20).map(n => {
    const w = (n.count / maxCount * 100).toFixed(1);
    const label = n.resolved_name || n.nid_hash;
    return `<div class="hbar"><div class="hbar-label" title="${n.nid_hash} &rarr; ${label}"><code style="font-size:0.7rem;color:#8b949e;margin-right:6px">${n.nid_hash}</code>${trunc(label,24)}</div><div class="hbar-track"><div class="hbar-fill fill-blue" style="width:${w}%"></div></div><div class="hbar-count">${fmt(n.count)}</div></div>`;
  }).join('');

  const groups = D.library_nid_breakdown;
  $('#libNidBreakdown').innerHTML = groups.map((g, i) => {
      const nidRows = g.top_nids.map(n => {
      const label = n.resolved_name || n.nid_hash;
      const mc = g.top_nids[0].count;
      const w = mc > 0 ? (n.count / mc * 100).toFixed(1) : '0';
      return `<div class="hbar"><div class="hbar-label" title="${n.nid_hash}"><code style="font-size:0.7rem;color:#8b949e;margin-right:6px">${n.nid_hash}</code>${trunc(label,24)}</div><div class="hbar-track"><div class="hbar-fill fill-blue" style="width:${w}%"></div></div><div class="hbar-count">${fmt(n.count)}</div></div>`;
    }).join('');
    return `<details style="margin-bottom:8px" ${i < 5 ? 'open' : ''}>
      <summary style="cursor:pointer;padding:8px 12px;background:#0d1117;border:1px solid #30363d;border-radius:6px;font-size:0.88rem;color:#c9d1d9">
        <strong style="color:#58a6ff">${g.library}</strong> &mdash; ${g.game_count} games, ${fmt(g.total_imports)} imports, ${fmt(g.unique_nid_count)} unique NIDs
      </summary>
      <div style="padding:12px 12px 4px;border:1px solid #30363d;border-top:0;border-radius:0 0 6px 6px">${nidRows}</div>
    </details>`;
  }).join('');
})();

// --- SDK TIMELINE ---
(function() {
  const rows = [
    ["0.70","—","—","2018-11-05 — PS5 0.70.060 build","High"],
    ["0.83","—","—","2019-06-05 — PS5 0.83.00 build","High"],
    ["0.85.070","—","0.7.6","2019 — SDK/EMC association","High"],
    ["1.x","—","1.0.4","2020 — SDK generation / EMC association","High"],
    ["2.x","—","1.2.3","2020–2021 — SDK generation / EMC association","Medium"],
    ["3.00","—","1.4.2","2021-04-06 — PS5 3.00 build","High"],
    ["3.20","—","1.4.2","2021 — 3.20 generation","High"],
    ["3.21","—","1.4.2","2021 — 3.21 generation","High"],
    ["4.00","UE 5.0 → 4.00.00.31","1.6.0","2021-09-03 — PS5 4.00 build","High"],
    ["4.50","UE 5.0 → 4.00.00.31","1.6.0","2021-11-17 — PS5 4.50 build","High"],
    ["4.51","UE 5.0 → 4.00.00.31","1.6.0","2022 — UE explicitly pairs SDK 4.00.00.31","High"],
    ["5.00","UE 5.1 → 5.00.00.33","1.8.2","2022 — SDK/EMC association","High"],
    ["5.50","—","1.8.3","2022 — SDK/EMC association","High"],
    ["6.x","—","—","2022–2023 — generation inferred from firmware/SDK evidence","Medium"],
    ["7.00","UE 5.2 → 7.00.00.38","—","2023 — Epic SDK pairing","High"],
    ["7.00.00.45","UE 5.3 → 7.00.00.45","—","2023 — Epic documents patched SDK","High"],
    ["9.00","UE 5.4 → 9.00.00.40","—","2024 — Epic SDK pairing","High"],
    ["9.20","—","1.14.3","2024 — EMC association","High"],
    ["10.00","UE 5.5 → 10.00.00.40","—","2024 — Epic SDK pairing","High"],
    ["11.00","UE 5.6 → 11.00.00.40","—","2025 — Epic SDK pairing","High"],
    ["11.00","UE 5.7 → 11.00.00.40","—","2025 — same SDK documented by Epic","High"],
    ["12.x","—","—","2026 — current/recent generation; insufficient public evidence","Medium/Low"],
  ];
  const confPill = c => {
    if(c.startsWith("High")) return '<span class="pill" style="background:#23863622;color:#3fb950;border:1px solid #23863644">High</span>';
    if(c.startsWith("Medium")) return '<span class="pill" style="background:#d2992222;color:#d29922;border:1px solid #d2992244">'+c+'</span>';
    return '<span class="pill" style="background:#da363322;color:#f85149;border:1px solid #da363344">'+c+'</span>';
  };
  const tbody = document.getElementById('sdkBody');
  function render(data) {
    tbody.innerHTML = data.map(r => `<tr>
      <td style="font-variant-numeric:tabular-nums"><strong>${r[0]}</strong></td>
      <td style="color:#8b949e">${r[1]}</td>
      <td style="font-family:monospace">${r[2]}</td>
      <td style="font-size:0.78rem">${r[3]}</td>
      <td>${confPill(r[4])}</td>
    </tr>`).join('');
  }
  render(rows);
  let sortState = {col:-1, asc:true};
  document.querySelector('#sdkTable thead').addEventListener('click', e => {
    const th = e.target.closest('th');
    if(!th) return;
    const col = +th.dataset.col;
    if(sortState.col===col) sortState.asc=!sortState.asc; else {sortState.col=col; sortState.asc=true;}
    document.querySelectorAll('#sdkTable th').forEach(h=>h.classList.remove('sorted'));
    th.classList.add('sorted');
    th.querySelector('.arrow').innerHTML = sortState.asc?'&#9650;':'&#9660;';
    rows.sort((a,b)=>{
      const va=a[col], vb=b[col];
      return sortState.asc ? String(va).localeCompare(String(vb)) : String(vb).localeCompare(String(va));
    });
    render(rows);
  });
}());

// --- SEGMENTS ---
(function() {
  const segs = D.segments.sort((a, b) => b.total_mb - a.total_mb);
  const maxTotal = segs.length > 0 ? segs[0].total_mb : 1;
  $('#segBars').innerHTML = segs.map(s => {
    const total = s.rx_mb + s.r_mb + s.rw_mb + s.other_mb;
    const w = total > 0 ? (total / maxTotal * 100).toFixed(1) : '0';
    return `<div style="display:flex;align-items:center;margin-bottom:3px">
      <div style="width:160px;font-size:0.78rem;color:#c9d1d9;text-overflow:ellipsis;overflow:hidden;white-space:nowrap" title="${s.game}">${trunc(s.game,22)}</div>
      <div class="seg-bar" style="width:${w}%">
        <div class="seg-rx" style="width:${total>0?(s.rx_mb/total*100).toFixed(1):'0'}%" title="RX: ${s.rx_mb.toFixed(1)} MB"></div>
        <div class="seg-r" style="width:${total>0?(s.r_mb/total*100).toFixed(1):'0'}%" title="R: ${s.r_mb.toFixed(1)} MB"></div>
        <div class="seg-rw" style="width:${total>0?(s.rw_mb/total*100).toFixed(1):'0'}%" title="RW: ${s.rw_mb.toFixed(1)} MB"></div>
        <div class="seg-other" style="width:${total>0?(s.other_mb/total*100).toFixed(1):'0'}%" title="Other: ${s.other_mb.toFixed(1)} MB"></div>
      </div>
      <div style="width:70px;text-align:right;font-size:0.75rem;color:#8b949e;margin-left:8px">${total.toFixed(1)} MB</div>
    </div>`;
  }).join('');
})();

// --- STATISTICS ---
(function() {
  const s = D.statistics;
  if (!s) return;
  const entry = (label, game, value) => `<div class="stat-row"><span>${label}: ${trunc(game,20)}</span><span class="sv">${value}</span></div>`;
  const section = (title, entries) => `<div class="stat-card"><h3>${title}</h3>${entries}</div>`;
  let html = '';
  html += section('Largest Binaries', s.top_5_largest.map(e => entry('', e.game, e.value.toFixed(1) + ' MB')).join(''));
  html += section('Smallest Binaries', s.top_5_smallest.map(e => entry('', e.game, e.value.toFixed(1) + ' MB')).join(''));
  html += section('Most Imports', s.top_5_most_imports.map(e => entry('', e.game, fmt(Math.round(e.value)))).join(''));
  html += section('Most Libraries', s.top_5_most_libs.map(e => entry('', e.game, fmt(Math.round(e.value)))).join(''));
  html += section('Highest Unknown %', s.top_5_highest_unknown.map(e => entry('', e.game, e.value.toFixed(1) + '%')).join(''));
  html += section('Averages', [
    `<div class="stat-row"><span>Code (RX)</span><span class="sv">${s.avg_code_size_mb.toFixed(1)} MB</span></div>`,
    `<div class="stat-row"><span>Data (RW)</span><span class="sv">${s.avg_data_size_mb.toFixed(1)} MB</span></div>`,
    `<div class="stat-row"><span>Read-only (R)</span><span class="sv">${s.avg_rodata_size_mb.toFixed(1)} MB</span></div>`,
    `<div class="stat-row"><span>Other</span><span class="sv">${s.avg_other_size_mb.toFixed(1)} MB</span></div>`,
    `<div class="stat-row"><span>Total Code</span><span class="sv">${s.total_code_mb.toFixed(1)} MB</span></div>`,
    `<div class="stat-row"><span>Total Data</span><span class="sv">${s.total_data_mb.toFixed(1)} MB</span></div>`,
  ].join(''));
  $('#statGrid').innerHTML = html;

  const lv = D.library_versions || [];
  if (lv.length) {
    const totalGames = D.games.length;
    const uniqueLibs = [...new Set(lv.map(v => v.library))].length;
    const gamesWithVersions = new Set();
    lv.forEach(v => v.game_ids.forEach(g => gamesWithVersions.add(g)));
    let lvHtml = `<div class="section"><h2>SDK Library Version Distribution</h2>
      <div class="cards" style="margin-bottom:16px">
        <div class="card"><div class="card-label">Games with Version Info</div><div class="card-value blue">${gamesWithVersions.size}/${totalGames}</div></div>
        <div class="card"><div class="card-label">Unique Libraries</div><div class="card-value yellow">${uniqueLibs}</div></div>
        <div class="card"><div class="card-label">Library-Version Pairs</div><div class="card-value">${lv.length}</div></div>
      </div>`;
    const grouped = {};
    lv.forEach(v => {
      if (!grouped[v.library]) grouped[v.library] = [];
      grouped[v.library].push(v);
    });
    Object.keys(grouped).sort().forEach(lib => {
      const versions = grouped[lib].sort((a, b) => b.version_raw - a.version_raw);
      lvHtml += `<details style="margin-bottom:6px" open>
        <summary style="cursor:pointer;padding:8px 12px;background:#0d1117;border:1px solid #30363d;border-radius:6px;font-size:0.85rem;color:#c9d1d9">
          <strong style="color:#58a6ff">${lib}</strong> &mdash; ${versions.length} version(s), ${versions[0].game_count} game(s)
        </summary>
        <div style="padding:8px 12px;border:1px solid #30363d;border-top:0;border-radius:0 0 6px 6px">
          <table style="width:100%;font-size:0.82rem;border-collapse:collapse">
            <thead><tr style="color:#8b949e"><th style="text-align:left;padding:4px 8px">Version</th><th style="text-align:left;padding:4px 8px">Raw</th><th style="text-align:left;padding:4px 8px">Games</th></tr></thead>
            <tbody>${versions.map(v => `<tr><td style="padding:4px 8px;font-variant-numeric:tabular-nums">${v.version_string}</td><td style="padding:4px 8px;font-family:monospace;font-size:0.72rem;color:#8b949e">0x${v.version_raw.toString(16).padStart(8,'0')}</td><td style="padding:4px 8px;font-size:0.78rem">${v.games.map((g, i) => `<a href="#" class="game-link" data-game-id="${v.game_ids[i]}" style="color:#58a6ff">${trunc(g,24)}</a>`).join(', ')}</td></tr>`).join('')}</tbody>
          </table>
        </div>
      </details>`;
    });
    lvHtml += '</div>';
    const el = document.createElement('div');
    el.innerHTML = lvHtml;
    $('#statGrid').parentNode.appendChild(el);
  }

  $('#statGrid').addEventListener('click', e => {
    const link = e.target.closest('.game-link');
    if (!link) return;
    e.preventDefault();
    const gameId = link.dataset.gameId;
    showGameDetail(gameId);
  });
})();

// --- GRAPH ---
(function() {
  const libs = D.library_priority;
  const games = D.games;
  if (!libs.length || !games.length) return;

  const W = 1200, H = Math.max(500, libs.length * 50 + 100);
  const libX = 200, gameX = W - 200;
  const libSpacing = H / (libs.length + 1);
  const gameSpacing = H / (games.length + 1);

  const libPositions = {};
  libs.forEach((l, i) => { libPositions[l.name] = { x: libX, y: libSpacing * (i + 1) }; });

  const gamePositions = {};
  games.forEach((g, i) => { gamePositions[g.name] = { x: gameX, y: gameSpacing * (i + 1) }; });

  let svg = `<svg width="${W}" height="${H}" xmlns="http://www.w3.org/2000/svg" style="font-family:system-ui,sans-serif;">`;

  for (const [game, doc] of Object.entries(D.game_details || {})) {
    const gp = gamePositions[game];
    if (!gp) continue;
    for (const imp of doc.imports || []) {
      const lp = libPositions[imp.library_name];
      if (!lp) continue;
      svg += `<line x1="${lp.x+80}" y1="${lp.y}" x2="${gp.x-60}" y2="${gp.y}" stroke="#30363d" stroke-width="0.5" opacity="0.3"/>`;
    }
  }

  libs.forEach(l => {
    const p = libPositions[l.name];
    svg += `<rect x="${p.x-10}" y="${p.y-8}" width="160" height="16" rx="3" fill="#1f6feb" opacity="0.8" class="graph-node" data-type="lib" data-name="${l.name}" style="cursor:pointer"/>`;
    svg += `<text x="${p.x+70}" y="${p.y+4}" text-anchor="middle" fill="#e6edf3" font-size="9">${trunc(l.name,20)} (${l.game_count})</text>`;
  });

  games.forEach(g => {
    const p = gamePositions[g.name];
    svg += `<rect x="${p.x-60}" y="${p.y-6}" width="120" height="12" rx="3" fill="#238636" opacity="0.7" class="graph-node" data-type="game" data-name="${g.name}" style="cursor:pointer"/>`;
    svg += `<text x="${p.x}" y="${p.y+3}" text-anchor="middle" fill="#e6edf3" font-size="7">${trunc(g.title_name || g.name,16)}</text>`;
  });

  svg += '</svg>';
  $('#graphWrap').innerHTML = svg;

  $$('.graph-node').forEach(node => node.addEventListener('click', () => {
    const type = node.dataset.type;
    const name = node.dataset.name;
    if (type === 'lib') {
      const d = (D.library_details || []).find(x => x.name === name);
      if (d) {
        const gamesHtml = d.games.map(g => `<tr><td>${trunc(g.title_name||g.game,30)}</td><td>${fmt(g.import_count)}</td></tr>`).join('');
        openDetail(d.name, `<div class="detail-section"><div class="detail-kv"><div class="k">Games</div><div class="v">${d.game_count}</div><div class="k">Imports</div><div class="v">${fmt(d.total_imports)}</div></div></div><div class="detail-section"><h3>Games</h3><div class="table-wrap"><table class="detail-table"><thead><tr><th>Game</th><th>Imports</th></tr></thead><tbody>${gamesHtml}</tbody></table></div></div>`);
      }
    } else {
      const d = (D.game_details || []).find(x => x.name === name);
      if (d) openDetail(d.title_name||d.name, `<div class="detail-kv"><div class="k">Imports</div><div class="v">${fmt(d.imports.length)}</div><div class="k">Libraries</div><div class="v">${d.import_summary.length}</div></div>`);
    }
}));
})();

// --- LOADER ---
(function() {
  if (!D.loader_summary) return;
  const s = D.loader_summary;
  $('#loaderTab').style.display = '';
  $('#loaderEmpty').style.display = 'none';
  $('#loaderContent').style.display = '';

  function pctBar(label, pct, color) {
    return `<div class="hbar"><div class="hbar-label">${label}</div><div class="hbar-track"><div class="hbar-fill fill-${color}" style="width:${pct.toFixed(1)}%"></div></div><span class="hbar-count">${pct.toFixed(1)}%</span></div>`;
  }

  function tripleBar(label, a, b, c) {
    const total = a + b + c;
    if (!total) return `<div class="hbar"><div class="hbar-label">${label}</div><div class="hbar-track" style="background:#0d1117;display:flex"><div style="width:100%;text-align:center;color:#8b949e;line-height:14px;font-size:0.7rem">No data</div></div></div>`;
    const rp = a / total * 100;
    const kp = b / total * 100;
    const sp = c / total * 100;
    return `<div class="hbar"><div class="hbar-label">${label}</div><div class="hbar-track" style="padding:0;display:flex;gap:0;background:#21262d"><div style="width:${rp.toFixed(1)}%;height:14px;background:#3fb950;min-width:2px" title="Resolved: ${fmt(a)}"></div><div style="width:${kp.toFixed(1)}%;height:14px;background:#58a6ff;min-width:2px" title="Known: ${fmt(b)}"></div><div style="width:${sp.toFixed(1)}%;height:14px;background:#f85149;min-width:2px" title="Stubbed: ${fmt(c)}"></div></div><span class="hbar-count" style="font-size:0.68rem;color:#8b949e;width:90px;text-align:right">${rp.toFixed(0)}% / ${kp.toFixed(0)}% / ${sp.toFixed(0)}%</span></div>`;
  }

  const totalImports = s.total_imports_resolved + s.total_imports_known + s.total_imports_stubbed;
  $('#loaderCards').innerHTML = `
    <div class="card"><div class="card-label">Games Loaded</div><div class="card-value">${s.successful}<span style="font-size:0.85rem;color:#8b949e;margin-left:6px">/ ${s.total_games}</span></div></div>
    <div class="card"><div class="card-label">Failed</div><div class="card-value yellow">${s.failed}</div></div>
    <div class="card"><div class="card-label">Modules</div><div class="card-value blue">${fmt(s.total_modules)}</div></div>
    <div class="card"><div class="card-label">Exports</div><div class="card-value blue">${fmt(s.total_exports)}</div></div>
    <div class="card"><div class="card-label">Resolution Rate</div><div class="card-value ${pctCls(s.avg_resolution_rate)}">${s.avg_resolution_rate.toFixed(1)}%</div></div>
    <div class="card"><div class="card-label">Imports</div><div class="card-value">${fmt(totalImports)}<span style="font-size:0.75rem;color:#8b949e;display:block">${fmt(s.total_imports_resolved)} resolved / ${fmt(s.total_imports_known)} known / ${fmt(s.total_imports_stubbed)} stubbed</span></div></div>`;

  const gamesWithLoader = (D.game_details || []).filter(g => g.imports_resolved != null);
  if (gamesWithLoader.length) {
    $('#loaderGameBars').innerHTML = '<div style="font-size:0.75rem;color:#8b949e;margin-bottom:8px"><span style="display:inline-block;width:12px;height:12px;background:#3fb950;margin-right:4px;vertical-align:middle"></span>Resolved <span style="display:inline-block;width:12px;height:12px;background:#58a6ff;margin:0 4px 0 12px;vertical-align:middle"></span>Known <span style="display:inline-block;width:12px;height:12px;background:#f85149;margin:0 4px 0 12px;vertical-align:middle"></span>Stubbed</div>'
      + gamesWithLoader.map(g => tripleBar(g.title_name || g.name, g.imports_resolved, g.imports_known, g.imports_stubbed)).join('');
  }

  if (s.top_unavailable && s.top_unavailable.length) {
    const maxUc = s.top_unavailable[0].game_count || 1;
    $('#loaderUnavailableBars').innerHTML = s.top_unavailable.map(u => {
      const pct = u.game_count / s.total_games * 100;
      return `<div class="hbar"><div class="hbar-label" style="width:280px">${u.module}</div><div class="hbar-track"><div class="hbar-fill fill-red" style="width:${(u.game_count / maxUc * 100).toFixed(1)}%"></div></div><span class="hbar-count">${u.game_count} games (${pct.toFixed(0)}%)</span></div>`;
    }).join('');
  }

  if (s.worst_games && s.worst_games.length) {
    const maxStub = s.worst_games[0].rate || 1;
    $('#loaderWorstBars').innerHTML = s.worst_games.map(w => {
      return `<div class="hbar"><div class="hbar-label" style="width:280px">${w.game}</div><div class="hbar-track"><div class="hbar-fill fill-red" style="width:${(w.rate / maxStub * 100).toFixed(1)}%"></div></div><span class="hbar-count">${fmt(w.stubbed)} / ${fmt(w.total)} (${w.rate.toFixed(1)}%)</span></div>`;
    }).join('');
  }
})();

// --- MIDDLEWARE ---
(function() {
  if (!D.middleware) return;
  const m = D.middleware;
  $('#middlewareTab').style.display = '';
  $('#middlewareEmpty').style.display = 'none';
  $('#middlewareContent').style.display = '';

  const s = m.summary;
  $('#middlewareCards').innerHTML = `
    <div class="card"><div class="card-label">Third-Party Modules</div><div class="card-value">${fmt(s.third_party_modules)}</div></div>
    <div class="card"><div class="card-label">Sony Modules</div><div class="card-value blue">${fmt(s.sony_modules)}</div></div>
    <div class="card"><div class="card-label">Unknown Modules</div><div class="card-value yellow">${fmt(s.unknown_modules)}</div></div>
    <div class="card"><div class="card-label">Games with Third-Party</div><div class="card-value green">${s.games_with_third_party}<span style="font-size:0.85rem;color:#8b949e;margin-left:6px">/ ${m.games.length}</span></div></div>`;

  if (s.products && s.products.length) {
    const maxMods = s.products[0].module_count || 1;
    $('#middlewareProductBars').innerHTML = '<div style="font-size:0.75rem;color:#8b949e;margin-bottom:8px">Detected middleware products across all games, ordered by total module count.</div>'
      + s.products.map(p => {
        const pct = p.module_count / maxMods * 100;
        return `<div class="hbar"><div class="hbar-label" style="width:280px">${p.vendor} &mdash; ${p.product}</div><div class="hbar-track"><div class="hbar-fill fill-blue" style="width:${pct.toFixed(1)}%"></div></div><span class="hbar-count">${p.module_count} module${p.module_count === 1 ? '' : 's'} / ${p.game_count} game${p.game_count === 1 ? '' : 's'}</span></div>`;
      }).join('');
  }

  const rows = m.games || [];
  const sel = $('#middlewareGameSelect');
  sel.innerHTML = rows.map((g, i) => `<option value="${i}">${g.name}${g.title_id ? ' (' + g.title_id + ')' : ''} &mdash; ${(g.third_party || []).length} / ${(g.sony || []).length} / ${(g.unknown || []).length}</option>`).join('');

  function badge(row) {
    return `<span style="display:inline-block;padding:1px 6px;border-radius:10px;font-size:0.65rem;font-weight:600;margin-right:6px;background:${row.kind === '3rd' ? 'rgba(63,185,80,0.15)' : row.kind === 'sce' ? 'rgba(88,166,255,0.15)' : 'rgba(177,186,196,0.15)'};color:${row.kind === '3rd' ? '#3fb950' : row.kind === 'sce' ? '#58a6ff' : '#b1bac4'}">${row.kind === '3rd' ? '3RD' : row.kind === 'sce' ? 'SCE' : '?'}</span>`;
  }

  function renderGame(i) {
    const g = rows[i];
    if (!g) return;
    const all = (g.third_party || []).map(x => Object.assign({kind: '3rd'}, x))
      .concat((g.sony || []).map(x => Object.assign({kind: 'sce'}, x)))
      .concat((g.unknown || []).map(x => Object.assign({kind: 'unk'}, x)));
    $('#middlewareGameBody').innerHTML = all.length
      ? all.map(x => `<tr>
        <td>${badge(x)}${x.file_name}<span style="color:#8b949e;font-size:0.7rem;margin-left:6px">${x.parseable ? '' : '(unreadable)'}</span></td>
        <td>${x.vendor || '-'}</td>
        <td>${x.product || '-'}</td>
        <td>${x.description || '-'}</td>
        <td>${fmt(x.imports)}</td>
      </tr>`).join('')
      : '<tr><td colspan="5" style="text-align:center;color:#8b949e">No PRX modules detected.</td></tr>';
  }

  sel.addEventListener('change', () => renderGame(parseInt(sel.value || '0', 10)));
  if (rows.length) renderGame(0);
})();

// --- ARTIFACTS ---
(function() {
  if (!D.artifacts) return;
  const a = D.artifacts;
  $('#artifactTab').style.display = '';
  $('#artifactEmpty').style.display = 'none';
  $('#artifactContent').style.display = '';
  $('#artifactCards').innerHTML = [
    ['Games', a.total_games, 'blue'],
    ['Files', fmt(a.total_files), 'green'],
    ['Categories', Object.keys(a.by_category||{}).length, 'yellow'],
    ['Extensions', Object.keys(a.by_extension||{}).length, ''],
  ].map(([l,v,c]) => `<div class="card"><div class="card-label">${l}</div><div class="card-value ${c}">${v}</div></div>`).join('');
  const catMax = Math.max(1, ...Object.values(a.by_category||{}));
  $('#artifactCategoryBars').innerHTML = Object.entries(a.by_category||{}).sort((x,y)=>y[1]-x[1]).map(([k,v]) => `<div class="hbar"><div class="hbar-label">${k}</div><div class="hbar-track"><div class="hbar-fill fill-blue" style="width:${(v/catMax*100).toFixed(1)}%"></div></div><div class="hbar-count">${fmt(v)}</div></div>`).join('') || '<p style="color:#8b949e">No category data.</p>';
  const extMax = Math.max(1, ...Object.values(a.by_extension||{}));
  const extSorted = Object.entries(a.by_extension||{}).sort((x,y)=>y[1]-x[1]).slice(0,20);
  $('#artifactExtBars').innerHTML = extSorted.map(([k,v]) => `<div class="hbar"><div class="hbar-label">${k}</div><div class="hbar-track"><div class="hbar-fill fill-purple" style="width:${(v/extMax*100).toFixed(1)}%"></div></div><div class="hbar-count">${fmt(v)}</div></div>`).join('');
  let perHtml = '<div style="font-size:0.75rem;color:#8b949e;margin-bottom:8px">Bare content is first-class — .pak not required. Relative paths preserved; unknown formats kept as structured artifacts.</div>';
  a.games.forEach(g => {
    const cat = g.by_category || {};
    const ext = g.by_extension || {};
    perHtml += `<details style="margin-bottom:6px"><summary style="cursor:pointer;padding:8px 12px;background:#0d1117;border:1px solid #30363d;border-radius:6px;font-size:0.85rem;color:#c9d1d9"><strong style="color:#58a6ff">${gameLabel(g.game)}</strong> &mdash; ${fmt(g.total_files)} files, ${(g.total_bytes/1048576).toFixed(1)} MB &mdash; shader:${cat.shader||0} texture:${cat.texture||0} audio:${cat.audio||0} executable:${cat.executable||0}</summary><div style="padding:8px 12px;border:1px solid #30363d;border-top:0;border-radius:0 0 6px 6px"><div style="display:flex;gap:12px;flex-wrap:wrap;margin-bottom:8px">`;
    Object.entries(cat).sort((x,y)=>y[1]-x[1]).forEach(([k,v]) => { perHtml += `<span style="background:#21262d;padding:2px 8px;border-radius:10px;font-size:0.72rem;color:#8b949e">${k}:${fmt(v)}</span>`; });
    perHtml += `</div><div style="max-height:200px;overflow-y:auto;border:1px solid #21262d;border-radius:4px;padding:6px"><table style="width:100%;font-size:0.72rem;border-collapse:collapse"><thead><tr style="color:#8b949e"><th style="text-align:left">Path</th><th>Ext</th><th>Category</th><th>Size</th></tr></thead><tbody>`;
    (g.artifacts||[]).slice(0,100).forEach(art => {
      perHtml += `<tr><td style="font-family:monospace;word-break:break-all">${art.relative_path}</td><td>${art.extension}</td><td style="color:#58a6ff">${art.category}</td><td style="text-align:right">${(art.size/1024).toFixed(1)} KB</td></tr>`;
    });
    if ((g.artifacts||[]).length > 100) perHtml += `<tr><td colspan="4" style="text-align:center;color:#8b949e">... and ${g.artifacts.length-100} more (full inventory in JSON)</td></tr>`;
    perHtml += `</tbody></table></div></div></details>`;
  });
  $('#artifactPerGame').innerHTML = perHtml;
  const crossMax = Math.max(1, ...a.games.map(g => (g.by_category.shader||0) + (g.by_category.texture||0)));
  let crossHtml = '';
  a.games.forEach(g => {
    const s = g.by_category.shader||0, t = g.by_category.texture||0, au = g.by_category.audio||0, e = g.by_category.executable||0;
    const total = s+t+au+e || 1;
    crossHtml += `<div class="hbar"><div class="hbar-label" style="width:220px" title="${gameLabel(g.game)}">${gameDisplay(g.game,22)}</div><div class="hbar-track" style="display:flex;gap:0;background:#21262d"><div style="width:${(s/total*100).toFixed(1)}%;height:14px;background:#8957e5" title="shader:${s}"></div><div style="width:${(t/total*100).toFixed(1)}%;height:14px;background:#1f6feb" title="texture:${t}"></div><div style="width:${(au/total*100).toFixed(1)}%;height:14px;background:#3fb950" title="audio:${au}"></div><div style="width:${(e/total*100).toFixed(1)}%;height:14px;background:#d29922" title="executable:${e}"></div></div><span class="hbar-count" style="font-size:0.68rem;width:90px;text-align:right">${s}/ ${t}/ ${au}/ ${e}</span></div>`;
  });
  $('#artifactCrossBars').innerHTML = crossHtml || '<p style="color:#8b949e">No cross-artifact data.</p>';
})();

// --- GLOBAL SEARCH ---
(function() {
  const searchIndex = [];
  (D.game_details || []).forEach(d => {
    d.imports.forEach(imp => {
      searchIndex.push({
        type: 'import',
        nid: imp.nid_hash,
        name: imp.resolved_name || '',
        library: imp.library_name,
        game: d.name,
        gameTitle: d.title_name || d.name,
      });
    });
  });
  (D.library_details || []).forEach(d => {
    d.top_nids.forEach(n => {
      searchIndex.push({
        type: 'library-nid',
        nid: n.nid_hash,
        name: n.resolved_name,
        library: d.name,
        game: '',
        gameTitle: '',
      });
    });
  });
  (D.game_details || []).forEach(d => {
    (d.lib_versions || []).forEach(lv => {
      searchIndex.push({
        type: 'lib-version',
        name: lv.name,
        version: lv.version_string,
        versionRaw: '0x' + lv.version_raw.toString(16).padStart(8,'0'),
        game: d.name,
        gameTitle: d.title_name || d.name,
      });
    });
  });

  // --- SHADER ---
  (function() {
    const s = D.shader_summary || { total_shaders: 0, by_stage: {}, total_resources: 0 };
    const hasData = s.total_shaders > 0 || Object.keys(s.by_stage||{}).length > 0 || s.total_resources > 0;
    const cards = [
      ['Total Shaders', hasData ? s.total_shaders : '—', 'blue'],
      ['Total Resources', hasData ? s.total_resources : '—', 'yellow'],
      ['Stages', hasData ? Object.keys(s.by_stage||{}).length : '—', ''],
    ];
    const el = document.getElementById('shaderCards');
    if (el) el.innerHTML = cards.map(([l,v,c]) => `<div class="card"><div class="card-label">${l}</div><div class="card-value ${c}">${v}</div></div>`).join('');
    const bars = document.getElementById('shaderStageBars');
    if (bars) {
      if (!hasData) {
        bars.innerHTML = '<p style="color:#8b949e">Not yet analyzed — shader analysis not wired to real data.</p>';
      } else {
        const max = Math.max(1, ...Object.values(s.by_stage||{}));
        bars.innerHTML = Object.entries(s.by_stage||{}).map(([k,v]) => `<div class="hbar"><div class="hbar-label">${k}</div><div class="hbar-track"><div class="hbar-fill fill-purple" style="width:${(v/max*100).toFixed(1)}%"></div></div><div class="hbar-count">${v}</div></div>`).join('');
      }
      const resEl = document.getElementById('shaderResources');
      if (resEl) {
        if (!hasData) resEl.innerHTML = '<p style="color:#8b949e;font-size:0.82rem">Not available in current dataset.</p>';
        else resEl.innerHTML = `<p style="color:#8b949e;font-size:0.82rem">Resources derived from ${s.total_resources} bindings.</p>`;
      }
    }
  })();

  // --- FIRMWARE ---
  (function() {
    const f = D.firmware_summary || { total_modules: 0, total_libraries: 0, by_version: {} };
    const hasData = f.total_modules > 0 || f.total_libraries > 0 || Object.keys(f.by_version||{}).length > 0;
    const cards = [
      ['Modules', hasData ? f.total_modules : '—', 'blue'],
      ['Libraries', hasData ? f.total_libraries : '—', 'green'],
      ['Versions', hasData ? Object.keys(f.by_version||{}).length : '—', 'yellow'],
    ];
    const el = document.getElementById('firmwareCards');
    if (el) el.innerHTML = cards.map(([l,v,c]) => `<div class="card"><div class="card-label">${l}</div><div class="card-value ${c}">${v}</div></div>`).join('');
    const bars = document.getElementById('firmwareVersionBars');
    if (bars) {
      if (!hasData) bars.innerHTML = '<p style="color:#8b949e">Not yet analyzed — firmware catalog not wired to dataset.</p>';
      else {
        const max = Math.max(1, ...Object.values(f.by_version||{}));
        bars.innerHTML = Object.entries(f.by_version||{}).map(([k,v]) => `<div class="hbar"><div class="hbar-label">${k}</div><div class="hbar-track"><div class="hbar-fill fill-green" style="width:${(v/max*100).toFixed(1)}%"></div></div><div class="hbar-count">${v}</div></div>`).join('');
      }
    }
    const checks = D.firmware_checks || [];
    const el2 = document.getElementById('firmwareChecks');
    if (el2) {
      if (!checks.length) el2.innerHTML = '<p style="color:#8b949e">No per-game firmware checks — no lib_version requirements or no catalog. Run dashboard --games with system_modules.</p>';
      else {
        el2.innerHTML = checks.map(g => {
          const rows = g.checks.map(c => `<tr><td style="font-family:monospace">${c.library}</td><td>${c.required}</td><td><span class="pill" style="background:${c.status==='compatible'?'#23863622':'#da363322'};color:${c.status==='compatible'?'#3fb950':'#f85149'};border:1px solid ${c.status==='compatible'?'#23863644':'#da363344'}">${c.status}</span></td></tr>`).join('');
          return `<details style="margin-bottom:6px"><summary style="cursor:pointer;padding:8px 12px;background:#0d1117;border:1px solid #30363d;border-radius:6px;font-size:0.85rem;color:#c9d1d9"><strong style="color:#58a6ff">${g.game}</strong> &mdash; ${g.checks.length} libs</summary><div style="padding:8px 12px;border:1px solid #30363d;border-top:0;border-radius:0 0 6px 6px"><table style="width:100%;font-size:0.82rem;border-collapse:collapse"><thead><tr style="color:#8b949e"><th style="text-align:left">Library</th><th>Required</th><th>Status</th></tr></thead><tbody>${rows}</tbody></table></div></details>`;
        }).join('');
      }
    }
  })();

  const input = $('#globalSearch');
  const results = $('#searchResults');

  input.addEventListener('input', () => {
    const q = input.value.trim().toLowerCase();
    if (q.length < 2) { results.classList.remove('show'); return; }
    const matches = searchIndex.filter(e => {
      if (e.type === 'lib-version') {
        return (e.name||'').toLowerCase().includes(q) ||
               (e.version||'').includes(q) ||
               (e.versionRaw||'').includes(q) ||
               (e.game||'').toLowerCase().includes(q) ||
               (e.gameTitle||'').toLowerCase().includes(q);
      }
      return (e.nid||'').toLowerCase().includes(q) ||
             (e.name||'').toLowerCase().includes(q) ||
             (e.library||'').toLowerCase().includes(q) ||
             (e.game||'').toLowerCase().includes(q) ||
             (e.gameTitle||'').toLowerCase().includes(q);
    }).slice(0, 30);

    if (matches.length === 0) { results.classList.remove('show'); return; }

    const grouped = {};
    matches.forEach(m => {
      if (m.type === 'lib-version') {
        const key = 'lv:' + m.name + ':' + m.version + ':' + m.game;
        if (!grouped[key]) grouped[key] = { ...m, games: new Set(), type: 'lib-version' };
        if (m.game) grouped[key].games.add(m.gameTitle || m.game);
        return;
      }
      const key = m.nid + m.library;
      if (!grouped[key]) grouped[key] = { ...m, games: new Set() };
      if (m.game) grouped[key].games.add(m.gameTitle || m.game);
    });

    results.innerHTML = Object.values(grouped).map(m => `
      <div class="sr-item" data-nid="${m.nid}" data-lib="${m.library}" data-game="${m.game}" data-type="${m.type}">
        <div class="sr-type">${m.type}${m.type !== 'lib-version' ? ` &middot; ${m.library||''}` : ''}</div>
        <div class="sr-name">${m.type === 'lib-version' ? m.name + ' ' + m.version : (m.name || m.nid)}</div>
        <div class="sr-detail">${[...m.games].slice(0,3).join(', ')}${m.games.size > 3 ? ` +${m.games.size-3} more` : ''}</div>
      </div>
    `).join('');
    results.classList.add('show');
  });

  results.addEventListener('click', e => {
    const item = e.target.closest('.sr-item');
    if (!item) return;
    const type = item.dataset.type;
    if (type === 'lib-version') {
      const gameId = item.dataset.game;
      if (gameId) showGameDetail(gameId);
    } else {
      const lib = item.dataset.lib;
      const d = (D.library_details || []).find(x => x.name === lib);
      if (d) {
        const gamesHtml = d.games.map(g => `<tr><td>${trunc(g.title_name||g.game,30)}</td><td>${fmt(g.import_count)}</td></tr>`).join('');
        openDetail(d.name, `<div class="detail-section"><div class="detail-kv"><div class="k">Games</div><div class="v">${d.game_count}</div></div></div><div class="detail-section"><h3>Games</h3><div class="table-wrap"><table class="detail-table"><thead><tr><th>Game</th><th>Imports</th></tr></thead><tbody>${gamesHtml}</tbody></table></div></div>`);
      }
    }
    results.classList.remove('show');
    input.value = '';
  });

  input.addEventListener('blur', () => { setTimeout(() => results.classList.remove('show'), 200); });
})();
"##;
