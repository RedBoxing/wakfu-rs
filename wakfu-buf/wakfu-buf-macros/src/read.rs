use quote::quote;
use syn::{punctuated::Punctuated, token::Comma, Data, DataStruct, Field, FieldsNamed, Ident};

fn read_named_fields(
    named: &Punctuated<Field, Comma>,
) -> (Vec<proc_macro2::TokenStream>, Vec<&Option<Ident>>) {
    let read_fields = named
        .iter()
        .map(|f| {
            let field_name = &f.ident;
            let field_type = &f.ty;

            match field_type {
                syn::Type::Path(_) | syn::Type::Array(_) => {
                    if f.attrs.iter().any(|attr| attr.path().is_ident("ignore")) {
                        quote! {
                            let #field_name = Default::default();
                        }
                    } else {
                        quote! {
                            let #field_name = wakfu_buf::WakfuBufReadable::read_from(buf)?;
                        }
                    }
                }
                _ => panic!("Unsupported field type!"),
            }
        })
        .collect::<Vec<_>>();

    let read_fields_names = named.iter().map(|f| &f.ident).collect::<Vec<_>>();

    (read_fields, read_fields_names)
}

pub fn create_impl_readable(ident: &Ident, data: &Data) -> proc_macro2::TokenStream {
    match data {
        syn::Data::Struct(DataStruct { fields, .. }) => {
            let syn::Fields::Named(FieldsNamed { named, .. }) = fields else {
                panic!("#[derive(WakfuBuf)] can only be used on structs with named fields")
            };

            let (read_fields, read_fields_names) = read_named_fields(named);

            quote! {
                impl wakfu_buf::WakfuBufReadable for #ident {
                    fn read_from(buf: &mut std::io::Cursor<&[u8]>) -> Result<Self, wakfu_buf::BufReadError> {
                        #(#read_fields)*
                        Ok(#ident {
                            #(#read_fields_names: #read_fields_names),*
                        })
                    }
                }
            }
        }
        _ => panic!("Not implemented yet"),
    }
}
