pub fn generate(src:&str)->String{
    let mut html=String::from("<!doctype html><meta charset='utf-8'><title>vitte-doc</title><style>body{font:16px system-ui;max-width:900px;margin:2rem auto;line-height:1.6}pre{background:#0b1020;color:#e6e6e6;padding:.8rem;border-radius:8px;overflow:auto}</style><h1>Documentation</h1>");
    for line in src.lines(){
        let t=line.trim_start();
        if t.starts_with("# ") { html.push_str(&format!("<h2>{}</h2>", &t[2..])); }
        else if t.starts_with("## ") { html.push_str(&format!("<h3>{}</h3>", &t[3..])); }
        else if t.starts_with("#") { /* comment */ }
        else { html.push_str("<pre><code class='vitte'>"); html.push_str(line); html.push_str("</code></pre>"); }
    }
    html
}
