# Imports

Trois formes supportées :

- `import "fichier.vitte"` : inline direct du module (unique, avec garde anti-boucle).
- `import "fichier.vitte" as Alias` : crée des *bindings* préfixés `Alias_foo` pour chaque symbole top-level du module, et réécrit `Alias.foo(...)` en `Alias_foo(...)` dans le fichier importeur.
- `from "fichier.vitte" import a, b` : n’importe que les symboles listés.

> Remarque: la résolution est *préprocesseur* (réécriture du source) — simple, rapide, efficace, en attendant une table de symboles multi-fichiers.
