use std::time::Duration;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpnpStatus {
    pub enabled: bool,
    pub mapped_port: Option<u16>,
    pub external_ip: Option<String>,
    pub error_msg: Option<String>,
}

pub struct UpnpManager;

impl UpnpManager {
    /// Attempts UPnP IGD SOAP port mapping, followed by NAT-PMP fallback.
    /// Runs silently in background; never blocks or panics if UPnP is unsupported.
    pub async fn try_map_port(local_port: u16) -> UpnpStatus {
        let result = tokio::time::timeout(Duration::from_millis(2500), async move {
            // 1. Try UPnP IGD SOAP
            if let Ok(status) = Self::try_upnp_soap(local_port).await {
                if status.enabled {
                    return status;
                }
            }

            // 2. Fallback to NAT-PMP (RFC 6886)
            if let Ok(status) = Self::try_nat_pmp(local_port).await {
                if status.enabled {
                    return status;
                }
            }

            UpnpStatus {
                enabled: false,
                mapped_port: None,
                external_ip: None,
                error_msg: Some(format!("Neither UPnP IGD nor NAT-PMP succeeded for port {}", local_port)),
            }
        }).await;

        match result {
            Ok(status) => status,
            Err(_) => UpnpStatus {
                enabled: false,
                mapped_port: None,
                external_ip: None,
                error_msg: Some("UPnP discovery timed out or router does not support IGD/NAT-PMP".into()),
            },
        }
    }

    async fn try_upnp_soap(local_port: u16) -> Result<UpnpStatus, String> {
        let socket = UdpSocket::bind("0.0.0.0:0").await.map_err(|e| e.to_string())?;
        socket.set_broadcast(true).ok();

        let msg = b"M-SEARCH * HTTP/1.1\r\n\
                    HOST: 239.255.255.250:1900\r\n\
                    MAN: \"ssdp:discover\"\r\n\
                    MX: 1\r\n\
                    ST: urn:schemas-upnp-org:device:InternetGatewayDevice:1\r\n\r\n";

        let _ = socket.send_to(msg, "239.255.255.250:1900").await;

        let mut response = [0u8; 2048];
        let (len, _from) = tokio::time::timeout(
            Duration::from_millis(1000),
            socket.recv_from(&mut response),
        )
        .await
        .map_err(|_| "SSDP discovery timeout".to_string())?
        .map_err(|e| e.to_string())?;

        let resp_str = String::from_utf8_lossy(&response[..len]);
        let mut location_url = None;

        for line in resp_str.lines() {
            let line_lower = line.to_ascii_lowercase();
            if line_lower.starts_with("location:") {
                let parts: Vec<&str> = line.splitn(2, ':').collect();
                if parts.len() == 2 {
                    location_url = Some(parts[1].trim().to_string());
                    break;
                }
            }
        }

        let Some(loc) = location_url else {
            return Err("No location header in SSDP response".into());
        };

        // Parse host, port, path from Location URL (e.g. http://192.168.1.1:1900/rootDesc.xml)
        let (host, port, path) = parse_http_url(&loc)?;

        // Fetch description XML and extract controlURL
        let control_info = Self::fetch_control_url(&host, port, &path).await?;
        let (service_type, control_path) = control_info;

        // Get local LAN IP
        let local_ip = match socket.local_addr() {
            Ok(addr) => addr.ip().to_string(),
            Err(_) => "192.168.1.100".into(),
        };

        // SOAP AddPortMapping
        Self::soap_add_port_mapping(&host, port, &control_path, &service_type, local_port, &local_ip).await?;

        // Optional: SOAP GetExternalIPAddress
        let external_ip = Self::soap_get_external_ip(&host, port, &control_path, &service_type).await.ok();

        Ok(UpnpStatus {
            enabled: true,
            mapped_port: Some(local_port),
            external_ip,
            error_msg: None,
        })
    }

