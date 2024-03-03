use crate::packets::ClientboundPacket;
use wakfu_buf::WakfuBuf;
use wakfu_protocol_macros::ClientboundConnectionPacket;

#[derive(Clone, Debug, WakfuBuf, ClientboundConnectionPacket)]
pub struct ClientboundPublicKeyRequestPacket {
    #[ignore]
    pub server_id: u8,
}

impl ClientboundPacket for ClientboundPublicKeyRequestPacket {
    fn architecture_target(&self) -> u8 {
        self.server_id
    }
}
