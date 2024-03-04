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
        deserialize_packet, ProtocolAdapter, ProtocolPacket, ProtocolPacketHeader,
    },
    Connection, RawConnection,
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
    let client_conn = Connection::wrap(
        stream,
        ProtocolAdapter::ClientToServer,
        ProtocolAdapter::ServerToClient,
    );

    let server_conn = Connection::new("dispatch.platforms.wakfu.com:5558".to_string()).await?;

    let (client_raw_read_connection, client_raw_write_connection) = client_conn.split();
    let (server_raw_read_connection, server_raw_write_connection) = server_conn.split();

    let (client_run_scheduler_sender, mut client_run_scheduler_receiver) =
        mpsc::unbounded_channel::<()>();
    let (server_run_scheduler_sender, mut server_run_scheduler_receiver) =
        mpsc::unbounded_channel::<()>();

    let client_conn = Arc::new(
        RawConnection::new(
            client_run_scheduler_sender,
            client_raw_read_connection,
            client_raw_write_connection,
            client_conn.protocol_adapter,
        )
        .await,
    );
    let server_conn = Arc::new(
        RawConnection::new(
            server_run_scheduler_sender,
            server_raw_read_connection,
            server_raw_write_connection,
            server_conn.protocol_adapter,
        )
        .await,
    );
    let client_conn_1 = client_conn.clone();
    let server_conn_1 = server_conn.clone();

    let client_handler = tokio::spawn(async move {
        loop {
            while let Ok(()) = client_run_scheduler_receiver.try_recv() {}
            client_run_scheduler_receiver.recv().await;

            let packet_queue = client_conn_1.incomming_packets_queue();
            let packet_queue = packet_queue.lock().await;
            if !packet_queue.is_empty() {
                for raw_packet in packet_queue.iter() {
                    match deserialize_packet::<ClientboundConnectionPacket>(
                        &mut std::io::Cursor::new(raw_packet),
                        ProtocolAdapter::ClientToServer,
                    ) {
                        Ok(packet) => {
                            info!("[CLIENT->PROXY] {:?}", packet);
                            if let Err(err) = server_conn_1.write_packet(packet) {
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
        }
    });
    let client_conn = client_conn.clone();
    let server_conn = server_conn.clone();

    let server_handler = tokio::spawn(async move {
        loop {
            while let Ok(()) = server_run_scheduler_receiver.try_recv() {}
            server_run_scheduler_receiver.recv().await;

            let packet_queue = server_conn.incomming_packets_queue();
            let packet_queue = packet_queue.lock().await;
            if !packet_queue.is_empty() {
                for raw_packet in packet_queue.iter() {
                    match deserialize_packet::<ServerboundConnectionPacket>(
                        &mut std::io::Cursor::new(raw_packet),
                        ProtocolAdapter::ServerToClient,
                    ) {
                        Ok(packet) => {
                            info!("[SERVER->PROXY] {:?}", packet);
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
        }
    });

    client_handler.await?;
    server_handler.await?;

    Ok(())
}
