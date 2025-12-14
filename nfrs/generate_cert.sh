#!/bin/bash

# Default IP if not provided
IP_ADDRESS="${1:-127.0.0.1}"

echo "Generating self-signed certificate for IP: $IP_ADDRESS"

# Generate a self-signed certificate with the IP in the SAN
# WebTransport with serverCertificateHashes requires:
# 1. Validity <= 14 days
# 2. ECDSA (prime256v1) key (RSA is not supported for hash-based auth)
openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 -sha256 -days 10 \
  -nodes -keyout key.pem -out cert.pem \
  -subj "/CN=localhost" \
  -addext "subjectAltName=DNS:localhost,IP:$IP_ADDRESS"

echo "Certificate (cert.pem) and Private Key (key.pem) generated."
echo "Validity: 10 days (required for WebTransport hash verification)"
echo "Algorithm: ECDSA prime256v1"
echo "Subject Alternative Names encoded: DNS:localhost, IP:$IP_ADDRESS"
