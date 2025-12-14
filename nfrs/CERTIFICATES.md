# Certificate Generation Guide

The `nfrs_server` requires a TLS certificate to support WebTransport connections. We use a self-signed certificate for development.

## Generating a Certificate

A helper script `generate_cert.sh` is provided in the `nfrs` directory to generate the `cert.pem` and `key.pem` files.

### Usage

Run the script from the `nfrs` directory:

```bash
cd nfrs
bash generate_cert.sh [IP_ADDRESS]
```

Replace `[IP_ADDRESS]` with the IP address where the server is running (e.g., your local network IP). If no IP is provided, it defaults to `127.0.0.1`.

### Example

To generate a certificate for IP `192.168.29.43`:

```bash
bash generate_cert.sh 192.168.29.43
```

This will create (or overwrite) `cert.pem` and `key.pem` in the current directory.

## Automatic Client Configuration

The client (`nfrs_client`) is configured to **automatically** pick up the certificate digest during the build process.
- When you run `docker compose up --build` or `trunk build`, the `build.rs` script reads `nfrs/cert.pem`, calculates the SHA-256 hash, and injects it into the client code.
- **You do NOT need to manually update the client code.**

## Troubleshooting

If you see `ERR_QUIC_PROTOCOL_ERROR.QUIC_TLS_CERTIFICATE_UNKNOWN` or `CERTIFICATE_VERIFY_FAILED`:

1. Ensure you have regenerated the certificate with the correct IP address.
2. Ensure you have **rebuilt** the client (`docker compose up -d --build`) after generating a new certificate.
3. In a browser environment, you may need to accept the self-signed certificate manually (e.g., by navigating to the API endpoint and bypassing the warning).
