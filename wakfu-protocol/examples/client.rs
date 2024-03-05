use clap::Parser;
use log::{debug, error, info};
use rsa::{
    pkcs8::{der::Writer, DecodePublicKey},
    Pkcs1v15Encrypt, RsaPublicKey,
};
use wakfu_buf::WakfuBufWritable;
use wakfu_core::LoginPhase;
use wakfu_protocol::{
    packets::{
        connection::{
            clientbound_dispatch_credentials_authentication_packet::ClientboundDispatchCredentialsAuthenticationPacket,
            clientbound_publickey_request_packet::ClientboundPublicKeyRequestPacket,
            clientbound_version_packet::ClientboundVersionPacket, ServerboundConnectionPacket,
        },
        deserialize_packet, ProtocolAdapter,
    },
    Connection,
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    username: String,

    #[arg(short, long)]
    password: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    env_logger::init();

    let (conn, mut event_receiver) =
        Connection::new_client("dispatch.platforms.wakfu.com:5558".to_string()).await?;

    conn.write_packet(
        ClientboundVersionPacket {
            major: 1,
            minor: 82,
            revision: 3,
            build_version: "-1".to_string(),
        }
        .get(),
    )?;

    conn.write_packet(
        ClientboundPublicKeyRequestPacket {
            server_id: LoginPhase::Dispatch,
        }
        .get(),
    )?;

    loop {
        if let Some(raw_packet) = event_receiver.recv().await {
            let packet = deserialize_packet::<ServerboundConnectionPacket>(
                &mut std::io::Cursor::new(&raw_packet),
                ProtocolAdapter::ServerToClient,
            )?;

            debug!("Received packet: {:?}", packet);

            match packet {
                ServerboundConnectionPacket::ClientIp(packet) => {
                    info!(
                        "Got client ip: {}.{}.{}.{}",
                        packet.ip[0], packet.ip[1], packet.ip[2], packet.ip[3]
                    );
                }
                ServerboundConnectionPacket::VersionResult(packet) => {
                    if packet.accepted {
                        info!("Server accepted version");
                    } else {
                        error!("Server refused version");
                        break;
                    }
                }
                ServerboundConnectionPacket::PublicKey(packet) => {
                    info!("Got public key!");
                    info!("{}", String::from_utf8_lossy(&packet.public_key));
                    let public_key = RsaPublicKey::from_public_key_der(&packet.public_key)?;

                    let mut buffer: Vec<u8> = Vec::new();
                    packet.salt.write_into(&mut buffer)?;

                    let login = args.username.as_bytes();
                    (login.len() as u8).write_into(&mut buffer)?;
                    buffer.write(login)?;

                    let password = args.password.as_bytes();
                    (password.len() as u8).write_into(&mut buffer)?;
                    buffer.write(password)?;

                    let mut rng = rand::thread_rng();
                    let encrypted_data = public_key.encrypt(&mut rng, Pkcs1v15Encrypt, &buffer)?;
                    conn.write_packet(
                        ClientboundDispatchCredentialsAuthenticationPacket {
                            encrypted_credentials: encrypted_data,
                        }
                        .get(),
                    )?;
                }
                ServerboundConnectionPacket::ForceDisconnectionReason(packet) => {
                    info!(
                        "Server forced disconnected us: {}",
                        match packet.reason {
                            79 => "disconnection.spam",
                            7 => "disconnection.timeout",
                            52 => "disconnection.kickedByReco",
                            50 => "disconnection.kickedByAdmin",
                            14 | 17 => "disconnection.accountBanned",
                            49 => "disconnection.bannedByAdmin",
                            4 => "disconnection.architectureNotReady",
                            98 => "disconnection.sessionDestroyed",
                            61 => "disconnection.remoteServerDoesNotAnswer",
                            64 => "disconnection.serverShutdown",
                            9 => "disconnection.invalidPosition",
                            90 => "disconnection.openOfflineFlea",
                            35 => "disconnection.serverError",
                            33 => "disconnection.unknown",
                            63 => "disconnection.synchronisationError",
                            40 => "disconnection.serverFull",
                            53 => "disconnection.timedSessionEnd",
                            _ => "connection.closed",
                        }
                    );

                    debug!("disconnection reason: {}", packet.reason);
                    break;
                }
            };
        }
    }

    Ok(())
}
