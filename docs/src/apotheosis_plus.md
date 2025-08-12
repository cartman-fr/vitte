
# APOTHEOSIS+

- **REPL** : `:graphf` génère un DOT **réduit** (constantes élaguées) pour un focus lisible.
- **LLVM** : concat str **généralisée** quand **au moins un** opérande est un littéral (conversion de l’autre via `str_from_i64`).
- **LSP** : hover positionnel enrichi — tentative d’inférence par lecture des `Assign` top-level.
