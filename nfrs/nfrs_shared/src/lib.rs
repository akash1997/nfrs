use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::info;

pub const FIXED_TIMESTEP_HZ: f64 = 60.0;
pub const SERVER_REPLICATION_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Clone)]
pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        println!("Building ProtocolPlugin: Registering channels and messages");
        info!("Building ProtocolPlugin: Registering channels and messages");
        // Register components for replication
        app.register_component::<Player>();
        app.register_component::<Car>();
        app.register_component::<PlayerPosition>();
        app.register_component::<Transform>();

        // Register the message protocol
        app.add_message::<CarInput>();
        app.add_message::<JoinRequest>();

        // Register the input channel
        app.add_channel::<InputChannel>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
            ..default()
        })
        .add_direction(NetworkDirection::ClientToServer);
    }
}

#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct Player {
    pub client_id: u64,
    pub username: String,
    // Store color as RGB [r, g, b]
    pub color: [f32; 3],
}

#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct PlayerPosition {
    pub x: f32,
    pub y: f32,
}

#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct Car {
    pub max_speed: f32,
    pub acceleration: f32,
    pub steering_speed: f32,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy, Default, Reflect)]
pub struct CarInput {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default, Reflect)]
pub struct JoinRequest {
    pub username: String,
}

// Channel for sending car inputs
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct InputChannel;
