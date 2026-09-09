//! `libraries` tab markup.

pub fn render() -> &'static str {
    r##"<div class="table-wrap"><table id="libTable">
<thead><tr>
<th data-col="0">Library <span class="arrow">&#9650;</span></th>
<th data-col="1">Games <span class="arrow">&#9650;</span></th>
<th data-col="2">Imports <span class="arrow">&#9650;</span></th>
<th data-col="3">Unique NIDs <span class="arrow">&#9650;</span></th>
</tr></thead>
<tbody id="libBody"></tbody>
</table></div>
<div class="section" style="margin-top:20px"><h2>Library Heatmap (log&sup2;)</h2><div class="heatmap-wrap" id="heatmapWrap"></div></div>
<div class="section" style="margin-top:20px"><h2>Third Party Libraries</h2><div id="thirdPartyBars"></div></div>
<div class="section" style="margin-top:20px"><h2>SCE System Libraries</h2>
<div id="sceCategoryBars"></div>
<h3 style="color:#8b949e;font-size:0.82rem;margin:16px 0 8px">SCE Dependency Matrix</h3>
<div class="heatmap-wrap" id="sceHeatmapWrap"></div>
<h3 style="color:#8b949e;font-size:0.82rem;margin:16px 0 8px">SDK Versions</h3>
<div id="sceVersionDist"></div>
</div>"##
}
