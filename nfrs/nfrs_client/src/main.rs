use bevy::prelude::*;
use clap::Parser;
use lightyear::netcode::Key;
use lightyear::prelude::client::*;
use lightyear::prelude::*;
use nfrs_shared::{CarInput, InputChannel, Player, ProtocolPlugin};
use std::net::{Ipv4Addr, SocketAddr};
use tracing::info;

#[cfg(not(target_arch = "wasm32"))]
use lightyear::prelude::UdpIo;

#[cfg(target_arch = "wasm32")]
use lightyear::prelude::client::WebTransportClientIo;

#[derive(Parser, Debug, Resource)]
#[command(version, about, long_about = None)]
struct Args {
    /// Server IP address
    #[arg(short, long, default_value = "127.0.0.1")]
    ip: String,
}

fn main() {
    // Set up panic hook and logging for WASM
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
        tracing_wasm::set_as_global_default_with_config(
            tracing_wasm::WASMLayerConfigBuilder::new()
                .set_max_level(tracing::Level::INFO)
                .build(),
        );
    }

    let args = Args::parse();

    App::new()
        .insert_resource(args)
        .add_plugins(DefaultPlugins)
        .add_plugins(ClientPlugins::default())
        .add_plugins(ProtocolPlugin)
        .add_systems(Startup, setup_client)
        .add_systems(Update, spawn_cars)
        .add_systems(Update, input_system)
        .add_systems(Update, (handle_connect, handle_disconnect))
        .add_systems(Update, debug_entities)
        .add_observer(debug_player_spawn)
        .run();
}

fn debug_player_spawn(trigger: Trigger<OnAdd, Player>, query: Query<&Player>) {
    if let Ok(player) = query.get(trigger.target()) {
        info!("Client: Player entity spawned! ID: {}", player.client_id);
    }
}

fn handle_connect(query: Query<Entity, Added<Connected>>) {
    for _ in query.iter() {
        info!("Client connected to server!");
    }
}

fn handle_disconnect(mut removals: RemovedComponents<Connected>) {
    for _ in removals.read() {
        info!("Client disconnected from server!");
    }
}

fn setup_client(mut commands: Commands, args: Res<Args>) {
    // Spawn camera
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scale: 0.05,
            ..OrthographicProjection::default_2d()
        }),
    ));

    let client_id = rand::random::<u64>();
    // Use WSL IP to avoid localhost UDP forwarding issues
    let server_addr = SocketAddr::new(Ipv4Addr::new(172, 20, 137, 119).into(), 5001);
    let client_addr = SocketAddr::new(Ipv4Addr::new(0, 0, 0, 0).into(), 0);

    info!(
        "Connecting to server at {} with client id {}",
        server_addr, client_id
    );

    let auth = Authentication::Manual {
        server_addr,
        client_id,
        private_key: Key::default(),
        protocol_id: 0,
    };

    #[cfg(target_arch = "wasm32")]
    let client = commands
        .spawn((
            Client::default(),
            LocalAddr(client_addr),
            PeerAddr(server_addr),
            Link::new(None),
            ReplicationReceiver::default(),
            NetcodeClient::new(auth, NetcodeConfig::default()).unwrap(),
            WebTransportClientIo {
                certificate_digest: String::from(
                    "563e1c873b620196bce6d4baff29210493290918e8e42ae1d8208b07e6121057",
                ),
            }, // WASM uses WebTransport
        ))
        .id();

    #[cfg(not(target_arch = "wasm32"))]
    let client = commands
        .spawn((
            Client::default(),
            LocalAddr(client_addr),
            PeerAddr(server_addr),
            Link::new(None),
            ReplicationReceiver::default(),
            NetcodeClient::new(auth, NetcodeConfig::default()).unwrap(),
            UdpIo::default(), // Native uses UDP
        ))
        .id();

    // Add message sender for inputs
    commands
        .entity(client)
        .insert(MessageSender::<CarInput>::default());

    // Start the link first
    commands.entity(client).trigger(LinkStart);
    // Then start the connection
    commands.entity(client).trigger(Connect);
}

fn input_system(
    mut input_sender: Query<&mut MessageSender<CarInput>>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if let Ok(mut sender) = input_sender.single_mut() {
        let mut input = CarInput::default();

        if keyboard.pressed(KeyCode::KeyW) {
            input.forward = true;
        }
        if keyboard.pressed(KeyCode::KeyS) {
            input.backward = true;
        }
        if keyboard.pressed(KeyCode::KeyA) {
            input.left = true;
        }
        if keyboard.pressed(KeyCode::KeyD) {
            input.right = true;
        }

        // Only send if any key is pressed
        if input.forward || input.backward || input.left || input.right {
            // println!("Sending input: {:?}", input);
            info!("Sending input: {:?}", input);
            sender.send::<InputChannel>(input);
        }
    }
}

fn spawn_cars(
    mut commands: Commands,
    query: Query<(Entity, &Player), Added<Player>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for (entity, _player) in query.iter() {
        info!("Spawning visual representation for player car");
        commands.entity(entity).insert((
            Mesh2d(meshes.add(Rectangle::new(2.0, 4.0))),
            MeshMaterial2d(materials.add(Color::srgb(0.8, 0.2, 0.3))),
            Transform::default(),
        ));
    }
}

fn debug_entities(query: Query<Entity>, player_query: Query<&Player>, time: Res<Time>) {
    // Log every 5 seconds using elapsed_secs as integer
    let elapsed = time.elapsed_secs() as u32;
    if elapsed % 5 == 0 && time.delta_secs() < 0.1 {
        info!(
            "Total entities: {}, Player entities: {}",
            query.iter().count(),
            player_query.iter().count()
        );
    }
}
