use quote::quote;
use syn::{punctuated::Punctuated, token::Comma, Data, DataStruct, Field, FieldsNamed, Ident};

fn write_named_fields(named: &Punctuated<Field, Comma>) -> Vec<proc_macro2::TokenStream> {
    let write_fields = named
        .iter()
        .map(|f| {
            let field_name = &f.ident;
            let field_type = &f.ty;
            let ident_dot_field = quote! { &self.#field_name };

            match field_type {
                syn::Type::Path(_) | syn::Type::Array(_) => {
                    if f.attrs.iter().any(|attr| attr.path().is_ident("ignore")) {
                        quote!()
                    } else {
                        quote! {
                            wakfu_buf::WakfuBufWritable::write_into(#ident_dot_field, buf)?;
                        }
                    }
                }
                _ => panic!("Unsupported field type!"),
            }
        })
        .collect::<Vec<_>>();

    write_fields
}

pub fn create_impl_writable(ident: &Ident, data: &Data) -> proc_macro2::TokenStream {
    match data {
        syn::Data::Struct(DataStruct { fields, .. }) => {
            let syn::Fields::Named(FieldsNamed { named, .. }) = fields else {
                panic!("#[derive(WakfuBuf)] can only be used on structs with named fields")
            };

            let write_fields = write_named_fields(named);

            quote! {
                impl wakfu_buf::WakfuBufWritable for #ident {
                    fn write_into(&self, buf: &mut impl std::io::Write) -> Result<(), std::io::Error> {
                        #(#write_fields)*
                        Ok(())
                    }
                }
            }
        }
        _ => panic!("Not implemented yet"),
    }
}
