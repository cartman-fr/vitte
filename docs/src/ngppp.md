# NG+++

- **Exports stricts** : `from "mod" import X` lève une erreur si `X` n’est pas exporté par `mod` (ou est privé `_X`). 
- **Concat `+` typée (LLVM)** : détection **profonde** des expressions string (`"..."`, `str(x)`, additions déjà str…), conversion de l’autre opérande si besoin → `str_concat`.
- **Hover** : affiche `Type: ...` lorsque déductible (top-level).
- **Fmt** : regroupe et trie les imports (`import`/`from`) en tête de fichier, pour des diffs propres.
