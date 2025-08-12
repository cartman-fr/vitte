//! Build entry: dispatch to real (feature="real") or fallback (C shim).
use vitte_backend_api::{Backend, Target, BuildResult, MirFn, write_c_print, cc_build};
use crate::isa;

pub struct LlvmBackend { target: Target, opt: &'static str }
impl LlvmBackend { pub fn new(triple: &str, opt: &'static str) -> Self { Self{ target: isa::detect(triple), opt } } }

#[cfg(feature="real")]
impl Backend for LlvmBackend {
    fn target(&self) -> &Target { &self.target }
    fn compile_fn(&mut self, func: &MirFn, out_stem: &str) -> std::io::Result<BuildResult> {
        let mut real = crate::real::LlvmReal::new(&self.target.triple).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        real.compile_fn(func, out_stem)
    }
    fn link(&mut self, _objects: &[String], _out_vitx: &str) -> std::io::Result<BuildResult> {
        Err(std::io::Error::new(std::io::ErrorKind::Other, "not implemented"))
    }
}

#[cfg(not(feature="real"))]
impl Backend for LlvmBackend {
    fn target(&self) -> &Target { &self.target }
    fn compile_fn(&mut self, func: &MirFn, out_stem: &str) -> std::io::Result<BuildResult> {
        let msg = format!("LLVM[{}:{}] compiled fn {}", self.target.triple, self.opt, func.name);
        let c = write_c_print(out_stem, &msg)?;
        cc_build(&c, out_stem)
    }
    fn link(&mut self, objects: &[String], out_vitx: &str) -> std::io::Result<BuildResult> {
        if let Some(obj) = objects.first() {
            Ok(BuildResult{ artifact: out_vitx.to_string(), log: format!("linked {}", obj) })
        } else {
            Ok(BuildResult{ artifact: out_vitx.to_string(), log: "link: no objects".into() })
        }
    }
}
