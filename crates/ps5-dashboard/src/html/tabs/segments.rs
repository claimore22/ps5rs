//! `segments` tab markup.

pub fn render() -> &'static str {
    r##"<div class="section"><h2>Segment Sizes by Game</h2><div id="segBars"></div>
<div class="legend"><span class="l-rx">RX (code)</span><span class="l-r">R (rodata)</span><span class="l-rw">RW (data)</span><span class="l-other">Other</span></div></div>"##
}
