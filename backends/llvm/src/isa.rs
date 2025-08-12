use vitte_backend_api::{Target, Endianness};
pub fn detect(triple: &str) -> Target {
    let pointer_width = if triple.contains("64") { 64 } else { 32 };
    let endian = Endianness::Little;
    Target{ triple: triple.to_string(), pointer_width, endian }
}
