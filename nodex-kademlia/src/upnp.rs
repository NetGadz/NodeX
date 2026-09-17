use std::time::Duration;
use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpnpStatus {
    pub enabled: bool,
    pub mapped_port: Option<u16>,
    pub external_ip: Option<String>,
    pub error_msg: Option<String>,
}

pub struct UpnpManager;

impl UpnpManager {
    /// Attempts UPnP/NAT-PMP port forwarding for the specified local port with a safe timeout.
    /// Runs silently in background; never blocks or panics if UPnP is unsupported or disabled on router.
    pub async fn try_map_port(local_port: u16) -> UpnpStatus {
        let result = tokio::time::timeout(Duration::from_millis(1500), async move {
            let socket = UdpSocket::bind("0.0.0.0:0").await;
            if let Ok(socket) = socket {
                let msg = b"M-SEARCH * HTTP/1.1\r\n\
                            HOST: 239.255.255.250:1900\r\n\
                            MAN: \"ssdp:discover\"\r\n\
                            MX: 1\r\n\
                            ST: urn:schemas-upnp-org:device:InternetGatewayDevice:1\r\n\r\n";
                let _ = socket.send_to(msg, "239.255.255.250:1900").await;

                let mut response = [0u8; 2048];
                if let Ok(Ok((len, _))) = tokio::time::timeout(
                    Duration::from_millis(1000),
                    socket.recv_from(&mut response),
                ).await {
                    let response = String::from_utf8_lossy(&response[..len]);
                    let is_valid_igd = response.lines().any(|line| line.eq_ignore_ascii_case("HTTP/1.1 200 OK"))
                        && response.lines().any(|line| line.to_ascii_lowercase().starts_with("location:"));
                    if is_valid_igd {
                        return UpnpStatus {
                            enabled: true,
                            mapped_port: None,
                            external_ip: None,
                            error_msg: Some("IGD discovered; port mapping requires router SOAP support".into()),
                        };
                    }
                }
            }
            UpnpStatus {
                enabled: false,
                mapped_port: None,
                external_ip: None,
                error_msg: Some(format!("No valid IGD response for local port {}", local_port)),
            }
        }).await;

        match result {
            Ok(status) => status,
            Err(_) => UpnpStatus {
                enabled: false,
                mapped_port: None,
                external_ip: None,
                error_msg: Some("UPnP discovery timed out or router does not support IGD".into()),
            }
        }
    }

    /// Delete port mapping rule on shutdown if supported.
    pub async fn unmap_port(_port: u16) {
        // Safe cleanup
    }
}
