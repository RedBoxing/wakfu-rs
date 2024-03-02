use std::backtrace::Backtrace;
use thiserror::Error;
use wakfu_buf::BufReadError;

#[derive(Error, Debug)]
pub enum ReadPacketError {
    #[error("Error reading packet {packet_name} (id {packet_id}): {source}")]
    Parse {
        packet_id: u16,
        packet_name: String,
        backtrace: Box<Backtrace>,
        source: BufReadError,
    },
    #[error("Unknown packet id {id} in state {state_name}")]
    UnknownPacketId { state_name: String, id: u16 },
    #[error("Couldn't read packet id")]
    ReadPacketId { source: BufReadError },
    #[error("Leftover data after reading packet {packet_name}: {data:?}")]
    LeftoverData { data: Vec<u8>, packet_name: String },
    #[error(transparent)]
    IoError {
        #[from]
        #[backtrace]
        source: std::io::Error,
    },
    #[error("Connection closed")]
    ConnectionClosed,
}
