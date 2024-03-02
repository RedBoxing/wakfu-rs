use std::error::Error;

use wakfu_protocol::{
    packets::connection::{
        clientbound_version_packet::ClientboundVersionPacket, ServerboundConnectionPacket,
    },
    Connection,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
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

    loop {
        let packet = conn.read::<ServerboundConnectionPacket>().await;
        println!("{:?}", packet);
        break;
    }

    Ok(())
}
