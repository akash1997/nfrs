use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use std::collections::HashMap;

use lightyear::prelude::*;
use nfrs_shared::{Car, CarInput, Player, PlayerPosition, SERVER_REPLICATION_INTERVAL};
use tracing::{info, warn};

// Resource to track which car entity belongs to which client entity
#[derive(Resource, Default)]
struct ClientCarMap {
    client_to_car: HashMap<Entity, Entity>,
}

// Marker component for clients that need initial state sync
// Includes a frame counter to delay sync until after Replicate components are updated
#[derive(Component)]
struct NeedsInitialSync {
    frames_to_wait: u8,
}

pub struct CarPlugin;

impl Plugin for CarPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ClientCarMap>();
        app.add_systems(Startup, spawn_boundaries);
        app.add_systems(FixedUpdate, apply_car_input);
        app.add_observer(handle_new_client);
        app.add_observer(handle_client_disconnect);
        // Add receiver for JoinRequest
        app.add_systems(
            Update,
            (debug_cars, sync_initial_state, handle_join_request),
        );
    }
}

fn spawn_boundaries(mut commands: Commands) {
    // Map size: +/- 32.0 X, +/- 18.0 Y
    let hx = 34.0; // Horizontal half-extent (top/bottom walls)
    let hy = 19.0; // Vertical half-extent (left/right walls)
    let thickness = 1.0;

    // Top Wall
    commands.spawn((
        Transform::from_xyz(0.0, hy, 0.0),
        Collider::cuboid(hx, thickness),
    ));

    // Bottom Wall
    commands.spawn((
        Transform::from_xyz(0.0, -hy, 0.0),
        Collider::cuboid(hx, thickness),
    ));

    // Left Wall
    commands.spawn((
        Transform::from_xyz(-33.0, 0.0, 0.0),
        Collider::cuboid(thickness, hy),
    ));

    // Right Wall
    commands.spawn((
        Transform::from_xyz(33.0, 0.0, 0.0),
        Collider::cuboid(thickness, hy),
    ));

    info!("Spawned map boundaries");
}

fn debug_cars(query: Query<Entity, (With<Car>, With<Replicate>)>, time: Res<Time>) {
    if time.elapsed_secs() % 5.0 < 0.1 {
        info!(
            "Server Car entities with Replicate: {}",
            query.iter().count()
        );
    }
}

/// Force initial state sync for newly connected clients
/// This ensures they receive current Transform state of all cars
fn sync_initial_state(
    mut commands: Commands,
    mut new_clients: Query<(Entity, &mut NeedsInitialSync)>,
    mut cars: Query<(Entity, &mut Transform, Option<&mut Velocity>, &Player), With<Car>>,
) {
    for (client, mut sync_marker) in new_clients.iter_mut() {
        if sync_marker.frames_to_wait > 0 {
            // Wait a few frames to ensure Replicate components are updated
            sync_marker.frames_to_wait -= 1;
            info!(
                "Waiting for replication setup, frames left: {}",
                sync_marker.frames_to_wait
            );
            continue;
        }

        // Mark all car components as changed to force replication
        for (car_entity, mut transform, velocity, player) in cars.iter_mut() {
            info!(
                "Syncing car {:?} (client_id: {}) position: {:?}",
                car_entity, player.client_id, transform.translation
            );
            transform.set_changed();
            if let Some(mut vel) = velocity {
                vel.set_changed();
            }
        }

        // Remove the marker component
        commands.entity(client).remove::<NeedsInitialSync>();
        info!("Completed initial state sync for client {:?}", client);
    }
}

