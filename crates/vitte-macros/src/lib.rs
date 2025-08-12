extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(DisplayName)]
pub fn display_name(input: TokenStream) -> TokenStream {
    let _ast = parse_macro_input!(input as DeriveInput);
    let gen = quote! { /* demo */ };
    gen.into()
}
