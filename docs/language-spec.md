# Vitte Language — Specification (édition 2025, **ultra-complète**)

> **Compat toolchain** : ≥ 0.6.0 • **Statut** : _MVP_ + sections **preview/experimental** balisées  
> **But** : spécification **implémentable** (parser → typer → emprunts → backends VM/LLVM), truffée **d’exemples** concrets.

---

## 0) Design goals (boussole)

- **Sûreté par défaut** : pas de `null` implicite, erreurs prévues via `Result`.
- **Coût explicite** : alloc, I/O, locks, `spawn` visibles dans les signatures.
- **Lisible & concise** : `do`, `match`, `=>`, imports clairs.
- **Concurrence pragmatique** : threads + `async/await` (preview).
- **Interop** : `extern(c)` net, passage de buffers clair.
- **Évolution maîtrisée** : RFCs, éditions, rétro-compat pour le *stable*.

---

## 1) Lexical & tokens

- Encodage : **UTF-8**.  
- Espaces : espaces / tabs / LF/CRLF séparent les tokens.  
- Commentaires : `// …` (ligne), `/* … */` (multi-lignes, non imbriqués).  
- Identifiants : `[A-Za-z_][A-Za-z0-9_]*` (lettres Unicode *preview*).  
- Mots-clés (MVP) :  
  `do let const mut if else for while match return break continue use module pub type struct enum trait impl extern async await spawn`.

### 1.1 Littéraux

- **Entiers** : `42`, `1_000`, `0xFF`, `0o755`, `0b1010`  
- **Flottants** : `3.14`, `2e10`, `1_000.0`  
- **Bool** : `true` / `false`  
- **Char** : `'a'`, `'\n'`, `'\x7F'` (rune)  
- **String** : `"hello"`, échappements `\n \t \" \\ \xNN \u{...}`  
  *Preview* : brutes `r"..."`, `r#"..."#`.

### 1.2 Opérateurs & précédences (haut → bas)

| Catégorie        | Opérateurs                         | Assoc. |
|------------------|------------------------------------|--------|
| Postfixe         | `f(x)` `x.y` `a[b]`                | gauche |
| Una ires         | `!x` `-x` `*p` `&x` `&mut x`       | droite |
| Multiplicatifs   | `* / %`                            | gauche |
| Additifs         | `+ -`                              | gauche |
| Comparaisons     | `< <= > >=`                        | gauche |
| Égalité          | `== !=`                            | gauche |
| Conjonctions     | `&&`                               | gauche |
| Disjonctions     | `||`                               | gauche |
| Assignations     | `= += -= *= /=`                    | droite |

> Éval **gauche→droite**, court-circuit sur `&&` et `||`. Pas de promotions numériques implicites.

---

## 2) Types & valeurs

### 2.1 Primitifs

- signés : `i8 i16 i32 i64 isize`  
- non signés : `u8 u16 u32 u64 usize`  
- flottants : `f32 f64`  
- bool : `bool` • char : `char` • unité : `Unit` (équiv. `()`)

### 2.2 Composés

- **Tuple** : `(T1, T2, ...)`  
- **Slice** : `[]T` (vue non propriétaire)  
- **Pointeurs bruts** : `*T` non nul, `?*T` nullable (**FFI**)  
- **Chaînes** : `str` (slice) / `String` (propriétaire)

### 2.3 Utilisateur

```vitte
struct Point { x: f64, y: f64 }

enum Result[T,E] { Ok(T), Err(E) }
enum Option[T] { Some(T), None }

type Meters = f64    // alias
```

> **Zéro coût** : pas de champs cachés, layout prévisible (sauf optimisations autorisées).

---

## 3) Bindings, mutabilité, constantes

```vitte
let x = 1              // immuable par défaut
let mut y = 2          // mutable
let n: i64 = 7         // annotation possible
const PI: f64 = 3.14159
```

---

## 4) Fonctions & génériques

```vitte
do add(a: i32, b: i32) -> i32 { a + b }  // retour implicite (dernière expr)
do shout(msg: str) { print(msg); }       // Unit
do id[T](x: T) -> T { x }                 // générique
```

- Passage **par valeur** (move si propriétaire).  
- Pas d’overload par arité/type (MVP) → privilégier génériques.

---

## 5) Contrôle de flux

```vitte
if cond { ... } else { ... }
while cond { ... }
for i in 0..10 { ... }      // itère sur une séquence/itérateur
break; continue;
```

### 5.1 Match exhaustif + guards

```vitte
enum Shape { Circle(f64), Rect(w: f64, h: f64) }

do area(s: Shape) -> f64 {
  match s {
    Circle(r)        => 3.14159 * r * r,
    Rect{w, h} if w>0 && h>0 => w*h,
  }
}
```

> **Exhaustif** obligatoire (sinon erreur).

