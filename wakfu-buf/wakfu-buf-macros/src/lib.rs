use proc_macro::TokenStream;
use quote::quote;
use read::create_impl_readable;
use syn::{parse_macro_input, DeriveInput};
use write::create_impl_writable;

mod read;
mod write;

#[proc_macro_derive(WakfuBuf)]
pub fn derive_wakfubuf(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);

    let readable = create_impl_readable(&ident, &data);
    let writable = create_impl_writable(&ident, &data);

    quote! {
        #readable
        #writable
    }
    .into()
}
