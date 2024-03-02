use wakfu_buf::{WakfuBufReadable, WakfuBufWritable};

#[derive(Clone, Debug)]
pub struct ClientboundVersionPacket {
    pub major: u8,
    pub minor: u16,
    pub build_version: String,
}

impl WakfuBufReadable for ClientboundVersionPacket {
    fn read_from(buf: &mut std::io::Cursor<&[u8]>) -> Result<Self, wakfu_buf::BufReadError> {
        let major = u8::read_from(buf)?;
        let minor = i16::read_from(buf)?;
        let build_version = String::read_from(buf)?;

        Ok(ClientboundVersionPacket {
            major,
            minor,
            build_version,
        })
    }
}

impl WakfuBufWritable for ClientboundVersionPacket {
    fn write_into(&self, buf: &mut impl std::io::prelude::Write) -> Result<(), std::io::Error> {
        self.major.write_into(buf)?;
        self.minor.write_into(buf)?;
        self.build_version.write_into(buf)?;

        Ok(())
    }
}
