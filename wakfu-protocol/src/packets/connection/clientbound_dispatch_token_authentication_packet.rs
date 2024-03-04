use wakfu_buf::WakfuBuf;
use wakfu_protocol_macros::{clientbound_packet, ClientboundConnectionPacket};

#[derive(Clone, Debug, WakfuBuf, ClientboundConnectionPacket)]
#[clientbound_packet(8)]
pub struct ClientboundDispatchTokenAuthenticationPacket {
    pub encrypted_token: Vec<u8>,
}
