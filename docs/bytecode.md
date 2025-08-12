# Vitte Bytecode (v0.1, MVP)

Design goals: portable, quick to interpret, JIT‑friendly. Register‑based IR with fixed‑width opcodes, separate constant pools, explicit GC barriers, and structured error/async hooks.

## 1. File/Module Layout
- **Chunk** = one compiled unit (module/function group).
- Sections (little‑endian):
  1. `HEAD` magic `VITBC\0`, version `0x0001`
  2. `STRS` string pool (UTF‑8)
  3. `CONS` constants (numbers, tuples, function refs)
  4. `TBLT` type table (optional in MVP)
  5. `IMPT` import table (symbol name → link index)
  6. `FUNC` function bodies
  7. `DATA` read‑only data blobs
  8. `SYMS` debug (line map, names)
  9. `END!`

## 2. Function Body
Header per function:
```
u32 n_regs         # virtual registers (R0..R{n-1})
u32 n_params       # parameters in R0..
u32 n_locals       # non‑param locals
u32 n_blocks
u32 n_insts
u32 const_off, dbg_off
u8  flags          # bit0=variadic, bit1=generator, bit2=async
```
Body = basic blocks with labels: `[label:u32][n:u32]{inst}^n`

## 3. Encoding
- 32‑bit opcode word, followed by 0..N operands (16‑bit unless specified).
- Canonical form: `OP dst, a, b, imm32`
- Registers: `u16` indexes into function register file.
- Immediates: `i32/u32` as needed.
- All arithmetic is **wrapping** by default; `*_CHK` variants trap on overflow.

## 4. Core Opcodes (MVP)
### 4.1 Move/const
- `MOV  rd, rs`
- `K32  rd, imm32`         ; load small immediates
- `KIDX rd, kidx:u32`      ; load from constant pool

### 4.2 Arithmetic (int/float)
- `ADD rd, ra, rb`, `SUB`, `MUL`, `DIV`, `MOD`
- `NEG rd, ra`
- `ADD_CHK`… (trap on overflow)
- `FADD`, `FSUB`, `FMUL`, `FDIV`

### 4.3 Bitwise
- `AND`, `OR`, `XOR`, `NOT`, `SHL`, `SHR`, `SAR`

### 4.4 Compare & select
- `CMP rd, ra, rb, cmp:i8` ; cmp: EQ,NE,LT,LE,GT,GE
- `SEL rd, cond, a, b`     ; rd = cond ? a : b

### 4.5 Control flow
- `JMP label`
- `JIF cond, label_true, label_false` (fallthrough optional)
- `RET r0`
- `TRAP code:i16`          ; raise low‑level trap (mapped to exception)

### 4.6 Memory (GC and raw)
- `LDF rd, base, off:i32`      ; load field (safe, with GC read barrier)
- `STF base, off:i32, rs`      ; store field (GC write barrier)
- `LD rd, addr`                ; raw load from `ptr<T>` (no barrier)
- `ST addr, rs`                ; raw store
- `ALOC rd, tyidx:u32`         ; allocate GC object of type
- `ARR_NEW rd, len`
- `ARR_GET rd, arr, idx`
- `ARR_SET arr, idx, rs`

### 4.7 Calls
- `CALL rd, f, argc:u8 [arg0..argN]`
- `TCALL f, argc:u8 [args]`     ; tail call
- `ICALL rd, funcref, argc:u8`  ; indirect
- `INTR rd, id:u16, argc:u8 [args]` ; intrinsic (fence, memcpy, etc.)

### 4.8 Exception/async (stub MVP)
- `RAISE rs`             ; convert to high‑level exception on boundary
- `RESUME cont`          ; for generators/async (reserved)

### 4.9 Stack/Frames (minimal)
- Register calling convention: args in R0..Rn, return in R0; callee may clobber all unless `preserve` flag set (non‑MVP).

## 5. Intrinsics Table (MVP)
| id | name         | semantics |
|----|--------------|-----------|
| 1  | `memcpy`     | Raw copy; UB on overlap (use `memmove` later) |
| 2  | `memset`     | Raw fill |
| 3  | `fence`      | Full memory fence |
| 4  | `sqrt`       | f64 sqrt |
| 5  | `ctz`        | count trailing zeros |

## 6. GC Barriers
- `LDF/STF` carry implicit read/write barriers implemented by the VM runtime.
- Raw `LD/ST` bypass the GC; safe only in `@low` sections with pinned/opaque regions.

## 7. Linking
- `IMPT` entries bind to external functions by name; VM provides a resolver. Native codegen will lower to relocations.

## 8. Debug
- `SYMS` stores file:line↔label mappings, register names for parameters, and pretty names for functions.

## 9. Compatibility
- Stable within minor versions; forward‑compatible by opcode table versioning.
