use std::error::Error;

use enumeration::{enumerate, Enumeration};
use log::{debug, error, info};
use wakfu_protocol::{
    packets::{
        connection::{
            clientbound_publickey_request_packet::ClientboundPublicKeyRequestPacket,
            clientbound_version_packet::ClientboundVersionPacket, ClientboundConnectionPacket,
            ServerboundConnectionPacket,
        },
        ClientboundPacket,
    },
    Connection,
};

enumerate!(pub LoginPhase(u8; u8)
    SteamLink = 8
    Dispatch = 8
    Connection = 1
    Connected = 0
);

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let mut conn = Connection::new("dispatch.platforms.wakfu.com:5558".to_string()).await?;

    conn.write(
        ClientboundVersionPacket {
            major: 1,
            minor: 82,
            revision: 3,
            build_version: "-1".to_string(),
        }
        .get(),
    )
    .await?;

    conn.write(
        ClientboundPublicKeyRequestPacket {
            server_id: *LoginPhase::Dispatch.value(),
        }
        .get(),
    )
    .await?;

    loop {
        let packet = conn.read::<ServerboundConnectionPacket>().await?;
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
            }
        }
    }

    Ok(())
}
