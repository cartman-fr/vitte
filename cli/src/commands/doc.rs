
use color_eyre::eyre::Result;
use std::path::Path;
use crate::util;

fn esc(s:&str)->String{ s.replace('&',"&amp;").replace('<',"&lt;").replace('>',"&gt;").replace('"',"&quot;").replace("'","&#39;") }

pub fn generate(input: &Path, output: &Path) -> Result<()> {
    let s = util::read(input)?;
    let mut html = String::new();
    html.push_str("<!doctype html><meta charset='utf-8'><title>vitte-doc</title>");
    html.push_str("<style>body{font:16px system-ui;max-width:960px;margin:2rem auto;line-height:1.6}pre{background:#0b1020;color:#e6e6e6;padding:.8rem;border-radius:8px;overflow:auto} code.kw{color:#9cdcfe} code.num{color:#c586c0} h1,h2,h3{scroll-margin-top:80px}</style>");
    let mut toc = String::new();
    let mut out = String::new();
    for line in s.lines() {
        let t=line.trim_start();
        if t.starts_with("# ") { let id=esc(&t[2..]).to_lowercase().replace(' ','-'); toc.push_str(&format!("<li><a href='#{id}'>{}</a></li>", esc(&t[2..]))); out.push_str(&format!("<h2 id='{id}'>{}</h2>", esc(&t[2..]))); }
        else if t.starts_with("## ") { let id=esc(&t[3..]).to_lowercase().replace(' ','-'); toc.push_str(&format!("<li style='margin-left:1rem'><a href='#{id}'>{}</a></li>", esc(&t[3..]))); out.push_str(&format!("<h3 id='{id}'>{}</h3>", esc(&t[3..]))); }
        else if t.starts_with("# EXPECT:") { out.push_str(&format!("<div><code class='kw'>{}</code></div>", esc(t))); }
        else if t.starts_with("#") { /* ignore */ }
        else { out.push_str("<pre><code class='vitte'>"); out.push_str(&esc(line)); out.push_str("</code></pre>"); }
    }
    html.push_str("<h1>Documentation</h1><nav><ol>");
    html.push_str(&toc);
    html.push_str("</ol></nav>");
    html.push_str(&out);
    util::write(output, &html)?;
    eprintln!("Doc écrite: {}", output.display());
    Ok(())
}
