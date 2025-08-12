//! Bootstrapping transpiler Vitte -> C (toy, for bring-up)
/*
This is a pragmatic engineering bridge to produce working binaries early:
- Parse a tiny subset of Vitte: `fn main(){ print("...") }`
- Emit C code that calls puts() / printf()
- Let system toolchains link it, then tag as .vitx
*/
