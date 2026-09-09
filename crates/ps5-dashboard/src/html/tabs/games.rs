//! `games` tab markup.

pub fn render() -> &'static str {
    r##"<div class="filter-bar" id="gameFilters">
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
<th data-col="3">Libs <span class="arrow">&#9650;</span></th>
<th data-col="4">SCE Libs <span class="arrow">&#9650;</span></th>
<th data-col="5">Unknown NIDs <span class="arrow">&#9650;</span></th>
<th data-col="6">Size MB <span class="arrow">&#9650;</span></th>
</tr></thead>
<tbody id="gamesBody"></tbody>
</table></div>"##
}
