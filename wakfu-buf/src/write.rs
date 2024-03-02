use byteorder::{WriteBytesExt, BE};
use std::{collections::HashMap, hash::Hash, io::Write};

pub trait WakfuBufWritable {
    fn write_into(&self, buf: &mut impl Write) -> Result<(), std::io::Error>;
}

impl WakfuBufWritable for u64 {
    fn write_into(&self, buf: &mut impl Write) -> Result<(), std::io::Error> {
        WriteBytesExt::write_u64::<BE>(buf, *self)
    }
}

impl WakfuBufWritable for i64 {
    fn write_into(&self, buf: &mut impl Write) -> Result<(), std::io::Error> {
        WriteBytesExt::write_i64::<BE>(buf, *self)
    }
}

impl WakfuBufWritable for u32 {
    fn write_into(&self, buf: &mut impl Write) -> Result<(), std::io::Error> {
        WriteBytesExt::write_u32::<BE>(buf, *self)
    }
}

impl WakfuBufWritable for i32 {
    fn write_into(&self, buf: &mut impl Write) -> Result<(), std::io::Error> {
        WriteBytesExt::write_i32::<BE>(buf, *self)
    }
}

impl WakfuBufWritable for u16 {
    fn write_into(&self, buf: &mut impl Write) -> Result<(), std::io::Error> {
        WriteBytesExt::write_u16::<BE>(buf, *self)
    }
}

impl WakfuBufWritable for i16 {
    fn write_into(&self, buf: &mut impl Write) -> Result<(), std::io::Error> {
        WriteBytesExt::write_i16::<BE>(buf, *self)
    }
}

impl WakfuBufWritable for u8 {
    fn write_into(&self, buf: &mut impl Write) -> Result<(), std::io::Error> {
        WriteBytesExt::write_u8(buf, *self)
    }
}

impl WakfuBufWritable for i8 {
    fn write_into(&self, buf: &mut impl Write) -> Result<(), std::io::Error> {
        WriteBytesExt::write_i8(buf, *self)
    }
}

impl WakfuBufWritable for f64 {
    fn write_into(&self, buf: &mut impl Write) -> Result<(), std::io::Error> {
        WriteBytesExt::write_f64::<BE>(buf, *self)
    }
}

impl WakfuBufWritable for f32 {
    fn write_into(&self, buf: &mut impl Write) -> Result<(), std::io::Error> {
        WriteBytesExt::write_f32::<BE>(buf, *self)
    }
}

impl WakfuBufWritable for bool {
    fn write_into(&self, buf: &mut impl Write) -> Result<(), std::io::Error> {
        let byte = u8::from(*self);
        byte.write_into(buf)
    }
}

impl WakfuBufWritable for String {
    fn write_into(&self, buf: &mut impl Write) -> Result<(), std::io::Error> {
        self.as_bytes().to_vec().write_into(buf)
    }
}

impl WakfuBufWritable for &str {
    fn write_into(&self, buf: &mut impl Write) -> Result<(), std::io::Error> {
        self.as_bytes().to_vec().write_into(buf)
    }
}

impl<T: WakfuBufWritable> WakfuBufWritable for Option<T> {
    fn write_into(&self, buf: &mut impl Write) -> Result<(), std::io::Error> {
        if let Some(s) = self {
            true.write_into(buf)?;
            s.write_into(buf)?;
        } else {
            false.write_into(buf)?;
        }

        Ok(())
    }
}

impl<T: WakfuBufWritable> WakfuBufWritable for Vec<T> {
    fn write_into(&self, buf: &mut impl Write) -> Result<(), std::io::Error> {
        u8::write_into(&(self.len() as u8), buf)?;

        for element in self {
            element.write_into(buf)?;
        }

        Ok(())
    }
}

impl<K: WakfuBufWritable + Eq + Hash, V: WakfuBufWritable + Eq + Hash> WakfuBufWritable
    for HashMap<K, V>
{
    fn write_into(&self, buf: &mut impl Write) -> Result<(), std::io::Error> {
        u8::write_into(&(self.len() as u8), buf)?;

        for (key, value) in self {
            key.write_into(buf)?;
            value.write_into(buf)?;
        }

        Ok(())
    }
}