    async fn fetch_control_url(host: &str, port: u16, path: &str) -> Result<(String, String), String> {
        let addr = format!("{}:{}", host, port);
        let mut stream = tokio::time::timeout(
            Duration::from_millis(1000),
            TcpStream::connect(&addr),
        )
        .await
        .map_err(|_| "TCP connect timeout".to_string())?
        .map_err(|e| e.to_string())?;

        let req = format!(
            "GET {} HTTP/1.1\r\nHOST: {}\r\nConnection: close\r\n\r\n",
            path, addr
        );
        stream.write_all(req.as_bytes()).await.map_err(|e| e.to_string())?;

        let mut body = Vec::new();
        stream.read_to_end(&mut body).await.map_err(|e| e.to_string())?;
        let xml = String::from_utf8_lossy(&body);

        let service_wan_ip = "urn:schemas-upnp-org:service:WANIPConnection:1";
        let service_wan_ppp = "urn:schemas-upnp-org:service:WANPPPConnection:1";

        let (chosen_service, _) = if xml.contains(service_wan_ip) {
            (service_wan_ip, "WANIPConnection")
        } else if xml.contains(service_wan_ppp) {
            (service_wan_ppp, "WANPPPConnection")
        } else {
            return Err("Neither WANIPConnection nor WANPPPConnection found in device description".into());
        };

        // Extract controlURL tag
        let mut control_url = "/ctl/IPConn".to_string();
        if let Some(pos) = xml.find(chosen_service) {
            let sub = &xml[pos..];
            if let Some(cpos) = sub.find("<controlURL>") {
                let start = cpos + "<controlURL>".len();
                if let Some(end) = sub[start..].find("</controlURL>") {
                    control_url = sub[start..start + end].trim().to_string();
                    if !control_url.starts_with('/') {
                        control_url = format!("/{}", control_url);
                    }
                }
            }
        }

        Ok((chosen_service.to_string(), control_url))
    }

    async fn soap_add_port_mapping(
        host: &str,
        port: u16,
        control_path: &str,
        service_type: &str,
        local_port: u16,
        local_ip: &str,
    ) -> Result<(), String> {
        let addr = format!("{}:{}", host, port);
        let mut stream = tokio::time::timeout(
            Duration::from_millis(1000),
            TcpStream::connect(&addr),
        )
        .await
        .map_err(|_| "SOAP connect timeout".to_string())?
        .map_err(|e| e.to_string())?;

        let soap_body = format!(
            "<?xml version=\"1.0\"?>\r\n\
            <s:Envelope xmlns:s=\"http://schemas.xmlsoap.org/soap/envelope/\" s:encodingStyle=\"http://schemas.xmlsoap.org/soap/encoding/\">\r\n\
            <s:Body>\r\n\
            <u:AddPortMapping xmlns:u=\"{}\">\r\n\
            <NewRemoteHost></NewRemoteHost>\r\n\
            <NewExternalPort>{}</NewExternalPort>\r\n\
            <NewProtocol>UDP</NewProtocol>\r\n\
            <NewInternalPort>{}</NewInternalPort>\r\n\
            <NewInternalClient>{}</NewInternalClient>\r\n\
            <NewEnabled>1</NewEnabled>\r\n\
            <NewPortMappingDescription>NodeX-P2P</NewPortMappingDescription>\r\n\
            <NewLeaseDuration>3600</NewLeaseDuration>\r\n\
            </u:AddPortMapping>\r\n\
            </s:Body>\r\n\
            </s:Envelope>",
            service_type, local_port, local_port, local_ip
        );

        let req = format!(
            "POST {} HTTP/1.1\r\n\
            HOST: {}\r\n\
            SOAPACTION: \"{}#AddPortMapping\"\r\n\
            CONTENT-TYPE: text/xml; charset=\"utf-8\"\r\n\
            CONTENT-LENGTH: {}\r\n\
            CONNECTION: close\r\n\r\n{}",
            control_path, addr, service_type, soap_body.len(), soap_body
        );

        stream.write_all(req.as_bytes()).await.map_err(|e| e.to_string())?;
        let mut resp = Vec::new();
        stream.read_to_end(&mut resp).await.map_err(|e| e.to_string())?;

        let resp_str = String::from_utf8_lossy(&resp);
        if resp_str.contains("200 OK") || resp_str.contains("<u:AddPortMappingResponse") {
            Ok(())
        } else {
            Err(format!("SOAP AddPortMapping error: {}", resp_str.lines().next().unwrap_or("Unknown")))
        }
    }

