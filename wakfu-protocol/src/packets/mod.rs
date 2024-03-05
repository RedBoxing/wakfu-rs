use std::io::{Cursor, Write};

use wakfu_buf::{WakfuBufReadable, WakfuBufWritable};

use crate::read::ReadPacketError;

pub mod connection;

#[derive(Copy, Clone, Debug)]
pub enum ProtocolAdapter {
    ClientToServer,
    ServerToClient,
}

impl ProtocolAdapter {
    pub fn read_packet_header(
        &self,
        buf: &mut std::io::Cursor<&[u8]>,
    ) -> Result<ProtocolPacketHeader, wakfu_buf::BufReadError> {
        match self {
            Self::ClientToServer => {
                let length = u16::read_from(buf)?;
                let architecture_target = u8::read_from(buf)?;
                let id = u16::read_from(buf)?;

                Ok(ProtocolPacketHeader {
                    length,
                    architecture_target: Some(architecture_target),
                    id,
                })
            }
            Self::ServerToClient => {
                let length = u16::read_from(buf)?;
                let id = u16::read_from(buf)?;

                Ok(ProtocolPacketHeader {
                    length,
                    architecture_target: None,
                    id,
                })
            }
        }
    }

    pub fn get_header_size(&self) -> usize {
        match self {
            Self::ClientToServer => 5,
            Self::ServerToClient => 4,
        }
    }
}

pub trait ProtocolPacket
where
    Self: Sized,
{
    fn id(&self) -> u16;

    fn architecture_target(&self) -> Option<u8>;

    fn read(
        id: ProtocolPacketHeader,
        buf: &mut Cursor<&[u8]>,
    ) -> Result<Self, Box<ReadPacketError>>;

    fn write(&self, buf: &mut impl Write) -> Result<(), std::io::Error>;
}

#[derive(Debug, Copy, Clone)]
pub struct ProtocolPacketHeader {
    pub length: u16,
    pub architecture_target: Option<u8>, // only sent by the client
    pub id: u16,
}

impl WakfuBufWritable for ProtocolPacketHeader {
    fn write_into(&self, buf: &mut impl Write) -> Result<(), std::io::Error> {
        self.length.write_into(buf)?;

        if let Some(architecture_target) = self.architecture_target {
            architecture_target.write_into(buf)?;
        }

        self.id.write_into(buf)?;

        Ok(())
    }
}

pub trait ClientboundPacket {
    fn architecture_target(&self) -> u8;
}

pub fn serialize_packet<T: ProtocolPacket>(
    packet: T,
    protocol_adapter: ProtocolAdapter,
) -> Result<Vec<u8>, std::io::Error> {
    let mut data = Vec::new();
    packet.write(&mut data)?;

    let header = ProtocolPacketHeader {
        length: (data.len() + protocol_adapter.get_header_size()) as u16,
        architecture_target: packet.architecture_target(),
        id: packet.id(),
    };

    let mut buffer = Vec::new();
    header.write_into(&mut buffer)?;

    for byte in data {
        byte.write_into(&mut buffer)?;
    }

    Ok(buffer)
}

pub fn deserialize_packet<T: ProtocolPacket>(
    buffer: &mut std::io::Cursor<&[u8]>,
    protocol_adapter: ProtocolAdapter,
) -> Result<T, Box<ReadPacketError>> {
    let header = protocol_adapter
        .read_packet_header(buffer)
        .map_err(|err| ReadPacketError::ReadPacketId { source: err })?;
    T::read(header, buffer)
}
