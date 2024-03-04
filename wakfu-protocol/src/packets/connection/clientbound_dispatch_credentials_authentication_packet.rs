use wakfu_buf::WakfuBuf;
use wakfu_protocol_macros::{clientbound_packet, ClientboundConnectionPacket};

#[derive(Clone, Debug, WakfuBuf, ClientboundConnectionPacket)]
#[clientbound_packet(8)]
pub struct ClientboundDispatchCredentialsAuthenticationPacket {
    pub encrypted_credentials: Vec<u8>,
}