---

## 6) Modules, imports, visibilité

- Un fichier = un **module** ; `module` (optionnel) fixe le nom logique.  
- Import : `use` (+ alias)  
- Visibilité : `pub` expose.

```vitte
module geo

pub struct Point { x: f64, y: f64 }

use std::string
use math::pi as PI
```

> Convention : 1 dossier = 1 module parent ; *preview* `mod.vitte` pour re-exports.

---

## 7) Structs, Enums, Méthodes

```vitte
struct Counter { value: i64 }

impl Counter {
  do new() -> Counter { Counter{ value: 0 } }
  do inc(self &mut, by: i64) { self.value = self.value + by }
  do get(self &) -> i64 { self.value }
}
```

Récepteurs : `self` (move), `self &` (borrow immuable), `self &mut` (borrow mutable).

---

## 8) Traits & impl (interfaces)

```vitte
trait Display { do fmt(self &) -> String }

impl Display for i32 {
  do fmt(self &) -> String { to_string(self) }
}

impl Display for Counter {
  do fmt(self &) -> String { "Counter(" + to_string(self.value) + ")" }
}
```

- **Règle de cohérence** : trait local + type externe **ou** trait externe + type local, mais pas externe+externe.

### 8.1 Génériques contraints

```vitte
trait Ord { do cmp(self &, other: Self) -> i32 }

do max_of[T: Ord](a: T, b: T) -> T {
  if a.cmp(b) >= 0 { a } else { b }
}
```

> *Where-clauses preview* : `do f[T](x: T) where T: Ord { … }`.

---

## 9) Ownership & Borrowing

- Propriétaires (ex: `String`, `Vec[T]`) se **déplacent** (move) à l’affectation / appel.  
- Emprunts : `&T` **partagé** ; `&mut T` **exclusif**.  
- Règles :
  1. Zéro/plusieurs `&T` **ou** un seul `&mut T`.  
  2. Pas d’usage après **move**.  
  3. La durée d’un emprunt ≤ durée du propriétaire (lifetimes **inférées** ; annotations preview).

```vitte
let s = String::from("abc")
let r = &s
print(*r)
let t = s   // move
// s n’est plus utilisable ici
```

> Pointeurs bruts `*T`/`?*T` : **FFI** ; deref **unsafe** (preview).

---

## 10) Erreurs & propagation

- Types algébriques : `Option[T]`, `Result[T,E]`.  
- Opérateur `?` :
```vitte
do read_file(p: str) -> Result[String, str] {
  let s = fs::read_to_string(p)?   // propage l’erreur
  Ok(s)
}
```
- **Pas d’exceptions**. `panic!` réservé aux invariants internes (bug).

---

## 11) Concurrence

### 11.1 Threads (OS)

```vitte
use thread; use channel

do main() {
  let (tx, rx) = channel::channel
  let h = thread::spawn({
    tx.send("ping")
  })
  let msg = rx.recv().unwrap()
  print(msg)
  h.join()
}
```

### 11.2 Async/await (preview)

```vitte
async do fetch_json(url: str) -> Result[String, str] {
  let r = await http_client::get(url)
  Ok(string::from_bytes(r.body))
}

async do main() {
  let a = fetch_json("https://api.example/a")
  let b = fetch_json("https://api.example/b")
  let (ra, rb) = await (a, b)         // *preview*: join combinator
  print(ra.unwrap() + rb.unwrap())
}
```

> Runtime async **coopératif** fourni par la stdlib (preview).

---

## 12) Collections & itération (stdlib)

```vitte
use collections

do sum(v: Vec[i32]) -> i32 {
  let mut acc = 0
  for x in v { acc += x }
  acc
}
```

> `Vec.push` amorti O(1), `Map` hash O(1) attendu.

---

## 13) Chaînes & slices

```vitte
let s = String::from("hello")
let t = s + " world"             // move + concat
let words = string::split(t, " ")  // []str
```

---

## 14) FFI & ABI

### 14.1 Appeler C

```vitte
extern(c) do puts(s: *u8) -> i32

do main(){ puts("hello\n") }
```

**Règles** :
- ABI **C** : scalaires par valeur ; buffers via `*u8` + longueur (si nécessaire).  
- Ne pas passer `String` propriétaire cross-langage.  
- L’appelant garantit non-null, alignement, validité & durée de vie.

### 14.2 Recevoir un callback C (preview)

```vitte
extern(c) do qsort(base: *u8, n: usize, size: usize, cmp: ?*extern(c)(*u8,*u8)->i32) -> i32
```

> Pointeurs de fonctions externes autorisés ; closures non capturantes *preview*.

---

## 15) Formatage, attributs & tests

- Format : 2 espaces, 100 colonnes, virgules terminales en multi-ligne.  
- Attributs (preview) : `@inline`, `@deprecated("msg")`, `@test`.