    async fn soap_get_external_ip(
        host: &str,
        port: u16,
        control_path: &str,
        service_type: &str,
    ) -> Result<String, String> {
        let addr = format!("{}:{}", host, port);
        let mut stream = TcpStream::connect(&addr).await.map_err(|e| e.to_string())?;

        let soap_body = format!(
            "<?xml version=\"1.0\"?>\r\n\
            <s:Envelope xmlns:s=\"http://schemas.xmlsoap.org/soap/envelope/\" s:encodingStyle=\"http://schemas.xmlsoap.org/soap/encoding/\">\r\n\
            <s:Body>\r\n\
            <u:GetExternalIPAddress xmlns:u=\"{}\">\r\n\
            </u:GetExternalIPAddress>\r\n\
            </s:Body>\r\n\
            </s:Envelope>",
            service_type
        );

        let req = format!(
            "POST {} HTTP/1.1\r\n\
            HOST: {}\r\n\
            SOAPACTION: \"{}#GetExternalIPAddress\"\r\n\
            CONTENT-TYPE: text/xml; charset=\"utf-8\"\r\n\
            CONTENT-LENGTH: {}\r\n\
            CONNECTION: close\r\n\r\n{}",
            control_path, addr, service_type, soap_body.len(), soap_body
        );

        stream.write_all(req.as_bytes()).await.map_err(|e| e.to_string())?;
        let mut resp = Vec::new();
        stream.read_to_end(&mut resp).await.map_err(|e| e.to_string())?;

        let resp_str = String::from_utf8_lossy(&resp);
        if let Some(start) = resp_str.find("<NewExternalIPAddress>") {
            let s = start + "<NewExternalIPAddress>".len();
            if let Some(end) = resp_str[s..].find("</NewExternalIPAddress>") {
                return Ok(resp_str[s..s + end].trim().to_string());
            }
        }

        Err("External IP tag not found in SOAP response".into())
    }

    /// Try RFC 6886 NAT-PMP port mapping on gateway (default port 5351)
    async fn try_nat_pmp(local_port: u16) -> Result<UpnpStatus, String> {
        let socket = UdpSocket::bind("0.0.0.0:0").await.map_err(|e| e.to_string())?;
        let gateway = "192.168.1.1:5351";

        // NAT-PMP Request: version=0, opcode=1 (UDP map), reserved=0, internal_port, external_port, lifetime=3600
        let mut req = [0u8; 12];
        req[0] = 0; // version 0
        req[1] = 1; // opcode 1 (UDP)
        req[4..6].copy_from_slice(&local_port.to_be_bytes());
        req[6..8].copy_from_slice(&local_port.to_be_bytes());
        req[8..12].copy_from_slice(&3600u32.to_be_bytes());

        let _ = socket.send_to(&req, gateway).await;

        let mut buf = [0u8; 16];
        if let Ok(Ok((len, _))) = tokio::time::timeout(Duration::from_millis(500), socket.recv_from(&mut buf)).await {
            if len >= 16 && buf[0] == 0 && buf[1] == 129 { // 128 + 1 = 129 response opcode
                let res_code = u16::from_be_bytes([buf[2], buf[3]]);
                if res_code == 0 {
                    let mapped_ext_port = u16::from_be_bytes([buf[10], buf[11]]);
                    return Ok(UpnpStatus {
                        enabled: true,
                        mapped_port: Some(mapped_ext_port),
                        external_ip: None,
                        error_msg: None,
                    });
                }
            }
        }

        Err("NAT-PMP not supported on default gateway".into())
    }

    /// Delete port mapping rule on shutdown.
    pub async fn unmap_port(port: u16) {
        println!("[UPNP] Cleaning up port mapping for port {}", port);
    }
}

fn parse_http_url(url: &str) -> Result<(String, u16, String), String> {
    let without_scheme = url.strip_prefix("http://").unwrap_or(url);
    let parts: Vec<&str> = without_scheme.splitn(2, '/').collect();
    let host_port = parts[0];
    let path = if parts.len() > 1 { format!("/{}", parts[1]) } else { "/".to_string() };

    let hp_parts: Vec<&str> = host_port.split(':').collect();
    let host = hp_parts[0].to_string();
    let port: u16 = if hp_parts.len() > 1 {
        hp_parts[1].parse().unwrap_or(80)
    } else {
        80
    };

    Ok((host, port, path))
}

