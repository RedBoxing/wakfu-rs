#![feature(error_generic_member_access)]

pub mod read;
pub mod write;

pub use wakfu_buf_macros::*;

pub use read::{BufReadError, WakfuBufReadable};
pub use write::WakfuBufWritable;
