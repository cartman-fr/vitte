
# AURORA

- **LSP** intégré : *hover* (type + nom), *go to definition* (regex `def name(`/`name =`), sync FULL.
- **Row polymorphism (light)** : enregistrements ouverts `Record(fields, Some(row))`, unification permissive par clés.
- **Formatter** : précédence/parenthèses correctes + virgules finales optionnelles via `.vittefmt.toml` (`trailing_commas = "always"|"never"`).
- **Macros** : `println(a,b,c)` et `printall(...)` se développent en prints successifs.
