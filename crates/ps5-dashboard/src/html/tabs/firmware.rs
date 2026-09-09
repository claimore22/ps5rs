//! `firmware` tab markup.

pub fn render() -> &'static str {
    r##"<div class="section"><h2>Firmware Summary</h2><div class="cards" id="firmwareCards"></div></div>
<div class="section"><h2>Modules by Version</h2><div id="firmwareVersionBars"></div></div>
<div class="section"><h2>Per-Game Compatibility</h2><div id="firmwareChecks"></div></div>"##
}
