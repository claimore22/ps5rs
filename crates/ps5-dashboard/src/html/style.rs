#[allow(dead_code)]
pub const CSS: &str = r#"*{margin:0;padding:0;box-sizing:border-box;}
body{background:#0d1117;color:#c9d1d9;font-family:'Segoe UI',system-ui,-apple-system,sans-serif;line-height:1.5;}
a{color:#58a6ff;text-decoration:none;}
a:hover{text-decoration:underline;}
.header{background:#161b22;border-bottom:1px solid #30363d;padding:12px 24px;display:flex;align-items:center;gap:16px;position:sticky;top:0;z-index:100;}
.header h1{font-size:1.2rem;color:#58a6ff;white-space:nowrap;}
.header .subtitle{color:#8b949e;font-size:0.78rem;}
.search-box{background:#0d1117;border:1px solid #30363d;color:#c9d1d9;padding:6px 12px;border-radius:6px;width:280px;font-size:0.85rem;margin-left:auto;}
.search-box:focus{outline:none;border-color:#58a6ff;}
.tabs{background:#161b22;border-bottom:1px solid #30363d;display:flex;gap:0;padding:0 24px;overflow-x:auto;position:sticky;top:49px;z-index:99;}
.tab{padding:10px 16px;cursor:pointer;color:#8b949e;font-size:0.85rem;border-bottom:2px solid transparent;white-space:nowrap;transition:all 0.15s;}
.tab:hover{color:#c9d1d9;}
.tab.active{color:#58a6ff;border-bottom-color:#58a6ff;}
.container{max-width:1400px;margin:0 auto;padding:24px;}
.tab-content{display:none;}
.tab-content.active{display:block;}
.cards{display:grid;grid-template-columns:repeat(auto-fit,minmax(180px,1fr));gap:12px;margin-bottom:24px;}
.card{background:#161b22;border:1px solid #30363d;border-radius:8px;padding:16px;}
.card-label{color:#8b949e;font-size:0.75rem;text-transform:uppercase;letter-spacing:0.05em;}
.card-value{font-size:1.5rem;font-weight:600;color:#e6edf3;margin-top:4px;}
.card-value.green{color:#3fb950;}
.card-value.blue{color:#58a6ff;}
.card-value.yellow{color:#d29922;}
.section{background:#161b22;border:1px solid #30363d;border-radius:8px;padding:20px;margin-bottom:20px;}
.section h2{font-size:1.05rem;color:#e6edf3;margin-bottom:12px;border-bottom:1px solid #21262d;padding-bottom:8px;}
.table-wrap{max-height:500px;overflow-y:auto;}
table{width:100%;border-collapse:collapse;font-size:0.82rem;}
th{background:#0d1117;color:#8b949e;text-align:left;padding:8px 10px;cursor:pointer;user-select:none;position:sticky;top:0;}
th:hover{color:#58a6ff;}
th .arrow{font-size:0.65rem;margin-left:4px;color:#484f58;}
th.sorted .arrow{color:#58a6ff;}
td{padding:6px 10px;border-top:1px solid #21262d;}
tr:hover{background:#1c2128;}
tr.clickable{cursor:pointer;}
tr.clickable:hover{background:#1c2128;outline:1px solid #30363d;}
.pct{font-variant-numeric:tabular-nums;}
.pct-high{color:#3fb950;}.pct-med{color:#d29922;}.pct-low{color:#f85149;}
.pill{display:inline-block;padding:1px 8px;border-radius:10px;font-size:0.72rem;font-weight:500;}
.pill-self{background:#1f6feb22;color:#58a6ff;border:1px solid #1f6feb44;}
.pill-elf{background:#23863622;color:#3fb950;border:1px solid #23863644;}
.pill-eng{background:#d2992222;color:#d29922;border:1px solid #d2992244;}
.filter-bar{display:flex;gap:8px;margin-bottom:12px;flex-wrap:wrap;align-items:center;}
.filter-input,.filter-select{background:#0d1117;border:1px solid #30363d;color:#c9d1d9;padding:6px 12px;border-radius:6px;font-size:0.82rem;}
.filter-input:focus,.filter-select:focus{outline:none;border-color:#58a6ff;}
.filter-select{cursor:pointer;}
.hbar{display:flex;align-items:center;margin-bottom:4px;}
.hbar-label{width:260px;font-size:0.78rem;color:#c9d1d9;text-overflow:ellipsis;overflow:hidden;white-space:nowrap;display:flex;align-items:center;}
.hbar-track{flex:1;height:14px;background:#21262d;border-radius:3px;overflow:hidden;}
.hbar-fill{height:100%;border-radius:3px;min-width:1px;transition:width 0.3s;}
.hbar-count{width:60px;text-align:right;font-size:0.75rem;color:#8b949e;margin-left:6px;}
.fill-blue{background:linear-gradient(90deg,#1f6feb,#58a6ff);}
.fill-green{background:linear-gradient(90deg,#238636,#3fb950);}
.fill-red{background:linear-gradient(90deg,#da3633,#f85149);}
.fill-yellow{background:linear-gradient(90deg,#9e6a03,#d29922);}
.fill-purple{background:linear-gradient(90deg,#8957e5,#bc8cff);}
.heatmap-wrap{overflow-x:auto;}
.heatmap{border-collapse:collapse;font-size:0.7rem;}
.heatmap th{position:static;padding:4px 6px;white-space:nowrap;writing-mode:horizontal-tb;cursor:default;}
.heatmap th:hover{color:#8b949e;}
.heatmap td{width:28px;height:28px;padding:0;text-align:center;font-size:0;cursor:default;}
.heatmap td:hover{outline:2px solid #58a6ff;position:relative;z-index:1;}
.heatmap .lib-name{text-align:right;padding-right:8px;white-space:nowrap;color:#c9d1d9;}
.presence-cell{width:28px;height:28px;text-align:center;font-size:0.85rem;line-height:28px;cursor:default;}
.presence-cell:hover{outline:2px solid #58a6ff;position:relative;z-index:1;}
.seg-bar{display:flex;height:18px;border-radius:3px;overflow:hidden;min-width:100px;}
.seg-rx{background:#238636;}.seg-r{background:#1f6feb;}.seg-rw{background:#d29922;}.seg-other{background:#484f58;}
.legend{display:flex;gap:16px;margin-top:8px;font-size:0.75rem;color:#8b949e;flex-wrap:wrap;}
.legend span::before{content:'';display:inline-block;width:10px;height:10px;border-radius:2px;margin-right:4px;vertical-align:middle;}
.legend .l-rx::before{background:#238636;}.legend .l-r::before{background:#1f6feb;}
.legend .l-rw::before{background:#d29922;}.legend .l-other::before{background:#484f58;}
.detail-overlay{position:fixed;top:0;right:0;width:600px;max-width:90vw;height:100vh;background:#161b22;border-left:1px solid #30363d;z-index:200;transform:translateX(100%);transition:transform 0.25s ease;overflow-y:auto;box-shadow:-4px 0 24px rgba(0,0,0,0.5);}
.detail-overlay.open{transform:translateX(0);}
.detail-header{position:sticky;top:0;background:#161b22;border-bottom:1px solid #30363d;padding:16px 20px;display:flex;justify-content:space-between;align-items:center;z-index:1;}
.detail-header h2{font-size:1rem;color:#e6edf3;}
.detail-close{background:none;border:1px solid #30363d;color:#8b949e;cursor:pointer;border-radius:4px;padding:4px 10px;font-size:0.85rem;}
.detail-close:hover{color:#c9d1d9;border-color:#58a6ff;}
.detail-body{padding:20px;}
.detail-section{margin-bottom:20px;}
.detail-section h3{font-size:0.9rem;color:#58a6ff;margin-bottom:8px;}
.detail-kv{display:grid;grid-template-columns:140px 1fr;gap:4px 12px;font-size:0.82rem;}
.detail-kv .k{color:#8b949e;}.detail-kv .v{color:#c9d1d9;word-break:break-all;}
.detail-table{font-size:0.78rem;width:100%;}
.detail-table td{padding:4px 8px;border-top:1px solid #21262d;}
.lib-pri td:nth-child(3),.lib-pri td:nth-child(4){font-variant-numeric:tabular-nums;}
.stat-grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(300px,1fr));gap:16px;}
.stat-card{background:#0d1117;border:1px solid #21262d;border-radius:6px;padding:14px;}
.stat-card h3{font-size:0.85rem;color:#58a6ff;margin-bottom:8px;}
.stat-row{display:flex;justify-content:space-between;font-size:0.8rem;padding:3px 0;border-bottom:1px solid #21262d22;}
.stat-row .sv{color:#e6edf3;font-variant-numeric:tabular-nums;}
.graph-wrap{overflow:auto;background:#0d1117;border:1px solid #21262d;border-radius:6px;padding:16px;}
.search-results{position:absolute;top:100%;left:0;right:0;background:#161b22;border:1px solid #30363d;border-radius:0 0 6px 6px;max-height:400px;overflow-y:auto;z-index:201;display:none;}
.search-results.show{display:block;}
.sr-item{padding:8px 12px;cursor:pointer;font-size:0.82rem;border-bottom:1px solid #21262d;}
.sr-item:hover{background:#1c2128;}
.sr-item .sr-type{color:#8b949e;font-size:0.72rem;}
.sr-item .sr-name{color:#e6edf3;}
.sr-item .sr-detail{color:#8b949e;font-size:0.78rem;}
.search-wrap{position:relative;}
"#;

