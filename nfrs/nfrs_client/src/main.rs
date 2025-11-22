use bevy::prelude::*;
use bevy::sprite::MaterialMesh2dBundle;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::{
    renet::{
        transport::{ClientAuthentication, NetcodeClientTransport},
        ConnectionConfig, RenetClient,
    },
};
use nfrs_shared::{CarInput, Player, SharedPlugin};
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::SystemTime;

use clap::Parser;

#[derive(Parser, Debug, Resource)]
#[command(version, about, long_about = None)]
struct Args {
    /// Server IP address
    #[arg(short, long, default_value = "127.0.0.1")]
    ip: String,
}

fn main() {
    let args = Args::parse();
    
    App::new()
        .insert_resource(args) // Store args as resource
        .add_plugins((
            DefaultPlugins,
            SharedPlugin,
            bevy_replicon_renet::client::RepliconRenetClientPlugin,
        ))
        .add_systems(Startup, setup_client)
        .add_systems(Update, (input_system, spawn_cars))
        .run();
}

fn setup_client(mut commands: Commands, _network_channels: Res<RepliconChannels>, args: Res<Args>) {
    commands.spawn(Camera2dBundle::default());

    let client = RenetClient::new(ConnectionConfig::default());

    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let client_id = current_time.as_millis() as u64;
    let server_addr = SocketAddr::new(args.ip.parse().expect("Invalid IP address"), 5000);
    let socket = UdpSocket::bind("0.0.0.0:0").expect("Could not bind to socket");
    
    let authentication = ClientAuthentication::Unsecure {
        client_id,
        protocol_id: 0,
        server_addr,
        user_data: None,
    };

    let transport = NetcodeClientTransport::new(current_time, authentication, socket)
        .expect("Could not create transport");

    commands.insert_resource(client);
    commands.insert_resource(transport);
}

fn input_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut car_input_events: EventWriter<CarInput>,
) {
    let mut throttle = 0.0;
    let mut steering = 0.0;

    if keys.pressed(KeyCode::ArrowUp) {
        throttle += 1.0;
    }
    if keys.pressed(KeyCode::ArrowDown) {
        throttle -= 1.0;
    }
    if keys.pressed(KeyCode::ArrowLeft) {
        steering += 1.0;
    }
    if keys.pressed(KeyCode::ArrowRight) {
        steering -= 1.0;
    }

    if throttle != 0.0 || steering != 0.0 {
        car_input_events.send(CarInput { throttle, steering });
    }
}

fn spawn_cars(
    mut commands: Commands,
    query: Query<(Entity, &Player), Added<Player>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for (entity, player) in query.iter() {
        info!("Spawning car for player {}", player.client_id);
        commands.entity(entity).insert(MaterialMesh2dBundle {
            mesh: meshes.add(Rectangle::new(20.0, 40.0)).into(),
            material: materials.add(ColorMaterial::from(Color::srgb(1.0, 0.0, 0.0))),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..Default::default()
        });
    }
}


