
window.PAGES = [
  { id:"intro", title:"Introduction", html:`
    <p>Bienvenue dans le bundle <b>Gold</b> de vitte‑lang : runtime, bytecode v8, VM avec GC, closures, OO, et un CLI pro.</p>
    <h2 id="getting-started">Commencer</h2>
    <pre><code>cargo build --release</code></pre>
  `},
  { id:"lang", title:"Langage", html:`
    <h2 id="syntax">Syntaxe</h2>
    <pre><code>class User { name: "Ada" greet(self) -> "Hello " + self.name }</code></pre>
    <pre><code>u = new User {} ; print(u.greet())</code></pre>
  `},
  { id:"vm", title:"VM & Bytecode", html:`
    <h2 id="ops">OpCodes majeurs</h2>
    <ul><li>Stack+GC</li><li>CallMethod/CallStatic/CallValue</li><li>Index/SetIndex, Slice, Range</li></ul>
  `},
  { id:"cli", title:"CLI", html:`
    <h2 id="commands">Commandes</h2>
    <pre><code>vitte run|bc|vm|fmt|repl|tests|bench|doc|llc|pm|completions|man|compile</code></pre>
  `},
  { id:"compiler", title:"Compiler crate", html:`
    <h2 id="api">API</h2>
    <pre><code>use vitte_compiler::{Compiler, CompilerConfig, OutputKind};</code></pre>
  `},
  { id:"faq", title:"FAQ", html:`<h2 id="faq1">Pourquoi?</h2><p>Parce qu'on aime les langages qui cognent.</p>`}
];