```vitte
@test
do parse_number_works() {
  assert(string::to_i32("42").unwrap()==42, "bad parse")
}
```

---

## 16) Backends & toolchain

- Pipeline : Lex/Parse → Résolution → Typage → Emprunts → IR →  
  (a) **VM/bytecode** ou (b) **LLVM** natif.  
- Fichiers : `.vitte` ; outils : `vitc`, `vitte-asm`, `vitte-disasm`, `vitte-link`, `vitte-fmt`.

---

## 17) Grammaire (EBNF *implémentable*)

```ebnf
Program     := { Item } ;
Item        := Fn | Struct | Enum | Trait | Impl | Use | Module | TypeAlias | Const ;

Module      := "module" Ident ";" ;
Use         := "use" Path [ "as" Ident ] ";" ;
TypeAlias   := "type" Ident "=" Type ";" ;
Const       := "const" Ident ":" Type "=" Expr ";" ;

Fn          := [ "pub" ] [ "async" ] "do" Ident "(" Params? ")" [ "->" Type ] Block ;
Params      := Param { "," Param } ;
Param       := Ident ":" Type ;

Struct      := [ "pub" ] "struct" Ident "{" Fields? "}" ;
Fields      := Field { "," Field } ;
Field       := Ident ":" Type ;

Enum        := [ "pub" ] "enum" Ident "{" Variants "}" ;
Variants    := Variant { "," Variant } ;
Variant     := Ident | Ident "(" Types? ")" | Ident "{" Fields? "}" ;

Trait       := "trait" Ident [ ":" TraitBounds ] "{" { TraitItem } "}" ;
TraitItem   := "do" Ident "(" Params? ")" [ "->" Type ] ";" ;
TraitBounds := TypePath { "+" TypePath } ;

Impl        := "impl" [ TypePath "for" ] TypePath "{" { Method } "}" ;
Method      := "do" Ident "(" MethParams? ")" [ "->" Type ] Block ;
MethParams  := ( "self" | "self &" | "self &mut" ) [ "," Params ] ;

Block       := "{" { Stmt } "}" ;
Stmt        := Let | Const | ExprStmt | While | For | If | Match | Return | Break | Continue ;
Let         := "let" [ "mut" ] Ident [ ":" Type ] "=" Expr ";" ;
While       := "while" Expr Block ;
For         := "for" Ident "in" Expr Block ;
If          := "if" Expr Block [ "else" Block ] ;
Match       := "match" Expr "{" Arms "}" ;
Arms        := Arm { "," Arm } ;
Arm         := Pattern [ "if" Expr ] "=>" ExprOrBlock ;

Return      := "return" [ Expr ] ";" ;
Break       := "break" ";" ;
Continue    := "continue" ";" ;

ExprStmt    := Expr ";" ;
ExprOrBlock := Expr | Block ;

Expr        := Assign ;
Assign      := LogicOr { AssignOp LogicOr } ;
AssignOp    := "=" | "+=" | "-=" | "*=" | "/=" ;
LogicOr     := LogicAnd { "||" LogicAnd } ;
LogicAnd    := Equality { "&&" Equality } ;
Equality    := Rel { ("==" | "!=") Rel } ;
Rel         := Add { ("<" | "<=" | ">" | ">=") Add } ;
Add         := Mul { ("+" | "-") Mul } ;
Mul         := Unary { ("*" | "/" | "%") Unary } ;
Unary       := ( "!" | "-" | "&" | "&mut" | "*" ) Unary | Postfix ;
Postfix     := Primary { Call | Index | Field } ;
Call        := "(" [ Args ] ")" ;
Args        := Expr { "," Expr } ;
Index       := "[" Expr "]" ;
Field       := "." Ident ;
Primary     := Literal | Ident | "(" Expr ")" | Block ;

Type        := TypePrimary { GenericSuffix } ;
TypePrimary := PathType | TupleType | SliceType | PtrType | Ident ;
GenericSuffix := "[" Types? "]" ;
Types       := Type { "," Type } ;
TupleType   := "(" Type { "," Type } ")" ;
SliceType   := "[]" Type ;
PtrType     := "*" Type | "?*" Type ;

Path        := Ident { "::" Ident } ;
PathType    := Path ;

Literal     := Int | Float | String | Char | "true" | "false" ;
Ident       := /[A-Za-z_][A-Za-z0-9_]*/ ;
```

---

## 18) UB & garanties

- **Data race** : UB.  
- Déréférencer un `?*T` nul : UB.  
- **FFI** : l’appelant garantit ABI / alignement / durée de vie.  
- **Panics** : non interceptables (MVP), arrêt du thread courant.

---

## 19) Exemples “carnet de recettes” (beaucoup)

### 19.1 CLI + config + validation

