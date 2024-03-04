#![feature(error_generic_member_access)]
#![feature(cursor_remaining)]

use std::{io::Cursor, sync::Arc};

use log::{debug, error};
use packets::{
    deserialize_packet, serialize_packet, ProtocolAdapter, ProtocolPacket, ProtocolPacketWithHeader,
};
use read::ReadPacketError;
use rustls::pki_types;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
    net::TcpStream,
    sync::{mpsc, Mutex},
};
use tokio_rustls::{
    rustls::{
        client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
        SignatureScheme,
    },
    TlsConnector, TlsStream,
};

pub mod packets;
pub mod read;

#[derive(Debug)]
pub struct NoCertificateVerification {}

impl ServerCertVerifier for NoCertificateVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls_pki_types::CertificateDer<'_>,
        _intermediates: &[rustls_pki_types::CertificateDer<'_>],
        _server_name: &rustls_pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls_pki_types::UnixTime,
    ) -> Result<tokio_rustls::rustls::client::danger::ServerCertVerified, tokio_rustls::rustls::Error>
    {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls_pki_types::CertificateDer<'_>,
        _dss: &tokio_rustls::rustls::DigitallySignedStruct,
    ) -> Result<
        tokio_rustls::rustls::client::danger::HandshakeSignatureValid,
        tokio_rustls::rustls::Error,
    > {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls_pki_types::CertificateDer<'_>,
        _dss: &tokio_rustls::rustls::DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, tokio_rustls::rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<tokio_rustls::rustls::SignatureScheme> {
        let mut vec = Vec::new();
        vec.push(SignatureScheme::RSA_PSS_SHA256);

        vec
    }
}

pub struct Connection {
    pub protocol_adapter: ProtocolAdapter,

    raw_reader: RawReadConnection,
    raw_writer: RawWriteConnection,
}

impl Connection {
    pub async fn new(address: String) -> Result<Connection, Box<dyn std::error::Error>> {
        debug!("Connecting to {}", address);

        let store = tokio_rustls::rustls::RootCertStore::empty();

        let mut config = tokio_rustls::rustls::ClientConfig::builder()
            .with_root_certificates(store)
            .with_no_client_auth();

        let mut config1 = config.dangerous();
        config1.set_certificate_verifier(Arc::new(NoCertificateVerification {}));

        let connector = TlsConnector::from(Arc::new(config));

        let stream = TcpStream::connect(&address).await?;

        let domain = pki_types::ServerName::try_from("localhost")?;
        let stream = connector.connect(domain, stream).await?;
        let stream = TlsStream::Client(stream);

        let (read_stream, write_stream) = tokio::io::split(stream);

        let raw_reader = RawReadConnection {
            read_stream: Arc::new(Mutex::new(read_stream)),
            protocol_adapter: ProtocolAdapter::ServerToClient,
        };

        let raw_writer = RawWriteConnection {
            write_stream: Arc::new(Mutex::new(write_stream)),
            protocol_adapter: ProtocolAdapter::ClientToServer,
        };

        Ok(Connection {
            protocol_adapter: ProtocolAdapter::ClientToServer,
            raw_reader,
            raw_writer,
        })
    }

    pub fn split(&self) -> (RawReadConnection, RawWriteConnection) {
        (self.raw_reader.clone(), self.raw_writer.clone())
    }

    pub fn wrap(
        stream: TlsStream<TcpStream>,
        read_protocol_adapter: ProtocolAdapter,
        write_protocol_adapter: ProtocolAdapter,
    ) -> Connection {
        let (read_stream, write_stream) = tokio::io::split(stream);

        Connection {
            protocol_adapter: read_protocol_adapter,
            raw_reader: RawReadConnection {
                read_stream: Arc::new(Mutex::new(read_stream)),
                protocol_adapter: read_protocol_adapter,
            },
            raw_writer: RawWriteConnection {
                write_stream: Arc::new(Mutex::new(write_stream)),
                protocol_adapter: write_protocol_adapter,
            },
        }
    }

    pub async fn read<T: ProtocolPacket>(
        &mut self,
    ) -> Result<ProtocolPacketWithHeader<T>, Box<ReadPacketError>> {
        let raw_packet = self.raw_reader.read().await?;
        deserialize_packet::<T>(
            &mut std::io::Cursor::new(&raw_packet),
            self.protocol_adapter,
        )
    }

    pub async fn write<T: ProtocolPacket>(&mut self, packet: T) -> Result<(), std::io::Error> {
        self.raw_writer.write(packet).await
    }
}

#[derive(Clone)]
pub struct RawReadConnection {
    pub read_stream: Arc<Mutex<ReadHalf<TlsStream<TcpStream>>>>,
    pub protocol_adapter: ProtocolAdapter,
}

