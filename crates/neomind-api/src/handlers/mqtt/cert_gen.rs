//! Self-signed TLS certificate generation for the embedded MQTT broker.
//!
//! Generates a CA certificate and a server certificate signed by that CA,
//! suitable for encrypting MQTT connections in IoT scenarios.

use std::net::IpAddr;
use std::path::PathBuf;

use rcgen::{BasicConstraints, CertificateParams, DnType, IsCa, KeyPair, SanType};

/// Paths to the generated PEM files.
pub struct CertPaths {
    pub ca_cert_path: String,
    pub ca_key_path: String,
    pub server_cert_path: String,
    pub server_key_path: String,
}

/// Get the TLS directory for storing certificates.
fn get_tls_dir() -> PathBuf {
    PathBuf::from("data/tls")
}

/// Get the local machine's LAN IP address (private IPv4).
fn get_local_ip() -> Option<IpAddr> {
    let interfaces = get_if_addrs::get_if_addrs().ok()?;
    for iface in interfaces {
        if !iface.is_loopback() {
            if let get_if_addrs::IfAddr::V4(iface_addr) = iface.addr {
                let ip = iface_addr.ip;
                let octets = ip.octets();
                if (octets[0] == 192 && octets[1] == 168)
                    || (octets[0] == 10)
                    || (octets[0] == 172 && octets[1] >= 16 && octets[1] <= 31)
                {
                    return Some(IpAddr::V4(ip));
                }
            }
        }
    }
    None
}

/// Generate self-signed CA + server certificates for the MQTT broker.
///
/// The CA certificate is valid for 5 years; the server certificate is valid
/// for 1 year and includes SANs for localhost, 127.0.0.1, and the local LAN IP.
pub fn generate_self_signed_certs() -> Result<CertPaths, String> {
    let tls_dir = get_tls_dir();
    std::fs::create_dir_all(&tls_dir)
        .map_err(|e| format!("Failed to create TLS directory: {}", e))?;

    // --- CA key + self-signed cert ---
    let ca_key =
        KeyPair::generate().map_err(|e| format!("Failed to generate CA key: {}", e))?;

    let mut ca_params = CertificateParams::new(Vec::<String>::new())
        .map_err(|e| format!("Failed to create CA params: {}", e))?;
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    ca_params
        .distinguished_name
        .push(DnType::CommonName, "NeoMind MQTT CA");
    ca_params
        .distinguished_name
        .push(DnType::OrganizationName, "NeoMind");

    // 5-year validity
    let five_years = rcgen::date_time_ymd(2031, 1, 1);
    ca_params.not_after = five_years;

    let ca_cert = ca_params
        .self_signed(&ca_key)
        .map_err(|e| format!("Failed to sign CA cert: {}", e))?;

    // --- Server key + cert signed by CA ---
    let server_key =
        KeyPair::generate().map_err(|e| format!("Failed to generate server key: {}", e))?;

    let mut server_params = CertificateParams::new(Vec::<String>::new())
        .map_err(|e| format!("Failed to create server params: {}", e))?;
    server_params
        .distinguished_name
        .push(DnType::CommonName, "NeoMind MQTT Server");

    // SANs: localhost, 127.0.0.1, and local LAN IP
    let mut sans = vec![
        SanType::DnsName("localhost".try_into().map_err(|_| "Invalid DNS name".to_string())?),
        SanType::IpAddress(IpAddr::from([127, 0, 0, 1])),
    ];
    if let Some(local_ip) = get_local_ip() {
        sans.push(SanType::IpAddress(local_ip));
    }
    server_params.subject_alt_names = sans;

    // 1-year validity
    let one_year = rcgen::date_time_ymd(2027, 1, 1);
    server_params.not_after = one_year;

    let server_cert = server_params
        .signed_by(&server_key, &ca_cert, &ca_key)
        .map_err(|e| format!("Failed to sign server cert: {}", e))?;

    // --- Write PEM files ---
    let ca_cert_path = tls_dir.join("mqtt-ca.crt");
    let ca_key_path = tls_dir.join("mqtt-ca.key");
    let server_cert_path = tls_dir.join("mqtt-server.crt");
    let server_key_path = tls_dir.join("mqtt-server.key");

    std::fs::write(&ca_cert_path, ca_cert.pem())
        .map_err(|e| format!("Failed to write CA cert: {}", e))?;
    std::fs::write(&ca_key_path, ca_key.serialize_pem())
        .map_err(|e| format!("Failed to write CA key: {}", e))?;
    std::fs::write(&server_cert_path, server_cert.pem())
        .map_err(|e| format!("Failed to write server cert: {}", e))?;
    std::fs::write(&server_key_path, server_key.serialize_pem())
        .map_err(|e| format!("Failed to write server key: {}", e))?;

    Ok(CertPaths {
        ca_cert_path: ca_cert_path.to_string_lossy().to_string(),
        ca_key_path: ca_key_path.to_string_lossy().to_string(),
        server_cert_path: server_cert_path.to_string_lossy().to_string(),
        server_key_path: server_key_path.to_string_lossy().to_string(),
    })
}
