//! Real LLVM codegen via Inkwell (feature="real")
#![cfg(feature="real")]

use inkwell::context::Context;
use inkwell::targets::{InitializationConfig, Target, TargetMachine, RelocMode, CodeModel, FileType, TargetTriple};
use inkwell::OptimizationLevel;
use vitte_backend_api::{Backend, Target as VTarget, BuildResult, MirFn};
use std::path::PathBuf;
use std::process::Command;

pub struct LlvmReal {
    target: VTarget,
    tm: TargetMachine,
}

impl LlvmReal {
    pub fn new(triple: &str) -> anyhow::Result<Self> {
        Target::initialize_all(&InitializationConfig::default());
        let triple = TargetTriple::create(triple);
        let target = Target::from_triple(&triple)?;
        let tm = target.create_target_machine(&triple, "generic", "", OptimizationLevel::Default, RelocMode::PIC, CodeModel::Default)
            .ok_or_else(|| anyhow::anyhow!("failed to create TargetMachine"))?;
        let vt = VTarget{ triple: triple.to_string(), pointer_width: 64, endian: vitte_backend_api::Endianness::Little };
        Ok(Self{ target: vt, tm })
    }

    pub fn compile_hello(&self, func: &MirFn, out_stem: &str) -> anyhow::Result<BuildResult> {
        // Build IR: declare puts, define main calling puts("LLVM OK")
        let ctx = Context::create();
        let module = ctx.create_module("vitte");
        let builder = ctx.create_builder();

        let i8ptr = ctx.i8_type().ptr_type(inkwell::AddressSpace::Generic);
        let puts_ty = ctx.i32_type().fn_type(&[i8ptr.into()], false);
        let puts = module.add_function("puts", puts_ty, None);

        let main_ty = ctx.void_type().fn_type(&[], false);
        let main = module.add_function(&func.name, main_ty, None);
        let entry = ctx.append_basic_block(main, "entry");
        builder.position_at_end(entry);

        // Create global "LLVM OK\0"
        let msg = format!("{}: LLVM OK\0", func.name);
        let str_ty = ctx.i8_type().array_type(msg.len() as u32);
        let g = module.add_global(str_ty, None, "msg");
        let bytes: Vec<_> = msg.bytes().map(|b| ctx.i8_type().const_int(b as u64, false)).collect();
        g.set_initializer(&ctx.i8_type().const_array(&bytes));
        g.set_constant(true);
        g.set_unnamed_address(inkwell::values::UnnamedAddress::Global);
        let zero = ctx.i32_type().const_zero();
        let gv_ptr = unsafe { g.as_pointer_value().const_gep(&[zero, zero]) };

        builder.build_call(puts, &[gv_ptr.into()], "call_puts");
        builder.build_return(None);

        // Emit object file
        let obj_path = PathBuf::from(format!("{out}.o", out=out_stem));
        self.tm.write_to_file(&module, FileType::Object, &obj_path)?;

        // Link with cc to produce .vitx
        let out = format!("{out}.vitx", out=out_stem);
        let cmd = format!("cc {} -o {} && strip {}", obj_path.display(), out, out);
        let st = Command::new("sh").arg("-lc").arg(cmd).status()?;
        let log = if st.success() { format!("linked {}", out) } else { "cc failed".into() };
        Ok(BuildResult{ artifact: out, log })
    }
}

impl Backend for LlvmReal {
    fn target(&self) -> &VTarget { &self.target }
    fn compile_fn(&mut self, func: &MirFn, out_stem: &str) -> std::io::Result<BuildResult> {
        self.compile_hello(func, out_stem).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
    }
    fn link(&mut self, _objects: &[String], _out_vitx: &str) -> std::io::Result<BuildResult> {
        Err(std::io::Error::new(std::io::ErrorKind::Other, "not implemented"))
    }
}
