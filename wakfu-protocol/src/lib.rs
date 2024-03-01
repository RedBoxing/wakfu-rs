use tokio::net::{TcpListener, TcpStream};

pub mod packets;

pub trait Connection
where
    Self: Sized,
{
    async fn new(address: String) -> Result<impl Connection, Box<dyn std::error::Error>>;
}

pub struct ClientConnection {}

pub struct ServerConnection {}

impl Connection for ClientConnection {
    async fn new(address: String) -> Result<impl Connection, Box<dyn std::error::Error>> {
        let stream = TcpStream::connect(address).await?;
        stream.set_nodelay(true);

        let (read_stream, write_stream) = stream.into_split();

        Ok(Connection {})
    }
}
