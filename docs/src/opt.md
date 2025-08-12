# Optimiseur

Passes: fold_consts, algebraic, const_prop, cse, cleanup; plus `introduce_temp_once`.

Ajouts: **copy-prop** (propagation de copies) et **CSE global** (naïf, par fonction).

VM: RC + **collect_cycles()** (marquage depuis globals/stack/locals) pour casser les cycles simples.
