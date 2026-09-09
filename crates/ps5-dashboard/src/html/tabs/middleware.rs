//! `middleware` tab markup.

pub fn render() -> &'static str {
    r##"<div id="middlewareEmpty" style="color:#8b949e;text-align:center;padding:60px 0;font-size:0.9rem">No middleware data found. Run <code style="background:#0d1117;padding:2px 8px;border-radius:4px">ps5rs dashboard --games <games_dir></code> to scan third-party and Sony PRX modules.</div>
<div id="middlewareContent" style="display:none">
<div class="cards" id="middlewareCards"></div>
<div class="section"><h2>Top Middleware Products</h2><div id="middlewareProductBars"></div></div>
<div class="section"><h2>Per-Game Middleware</h2>
<div class="filter-bar"><select class="filter-select" id="middlewareGameSelect"></select></div>
<div class="table-wrap"><table id="middlewareGameTable">
<thead><tr>
<th>Module</th>
<th>Vendor</th>
<th>Product</th>
<th>Description</th>
<th>Imports</th>
</tr></thead>
<tbody id="middlewareGameBody"></tbody>
</table></div>
</div>
</div>"##
}
