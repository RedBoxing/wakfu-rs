use wakfu_buf::WakfuBuf;
use wakfu_protocol_macros::ServerboundConnectionPacket;

#[derive(Debug, Clone, WakfuBuf, ServerboundConnectionPacket)]
pub struct ServerboundVersionResultPacket {
    pub accepted: bool,
    pub major: u8,
    pub minor: u16,
    pub revision: u8,
    pub build_version: String,
}
