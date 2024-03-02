use proc_macro::TokenStream;
use quote::quote;
use syn::{braced, parse::Parse, parse_macro_input, Ident, LitInt, Token};

/*
 * Credits: https://github.com/azalea-rs/azalea/blob/main/azalea-protocol/azalea-protocol-macros/src/lib.rs
 */

#[derive(Debug)]
struct PacketIdPair {
    id: u32,
    module: Ident,
    name: Ident,
}

#[derive(Debug)]
struct PacketIdMap {
    packets: Vec<PacketIdPair>,
}

impl Parse for PacketIdMap {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut packets = vec![];

        while let Ok(id) = input.parse::<LitInt>() {
            let id = id.base10_parse::<u32>()?;
            input.parse::<Token![:]>()?;

            let module: Ident = input.parse()?;
            input.parse::<Token![::]>()?;

            let name: Ident = input.parse()?;

            packets.push(PacketIdPair { id, module, name });

            if input.parse::<Token![,]>().is_err() {
                break;
            }
        }

        Ok(PacketIdMap { packets })
    }
}

#[derive(Debug)]
struct DeclareStatePackets {
    name: Ident,
    clientbound: PacketIdMap,
    serverbound: PacketIdMap,
}

impl Parse for DeclareStatePackets {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // parse the first token
        let name = input.parse()?;

        // parse the next token (",")
        input.parse::<Token![,]>()?;

        // parse the clientbound token
        let clientbound_token: Ident = input.parse()?;
        if clientbound_token != "Clientbound" {
            return Err(syn::Error::new(
                clientbound_token.span(),
                "Expected `Clientbound`",
            ));
        }

        // continue parsing
        input.parse::<Token![=>]>()?;

        // parse the content of the clientbound map
        let content;
        braced!(content in input);
        let clientbound = content.parse()?;

        // parse the delimiter
        input.parse::<Token![,]>()?;

        // parse the serverbound token
        let serverbound_token: Ident = input.parse()?;
        if serverbound_token != "Serverbound" {
            return Err(syn::Error::new(
                serverbound_token.span(),
                "Expected `Serverbound`",
            ));
        }

        input.parse::<Token![=>]>()?;

        // parse the content of the serverbound map
        let content;
        braced!(content in input);
        let serverbound = content.parse()?;

        Ok(DeclareStatePackets {
            name,
            clientbound,
            serverbound,
        })
    }
}

#[proc_macro]
pub fn declare_state_packets(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeclareStatePackets);

    let clientbound_state_name =
        Ident::new(&format!("Clientbound{}", input.name), input.name.span());
    let serverbound_state_name =
        Ident::new(&format!("Serverbound{}", input.name), input.name.span());

    let mut clientbound_enum_content = quote!();
    let mut clientbound_id_match_content = quote!();

    let mut serverbound_enum_content = quote!();
    let mut serverbound_id_match_content = quote!();

    for PacketIdPair { id, module, name } in &input.clientbound.packets {
        let variant_name = variant_from_name(&name);

        clientbound_enum_content.extend(quote! {
            #variant_name(#module::#name),
        });

        clientbound_id_match_content.extend(quote! {
            #clientbound_state_name::#variant_name(_) => #id,
        });
    }

    for PacketIdPair { id, module, name } in &input.serverbound.packets {
        let variant_name = variant_from_name(&name);

        serverbound_enum_content.extend(quote! {
            #variant_name(#module::#name),
        });

        serverbound_id_match_content.extend(quote! {
            #serverbound_state_name::#variant_name(_) => #id,
        });
    }

    if input.clientbound.packets.is_empty() {
        clientbound_id_match_content.extend(quote! {
            _ => unreachable!("This enum is empty and can't exist")
        });
    }

    if input.serverbound.packets.is_empty() {
        serverbound_id_match_content.extend(quote! {
            _ => unreachable!("This enum is empty and can't exist")
        });
    }

    let content = quote! {
        #[derive(Debug, Clone)]
        pub enum #clientbound_state_name
        where
            Self: Sized,
        {
            #clientbound_enum_content
        }

        #[derive(Debug, Clone)]
        pub enum #serverbound_state_name
        where
            Self: Sized,
        {
            #serverbound_enum_content
        }

        #[allow(unreachable_code)]
        impl crate::packets::ProtocolPacket for #clientbound_state_name {
            fn id(&self) -> u32 {
                match self {
                    #clientbound_id_match_content
                }
            }
        }

        #[allow(unreachable_code)]
        impl crate::packets::ProtocolPacket for #serverbound_state_name {
            fn id(&self) -> u32 {
                match self {
                    #serverbound_id_match_content
                }
            }
        }
    };

    content.into()
}

fn variant_from_name(name: &Ident) -> Ident {
    let mut variant_name = name.to_string();

    if variant_name.starts_with("Clientbound") {
        variant_name = variant_name["Clientbound".len()..].to_string();
    } else if variant_name.starts_with("Serverbound") {
        variant_name = variant_name["Serverbound".len()..].to_string();
    }

    if variant_name.ends_with("Packet") {
        variant_name = variant_name[..variant_name.len() - "Packet".len()].to_string();
    }

    Ident::new(&variant_name, name.span())
}
