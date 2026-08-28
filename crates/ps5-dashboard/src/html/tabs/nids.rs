//! `nids` tab markup.

pub fn render() -> &'static str {
    r##"<div class="section"><h2>NID Resolution</h2><div id="nidResDetail"></div></div>
<div class="section"><h2>Top 20 NIDs by Frequency</h2><div id="nidBars"></div></div>
<div class="section"><h2>Library NID Breakdown</h2><p style="color:#8b949e;font-size:0.82rem;margin-bottom:12px">Top 10 per library</p><div id="libNidBreakdown"></div></div>"##
}
