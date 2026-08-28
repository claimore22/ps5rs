//! `sdk` tab markup.

pub fn render() -> &'static str {
    r##"<div class="section">
<h2>Prospero SDK — UE — EMC Timeline <span style="font-size:0.72rem;color:#d29922;border:1px solid #d2992244;background:#d2992222;padding:2px 8px;border-radius:10px;margin-left:8px">Reference — not detected from current dataset</span></h2>
<p style="color:#8b949e;font-size:0.82rem;margin-bottom:12px">Reference timeline documenting SDK/UE/EMC associations from Epic/Sony documentation. Not inferred from scanned binaries. UE = Unreal Engine documented pairing, EMC = PS5 EMC (errMG) build association, Prospero = SDK distrib label.</p>
<div class="table-wrap"><table id="sdkTable">
<thead><tr>
<th data-col="0">Prospero SDK <span class="arrow">&#9650;</span></th>
<th data-col="1">SDK documented by UE <span class="arrow">&#9650;</span></th>
<th data-col="2">PS5 EMC <span class="arrow">&#9650;</span></th>
<th data-col="3">Earliest concrete date / evidence <span class="arrow">&#9650;</span></th>
<th data-col="4">Confidence <span class="arrow">&#9650;</span></th>
</tr></thead>
<tbody id="sdkBody"></tbody>
</table></div>
<div class="legend" style="margin-top:12px"><span class="l-rx">High</span><span class="l-r">Medium</span><span class="l-rw">Low</span></div>
</div>
<div class="section">
<h2>Timeline Notes</h2>
<ul style="color:#8b949e;font-size:0.82rem;line-height:1.6;margin-left:18px">
<li><strong>Prospero</strong> is the internal SDK distrib name (e.g. <code>PS5 - SDK-10_00_00_40</code>). UE docs map <code>PS5 - SDK-10_00_00_40-00_00_00_0_1</code> ↔ UE 5.5 etc.</li>
<li><strong>EMC</strong> = <code>errMG</code> PS5 system-software build tag embedded in games; association is via games built against that SDK generation (e.g. 10.00 ↔ EMC 1.14.3 is not direct, but 9.20 ↔ 1.14.3 is).</li>
<li>Generation inference for <code>6.x</code> and <code>12.x</code> is from firmware/SDK gaps, hence <span class="pill" style="background:#d2992222;color:#d29922;border:1px solid #d2992244">Medium/Low</span>.</li>
<li>UE 5.6 and 5.7 both document <code>11.00.00.40</code> — same SDK, different UE minor.</li>
</ul>
</div>"##
}
