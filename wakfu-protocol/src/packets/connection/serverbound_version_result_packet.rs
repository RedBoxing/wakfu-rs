#[derive(Debug, Clone)]
pub struct ServerboundVersionResultPacket {
    pub accepted: bool,
    pub major: u8,
    pub minor: u16,
    pub build_version: String,
}
