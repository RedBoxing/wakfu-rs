use wakfu_buf::WakfuBuf;
use wakfu_protocol_macros::ServerboundConnectionPacket;

#[derive(Debug, Clone, WakfuBuf, ServerboundConnectionPacket)]
pub struct ServerboundClientIpPacket {
    pub ip: [u8; 4],
}
