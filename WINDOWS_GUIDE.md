# Running nfrs_client on Windows 11

Since the server is running in a Linux environment (WSL/VM) and the client needs a windowing system, you should run the client directly on Windows.

## Prerequisites
1. **Install Rust on Windows**: Download and install `rustup-init.exe` from [rustup.rs](https://rustup.rs/).
2. **Install C++ Build Tools**:
   - Rust on Windows (MSVC ABI) requires the Microsoft C++ Build Tools.
   - Download **Visual Studio Build Tools** from [visualstudio.microsoft.com/visual-cpp-build-tools/](https://visualstudio.microsoft.com/visual-cpp-build-tools/).
   - During installation, ensure the **"Desktop development with C++"** workload is selected.
   - This installs `link.exe` and other necessary tools.
3. **Git**: Ensure you have Git installed to clone/copy the repo.

## Steps

1. **Copy the Project**: Copy the entire `nfrs` project folder from your Linux environment to a location accessible by Windows (e.g., `C:\Users\YourName\Projects\nfrs`).
   - If using WSL2, you can access the Linux file system via `\\wsl$\Ubuntu\home\akash\projects\nfrs`.

2. **Find Server IP**:
   - In your Linux terminal, run `ip addr` to find the IP address of the WSL/VM instance (usually `eth0`).
   - It will look something like `172.x.x.x` or `192.168.x.x`.

3. **Run the Client**:
   - Open a PowerShell or Command Prompt in the `nfrs` directory on Windows.
   - Run the client, specifying the server's IP address:
     ```powershell
     cargo run -p nfrs_client -- --ip <SERVER_IP>
     ```
     Example:
     ```powershell
     cargo run -p nfrs_client -- --ip 172.25.144.5
     ```

4. **Play**:
   - The window should open. Use Arrow Keys to control the car.

## Troubleshooting
- **Firewall**: Ensure Windows Firewall allows the connection on port 5000 (UDP).
- **WSL Networking**: If you can't connect, try port forwarding or ensure you are using the correct IP. `localhost` might not work if the server is bound to `0.0.0.0` inside WSL but Windows doesn't forward it automatically. Using the specific WSL IP is safer.
