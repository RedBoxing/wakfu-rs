#[derive(Clone, Debug)]
pub struct ClientboundVersionPacket {
    pub major: u8,
    pub minor: u16,
    pub build_version: String,
}
