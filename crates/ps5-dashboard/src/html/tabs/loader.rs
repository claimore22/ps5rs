//! `loader` tab markup.

pub fn render() -> &'static str {
    r##"<div id="loaderEmpty" style="color:#8b949e;text-align:center;padding:60px 0;font-size:0.9rem">No loader data found. Run <code style="background:#0d1117;padding:2px 8px;border-radius:4px">ps5rs batch-load <games_dir></code> to generate loader coverage statistics.</div>
<div id="loaderContent" style="display:none">
<div class="cards" id="loaderCards"></div>
<div class="section"><h2>Per-Game Import Resolution</h2><div id="loaderGameBars"></div></div>
<div class="section"><h2>Most Unavailable Modules</h2><div id="loaderUnavailableBars"></div></div>
<div class="section"><h2>Games with Highest Stub Rate</h2><div id="loaderWorstBars"></div></div>
</div>"##
}
