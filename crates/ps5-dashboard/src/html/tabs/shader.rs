//! `shader` tab markup.

pub fn render() -> &'static str {
    r##"<div class="section"><h2>Shader Summary</h2><div class="cards" id="shaderCards"></div></div>
<div class="section"><h2>Shaders by Stage</h2><div id="shaderStageBars"></div></div>
<div class="section"><h2>Resources</h2><div id="shaderResources"></div></div>"##
}
