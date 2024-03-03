use std::{
    fs::File,
    io::BufReader,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::Path,
    sync::{Arc, Mutex},
};

use log::{debug, error, info};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
    sync::mpsc,
};
use tokio_rustls::{TlsAcceptor, TlsStream};
use wakfu_protocol::{
    packets::{
        connection::{
            serverbound_client_ip_packet::ServerboundClientIpPacket, ClientboundConnectionPacket,
            ServerboundConnectionPacket,
        },
        ProtocolAdapter, ProtocolPacket, ProtocolPacketHeader,
    },
    Connection,
};

use wakfu_buf::{WakfuBufReadable, WakfuBufWritable};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    debug!("Loading certificates...");

    let certs = rustls_pemfile::certs(&mut BufReader::new(&mut File::open("certs/cert.pem")?))
        .collect::<Result<Vec<_>, _>>()?;
    let key = rustls_pemfile::private_key(&mut BufReader::new(&mut File::open(
        "certs/server.key.pem",
    )?))?
    .unwrap();

    let config = tokio_rustls::rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;

    let acceptor = TlsAcceptor::from(Arc::new(config));

    debug!("Starting TCP server on port 5558...");
    let listener = TcpListener::bind("0.0.0.0:5558").await?;

    debug!("Proxy server listening on port 5558");

    loop {
        let (stream, addr) = listener.accept().await?;
        stream.set_nodelay(true)?;
        debug!("Connection etablished, initializing ssl...");
        let acceptor = acceptor.clone();

        let stream = acceptor.accept(stream).await?;
        tokio::spawn(async move {
            if let Err(e) = handle_connection(TlsStream::Server(stream), addr).await {
                error!("Error occured while handling connection {} : {:?}", addr, e);
            }
        });
    }
}

async fn handle_connection(
    stream: TlsStream<TcpStream>,
    addr: SocketAddr,
) -> Result<(), Box<dyn std::error::Error>> {
    debug!("Handling connection {}", addr);
    let mut conn = Connection::wrap(
        stream,
        wakfu_protocol::packets::ProtocolAdapter::ClientToServer,
    );
    //let mut client = Connection::new("dispatch.platforms.wakfu.com:5558".to_string()).await?;

    Ok(())
}
