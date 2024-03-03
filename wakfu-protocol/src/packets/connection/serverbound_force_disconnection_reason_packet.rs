use wakfu_buf::WakfuBuf;
use wakfu_protocol_macros::ServerboundConnectionPacket;

#[derive(Debug, Clone, WakfuBuf, ServerboundConnectionPacket)]
pub struct ServerboundForceDisconnectionReasonPacket {
    pub reason: u8,
}
