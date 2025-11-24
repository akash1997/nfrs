use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use lightyear::prelude::server::*;
use lightyear::prelude::*;
use nfrs_shared::ProtocolPlugin;
use std::net::{Ipv4Addr, SocketAddr};
use tracing_subscriber::FmtSubscriber;
use wtransport::Identity;

mod car;

fn main() {
    // Setup logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let mut app = App::new();

    app.add_plugins((
        MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
            std::time::Duration::from_secs_f64(1.0 / 60.0),
        )),
        RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0),
        ServerPlugins::default(),
        ProtocolPlugin,
        car::CarPlugin,
    ));

    app.add_systems(Startup, start_server);

    app.run();
}

fn start_server(mut commands: Commands) {
    let server_addr = SocketAddr::new(Ipv4Addr::new(0, 0, 0, 0).into(), 5000);

    // Load certificate
    let cert_path = std::path::Path::new("cert.pem");
    let key_path = std::path::Path::new("key.pem");

    let rt = tokio::runtime::Runtime::new().unwrap();
    let certificate = rt
        .block_on(async { Identity::load_pemfiles(cert_path, key_path).await })
        .expect("Failed to load certificate files. Make sure cert.pem and key.pem exist.");

    let digest = certificate.certificate_chain().as_slice()[0].hash();
    println!("Certificate Digest: {}", digest);

    // UDP Server
    let udp_addr = SocketAddr::new(Ipv4Addr::new(0, 0, 0, 0).into(), 5000);
    let udp_server = commands
        .spawn((
            NetcodeServer::new(NetcodeConfig::default()),
            LocalAddr(udp_addr),
            ServerUdpIo::default(),
        ))
        .id();
    commands.entity(udp_server).trigger(LinkStart);
    commands.entity(udp_server).trigger(Start);

    // WebTransport Server
    let wt_addr = SocketAddr::new(Ipv4Addr::new(0, 0, 0, 0).into(), 5001);
    let wt_server = commands
        .spawn((
            NetcodeServer::new(NetcodeConfig::default()),
            LocalAddr(wt_addr),
            WebTransportServerIo { certificate },
        ))
        .id();
    commands.entity(wt_server).trigger(LinkStart);
    commands.entity(wt_server).trigger(Start);
}

// fn configure_physics(mut rapier_config: ResMut<RapierConfiguration>) {
//     rapier_config.gravity = Vec2::ZERO;
// }
