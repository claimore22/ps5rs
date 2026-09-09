//! `engines` tab markup.

pub fn render() -> &'static str {
    r##"<div class="cards" id="engineOverviewCards"></div>
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
</table></div></div>"##
}
