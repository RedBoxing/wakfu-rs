use std::error::Error;

use enumeration::{enumerate, Enumeration};
use log::{debug, error, info};
use tokio::sync::mpsc;
use wakfu_buf::WakfuBufWritable;
use wakfu_protocol::{
    packets::{
        connection::{
            clientbound_publickey_request_packet::ClientboundPublicKeyRequestPacket,
            clientbound_version_packet::ClientboundVersionPacket, ClientboundConnectionPacket,
            ServerboundConnectionPacket,
        },
        deserialize_packet, ClientboundPacket, ProtocolAdapter, ProtocolPacket,
        ProtocolPacketHeader,
    },
    Connection, ConnectionReader, ConnectionWriter, RawConnection, RawReadConnection,
    RawWriteConnection,
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

    let conn = Connection::new("dispatch.platforms.wakfu.com:5558".to_string()).await?;
    let (raw_read_connection, raw_write_connection) = conn.split();

    let (run_schedule_sender, mut run_schedule_receiver) = mpsc::unbounded_channel::<()>();

    let conn = RawConnection::new(
        run_schedule_sender,
        raw_read_connection,
        raw_write_connection,
        conn.protocol_adapter,
    )
    .await;

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
            server_id: *LoginPhase::Dispatch.value(),
        }
        .get(),
    )?;

    loop {
        while let Ok(()) = run_schedule_receiver.try_recv() {}
        run_schedule_receiver.recv().await;
        let packet_queue = conn.incomming_packets_queue();
        let packets_queue = packet_queue.lock().await;
        if !packets_queue.is_empty() {
            for raw_packet in packets_queue.iter() {
                let packet = deserialize_packet::<ServerboundConnectionPacket>(
                    &mut std::io::Cursor::new(raw_packet),
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
        }
    }

    //Ok(())
}
