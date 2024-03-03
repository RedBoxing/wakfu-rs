use proc_macro::TokenStream;
use quote::quote;
use syn::{braced, parse::Parse, parse_macro_input, DeriveInput, Ident, LitInt, Token};

/*
 * Credits: https://github.com/azalea-rs/azalea/blob/main/azalea-protocol/azalea-protocol-macros/src/lib.rs
 */

fn packet_derive(input: TokenStream, state: proc_macro2::TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);

    let syn::Data::Struct(syn::DataStruct { fields, .. }) = &data else {
        panic!("#[derive(*Packet)] can only be used on structs")
    };
    let syn::Fields::Named(_) = fields else {
        panic!("#[derive(*Packet)] can only be used on structs with named fields")
    };

    let variant_name = variant_from_name(&ident);

    let content = quote! {
        impl #ident {
            pub fn get(self) -> #state {
                #state::#variant_name(self)
            }

            pub fn write(&self, buf: &mut impl std::io::Write) -> Result<(), std::io::Error> {
                wakfu_buf::WakfuBufWritable::write_into(self, buf)
            }

            pub fn read(buf: &mut std::io::Cursor<&[u8]>) -> Result<#state, wakfu_buf::BufReadError> {
                use wakfu_buf::WakfuBufReadable;
                Ok(Self::read_from(buf)?.get())
            }
        }
    };

    content.into()
}

#[proc_macro_attribute]
pub fn clientbound_packet(attr: TokenStream, input: TokenStream) -> TokenStream {
    let input2 = input.clone();
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);

    let syn::Data::Struct(syn::DataStruct { fields, .. }) = &data else {
        panic!("#[derive(*Packet)] can only be used on structs")
    };

    let syn::Fields::Named(_) = fields else {
        panic!("#[derive(*Packet)] can only be used on structs with named fields")
    };

    let attr = proc_macro2::TokenStream::from(attr);
    let mut content = proc_macro2::TokenStream::from(input2);

    content.extend(quote! {
        impl crate::packets::ClientboundPacket for #ident {
            fn architecture_target(&self) -> u8 {
                #attr
            }
        }
    });

    content.into()
}

#[proc_macro_derive(ClientboundConnectionPacket, attributes(var))]
pub fn derive_clientbound_connection_packet(input: TokenStream) -> TokenStream {
    packet_derive(
        input,
        quote! { crate::packets::connection::ClientboundConnectionPacket },
    )
}

#[proc_macro_derive(ServerboundConnectionPacket, attributes(var))]
pub fn derive_serverbound_connection_packet(input: TokenStream) -> TokenStream {
    packet_derive(
        input,
        quote! { crate::packets::connection::ServerboundConnectionPacket },
    )
}

#[derive(Debug)]
struct PacketIdPair {
    id: u16,
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
            let id = id.base10_parse::<u16>()?;
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

    let state_name_litstr = syn::LitStr::new(&input.name.to_string(), input.name.span());

    let mut clientbound_enum_content = quote!();
    let mut clientbound_id_match_content = quote!();
    let mut clientbound_architecture_target_match_content = quote!();
    let mut clientbound_read_content = quote!();
    let mut clientbound_write_content = quote!();

    let mut serverbound_enum_content = quote!();
    let mut serverbound_id_match_content = quote!();
    let mut serverbound_read_content = quote!();
    let mut serverbound_write_content = quote!();

    for PacketIdPair { id, module, name } in &input.clientbound.packets {
        let variant_name = variant_from_name(&name);
        let name_litstr = syn::LitStr::new(&name.to_string(), name.span());

        clientbound_enum_content.extend(quote! {
            #variant_name(#module::#name),
        });

        clientbound_id_match_content.extend(quote! {
            #clientbound_state_name::#variant_name(_) => #id,
        });

        clientbound_architecture_target_match_content.extend(quote! {
            #clientbound_state_name::#variant_name(packet) => packet.architecture_target(),
        });

        clientbound_read_content.extend(quote! {
            #id => {
                let data = #module::#name::read(buf).map_err(|e| crate::read::ReadPacketError::Parse {
                    source: e,
                    packet_id: #id,
                    backtrace: Box::new(std::backtrace::Backtrace::capture()),
                    packet_name: #name_litstr.to_string(),
                })?;
                #[cfg(debug_assertions)]
                {
                    let mut leftover = Vec::new();
                    let _ = std::io::Read::read_to_end(buf, &mut leftover);
                    if !leftover.is_empty() {
                        return Err(Box::new(crate::read::ReadPacketError::LeftoverData { packet_name: #name_litstr.to_string(), data: leftover }));
                    }
                }
                data
            },
        });

        clientbound_write_content.extend(quote! {
            #clientbound_state_name::#variant_name(packet) => packet.write(buf),
        });
    }

