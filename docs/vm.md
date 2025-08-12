# Vitte VM (v0.1, MVP)

Interpreter for Vitte Bytecode with optional Cranelift JIT backend. Pure Rust, no `unsafe` across the public surface (internals gated).

## 1. Architecture
- **Core**: register file, block dispatcher, opcode handlers.
- **Runtime**: allocator + GC (generational, tri‑color), exception machinery, intrinsics.
- **Linker**: imports resolver (symbol → host function thunk).
- **JIT (opt.)**: hot blocks → Cranelift; guards maintain compatibility with interpreter state.

## 2. Execution Model
- Per‑function register array `Vec<Value>`; `PC` points to (block,label,inst).
- Dispatch loop:
  1. Fetch opcode
  2. Decode operands
  3. Execute handler
  4. Update `PC`
- Hotness counter per block; threshold triggers JIT compile & patch.

## 3. Value Representation
```
enum Value {
  I64(i64), F64(f64), Bool(bool),
  Ref(Handle), // GC object
  Ptr(*mut u8), // raw in @low paths
  Unit,
}
```
- `NaN-boxing` optional; MVP uses tagged enum.
- Type checks in handlers when needed (`debug` builds).

## 4. Memory & GC
- **Allocator**: bump + segregated lists for small objects; large objects via pages.
- **GC**: generational, write barrier on `STF`; safe points at calls & backedges.
- **Pinning**: `Pin<Ref>` for FFI and `@low` regions; epochs prevent movement.

## 5. Exceptions & Traps
- Bytecode `TRAP` maps to VM `Trap` with code (overflow, div0, OOB, null, user).
- High‑level `RAISE` builds an exception object and unwinds to boundary frames.
- Handlers table per block for fast landing pads (zero‑cost where possible).

## 6. Intrinsics
- Implemented in Rust; call via `INTR`. Some may fast‑path in JIT.
- Fences are passthrough to `atomic::fence(Ordering::SeqCst)`.

## 7. Calling Convention
- Arguments in R0..Rn; return in R0.
- Tail calls (`TCALL`) replace current frame; enables proper recursion.

## 8. Debugging & Profiling
- Step/run, break on label, watch registers, heap inspector.
- Perf counters: op counts, GC time, JIT time, block hotness.

## 9. Safety Model
- Interpreter defends: bounds checks for arrays, null checks, trap on UB‑prone ops.
- `@low` paths can request raw `LD/ST`; guarded by capabilities passed from compiler.

## 10. Embedding
- VM exposes `Vm::load(chunk)` and `Vm::call(func, args)`.
- Host can register native fns: `vm.register("std::io::print", fn_ptr)`.