```vitte
use cli; use config; use validate

do main() -> i32 {
  let args = cli::parse()   // --host 0.0.0.0 --port 8080
  let sch = validate::object({
    "host": validate::string(pattern: r"^\\d+\\.\\d+\\.\\d+\\.\\d+$"),
    "port": validate::integer(min: 1, max: 65535),
  }, required: ["host","port"])

  let cfg = config::from_args(args)?
  validate::validate(sch, cfg)?
  print("OK: " + to_string(cfg))
  0
}
```

### 19.2 Fichiers : atomique + Result + `?`

```vitte
use fs

do save_json_atomic(p: str, data: []u8) -> Result[Unit, FsError] {
  fs::write_atomic(p, data)?
  Ok(())
}
```

### 19.3 Réseau sync + retry

```vitte
use http_client; use retry; use log

do fetch_with_retry(url: str) -> Result[String, str] {
  let policy = retry::exponential_backoff(max_retries: 5, base_ms: 100, jitter: true)
  let res = retry::run(policy, || http_client::get(url))
  match res {
    Ok(r)  => Ok(string::from_bytes(r.body)),
    Err(e) => Err(log::error(e)),
  }
}
```

### 19.4 Threads + channels

```vitte
use thread; use channel

do sum_parallel(xs: Vec[i32]) -> i64 {
  let (tx, rx) = channel::channel
  let mid = xs.len() / 2

  let left = thread::spawn({
    let mut s = 0
    for x in xs[0..mid] { s += x as i64 }
    tx.send(s)
  })
  let right = thread::spawn({
    let mut s = 0
    for x in xs[mid..] { s += x as i64 }
    tx.send(s)
  })

  let a = rx.recv().unwrap()
  let b = rx.recv().unwrap()
  left.join(); right.join()
  a + b
}
```

### 19.5 Async (preview) : fetch + parse + join

```vitte
async do fetch_status(url: str) -> i32 {
  let r = await http_client::get(url)
  r.status
}

async do main() {
  let (a, b) = await (fetch_status("https://a"), fetch_status("https://b"))
  print(a); print(b)
}
```

### 19.6 FFI : printf + add

```vitte
extern(c) do printf(fmt: *u8, ...) -> i32
extern(c) do add(a: i32, b: i32) -> i32

do main(){
  printf("sum=%d\n", add(2,3))
}
```

### 19.7 Traits + génériques + map

```vitte
trait ToStr { do to_str(self &) -> String }
impl ToStr for i32 { do to_str(self &) -> String { to_string(self) } }

do join_display[T: ToStr](xs: Vec[T], sep: str) -> String {
  let mut out = String::new()
  let mut first = true
  for x in xs {
    if !first { out = out + sep } else { first = false }
    out = out + x.to_str()
  }
  out
}
```

### 19.8 Pattern matching avancé

```vitte
enum Tree[T] { Empty, Node(left: Box[Tree[T]], v: T, right: Box[Tree[T]]) }

do height[T](t: Tree[T]) -> i32 {
  match t {
    Empty => 0,
    Node{left, _, right} => 1 + max(height(*left), height(*right)),
  }
}
```

### 19.9 Slices & bounds (sécurité)

```vitte
do middle(xs: []i32) -> Option[i32] {
  if xs.len() == 0 { None } else { Some(xs[xs.len()/2]) }
}
```

### 19.10 Ids & UUIDs

```vitte
use uuid; use idgen

let u = uuid::v4()
let id = idgen::next()
```

---

## 20) Migration & compat (vivre longtemps)

- Changements **ruptifs** groupés par **édition** (ex: 2026).  
- Toute modif de langage passe par `rfcs/` (template `0000-template.md`).  
- Statuts : `stable` (gel), `preview` (retours), `experimental` (peut casser).

---

## 21) Anti-patterns (et alternatives)

- ❌ lever un `panic!` pour une erreur d’I/O → ✅ retourner `Result`.  
- ❌ partager `&mut` simultanés → ✅ `Mutex` / `channel`.  
- ❌ passer `String` au FFI → ✅ `*u8 + len` + propriété claire.

---

## 22) Appendix : table de correspondance (repères)

| Concept Vitte | Rust | C/C++ | Python |
|---|---|---|---|
| `Result[T,E]` | `Result<T,E>` | codes ret + errno | exceptions |
| `&T` / `&mut T` | borrow | `const T*` / `T*` (sans garanties) | n/a |
| `Vec[T]` | `Vec<T>` | `std::vector<T>` | `list[T]` |
| `match` | `match` | `switch` (limité) | `match` (3.10+) |
| `trait` | trait | interface / concepts | ABC |

---

**Fin — édition 2025.**  
Les sections *preview/experimental* sont ouvertes à feedback, RFC et benchmarks reproductibles.
