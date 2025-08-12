## CLI `vittc`

`--emit fmt|inl|opt|types|ids|graph`.


### Graph DOT
`vittc --emit graph --reduce` pour un graphe plus lisible.


### Focus
`vittc --emit graph --focus stmt[0].value` pour zoomer sur un sous-arbre précis.


### Types annotés
`vittc --emit graph --annot` ajoute `:Type` aux labels DOT.
