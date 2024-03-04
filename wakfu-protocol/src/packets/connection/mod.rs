pub mod clientbound_publickey_request_packet;
pub mod clientbound_version_packet;

pub mod serverbound_client_ip_packet;
pub mod serverbound_force_disconnection_reason_packet;
pub mod serverbound_publickey_packet;
pub mod serverbound_version_result_packet;

use wakfu_protocol_macros::declare_state_packets;

declare_state_packets!(
    ConnectionPacket,
    Clientbound => {
        2: clientbound_version_packet::ClientboundVersionPacket,
        500: clientbound_publickey_request_packet::ClientboundPublicKeyRequestPacket
    },
    Serverbound => {
        7: serverbound_version_result_packet::ServerboundVersionResultPacket,
        358: serverbound_client_ip_packet::ServerboundClientIpPacket,
        11: serverbound_force_disconnection_reason_packet::ServerboundForceDisconnectionReasonPacket,
        412: serverbound_publickey_packet::ServerboundPublicKeyPacket
    }
);
