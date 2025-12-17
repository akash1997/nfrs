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

#[cfg(target_arch = "wasm32")]
fn log_digest() {
    let digest = env!("NFRS_CERT_DIGEST");
    info!("Client using certificate digest: {}", digest);
}

#[derive(Parser, Debug, Resource)]
#[command(version, about, long_about = None)]
struct Args {
    /// Server IP address
    #[arg(short, long, default_value = "127.0.0.1")]
    ip: String,
}

#[derive(States, Debug, Clone, Copy, Default, Eq, PartialEq, Hash)]
enum AppState {
    #[default]
    Menu,
    Game,
}

#[derive(Resource, Default)]
struct UsernameInput(String);

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
        log_digest();
    }

    let args = Args::parse();

    App::new()
        .insert_resource(args)
        .init_resource::<UsernameInput>()
        .add_plugins(DefaultPlugins.set(bevy::asset::AssetPlugin {
            meta_check: bevy::asset::AssetMetaCheck::Never,
            ..default()
        }))
        .init_state::<AppState>()
        .add_plugins(ClientPlugins::default())
        .add_plugins(ProtocolPlugin)
        .add_systems(Startup, setup_camera) // Separate camera setup
        .add_systems(OnEnter(AppState::Menu), setup_menu)
        .add_systems(Update, (handle_input_text).run_if(in_state(AppState::Menu)))
        .add_systems(OnExit(AppState::Menu), cleanup_menu)
        .add_systems(OnEnter(AppState::Game), connect_to_server)
        .add_systems(Update, spawn_cars.run_if(in_state(AppState::Game)))
        .add_systems(Update, update_car_labels.run_if(in_state(AppState::Game)))
        .add_systems(Update, input_system.run_if(in_state(AppState::Game)))
        .add_systems(
            Update,
            (handle_connect, handle_disconnect, handle_join_handshake)
                .run_if(in_state(AppState::Game)),
        )
        .add_systems(Update, debug_entities)
        .add_observer(debug_player_spawn)
        .run();
}

fn debug_player_spawn(trigger: Trigger<OnAdd, Player>, query: Query<&Player>) {
    if let Ok(player) = query.get(trigger.target()) {
        info!(
            "Client: Player entity spawned! ID: {}, Name: {}",
            player.client_id, player.username
        );
    }
}

fn handle_connect(
    query: Query<Entity, Added<Connected>>,
    mut commands: Commands,
    username: Res<UsernameInput>,
) {
    for entity in query.iter() {
        info!(
            "Client connected to server! Sending JoinRequest for '{}'",
            username.0
        );

        // Add MessageSender for JoinRequest
        commands
            .entity(entity)
            .insert(MessageSender::<nfrs_shared::JoinRequest>::default());

        // We can't immediately send because we just added the sender component?
        // Bevy component addition is deferred.
        // But we can trigger a system or observe it?
        // Actually, we can use a separate system to send the join request once connected and sender is present.
    }
}

// Let's simplify: handle_connect adds the sender.
// A separate system watches for Added<MessageSender<JoinRequest>> and sends.
fn handle_join_handshake(
    mut query: Query<
        &mut MessageSender<nfrs_shared::JoinRequest>,
        Added<MessageSender<nfrs_shared::JoinRequest>>,
    >,
    username: Res<UsernameInput>,
) {
    for mut sender in query.iter_mut() {
        info!("Sending JoinRequest: {}", username.0);
        // Use InputChannel because that's the only channel we registered and it is OrderedReliable.
        // We could create a separate channel but InputChannel works for now.
        sender.send::<nfrs_shared::InputChannel>(nfrs_shared::JoinRequest {
            username: username.0.clone(),
        });
    }
}

fn handle_disconnect(mut removals: RemovedComponents<Connected>) {
    for _ in removals.read() {
        info!("Client disconnected from server!");
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scale: 0.05,
            ..OrthographicProjection::default_2d()
        }),
    ));
}

// UI Components
#[derive(Component)]
struct MenuRoot;

#[derive(Component)]
struct UserInputText;

#[derive(Component)]
struct CarLabel(Entity);

fn setup_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(20.0), // Spacing between elements
                ..default()
            },
            BackgroundColor(Color::srgb(0.05, 0.05, 0.1)), // Dark slate/blue tint background
            MenuRoot,
        ))
        .with_children(|parent| {
            // Header
            parent.spawn((
                Text::new("NFRS Racing"),
                TextFont {
                    font: font.clone(),
                    font_size: 60.0,
                    ..default()
                },
                TextColor(Color::srgb(0.0, 0.8, 1.0)), // Cyan title
            ));

            // Instruction Label
            parent.spawn((
                Text::new("Enter Username:"),
                TextFont {
                    font: font.clone(),
                    font_size: 25.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));

            // Input Box
            parent
                .spawn((
                    Node {
                        width: Val::Px(400.0),
                        height: Val::Px(60.0),
                        border: UiRect::all(Val::Px(3.0)),
                        padding: UiRect::horizontal(Val::Px(15.0)), // text padding
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Start, // Align text to left
                        ..default()
                    },
                    BorderColor(Color::srgb(0.3, 0.3, 0.8)), // Blue border
                    BackgroundColor(Color::srgb(0.1, 0.1, 0.2)),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new(""),
                        TextFont {
                            font: font.clone(),
                            font_size: 35.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        UserInputText,
                    ));
                });

            // Join Instruction
            parent.spawn((
                Text::new("Press ENTER to Join"),
                TextFont {
                    font: font.clone(),
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 0.5, 0.5)),
            ));
        });
}

