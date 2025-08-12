//! Real Cranelift codegen (feature = "real")
#![cfg(feature = "real")]

use cranelift_codegen::isa;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilderContext, FunctionBuilder, Variable};
use cranelift_module::{Linkage, Module};
use cranelift_object::{ObjectBackend, ObjectBuilder};
use target_lexicon::Triple;

use vitte_backend_api::{Backend, Target, BuildResult, MirFn, MirInst, Ty};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub struct CraneliftReal {
    target: Target,
    isa: Box<dyn isa::TargetIsa>,
}

impl CraneliftReal {
    pub fn new(triple: &str) -> anyhow::Result<Self> {
        let triple: Triple = triple.parse()?;
        let mut flag_builder = settings::builder();
        flag_builder.set("opt_level", "speed").ok();
        let isa = isa::lookup(triple)?.finish(settings::Flags::new(flag_builder))?;
        let target = vitte_backend_api::Target{ triple: isa.triple().to_string(), pointer_width: if isa.pointer_width().bytes()==8 {64} else {32}, endian: vitte_backend_api::Endianness::Little };
        Ok(Self{ target, isa })
    }

    fn lower_function<M: Module>(&self, func: &MirFn, module: &mut M) -> anyhow::Result<cranelift_module::FuncId> {
        use cranelift_codegen::ir::{AbiParam, Signature, InstBuilder, types::*};
        // Signature: main() -> void
        let mut sig = Signature::new(module.isa().default_call_conv());
        let func_id = module.declare_function(&func.name, Linkage::Export, &sig)?;

        let mut ctx = module.make_context();
        ctx.func.signature = sig;
        let mut fbc = FunctionBuilderContext::new();
        let mut fb = FunctionBuilder::new(&mut ctx.func, &mut fbc);

        let block0 = fb.create_block();
        fb.append_block_params_for_function_params(block0);
        fb.switch_to_block(block0);
        fb.seal_block(block0);

        // We'll implement: puts("CLIF OK"); return;
        // Create a string data object
        let msg = format!("{}: CLIF OK\0", func.name);
        let data_id = module.declare_data(&format!("_str_{}", func.name), Linkage::Local, false, false)?;
        let mut data_ctx = cranelift_module::DataContext::new();
        data_ctx.define(msg.as_bytes().to_vec().into_boxed_slice());
        module.define_data(data_id, &data_ctx)?;
        let gv = module.declare_data_in_func(data_id, fb.func);

        // Declare puts
        let mut puts_sig = Signature::new(module.isa().default_call_conv());
        puts_sig.params.push(AbiParam::new(module.target_config().pointer_type()));
        puts_sig.returns.push(AbiParam::new(I32));
        let puts = module.declare_function("puts", Linkage::Import, &puts_sig)?;
        let puts_ref = module.declare_func_in_func(puts, fb.func);

        // Build address of string
        let pty = module.target_config().pointer_type();
        let msg_ptr = fb.ins().global_value(pty, gv);

        // Call puts(msg_ptr)
        let call = fb.ins().call(puts_ref, &[msg_ptr]);
        let _ = fb.inst_results(call);

        fb.ins().return_(&[]);
        fb.finalize();

        module.define_function(func_id, &mut ctx)?;
        Ok(func_id)
    }

    fn link_object(obj_path: &PathBuf, out_stem: &str) -> anyhow::Result<BuildResult> {
        // Try cc to link the object (and link libc for puts)
        let out = format!("{out}.vitx", out=out_stem);
        let cmd = format!("cc {} -o {} && strip {}", obj_path.display(), out, out);
        let st = Command::new("sh").arg("-lc").arg(cmd).status()?;
        let log = if st.success() { format!("linked {}", out) } else { "cc failed".into() };
        Ok(BuildResult{ artifact: out, log })
    }
}

impl Backend for CraneliftReal {
    fn target(&self) -> &Target { &self.target }

    fn compile_fn(&mut self, func: &MirFn, out_stem: &str) -> std::io::Result<BuildResult> {
        let builder = ObjectBuilder::new(self.isa.clone(), "vitte", cranelift_module::default_libcall_names()).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        let mut module: cranelift_module::Module<ObjectBackend> = cranelift_module::Module::new(builder);

        let fid = self.lower_function(func, &mut module).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        module.clear_context(&mut module.make_context());
        module.finalize_definitions();

        let obj = module.finish();
        let obj_path = PathBuf::from(format!("{out}.o", out=out_stem));
        fs::write(&obj_path, obj.emit().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?)?;

        // link
        let res = Self::link_object(&obj_path, out_stem).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        Ok(res)
    }

    fn link(&mut self, _objects: &[String], _out_vitx: &str) -> std::io::Result<BuildResult> {
        Err(std::io::Error::new(std::io::ErrorKind::Other, "use compile_fn (single fn demo)"))
    }
}
