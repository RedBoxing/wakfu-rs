use wakfu_buf::WakfuBuf;
use wakfu_protocol_macros::{clientbound_packet, ClientboundConnectionPacket};

#[derive(Clone, Debug, WakfuBuf, ClientboundConnectionPacket)]
#[clientbound_packet(0)]
pub struct ClientboundVersionPacket {
    pub major: u8,
    pub minor: u16,
    pub revision: u8,
    pub build_version: String,
}
