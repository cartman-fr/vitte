# Vitte — Syntax v1.3 (Ultimate)

## Design tenets
- **Lisible**, **prévisible**, **optimisable**. Zéro magie cachée, coûts visibles.
- **Kernel-friendly** (no_std), **server-grade** (async), **embedded** (allocateurs opt-in).

## Lexique
- UTF-8 obligatoire. Commentaires `//` & `/* */` nestables.
- Ident = `[A-Za-z_][A-Za-z0-9_]*`
- Literaux: `123u32`, `0xFFu8`, `1.0f32`, `'x'`, `"str"`, `b"raw\xFF"`

## Mots-clés (réservés)
`fn let mut const static struct union enum trait impl type use mod pub extern return if else match for while loop break continue defer async await spawn try yield unsafe where macro consteval constexpr sizeof alignof typeof as in from cfg test bench doc inline cold no_mangle export repr pack align`

## Modules & imports
```vitte
mod net::tcp
use net::tcp::{Server, Client}
use std::fmt as f
```

## Déclarations
```vitte
const PI: f64 = 3.1415926535
static mut COUNTER: u64 = 0   # unsafe pour mut global

struct Point { x: f64, y: f64 }
union U { i: i32, f: f32 }

enum Color { Red, Green, Blue, Rgb{r:u8,g:u8,b:u8} }

type Millis = u64
```

## Fonctions
```vitte
fn add(a: i32, b: i32) -> i32 { a + b }

fn greet(name: string = "you") { print("hello ", name) }

fn map<T, U>(xs: &[T], f: fn(T)->U) -> Vec<U> where T: Copy {
  let mut out = Vec<U>::with_capacity(xs.len())
  for x in xs { out.push(f(*x)) }
  return out
}

#[inline] fn hot(x: i32)->i32 { x*2 }
#[cold]   fn slow()->never { panic("unreachable") }
```
- `defer { ... }` s’exécute à la sortie du scope, **sans allouer**.

## Contrôle & patterns
```vitte
if cond { ... } else if alt { ... } else { ... }

for i in 0..n { ... }        # exclusif
for b in bytes.iter() { ... }

while ready() { ... }
loop { if done { break } }

match p {
  Point { x, y } if x==y => diag(x),
  Point { x, y }         => plot(x, y),
  _ => {}
}
```

## Ownership & références
- Mouvement par défaut (`let y = x` bouge si non-Copy).
- `&T` lecture, `&mut T` unique écriture; lifetimes **inférés**.
- `Box<T>`, `Rc<T>`, `Arc<T>` dispo dans std (features).

## Traits & génériques
```vitte
trait Display { fn fmt(&self) -> string }
impl Display for Point { fn fmt(&self)->string { f::format("{},{}", self.x,self.y) } }

fn max<T: Ord>(a:T,b:T)->T { if a>b {a} else {b} }

fn buf<const N:usize>() -> [u8; N] { [0; N] }
```
- **Specialization** sous feature contrôlée: `#[feature(specialization)]`

## Async, tasks & générateurs
```vitte
async fn fetch(url: string) -> Result<string, NetErr> { ... }
let body = await fetch(u)?

fn numbers()->impl Iterator<i32> { yield 1; yield 2; }
```

## Erreurs (sucre & zéro coût)
```vitte
fn open(path: string) -> Result<File, IoErr> {
  let f = fs::open(path)?
  ensure!(f.size() > 0, IoErr::Empty)
  return Ok(f)
}
```
- `?` = test + early-return, sans unwind.  
- `ensure!/bail!` = sucre → `return Err(...)`

## FFI & repr
```vitte
#[repr(c)] struct Foo { x:i32, y:i32 }
extern(c) fn c_add(a:i32,b:i32)->i32
#[no_mangle] export fn vitte_symbol(){}
```

## Attributs clés
- `#[cfg(target="windows")]`, `#[repr(c|packed)]`, `#[align(64)]`  
- `#[test]`, `#[bench]`, `#[doc = "..."]`

## Opérateurs — précédence
1. `() [] . ::`  
2. `! ~ - * & &mut (unaires)`  
3. `* / %`  
4. `+ -`  
5. `<< >>`  
6. `< <= > >=`  
7. `== !=`  
8. `& ^ |`  
9. `&& ||`  
10. `= += -= *= /=`

## EBNF (résumé)
Voir `docs/EBNF_FULL.md`.
