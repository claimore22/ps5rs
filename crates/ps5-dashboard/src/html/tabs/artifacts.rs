//! `artifacts` tab markup.

pub fn render() -> &'static str {
    r##"<div id="artifactEmpty" style="color:#8b949e;text-align:center;padding:60px 0;font-size:0.9rem">No artifact data. Run <code style="background:#0d1117;padding:2px 8px;border-radius:4px">ps5rs dashboard --games <games_dir></code> to inventory bare content (prx/pssl/sb/gnf/at9/bank/json etc.).</div>
<div id="artifactContent" style="display:none">
<div class="cards" id="artifactCards"></div>
<div class="section"><h2>By Category</h2><div id="artifactCategoryBars"></div></div>
<div class="section"><h2>Top Extensions</h2><div id="artifactExtBars"></div></div>
<div class="section"><h2>Per-Game Artifacts</h2><div id="artifactPerGame"></div></div>
<div class="section"><h2>Cross-Artifact Linkage</h2><p style="color:#8b949e;font-size:0.82rem;margin-bottom:12px">Per-game executable / shader / texture / audio counts derived from inventory; relative paths preserved. Bare content is first-class — .pak not required.</p><div id="artifactCrossBars"></div></div>
</div>"##
}
