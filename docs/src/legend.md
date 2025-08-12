# LEGEND

- **Imports sémantiques (AST)** : résolution au niveau AST, réécriture des appels `Alias.foo(x)` en appels directs vers `Alias_foo(x)` sans réécrire le texte brut, fusion des `from ... import` dans le module courant.
- **Capture assign** : sucre `x <- expr` transformé en `__set__(x, expr)` et compilé en `SetCapture` si `x` est une capture (ou `SetLocal`/`StoreG` selon le cas).
- **LSP inline PNG** : commande `vitte.dot.inline` qui génère et embarque le PNG en data-URI si `dot` est disponible.
