//! `overview` tab markup.

pub fn render() -> &'static str {
    r##"<div class="cards" id="overviewCards"></div>
<div class="section"><h2>NID Resolution</h2><div id="nidResBar"></div></div>
<div class="section"><h2>Platform Distribution</h2><div id="platformBar"></div></div>
<div class="section"><h2>Engine Distribution</h2><div id="engineBar"></div></div>"##
}
