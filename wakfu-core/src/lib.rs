pub struct LoginPhase(u8);

impl LoginPhase {
    pub const SteamLink: u8 = 8;
    pub const Dispatch: u8 = 8;
    pub const Connection: u8 = 1;
    pub const Connected: u8 = 0;
}