/// Handle new client connections
fn handle_new_client(
    trigger: Trigger<OnAdd, LinkOf>,
    mut commands: Commands,
    // Query all existing client connections
    client_connections: Query<Entity, With<ReplicationSender>>,
    // Query all existing cars to update their replication
    mut existing_cars: Query<(Entity, &mut Replicate), With<Car>>,
) {
    let client_entity = trigger.target();
    info!("New client entity {:?} connected", client_entity);

    // Use client entity index as stable identifier for this connection
    let client_id = client_entity.index() as u64;

    info!("Client connected with ID: {}", client_id);

    // Add replication sender to the connection
    commands
        .entity(client_entity)
        .insert(ReplicationSender::new(
            SERVER_REPLICATION_INTERVAL,
            SendUpdatesMode::SinceLastAck,
            false,
        ));

    // Add input receiver for this client
    commands
        .entity(client_entity)
        .insert(MessageReceiver::<CarInput>::default());

    // Add JoinRequest receiver
    commands
        .entity(client_entity)
        .insert(MessageReceiver::<nfrs_shared::JoinRequest>::default());

    // Get all client entities (existing + new one) for replication
    let mut all_clients: Vec<Entity> = client_connections.iter().collect();
    all_clients.push(client_entity);

    // Update all existing cars to also replicate to this new client
    for (existing_car, _) in existing_cars.iter_mut() {
        // Replace the Replicate component with a new one that includes all clients
        commands
            .entity(existing_car)
            .insert(Replicate::manual(all_clients.clone()));
        info!(
            "Updated car {:?} to replicate to new client {:?}",
            existing_car, client_entity
        );
    }

    // Mark this client as needing initial state sync from all existing cars
    // Wait 3 frames to ensure Replicate components are properly updated and active
    commands
        .entity(client_entity)
        .insert(NeedsInitialSync { frames_to_wait: 3 });

    // Note: We do NOT spawn a car here anymore. We wait for JoinRequest.
    info!("Client initialized, waiting for JoinRequest...");
}

fn handle_join_request(
    mut commands: Commands,
    mut message_receivers: Query<(Entity, &mut MessageReceiver<nfrs_shared::JoinRequest>)>,
    mut car_map: ResMut<ClientCarMap>,
    client_connections: Query<Entity, With<ReplicationSender>>,
    mut existing_cars: Query<(Entity, &mut Replicate), With<Car>>,
) {
    for (client_entity, mut receiver) in message_receivers.iter_mut() {
        if let Some(request) = receiver.receive().next() {
            let client_id = client_entity.index() as u64;
            info!(
                "Received JoinRequest from client {}: {:?}",
                client_id, request
            );

            // Check if client already has a car
            if car_map.client_to_car.contains_key(&client_entity) {
                warn!(
                    "Client {} already has a car, ignoring JoinRequest",
                    client_id
                );
                continue;
            }

            // Generate unique color based on client_id to be deterministic/simple for now
            // or modify to use Golden Ratio if needed.
            // Simple HSL generation:
            let hue = (client_id as f32 * 137.508) % 360.0; // Golden angle approximation
            let color = Color::hsl(hue, 0.8, 0.5);
            let color_rgba = color.to_srgba();
            let color_array = [color_rgba.red, color_rgba.green, color_rgba.blue];

            // Get all client entities for replication
            let all_clients: Vec<Entity> = client_connections.iter().collect();

            // Create Replicate component
            let replicate = Replicate::manual(all_clients.clone());

            // Spawn car
            let car_entity = commands
                .spawn((
                    Car {
                        max_speed: 20.0,
                        acceleration: 10.0,
                        steering_speed: 2.0,
                    },
                    Player {
                        client_id,
                        username: request.username.clone(),
                        color: color_array,
                    },
                    PlayerPosition::default(),
                    Transform::from_xyz(0.0, 0.0, 0.0),
                    GlobalTransform::default(),
                    RigidBody::Dynamic,
                    Collider::cuboid(1.0, 2.0),
                    Velocity::default(),
                    GravityScale(0.0),
                    Damping {
                        linear_damping: 2.0,
                        angular_damping: 2.0,
                    },
                    replicate,
                    ReplicationGroup::default(),
                ))
                .id();

            // Update map
            car_map.client_to_car.insert(client_entity, car_entity);
            info!(
                "Spawned car {:?} for user '{}' (client {:?})",
                car_entity, request.username, client_entity
            );

            // Update existing cars to replicate to this (possibly new) client
            // Although handle_new_client handled the initial sync setup, ensuring manual replication list is current is good.
            // Actually, handle_new_client doesn't update existing cars' replication list anymore because we needed the list of clients.
            // We should ensure that when a NEW client joins, existing cars start replicating to it?
            // Wait, existing cars are replicated manually. We need to update their Replicate target list.

            // Re-evaluating logic:
            // handle_new_client adds the client to the "world".
            // existing cars need to know about this new client to replicate to it.
            // BUT, handle_new_client didn't have access to "existing cars" to update them in the previous logic?
            // Ah, looking at previous code: handle_new_client DID update existing_cars.
            // I removed that block. I should put it back in handle_new_client OR handle it here?
            // Ideally, cars should start replicating to a client as soon as it connects, even if that client hasn't joined yet?
            // Yes, so they can see other cars while in menu (if we wanted).
            // But currently cars are only spawned when joined.
            // Let's stick to: Update existing cars to replicate to ALL connected clients.

            for (existing_car, _) in existing_cars.iter_mut() {
                commands
                    .entity(existing_car)
                    .insert(Replicate::manual(all_clients.clone()));
            }
        }
    }
}

