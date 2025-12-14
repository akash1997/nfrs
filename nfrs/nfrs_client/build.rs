use base64::{engine::general_purpose, Engine as _};
use sha2::{Digest, Sha256};

use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=../cert.pem");

    let cert_path = Path::new("../cert.pem");
    if !cert_path.exists() {
        println!(
            "cargo:warning=Certificate file not found at {:?}. Using dummy digest.",
            cert_path
        );
        // Providing a dummy hash to allow build to pass if cert is missing (e.g. in CI without it)
        println!("cargo:rustc-env=NFRS_CERT_DIGEST=0000000000000000000000000000000000000000000000000000000000000000");
        return;
    }

    let cert_pem = fs::read_to_string(cert_path).expect("Failed to read ../cert.pem");

    // Extract base64 content
    let start_marker = "-----BEGIN CERTIFICATE-----";
    let end_marker = "-----END CERTIFICATE-----";

    let start = cert_pem
        .find(start_marker)
        .expect("Invalid PEM: missing start marker")
        + start_marker.len();
    let end = cert_pem
        .find(end_marker)
        .expect("Invalid PEM: missing end marker");

    let base64_content: String = cert_pem[start..end]
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();

    // Decode base64 to DER
    let der_bytes = general_purpose::STANDARD
        .decode(&base64_content)
        .expect("Failed to decode base64 certificate content");

    // Compute SHA-256 hash
    let mut hasher = Sha256::new();
    hasher.update(&der_bytes);
    let result = hasher.finalize();
    let hex_digest = hex::encode(result);

    println!(
        "cargo:warning=Calculated Certificate Digest: {}",
        hex_digest
    );
    println!("cargo:rustc-env=NFRS_CERT_DIGEST={}", hex_digest);
}