#[derive(Clone)]
pub struct RawWriteConnection {
    pub write_stream: Arc<Mutex<WriteHalf<TlsStream<TcpStream>>>>,
    pub protocol_adapter: ProtocolAdapter,
}

impl RawReadConnection {
    pub async fn read(&mut self) -> Result<Vec<u8>, Box<ReadPacketError>> {
        let mut read_stream = self.read_stream.lock().await;
        let mut header_buffer = vec![0u8; self.protocol_adapter.get_header_size()];
        read_stream
            .read_exact(&mut header_buffer)
            .await
            .map_err(|e| ReadPacketError::IoError { source: e })?;

        let mut buffer: Cursor<&[u8]> = Cursor::new(&header_buffer);

        let header = self
            .protocol_adapter
            .read_packet_header(&mut buffer)
            .map_err(|e| ReadPacketError::ReadPacketId { source: e })?;

        let mut buffer =
            vec![0u8; header.length as usize - self.protocol_adapter.get_header_size()];
        read_stream
            .read_exact(&mut buffer)
            .await
            .map_err(|e| ReadPacketError::IoError { source: e })?;

        header_buffer.append(&mut buffer);

        Ok(header_buffer)
    }
}

impl RawWriteConnection {
    pub async fn write<T: ProtocolPacket>(&mut self, packet: T) -> Result<(), std::io::Error> {
        let buffer = serialize_packet(packet, self.protocol_adapter)?;
        self.write_stream.lock().await.write(&buffer).await?;
        Ok(())
    }

    pub async fn write_raw(&mut self, raw_packet: Vec<u8>) -> Result<(), std::io::Error> {
        self.write_stream.lock().await.write(&raw_packet).await?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct ConnectionReader {
    pub incomming_packets_queue: Arc<Mutex<Vec<Vec<u8>>>>,
    pub run_scheduler_sender: mpsc::UnboundedSender<()>,
}

#[derive(Clone)]
pub struct ConnectionWriter {
    pub outgoing_packets_sender: mpsc::UnboundedSender<Vec<u8>>,
}

impl ConnectionReader {
    pub async fn read_task(self, mut reader: RawReadConnection) {
        loop {
            match reader.read().await {
                Ok(raw_packet) => {
                    self.incomming_packets_queue.lock().await.push(raw_packet);
                    if self.run_scheduler_sender.send(()).is_err() {
                        break;
                    }
                }
                Err(e) => {
                    error!("Failed to read packet : {}", e);
                    break;
                }
            }
        }
    }
}

impl ConnectionWriter {
    pub async fn write_task(
        self,
        mut writer: RawWriteConnection,
        mut outgoing_packets_receiver: mpsc::UnboundedReceiver<Vec<u8>>,
    ) {
        while let Some(raw_packet) = outgoing_packets_receiver.recv().await {
            if let Err(e) = writer.write_raw(raw_packet).await {
                error!("Faied to write packet : {}", e);
                break;
            }
        }
    }
}

pub struct RawConnection {
    protocol_adapter: ProtocolAdapter,

    reader: ConnectionReader,
    writer: ConnectionWriter,

    read_packets_task: tokio::task::JoinHandle<()>,
    write_packets_task: tokio::task::JoinHandle<()>,
}

impl RawConnection {
    pub async fn new(
        run_scheduler_sender: mpsc::UnboundedSender<()>,
        raw_read_connection: RawReadConnection,
        raw_write_connection: RawWriteConnection,
        protocol_adapter: ProtocolAdapter,
    ) -> RawConnection {
        let (outgoing_packets_sender, outgoing_packets_receiver) =
            mpsc::unbounded_channel::<Vec<u8>>();
        let incomming_packets_queue = Arc::new(Mutex::new(Vec::new()));

        let reader = ConnectionReader {
            incomming_packets_queue: incomming_packets_queue.clone(),
            run_scheduler_sender,
        };

        let writer = ConnectionWriter {
            outgoing_packets_sender,
        };

        let read_packets_task = tokio::spawn(reader.clone().read_task(raw_read_connection));
        let write_packets_task = tokio::spawn(
            writer
                .clone()
                .write_task(raw_write_connection, outgoing_packets_receiver),
        );

        RawConnection {
            protocol_adapter,
            reader,
            writer,
            read_packets_task,
            write_packets_task,
        }
    }

    pub fn write_packet<T: ProtocolPacket + std::fmt::Debug>(
        &self,
        packet: T,
    ) -> Result<(), Box<dyn std::error::Error>> {
        debug!("Writing packet : {:?}", packet);
        let buffer = serialize_packet(packet, self.protocol_adapter)?;
        self.writer.outgoing_packets_sender.send(buffer)?;
        Ok(())
    }

    pub fn incomming_packets_queue(&self) -> Arc<Mutex<Vec<Vec<u8>>>> {
        self.reader.incomming_packets_queue.clone()
    }
}
