# High → MIR → Bytecode — Plan par construction (MVP)

Objectif: chaque sucre high‑level a un schéma MIR déterministe, abaissé ensuite vers un petit set d’opcodes. Cette table sert de contrat entre `vitte-syntax`/`vitte-hir`/`vitte-mir` et `vitte-bytecode`/`vitte-vm`.

## 1. Assignations & expressions
### High
```vitte
let x = a + b*2
```
### MIR
```
t0 = MUL b, 2
x  = ADD a, t0
```
### Bytecode
```
K32  R2, 2
MUL  R3, R1, R2
ADD  R0, R0, R3      ; assume a→R0, b→R1, x→R0 (SSA renamed in real MIR)
```

## 2. `if` / `else`
### High
```vitte
if c: a else: b
```
### MIR
```
t = CMP c, true, EQ
br t -> L1 else L2
L1: r = a; goto L3
L2: r = b; goto L3
L3: φ(r) -> out
```
### Bytecode
```
CMP  R3, Rc, Rtrue, EQ
JIF  R3, L1, L2
L1:  MOV Rout, Ra; JMP L3
L2:  MOV Rout, Rb; JMP L3
L3:
```

## 3. `match` (patterns simples)
### High
```vitte
match n:
  0 => a
  1 | 2 => b
  _ => c
```
### MIR
```
switch n { case 0:L0, 1:L1, 2:L1, default:L2 }
```
### Bytecode
```
; Lowered to CMP/JIF chain (MVP)
K32 Rtmp,0; CMP Rk, Rn,Rtmp,EQ; JIF Rk, L0,Lx
K32 Rtmp,1; CMP Rk, Rn,Rtmp,EQ; JIF Rk, L1,Lx
K32 Rtmp,2; CMP Rk, Rn,Rtmp,EQ; JIF Rk, L1,L2
```

## 4. Boucles `for` itérateurs
### High
```vitte
for (i, v) in xs.enumerate():
  body(i,v)
```
### MIR
```
it = iter(xs)
i  = 0
Lh:
  ok, v = next(it)
  br !ok -> Lend
  call body(i,v)
  i = i + 1
  goto Lh
Lend:
```
### Bytecode
```
CALL Rit, iter, 1 [Rxs]
K32  Ri, 0
Lh:
CALL Rok, next, 1 [Rit]  ; returns (ok, v) in Rk,Rv
JIF  Rk, Lbody, Lend
CALL Rtmp, body, 2 [Ri,Rv]
ADD  Ri, Ri, 1
JMP  Lh
Lend:
```

## 5. Exceptions
### High
```vitte
try: f()? rescue e as IOError: g(e)
```
### MIR
```
call f -> Result
if is_err -> landing pad -> call g(e)
```
### Bytecode
```
CALL Rr, f, 0
; runtime protocol: Result is tagged in Rr
INTR Rk, is_err, 1 [Rr]
JIF  Rk, Lcatch, Lok
Lok:  ; unpack ok
JMP Lend
Lcatch:
INTR Re, unwrap_err, 1 [Rr]
CALL R0, g, 1 [Re]
Lend:
```

## 6. Slices, ptr & @low memcpy
### High
```vitte
def write_all(buf: Bytes, dst: ptr<u8>):
  @low do { memcpy(dst, buf.data.as_ptr(), buf.len) }
```
### MIR
```
p = as_ptr(buf.data)
call intr.memcpy(dst, p, buf.len)
```
### Bytecode
```
INTR Rtmp, as_ptr, 1 [Rbuf_data]
INTR R0, memcpy, 3 [Rdst, Rtmp, Rbuf_len]
```

## 7. Arrays (MVP)
### High
```vitte
a = Array::new(n)
a[i] = v
x = a[i]
```
### MIR
```
a = arr_new n
arr_set a i v
x = arr_get a i
```
### Bytecode
```
ARR_NEW Ra, Rn
ARR_SET Ra, Ri, Rv
ARR_GET Rx, Ra, Ri
```

## 8. Décorateurs `@memoize` (schéma)
### High
```vitte
@memoize
def fib(n): ...
```
### MIR
```
if cache.contains(n): return cache[n]
r = fib_impl(n)
cache[n] = r
return r
```
### Bytecode
```
; implemented via prologue expansion calling runtime helpers
CALL Rk, cache_get, 2 [Rcache, Rn]
JIF  Rk, Lhit, Lmiss
Lhit: MOV R0, Rval; RET R0
Lmiss: CALL R0, fib_impl, 1 [Rn]
CALL Rtmp, cache_put, 3 [Rcache, Rn, R0]
RET R0
```

## 9. Traits/protocoles (future)
- Desugar en tables de méthodes (vtable) et `ICALL` indirectes.

## 10. Mapping récap (opcode usage)
| Construction high | MIR clé | Bytecode |
|-------------------|---------|----------|
| `a+b`             | `ADD`   | `ADD`    |
| `a?b:c`           | `SEL`   | `CMP`+`JIF`+`MOV` |
| `for`             | `iter/next` | `CALL`+`JIF` |
| `match`           | `switch`| `CMP` chain |
| slice get/set     | `ARR_GET/SET` | `ARR_GET/SET` |
| `raise`           | `RAISE` | `RAISE`  |
| `try/rescue`      | `landing pad` | helpers + `JIF` |
| `@low memcpy`     | intrinsic | `INTR(memcpy)` |

---
Ce plan est suffisant pour coder le parser → HIR → MIR et l’interpréteur MVP, tout en gardant une voie claire vers le JIT et les optimisations.