/// Handle client disconnections and cleanup their car
fn handle_client_disconnect(
    trigger: Trigger<OnRemove, LinkOf>,
    mut car_map: ResMut<ClientCarMap>,
    mut commands: Commands,
    client_connections: Query<Entity, With<ReplicationSender>>,
    remaining_cars: Query<Entity, With<Car>>,
) {
    let client_entity = trigger.target();
    info!("Client entity {:?} disconnecting", client_entity);

    // Look up the car entity for this client
    let despawned_car = car_map.client_to_car.remove(&client_entity);

    if let Some(car_entity) = despawned_car {
        info!(
            "Despawning car {:?} for disconnected client {:?}",
            car_entity, client_entity
        );
        commands.entity(car_entity).despawn();
    } else {
        warn!("No car found for disconnecting client {:?}", client_entity);
    }

    // Get remaining client connections (excluding the disconnected one)
    let remaining_clients: Vec<Entity> = client_connections
        .iter()
        .filter(|&e| e != client_entity)
        .collect();

    info!(
        "Updating replication for remaining {} clients",
        remaining_clients.len()
    );

    // Update all remaining cars to replicate only to remaining clients
    // Skip the car we just despawned (since despawn is deferred)
    for car in remaining_cars.iter() {
        if Some(car) == despawned_car {
            continue; // Skip the car we just despawned
        }
        commands
            .entity(car)
            .insert(Replicate::manual(remaining_clients.clone()));
    }
}

fn apply_car_input(
    mut query: Query<(&Player, &Car, &mut Velocity, &Transform)>,
    mut input_receivers: Query<(Entity, &mut MessageReceiver<CarInput>)>,
) {
    for (client_entity, mut input_receiver) in input_receivers.iter_mut() {
        // Use client entity index as identifier
        let client_id = client_entity.index() as u64;

        // Get the latest input for this client
        for input in input_receiver.receive() {
            info!("Received input from client {}: {:?}", client_id, input);

            // Find the player's car
            for (player, car, mut velocity, transform) in query.iter_mut() {
                if player.client_id == client_id {
                    // Apply physics based on input
                    let forward = transform.rotation * Vec3::Y;
                    let forward_2d = Vec2::new(forward.x, forward.y);

                    let mut linear_vel = velocity.linvel;
                    let mut angular_vel = velocity.angvel;

                    // Forward/backward
                    if input.forward {
                        linear_vel += forward_2d * car.acceleration * 0.016; // Assuming 60 FPS
                    }
                    if input.backward {
                        linear_vel -= forward_2d * car.acceleration * 0.016;
                    }

                    // Steering
                    if input.left {
                        angular_vel += car.steering_speed * 0.016;
                    }
                    if input.right {
                        angular_vel -= car.steering_speed * 0.016;
                    }

                    // Clamp speed
                    let speed = linear_vel.length();
                    if speed > car.max_speed {
                        linear_vel = linear_vel.normalize() * car.max_speed;
                    }

                    velocity.linvel = linear_vel;
                    velocity.angvel = angular_vel;

                    info!(
                        "Applied input to car {}: linvel={:?}, angvel={}, rotation={:?}",
                        client_id, linear_vel, angular_vel, transform.rotation
                    );
                }
            }
        }
    }
}
