use wakfu_buf::{WakfuBufReadable, WakfuBufWritable};
use wakfu_protocol_macros::ServerboundConnectionPacket;

#[derive(Debug, Clone, ServerboundConnectionPacket)]
pub struct ServerboundPublicKeyPacket {
    pub salt: i64,
    pub public_key: Vec<u8>,
}

impl WakfuBufReadable for ServerboundPublicKeyPacket {
    fn read_from(buf: &mut std::io::Cursor<&[u8]>) -> Result<Self, wakfu_buf::BufReadError> {
        let salt = i64::read_from(buf)?;
        let public_key = buf.remaining_slice().to_vec();
        buf.set_position(buf.position() + public_key.len() as u64);

        Ok(ServerboundPublicKeyPacket { salt, public_key })
    }
}

impl WakfuBufWritable for ServerboundPublicKeyPacket {
    fn write_into(&self, buf: &mut impl std::io::prelude::Write) -> Result<(), std::io::Error> {
        self.salt.write_into(buf)?;
        buf.write(&self.public_key)?;

        Ok(())
    }
}
