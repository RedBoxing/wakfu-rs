#![feature(error_generic_member_access)]

use std::{io::Cursor, sync::Arc};

use packets::{ProtocolPacket, ProtocolPacketHeader, PACKET_HEADER_SIZE};
use read::ReadPacketError;
use rustls::pki_types;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
    net::TcpStream,
};
use tokio_rustls::{
    client::TlsStream,
    rustls::{
        client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
        SignatureScheme,
    },
    TlsConnector,
};
use wakfu_buf::{WakfuBufReadable, WakfuBufWritable};

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
    read_stream: ReadHalf<TlsStream<TcpStream>>,
    write_stream: WriteHalf<TlsStream<TcpStream>>,
}

impl Connection {
    pub async fn new(address: String) -> Result<Connection, Box<dyn std::error::Error>> {
        let mut store = tokio_rustls::rustls::RootCertStore::empty();
        //store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

        let mut config = tokio_rustls::rustls::ClientConfig::builder()
            .with_root_certificates(store)
            .with_no_client_auth();

        let mut config1 = config.dangerous();
        config1.set_certificate_verifier(Arc::new(NoCertificateVerification {}));

        let connector = TlsConnector::from(Arc::new(config));

        let stream = TcpStream::connect(&address).await?;

        let domain = pki_types::ServerName::try_from("localhost")?;
        let stream = connector.connect(domain, stream).await?;

        let (read_stream, write_stream) = tokio::io::split(stream);

        Ok(Connection {
            read_stream,
            write_stream,
        })
    }

    pub async fn read<T: ProtocolPacket + Sized>(&mut self) -> Result<T, Box<ReadPacketError>> {
        loop {
            let mut buffer = vec![0u8; PACKET_HEADER_SIZE];
            self.read_stream
                .read_exact(&mut buffer)
                .await
                .map_err(|e| ReadPacketError::IoError { source: e })?;

            let mut buffer: Cursor<&[u8]> = Cursor::new(&buffer);
            let header = ProtocolPacketHeader::read_from(&mut buffer)
                .map_err(|e| ReadPacketError::ReadPacketId { source: e })?;

            let mut buffer = vec![0u8; header.length as usize];
            self.read_stream
                .read_exact(&mut buffer)
                .await
                .map_err(|e| ReadPacketError::IoError { source: e })?;

            let mut buffer: Cursor<&[u8]> = Cursor::new(&buffer);
            let packet: T = ProtocolPacket::read(header.id, &mut buffer)?;

            return Ok(packet);
        }
    }

    pub async fn write<T: ProtocolPacket + Sized>(
        &mut self,
        packet: T,
    ) -> Result<(), std::io::Error> {
        let mut data = Vec::new();
        packet.write(&mut data)?;

        let header = ProtocolPacketHeader {
            length: data.len() as u16,
            architecture_target: None,
            id: packet.id(),
        };

        let mut buffer = Vec::new();
        header.write_into(&mut buffer)?;

        for byte in data {
            byte.write_into(&mut buffer)?;
        }

        self.write_stream.write(&buffer).await?;

        Ok(())
    }
}
