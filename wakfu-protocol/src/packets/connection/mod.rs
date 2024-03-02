pub mod clientbound_version_packet;
pub mod serverbound_version_result_packet;

use wakfu_protocol_macros::declare_state_packets;

declare_state_packets!(
    ConnectionPacket,
    Clientbound => {
        2: clientbound_version_packet::ClientboundVersionPacket
    },
    Serverbound => {
        7: serverbound_version_result_packet::ServerboundVersionResultPacket
    }
);
