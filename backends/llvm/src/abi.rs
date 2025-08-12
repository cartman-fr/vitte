use vitte_backend_api::CallConv;
pub fn default_callconv(triple: &str) -> CallConv {
    if triple.contains("windows") { CallConv::Windows } else { CallConv::SystemV }
}
