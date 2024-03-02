use wakfu_buf::{WakfuBufReadable, WakfuBufWritable};

#[derive(Debug, Clone)]
pub struct ServerboundVersionResultPacket {
    pub accepted: bool,
    pub major: u8,
    pub minor: u16,
    pub build_version: String,
}

impl WakfuBufReadable for ServerboundVersionResultPacket {
    fn read_from(buf: &mut std::io::Cursor<&[u8]>) -> Result<Self, wakfu_buf::BufReadError> {
        let accepted = bool::read_from(buf)?;
        let major = u8::read_from(buf)?;
        let minor = u16::read_from(buf)?;
        let build_version = String::read_from(buf)?;

        Ok(ServerboundVersionResultPacket {
            accepted,
            major,
            minor,
            build_version,
        })
    }
}

impl WakfuBufWritable for ServerboundVersionResultPacket {
    fn write_into(&self, buf: &mut impl std::io::prelude::Write) -> Result<(), std::io::Error> {
        self.accepted.write_into(buf)?;
        self.major.write_into(buf)?;
        self.minor.write_into(buf)?;
        self.build_version.write_into(buf)?;

        Ok(())
    }
}
