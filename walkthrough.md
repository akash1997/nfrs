# PvP Racing Server - Walkthrough

This document outlines the implementation of the PvP Racing Server using Bevy, Rapier2d, and Renet (via Bevy Replicon).

## Project Structure

- `nfrs_server`: The headless server binary.
- `nfrs_shared`: Shared library containing components, events, and protocol definitions.

## Running the Server

To run the server, execute the following command in the `nfrs` workspace directory:

```bash
cargo run -p nfrs_server
```

The server will start and listen on `127.0.0.1:5000`.

## Implementation Details

### Networking & Replication
- Uses `bevy_replicon` for high-level replication.
- Uses `bevy_replicon_renet` as the transport layer.
- **Replicated Components**: `Car`, `Player`, `PlayerPosition`, `Transform`.
- **Client Events**: `CarInput` (Throttle, Steering).

### Physics & Gameplay
- Uses `bevy_rapier2d` for 2D physics.
- **Car Controller**: Server-authoritative. The server receives `CarInput` events from clients and applies forces/torques to the car's `ExternalForce` component.
- **Spawning**: When a client connects, a car is automatically spawned with a `Player` component linked to the client ID.

## Client Implementation

A native client is included in `nfrs_client`. It connects to the server, spawns a red rectangle for each player, and allows movement using Arrow Keys.

To run the client (in a separate terminal):

```bash
cargo run -p nfrs_client
```

**Controls:**
- **Arrow Up/Down**: Accelerate/Brake
- **Arrow Left/Right**: Steer

**Note**: The client requires a windowing environment (X11/Wayland on Linux). If running on a headless server, you won't see the window, but the logs will show the connection.
