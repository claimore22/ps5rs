//! `archived` tab markup.

pub fn render() -> &'static str {
    r##"<div class="section"><h2>Archived Titles</h2><p style="color:#8b949e;font-size:0.82rem;margin-bottom:12px">Ingested games whose ROM is not present — last-known state from the durable per-game records</p><div id="archivedGames"></div></div>"##
}
