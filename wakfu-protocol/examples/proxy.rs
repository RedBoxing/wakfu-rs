use std::{fs::File, io::BufReader, net::SocketAddr, sync::Arc};

use clap::Parser;
use log::{debug, error, info};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{TlsAcceptor, TlsStream};
use wakfu_protocol::{
    packets::{
        connection::{ClientboundConnectionPacket, ServerboundConnectionPacket},
        deserialize_packet, ProtocolAdapter,
    },
    Connection, RawConnection,
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short = 'H', long, default_value = "0.0.0.0")]
    host: String,

    #[arg(short, long, default_value_t = 5558)]
    port: u16,

    #[arg(short, long, default_value = "certs/cert.pem")]
    cert: String,

    #[arg(short, long, default_value = "certs/server.key.pem")]
    key: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    env_logger::init();

    debug!("Loading certificates...");

    let certs = rustls_pemfile::certs(&mut BufReader::new(&mut File::open(args.cert)?))
        .collect::<Result<Vec<_>, _>>()?;
    let key =
        rustls_pemfile::private_key(&mut BufReader::new(&mut File::open(args.key)?))?.unwrap();

    let config = tokio_rustls::rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;

    let acceptor = TlsAcceptor::from(Arc::new(config));

    let listener = TcpListener::bind(format!("{}:{}", args.host, args.port)).await?;

    info!("Proxy server listening on port {}", args.port);

    loop {
        let (stream, addr) = listener.accept().await?;
        stream.set_nodelay(true)?;
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
    info!("Handling connection {}", addr);

    let (client_conn, mut client_event_receiver) = Connection::from_raw(RawConnection::wrap(
        stream,
        ProtocolAdapter::ClientToServer,
        ProtocolAdapter::ServerToClient,
    ));

    let (server_conn, mut server_event_receiver) =
        Connection::new_client("dispatch.platforms.wakfu.com:5558".to_string()).await?;

    let client_handler = tokio::spawn(async move {
        loop {
            if let Some(raw_packet) = client_event_receiver.recv().await {
                match deserialize_packet::<ClientboundConnectionPacket>(
                    &mut std::io::Cursor::new(&raw_packet),
                    ProtocolAdapter::ClientToServer,
                ) {
                    Ok(packet) => {
                        info!("[CLIENT->SERVER] {:?}", packet);
                        if let Err(err) = server_conn.write_packet(packet) {
                            error!("Failed to write packet to server : {}", err);
                            break;
                        }
                    }
                    Err(err) => {
                        error!("Failed to deserialize packet : {}", err);
                        break;
                    }
                }
            }
        }
    });

    let server_handler = tokio::spawn(async move {
        loop {
            if let Some(raw_packet) = server_event_receiver.recv().await {
                match deserialize_packet::<ServerboundConnectionPacket>(
                    &mut std::io::Cursor::new(&raw_packet),
                    ProtocolAdapter::ServerToClient,
                ) {
                    Ok(packet) => {
                        info!("[SERVER->CLIENT] {:?}", packet);
                        if let Err(err) = client_conn.write_packet(packet) {
                            error!("Failed to write packet to client : {}", err);
                            break;
                        }
                    }
                    Err(err) => {
                        error!("Failed to deserialize packet : {}", err);
                        break;
                    }
                }
            }
        }
    });

    client_handler.await?;
    server_handler.await?;

    Ok(())
}
