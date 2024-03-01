pub mod connection;

pub trait ProtocolPacket
where
    Self: Sized,
{
    fn id(&self) -> u32;
}