    for PacketIdPair { id, module, name } in &input.serverbound.packets {
        let variant_name = variant_from_name(&name);
        let name_litstr = syn::LitStr::new(&name.to_string(), name.span());

        serverbound_enum_content.extend(quote! {
            #variant_name(#module::#name),
        });

        serverbound_id_match_content.extend(quote! {
            #serverbound_state_name::#variant_name(_) => #id,
        });

        serverbound_read_content.extend(quote! {
            #id => {
                let data = #module::#name::read(buf).map_err(|e| crate::read::ReadPacketError::Parse {
                    source: e,
                    packet_id: #id,
                    backtrace: Box::new(std::backtrace::Backtrace::capture()),
                    packet_name: #name_litstr.to_string(),
                })?;
                #[cfg(debug_assertions)]
                {
                    let mut leftover = Vec::new();
                    let _ = std::io::Read::read_to_end(buf, &mut leftover);
                    if !leftover.is_empty() {
                        return Err(Box::new(crate::read::ReadPacketError::LeftoverData { packet_name: #name_litstr.to_string(), data: leftover }));
                    }
                }
                data
            },
        });

        serverbound_write_content.extend(quote! {
            #serverbound_state_name::#variant_name(packet) => packet.write(buf),
        });
    }

    if input.clientbound.packets.is_empty() {
        clientbound_id_match_content.extend(quote! {
            _ => unreachable!("This enum is empty and can't exist")
        });

        clientbound_architecture_target_match_content.extend(quote! {
            _ => unreachable!("This enum is empty and can't existe")
        });

        clientbound_write_content.extend(quote! {
            _ => unreachable!("This enum is empty and can't exist")
        });
    }

    if input.serverbound.packets.is_empty() {
        serverbound_id_match_content.extend(quote! {
            _ => unreachable!("This enum is empty and can't exist")
        });

        serverbound_write_content.extend(quote! {
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
            fn id(&self) -> u16 {
                match self {
                    #clientbound_id_match_content
                }
            }

            fn architecture_target(&self) -> Option<u8> {
                use crate::packets::ClientboundPacket;
                Some(match self {
                    #clientbound_architecture_target_match_content
                })
            }

            fn read(id: u16, buf: &mut std::io::Cursor<&[u8]>) -> Result<Self, Box<crate::read::ReadPacketError>>
            where
            Self: Sized,
            {
                Ok(match id {
                    #clientbound_read_content
                    _ => return Err(Box::new(crate::read::ReadPacketError::UnknownPacketId { state_name: #state_name_litstr.to_string(), id })),
                })
            }

            fn write(&self, buf: &mut impl std::io::Write) -> Result<(), std::io::Error> {
                match self {
                    #clientbound_write_content
                }
            }
        }

        #[allow(unreachable_code)]
        impl crate::packets::ProtocolPacket for #serverbound_state_name {
            fn id(&self) -> u16 {
                match self {
                    #serverbound_id_match_content
                }
            }

            fn architecture_target(&self) -> Option<u8> {
                None
            }

            fn read(id: u16, buf: &mut std::io::Cursor<&[u8]>) -> Result<Self, Box<crate::read::ReadPacketError>>
            where
            Self:Sized,
            {
                Ok(match id {
                    #serverbound_read_content
                    _ => return Err(Box::new(crate::read::ReadPacketError::UnknownPacketId { state_name: #state_name_litstr.to_string(), id })),
                })
            }

            fn write(&self, buf: &mut impl std::io::Write) -> Result<(), std::io::Error> {
                match self {
                    #serverbound_write_content
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
