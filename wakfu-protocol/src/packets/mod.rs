use std::io::{Cursor, Write};

use wakfu_buf::{WakfuBufReadable, WakfuBufWritable};

use crate::read::ReadPacketError;

pub mod connection;

pub const PACKET_HEADER_SIZE: usize = 5;

pub trait ProtocolPacket
where
    Self: Sized,
{
    fn id(&self) -> u16;

    fn read(id: u16, buf: &mut Cursor<&[u8]>) -> Result<Self, Box<ReadPacketError>>;

    fn write(&self, buf: &mut impl Write) -> Result<(), std::io::Error>;
}

pub struct ProtocolPacketHeader {
    pub length: u16,
    pub architecture_target: Option<u8>, // only sent by the client
    pub id: u16,
}

impl WakfuBufReadable for ProtocolPacketHeader {
    fn read_from(buf: &mut Cursor<&[u8]>) -> Result<Self, wakfu_buf::BufReadError> {
        let length = u16::read_from(buf)?;
        let id = u16::read_from(buf)?;

        Ok(ProtocolPacketHeader {
            length,
            architecture_target: None,
            id,
        })
    }
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
