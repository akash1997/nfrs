use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use lightyear::prelude::*;
use nfrs_shared::{Car, CarInput, Player, PlayerPosition, SERVER_REPLICATION_INTERVAL};
use tracing::info;

pub struct CarPlugin;

impl Plugin for CarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, apply_car_input);
        app.add_observer(handle_new_client);
        app.add_systems(Update, debug_cars);
    }
}

fn debug_cars(query: Query<Entity, (With<Car>, With<Replicate>)>, time: Res<Time>) {
    if time.elapsed_secs() % 5.0 < 0.1 {
        info!(
            "Server Car entities with Replicate: {}",
            query.iter().count()
        );
    }
}

/// Handle new client connections
fn handle_new_client(trigger: Trigger<OnAdd, LinkOf>, mut commands: Commands) {
    let client_entity = trigger.target();
    info!("New client entity {:?} connected", client_entity);

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

    // Spawn car for this client
    let client_id = client_entity.index() as u64; // Use entity index as dummy ID

    commands.spawn((
        Car {
            max_speed: 20.0,
            acceleration: 10.0,
            steering_speed: 2.0,
        },
        Player { client_id },
        PlayerPosition::default(),
        Transform::from_xyz(0.0, 0.0, 0.0),
        GlobalTransform::default(),
        RigidBody::Dynamic,
        Collider::cuboid(1.0, 2.0),
        Velocity::default(),
        Replicate::to_clients(NetworkTarget::All),
        ReplicationGroup::default(),
    ));
}

fn apply_car_input(
    mut query: Query<(&Player, &Car, &mut Velocity, &Transform)>,
    mut input_receivers: Query<(Entity, &mut MessageReceiver<CarInput>)>,
) {
    for (client_entity, mut input_receiver) in input_receivers.iter_mut() {
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
                }
            }
        }
    }
}
