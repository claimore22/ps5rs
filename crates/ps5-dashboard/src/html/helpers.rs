#[allow(dead_code)]
pub fn game_label_js() -> &'static str {
    r#"const gameLabel = x => {
  if (!x) return '';
  if (typeof x === 'string') {
    const g = (D.games||[]).find(y=>y.name===x);
    return g ? (g.title_name || g.name) : x;
  }
  return x.title_name || x.name || x.game || '';
};
const gameDisplay = (x,n=32) => trunc(gameLabel(x), n);"#
}
