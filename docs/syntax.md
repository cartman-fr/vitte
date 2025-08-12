# Vitte — Syntaxe mixte High/Low (v1, MVP)

> Un seul langage, deux registres: **High** (productif, expressif, proche Python/Ruby) et **Low** (précis, contrôlé, proche C/ASM). Le High se compile vers un MIR lisible; le Low est explicite et ne cache rien.

## 1. Lexique & tokens
- **Indentation signifiante** pour les blocs High. Les blocs Low utilisent `@low do { ... }` avec accolades.
- **Commentaires**: `#` jusqu’à fin de ligne; `#[[ ... ]]#` multilignes.
- **Identifiants**: `[_a-zA-Z][_a-zA-Z0-9]*` (Unicode letters autorisées).
- **Littéraux**: entiers (`123`, `0xFF`, `0b1010`), flottants (`1.0`, `2e-3`), chaînes `"..."` (échappements `\n`, `\xNN`, `\u{...}`), bool (`true/false`).

## 2. Types de base
```
i8 i16 i32 i64 i128, u8 u16 u32 u64 usize, f32 f64, bool, char, str, String
ptr<T>, raw<T>, slice<T>, &T, &mut T, Option<T>, Result<T,E>
```
- `ptr<T>`: pointeur non-null, dérivé de références sûres; **non alias** garanti si issu d’un `&mut T` unique.
- `raw<T>`: pointeur brut, aliasing libre (diagnostic `warn(aliased_write)`).
- `slice<T>`: vue `{ptr,len}`; conversion depuis `&[T]`/`String` via `.as_ptr()`/`.as_bytes()`.

## 3. Attributs & ABI
- `@low` (bloc) : section bas-niveau.
- `@repr(c)`, `@align(N)`, `@packed` : layout mémoire.
- `extern(c) do sig` : FFI C stable.
- Décorateurs high-level: `@memoize`, `@test`, `@inline(hint)`.

## 4. Déclarations
### 4.1 Variables et constantes
```vitte
let x = 42
let mut y: i32 = 0
const PI: f64 = 3.1415926535
```

### 4.2 Fonctions
```vitte
def add(a: i32, b: i32) -> i32: a + b

def map_sum(xs: List<i32>) -> i32:
  xs |> map(fn x => x * 2) |> filter(fn x => x % 3 == 0) |> sum()
```

### 4.3 Classes/structures
```vitte
class Counter:
  @repr(c)
  var n: i64 = 0
  def inc(by: i64=1): self.n += by
  def get() -> i64: self.n
```

### 4.4 Modules & imports
```vitte
module math/vec
from math.vec import Vec2, dot
```

## 5. Contrôle de flux
```vitte
if cond: body
elif cond2: body2
else: alt

while i < n: i += 1

for (i, v) in xs.enumerate():
  if v % 2 == 0: continue
  break if i > 10

match x:
  0 => "zero"
  1 | 2 => "small"
  _ => "large"
```

## 6. Fonctions anonymes & blocs Ruby-like
```vitte
xs.select(|x| x%2==0).map(|x| x*x).reduce(0, |acc,x| acc+x)

# Blocs nommés type DSL
route "/users" do
  get "/:id" do |ctx|
    ctx.json(user_repo.find(ctx.param("id")))
  end
end
```

## 7. Exceptions (High) vs status (Low)
- High: `raise/rescue`, propagation `?` depuis `Result`.
- Low: **jamais** de `raise` implicite; préférer `Result<T,E>` ou codes.

```vitte
def read_file(p: Path) -> String:
  try:
    fs.read_to_string(p)?
  rescue e as IOError:
    log.error("IO {}", e); raise e
```

## 8. Ponts High ↔ Low
1. Objets annotés `@repr(c)` ont un layout stable accessible depuis `@low`.
2. `&T / &mut T` → `ptr<T>` via `.as_ptr()` / `.as_mut_ptr()`; `String` → `slice<u8>` via `.as_bytes()`.
3. Emprunts exclusifs respectés dans le MIR; diagnostics alias sur `raw<T>`.

```vitte
def write_all(buf: Bytes, dst: ptr<u8>) -> Result<void, IOError>:
  @low
  do {
    memcpy(dst, buf.data.as_ptr(), buf.len)
    return Ok(())
  }
```

## 9. Intrinsics & asm
```vitte
@low
do fence(): intrin!.fence()

extern(c) do puts(msg: ptr<u8>) -> i32

@low
do fast_add(a: i32, b: i32) -> i32 {
  asm! { rax = a + b }   # Pseudo-IR abaissé par le backend
}
```

## 10. Collections & itérateurs (style Python/Ruby)
```vitte
nums = [1,2,3,4,5]
evsq = nums.select(|x| x%2==0).map(|x| x*x).to_vec()
```

## 11. Décorateurs, mixins, protocoles
```vitte
module Printable:
  def print(): std::io::println(self.to_string())

class Point include Printable:
  var x:i32; var y:i32
  def to_string()->String: "({},{})".format(self.x, self.y)
```

## 12. Règles de formatage
- Indent 2 espaces, largeur 100, virgule traînante en multilignes, lambdas compacts.
- Fichiers `.vitte` UTF‑8, fin de ligne `\n`.

## 13. Mini EBNF (MVP)
```
Program     := Item*
Item        := Def | Class | Module | Let | Const | ExternDecl
BlockHigh   := Indent Stmt+ Dedent
BlockLow    := "@low" "do" "{" Stmt* "}"
Stmt        := Let | Assign | While | For | Match | If | Expr ";"
Lambda      := ("fn" ParamList "=>" Expr) | ("|" ParamList "|" Expr)
PtrType     := "ptr" "<" Type ">" | "raw" "<" Type ">"
SliceType   := "slice" "<" Type ">"
Decorator   := "@" Ident
ExternDecl  := "extern" "(" "c" ")" "do" Sig
```

## 14. Abaissement High → MIR (exemples)
### 14.1 `map`/`select`/`reduce`
High:
```vitte
xs.select(|x| pred(x)).map(|x| f(x)).reduce(init, |acc,x| acc+g(x))
```
MIR (schéma):
```
acc := init
for tmp in xs:
  if !pred(tmp) { continue }
  y := f(tmp)
  acc := reduce_fn(acc, y)
return acc
```

### 14.2 `match`
High:
```vitte
match n:
  0 => a
  1 | 2 => b
  _ => c
```
MIR:
```
switch n {
  case 0: goto L_a
  case 1,2: goto L_b
  default: goto L_c
}
```

## 15. Sécurité mémoire (résumé)
- `@low` : frontières nettes, pas d’alloc implicite, intrinsics explicites, alias contrôlé.
- `High` : emprunts vérifiés au niveau MIR, itérateurs sûrs.

## 16. Standard lib (MVP)
- `std::io`, `std::mem`, `std::math`, `std::collections`, `std::fmt`, `std::iter`.

## 17. Conventions & lints
- `unused`, `shadow`, `perf.invariant_range`, `alias.write` (warn en `raw<T>`), `panic_in_low` (deny).

## 18. Exemples rapides
```vitte
def copy(dst: &mut [u8], src: &[u8]):
  assert dst.len >= src.len
  for (i, b) in src.enumerate(): dst[i] = b

@low
do copy_raw(dst: ptr<u8>, src: ptr<u8>, n: usize):
  let mut i = 0
  while i < n { *(dst+i) = *(src+i); i += 1 }
```

---
**Statut**: Spécification MVP destinée au parser/HIR/MIR. Les détails d’inférence, traits et borrow avancé suivront dans `type_system.md`.
