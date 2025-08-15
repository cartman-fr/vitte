# RFC TEMPLATE — Vitte Language

> Gabarit officiel pour toute **Request For Comments** du langage **Vitte**.  
> À copier tel quel dans `rfcs/NNNN-titre-kebab-case.md` et à remplir **intégralement** avant ouverture de la PR.

---

## Métadonnées

- **Numéro** : `NNNN` *(assigné lors de l’ouverture du PR, séquentiel)*  
- **Titre** : *Nom clair, concis, précis*  
- **Auteur(s)** : `@pseudo1`, `@pseudo2`  
- **Date** : `YYYY-MM-DD`  
- **Statut** : `DRAFT` | `DISCUSSION` | `ACCEPTED` | `REJECTED` | `IN PROGRESS` | `EXPERIMENTAL` | `IMPLEMENTED` | `STABILIZED` | `DEPRECATED` | `OBSOLETE`  
- **Version cible** : `vX.Y` ou *Edition YYYY*  
- **Domaines** : `Langage` | `Toolchain` | `Stdlib` | `Interop` | `Process` | ...  
- **WG concerné(s)** : `@lang-wg`, `@async-wg`, etc.  
- **Lien discussion initiale** : [Issue/Forum](url)

---

## 1. Résumé

En **1-3 phrases**, décrire la proposition de manière concise pour un lecteur qui ne connaît pas encore le sujet.

---

## 2. Motivation

- Quel problème résout cette proposition ?  
- Quel est l’impact sur l’expérience développeur (DX), les performances, la maintenabilité ?  
- Quels cas d’usage réels ou futurs justifient ce changement ?  
- Pourquoi est-ce le bon moment pour l’introduire ?

> *Une RFC sans motivation solide est une idée, pas un plan.*

---

## 3. Conception (Design)

- **Description détaillée** du comportement proposé.  
- **Nouvelle syntaxe / sémantique** (si langage) avec grammaire EBNF si pertinent.  
- **Nouvelles APIs** (si stdlib/toolchain) avec signatures complètes.  
- Diagrammes, exemples compilables, pseudo-code.  
- Flux de données, schémas d’état si nécessaire.

**Exemple (syntaxe)** :
```vitte
fn example() {
    let mut x = 5;
    x += 1;
}
```

**EBNF (exemple)** :
```
assignment ::= identifier '=' expression ';'
```

---

## 4. Compatibilité & migration

- Impact sur le code existant : rupture, addition pure, modif interne.  
- Si rupture : plan de migration, `lints` et outil `vitfix` proposés.  
- Compatibilité ascendante et descendante.  
- Comportement en présence de code `unsafe` ou FFI.

---

## 5. Alternatives envisagées

- Autres designs possibles (même rejetés).  
- Pourquoi ceux-ci ne sont pas retenus : ergonomie, perfs, cohérence.  
- Inspirations et différences avec d’autres langages.

---

## 6. Exemples d’utilisation

Montrer des **cas concrets** :

```vitte
// Exemple trivial
async fn fetch_data(url: str) -> Result<Data, Error> {
    let res = await http::get(url);
    parse(res.body)
}
```

```vitte
// Exemple plus complexe
let items = list.filter(|x| x.valid()).map(process).collect();
```

---

## 7. Implémentation

- Étapes techniques pour intégrer la RFC (compiler, VM, stdlib…).  
- Modules à modifier/créer.  
- Tests unitaires et d’intégration requis.  
- Estimation du temps d’implémentation.  
- Flags d’activation (`--unstable-feature`) si expérimental.

---

## 8. Considérations de performance

- Impact estimé sur le temps de compilation, la taille binaire, l’exécution.  
- Benchmarks initiaux si dispo (avec config précise : CPU, OS, flags).  
- Optimisations possibles.

---

## 9. Considérations de sécurité

- Risques introduits (UB, FFI, concurrency, sandbox, crypto).  
- Mesures de mitigation.  
- Audit nécessaire ?  
- Impact sur la mémoire et les invariants du langage.

---

## 10. Documentation & enseignement

- Changements nécessaires dans la documentation officielle.  
- Guides de migration.  
- Exemples et tutoriels à mettre à jour ou créer.  
- Formations internes / externes impactées.

---

## 11. Plan de déploiement

- Ordre des étapes (implémentation, tests, doc, release).  
- Communication (blog post, changelog, annonces).  
- Suivi post-déploiement (bugfix, feedback).

---

## 12. Références

- Liens vers issues, PRs, discussions de forum.  
- Liens vers spécifications ou articles externes.  
- Liens vers d’autres langages et implémentations similaires.

---

## 13. Historique des révisions

| Date       | Auteur(s)   | Changements |
|------------|-------------|-------------|
| YYYY-MM-DD | @pseudo     | Création    |
| YYYY-MM-DD | @pseudo     | Révisions suite aux retours |
| YYYY-MM-DD | @pseudo     | Acceptation / Rejet |
