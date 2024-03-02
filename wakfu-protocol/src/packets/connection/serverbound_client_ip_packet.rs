use wakfu_buf::WakfuBuf;
use wakfu_protocol_macros::ServerboundConnectionPacket;

#[derive(Debug, Clone, WakfuBuf, ServerboundConnectionPacket)]
pub struct ServerboundClientIpPacket {
    ip: [u8; 4],
}
