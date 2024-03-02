use std::{backtrace::Backtrace, collections::HashMap, hash::Hash, io::Cursor};

use byteorder::{ReadBytesExt, BE};
use thiserror::Error;

use tracing::warn;

#[derive(Error, Debug)]
pub enum BufReadError {
    #[error("Error reading bytes")]
    CouldNotReadBytes,

    #[error("{source}")]
    Io {
        #[from]
        #[backtrace]
        source: std::io::Error,
    },

    #[error("Invalid UTF-8: {bytes:?} (lossy: {lossy:?})")]
    InvalidUtf8 { bytes: Vec<u8>, lossy: String },

    #[error("Tried to read {attempted_read} bytes but there were only {actual_read}")]
    UnexpectedEof {
        attempted_read: usize,
        actual_read: usize,
        backtrace: Backtrace,
    },
}

fn read_bytes<'a>(buf: &'a mut Cursor<&[u8]>, length: usize) -> Result<&'a [u8], BufReadError> {
    if length > (buf.get_ref().len() - buf.position() as usize) {
        return Err(BufReadError::UnexpectedEof {
            attempted_read: length,
            actual_read: buf.get_ref().len() - buf.position() as usize,
            backtrace: Backtrace::capture(),
        });
    }

    let initial_position = buf.position() as usize;
    buf.set_position(buf.position() + length as u64);
    let data = &buf.get_ref()[initial_position..initial_position + length];
    Ok(data)
}

fn read_utf_with_len(buf: &mut Cursor<&[u8]>, length: usize) -> Result<String, BufReadError> {
    let data = read_bytes(buf, length)?;
    let string = std::str::from_utf8(data)
        .map_err(|_| BufReadError::InvalidUtf8 {
            bytes: data.to_vec(),
            lossy: String::from_utf8_lossy(data).to_string(),
        })?
        .to_string();
    Ok(string)
}

pub trait WakfuBufReadable
where
    Self: Sized,
{
    fn read_from(buf: &mut Cursor<&[u8]>) -> Result<Self, BufReadError>;
}

impl WakfuBufReadable for u64 {
    fn read_from(buf: &mut Cursor<&[u8]>) -> Result<Self, BufReadError> {
        Ok(buf.read_u64::<BE>()?)
    }
}

impl WakfuBufReadable for i64 {
    fn read_from(buf: &mut Cursor<&[u8]>) -> Result<Self, BufReadError> {
        Ok(buf.read_i64::<BE>()?)
    }
}

impl WakfuBufReadable for u32 {
    fn read_from(buf: &mut Cursor<&[u8]>) -> Result<Self, BufReadError> {
        Ok(buf.read_u32::<BE>()?)
    }
}

impl WakfuBufReadable for i32 {
    fn read_from(buf: &mut Cursor<&[u8]>) -> Result<Self, BufReadError> {
        Ok(buf.read_i32::<BE>()?)
    }
}

impl WakfuBufReadable for u16 {
    fn read_from(buf: &mut Cursor<&[u8]>) -> Result<Self, BufReadError> {
        Ok(buf.read_u16::<BE>()?)
    }
}

impl WakfuBufReadable for i16 {
    fn read_from(buf: &mut Cursor<&[u8]>) -> Result<Self, BufReadError> {
        Ok(buf.read_i16::<BE>()?)
    }
}

impl WakfuBufReadable for u8 {
    fn read_from(buf: &mut Cursor<&[u8]>) -> Result<Self, BufReadError> {
        Ok(buf.read_u8()?)
    }
}

impl WakfuBufReadable for i8 {
    fn read_from(buf: &mut Cursor<&[u8]>) -> Result<Self, BufReadError> {
        Ok(buf.read_i8()?)
    }
}

impl WakfuBufReadable for f64 {
    fn read_from(buf: &mut Cursor<&[u8]>) -> Result<Self, BufReadError> {
        Ok(buf.read_f64::<BE>()?)
    }
}

impl WakfuBufReadable for f32 {
    fn read_from(buf: &mut Cursor<&[u8]>) -> Result<Self, BufReadError> {
        Ok(buf.read_f32::<BE>()?)
    }
}

impl WakfuBufReadable for bool {
    fn read_from(buf: &mut Cursor<&[u8]>) -> Result<Self, BufReadError> {
        let byte = u8::read_from(buf)?;
        if byte > 1 {
            warn!("Boolean value was not 0 or 1 but {}", byte);
        }

        Ok(byte != 0)
    }
}

impl WakfuBufReadable for String {
    fn read_from(buf: &mut Cursor<&[u8]>) -> Result<Self, BufReadError> {
        let length = u8::read_from(buf)?;
        let string = read_utf_with_len(buf, length as usize)?;
        Ok(string)
    }
}

impl<T: WakfuBufReadable> WakfuBufReadable for Vec<T> {
    fn read_from(buf: &mut Cursor<&[u8]>) -> Result<Self, BufReadError> {
        let length = u8::read_from(buf)? as usize;
        let mut contents = Vec::with_capacity(usize::min(length, 65536));

        for _ in 0..length {
            contents.push(T::read_from(buf)?);
        }

        Ok(contents)
    }
}

impl<T: WakfuBufReadable, const N: usize> WakfuBufReadable for [T; N] {
    fn read_from(buf: &mut Cursor<&[u8]>) -> Result<Self, BufReadError> {
        let mut contents = Vec::with_capacity(N);

        for _ in 0..N {
            contents.push(T::read_from(buf)?);
        }

        contents.try_into().map_err(|_| {
            unreachable!("Panic is not possible since the Vec is the same size as the array")
        })
    }
}

impl<K: WakfuBufReadable + Eq + Hash, V: WakfuBufReadable + Eq + Hash> WakfuBufReadable
    for HashMap<K, V>
{
    fn read_from(buf: &mut Cursor<&[u8]>) -> Result<Self, BufReadError> {
        let length = u8::read_from(buf)? as usize;
        let mut contents = HashMap::with_capacity(usize::min(length, 65536));

        for _ in 0..length {
            contents.insert(K::read_from(buf)?, V::read_from(buf)?);
        }

        Ok(contents)
    }
}

impl<T: WakfuBufReadable> WakfuBufReadable for Option<T> {
    fn read_from(buf: &mut Cursor<&[u8]>) -> Result<Self, BufReadError> {
        let present = bool::read_from(buf)?;
        Ok(if present {
            Some(T::read_from(buf)?)
        } else {
            None
        })
    }
}