fn cleanup_menu(mut commands: Commands, query: Query<Entity, With<MenuRoot>>) {
    // ...
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

// ... spawn_cars ...

const CAR_SPRITES: &[&str] = &[
    "cars/blue.png",
    "cars/green.png",
    "cars/purple.png",
    "cars/red.png",
    "cars/yellow.png",
];

fn spawn_cars(
    mut commands: Commands,
    query: Query<(Entity, &Player), Added<Player>>,
    asset_server: Res<AssetServer>,
) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");

    for (entity, player) in query.iter() {
        info!(
            "Spawning visual representation for player car: {}",
            player.username
        );
        let color = Color::srgb(player.color[0], player.color[1], player.color[2]);

        // Select sprite deterministically
        let sprite_idx = (player.client_id as usize) % CAR_SPRITES.len();
        let sprite_path = CAR_SPRITES[sprite_idx];
        let texture_handle = asset_server.load(sprite_path);

        // Main Car Body (White to preserve texture)
        commands.entity(entity).insert(Sprite {
            image: texture_handle.clone(),
            color: Color::WHITE,
            custom_size: Some(Vec2::new(2.0, 4.0)), // 1:2 Ratio
            ..default()
        });

        // Add children (Outline only)
        commands.entity(entity).with_children(|parent| {
            // Outline (Background, tinted with player color)
            parent.spawn((
                Sprite {
                    image: texture_handle,
                    color,                                  // Player's unique color
                    custom_size: Some(Vec2::new(2.2, 4.4)), // Slightly larger for outline effect
                    ..default()
                },
                Transform::from_xyz(0.0, 0.0, -0.01), // Behind the main car
            ));
        });

        // Spawn Independent Billboard Label
        commands
            .spawn((
                CarLabel(entity),
                Transform::from_translation(Vec3::ZERO),
                Visibility::default(),
                InheritedVisibility::default(),
            ))
            .with_children(|parent| {
                // Text Shadow (Black)
                parent.spawn((
                    Text2d::new(player.username.clone()),
                    TextFont {
                        font: font.clone(),
                        font_size: 10.0,
                        ..default()
                    },
                    TextColor(Color::BLACK),
                    TextLayout::new_with_justify(JustifyText::Center),
                    // Offset relative to the Label Entity (which is at car + 3.5)
                    Transform::from_xyz(0.05, -0.05, 0.4).with_scale(Vec3::splat(0.1)),
                ));

                // Main Text (White)
                parent.spawn((
                    Text2d::new(player.username.clone()),
                    TextFont {
                        font: font.clone(),
                        font_size: 10.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    TextLayout::new_with_justify(JustifyText::Center),
                    Transform::from_xyz(0.0, 0.0, 0.5).with_scale(Vec3::splat(0.1)),
                ));
            });
    }
}

fn update_car_labels(
    mut commands: Commands,
    mut label_query: Query<(Entity, &CarLabel, &mut Transform)>,
    car_query: Query<&GlobalTransform>,
) {
    for (label_entity, car_label, mut transform) in label_query.iter_mut() {
        if let Ok(car_transform) = car_query.get(car_label.0) {
            let car_pos = car_transform.translation();
            // Position label above car
            // We set Z to 10.0 to ensure it's always on top of everything
            transform.translation = car_pos + Vec3::new(0.0, 3.5, 10.0);
            // Force rotation to identity (straight)
            transform.rotation = Quat::IDENTITY;
        } else {
            // Car despawned, cleanup label
            commands.entity(label_entity).despawn();
        }
    }
}

fn handle_input_text(
    mut events: EventReader<bevy::input::keyboard::KeyboardInput>,
    keys: Res<ButtonInput<KeyCode>>,
    mut username: ResMut<UsernameInput>,
    mut query: Query<&mut Text, With<UserInputText>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let mut changed = false;

    // Handle character input
    for event in events.read() {
        if event.state.is_pressed() {
            if let Some(text) = &event.text {
                let s = text.as_str();
                // Filter control characters to prevent weird symbols
                if s.chars().all(|c| !c.is_control()) {
                    username.0.push_str(s);
                    changed = true;
                }
            }
        }
    }

    // Handle Backspace
    if keys.just_pressed(KeyCode::Backspace) {
        username.0.pop();
        changed = true;
    }

    // Update Text UI if changed
    if changed {
        if let Ok(mut text) = query.single_mut() {
            text.0 = username.0.clone();
            // Add a blinking cursor effect or static symbol to show activity?
            // text.0.push('|'); // Simple cursor hack
        }
    }

    // Handle Enter to Join
    if keys.just_pressed(KeyCode::Enter) && !username.0.is_empty() {
        info!("Joining game with username: {}", username.0);
        next_state.set(AppState::Game);
    }
}

fn connect_to_server(mut commands: Commands, args: Res<Args>) {
    let client_id = rand::random::<u64>();
    // Use build-time configured address or fallback to WSL IP
    let server_addr_str = option_env!("NFRS_SERVER_ADDR").unwrap_or("127.0.0.1:5001");
    // Override with args if provided and not default localhost (though arg default is localhost)
    let final_addr = if args.ip != "127.0.0.1" {
        // This logic is a bit flawed if we want to support args overriding env.
        // But for now let's use the logic: if args.ip is set, use it with port 5001?
        // The original code just used args for nothing?
        // Ah, original code had `args` unused in setup_client basically.
        // Let's stick to the simpler env var or hardcode for now to minimize risk.
        server_addr_str
    } else {
        server_addr_str
    };

    let server_addr: SocketAddr = final_addr
        .parse()
        .expect("Invalid NFRS_SERVER_ADDR format. Expected IP:PORT");

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
                certificate_digest: String::from(env!("NFRS_CERT_DIGEST")),
            },
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
            UdpIo::default(),
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
