
const PAGES = window.PAGES || [];
const content = document.getElementById('content');
const nav = document.getElementById('nav');
const toc = document.getElementById('toc');
const q = document.getElementById('q');
function route(){
  const hash = decodeURIComponent(location.hash.replace(/^#/, '')) || 'intro';
  const p = PAGES.find(x=>x.id===hash) || PAGES[0];
  renderPage(p);
  renderNav(hash);
}
function renderPage(p){
  if(!p){ content.innerHTML = "<div class=card>Page not found.</div>"; return; }
  document.title = "vitte‑lang • "+p.title;
  content.innerHTML = `<article class='card'><h1>${p.title}</h1>${p.html}</article>`;
  renderToc();
}
function renderNav(active){
  nav.innerHTML = PAGES.map(x=>`<a href="#${x.id}" class="${x.id===active?'active':''}">${x.title}</a>`).join('');
}
function renderToc(){
  const hs = content.querySelectorAll('h2,h3');
  toc.innerHTML = Array.from(hs).map(h=>`<div>${h.tagName==='H2'?'•':'⤷'} <a href="#${h.id||h.textContent}">${h.textContent}</a></div>`).join('');
}
window.addEventListener('hashchange', route);
window.addEventListener('load', route);
window.liveSearch = function(){
  const term = (q.value||'').toLowerCase();
  const res = PAGES.flatMap(p => {
    const text = p.html.replace(/<[^>]+>/g,' ').toLowerCase();
    if(text.includes(term)) return [{id:p.id,title:p.title}];
    return [];
  });
  const box = document.getElementById('results');
  if(!term){ box.style.display="none"; return; }
  box.style.display="block";
  box.innerHTML = res.map(r=>`<div style="padding:8px"><a href="#${r.id}">${r.title}</a></div>`).join('') || "<div style='padding:8px'>Aucun résultat</div>";
}
