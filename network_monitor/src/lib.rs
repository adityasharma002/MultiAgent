use serde::{Serialize, Deserialize};
use tokio::sync::mpsc;
use std::error::Error;
use std::collections::HashMap;
use pcap::{Device, Capture};
use sysinfo::{System, SystemExt, CpuExt};
use std::net::IpAddr;
use pnet::packet::{ethernet, Packet};
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::tcp::TcpPacket; 
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkAlert {
    pub device_id: String,
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub description: String,
    pub source_ip: Option<String>,
    pub destination_ip: Option<String>,
    pub protocol: Option<String>,
    pub port: Option<u16>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AlertType {
    Intrusion,
    Malware,
    Anomaly,
    Performance,
    Resource,
    Bandwidth,
    UnauthorizedAccess,
    SuspiciousTraffic,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AlertSeverity {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Default)]
struct NetworkStats {
    bandwidth_usage: f64,
    connection_count: usize,
    packet_count: u64,
    last_check: DateTime<Utc>,
    known_ips: HashMap<IpAddr, ConnectionInfo>,
    port_scan_attempts: HashMap<IpAddr, Vec<u16>>,
}

struct ConnectionInfo {
    first_seen: DateTime<Utc>,
    last_seen: DateTime<Utc>,
    bytes_sent: u64,
    bytes_received: u64,
    ports_accessed: Vec<u16>,
}

pub struct NetworkMonitor {
    device_id: String,
    packet_capture: Capture<pcap::Active>,
    system_info: System,
    alert_tx: mpsc::Sender<NetworkAlert>,
    stats: NetworkStats,
    blocked_ips: Vec<IpAddr>,
    known_malware_signatures: Vec<Vec<u8>>,
    suspicious_ports: Vec<u16>,
}

impl NetworkMonitor {
   pub fn new(device_id: String, alert_tx: mpsc::Sender<NetworkAlert>) -> Result<Self, Box<dyn Error>> {
       let devices = Device::list()?;
       let default_device = devices.first().ok_or("No network devices found")?;
       
       let capture = Capture::from_device(default_device.clone())?
           .promisc(true)
           .snaplen(5000)
           .timeout(100)
           .open()?;

        Ok(Self {
            device_id,
            packet_capture: capture,
            system_info: System::new_all(),
            alert_tx,
            stats: NetworkStats::default(),
            blocked_ips: Vec::new(),
            known_malware_signatures: load_malware_signatures(),
            suspicious_ports: vec![21, 22, 23, 445, 3389], // Common attack ports
        })
    }

   async fn analyze_packet(&mut self, packet: &pcap::Packet<'_>) -> Option<NetworkAlert> {
       let eth_packet = ethernet::EthernetPacket::new(packet.data)?;
       
       match eth_packet.get_ethertype() {
           ethernet::EtherTypes::Ipv4 => {
               if let Some(ip_packet) = Ipv4Packet::new(eth_packet.payload()) {
                   let source = IpAddr::V4(ip_packet.get_source());
                   let destination = IpAddr::V4(ip_packet.get_destination());

                   // Blocked IPs check
                   if self.blocked_ips.contains(&source) {
                       return Some(NetworkAlert {
                           device_id: self.device_id.clone(),
                           alert_type: AlertType::UnauthorizedAccess,
                           severity: AlertSeverity::High,
                           description: format!("Traffic from blocked IP: {}", source),
                           source_ip: Some(source.to_string()),
                           destination_ip: Some(destination.to_string()),
                           protocol: Some(ip_packet.get_next_level_protocol().to_string()),
                           port: None,
                           timestamp: Utc::now(),
                       });
                   }

                   // Port scan detection
                   if let Some(tcp_packet) = TcpPacket::new(ip_packet.payload()) {
                       let entry = self.stats.port_scan_attempts
                           .entry(source)
                           .or_insert_with(Vec::new);
                       
                       entry.push(tcp_packet.get_destination());
                       
                       if entry.len() > 10 {
                           return Some(NetworkAlert {
                               device_id: self.device_id.clone(),
                               alert_type: AlertType::Intrusion,
                               severity: AlertSeverity::Critical,
                               description: format!("Possible port scan from {}", source),
                               source_ip: Some(source.to_string()),
                               destination_ip: Some(destination.to_string()),
                               protocol: Some("TCP".to_string()),
                               port: Some(tcp_packet.get_destination()),
                               timestamp: Utc::now(),
                           });
                       }
                   }

                // Malware signature detection
                if self.check_malware_signatures(ip_packet.payload()) {
                    return Some(NetworkAlert {
                        device_id: self.device_id.clone(),
                        alert_type: AlertType::Malware,
                        severity: AlertSeverity::Critical,
                        description: "Malware signature detected".to_string(),
                        source_ip: Some(source.to_string()),
                        destination_ip: Some(destination.to_string()),
                        protocol: Some(ip_packet.get_next_level_protocol().to_string()),
                        port: None,
                        timestamp: Utc::now(),
                });
}
                // Update bandwidth stats
                self.update_bandwidth_stats(ip_packet.payload().len() as u64);

               }
            }
            _ => {}
        }

        None
    }

    fn check_malware_signatures(&self, payload: &[u8]) -> bool {
        for signature in &self.known_malware_signatures {
            if payload.windows(signature.len()).any(|window| window == signature) {
                return true;
            }
        }
        false
    }

    async fn monitor_bandwidth(&mut self, tx: mpsc::Sender<NetworkAlert>) -> Result<(), Box<dyn Error>> {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        
        loop {
            interval.tick().await;
            
            let bandwidth_mbps = self.stats.bandwidth_usage / 1_000_000.0;
            if bandwidth_mbps > 100.0 {  // Alert if over 100 Mbps
                tx.send(NetworkAlert {
                    device_id: self.device_id.clone(),
                    alert_type: AlertType::Bandwidth,
                    severity: AlertSeverity::Medium,
                    description: format!("High bandwidth usage: {:.2} Mbps", bandwidth_mbps),
                    source_ip: None,
                    destination_ip: None,
                    protocol: None,
                    port: None,
                    timestamp: Utc::now(),
                }).await?;
            }
            
            self.stats.bandwidth_usage = 0.0;
        }
    }

    fn update_bandwidth_stats(&mut self, bytes: u64) {
        self.stats.bandwidth_usage += bytes as f64;
        self.stats.packet_count += 1;
    }

    fn detect_anomalies(&self) -> Option<NetworkAlert> {
    const PACKET_THRESHOLD: u64 = 10000;
    const CONN_THRESHOLD: usize = 100;
    const TIME_WINDOW: i64 = 60; // seconds

    // Check for traffic spikes
    if self.stats.packet_count > PACKET_THRESHOLD {
        return Some(NetworkAlert {
            device_id: self.device_id.clone(),
            alert_type: AlertType::Anomaly,
            severity: AlertSeverity::High,
            description: format!("Traffic spike detected: {} packets/min", self.stats.packet_count),
            source_ip: None,
            destination_ip: None,
            protocol: None,
            port: None,
            timestamp: Utc::now(),
        });
    }

    // Check for unusual connection patterns
    for (ip, info) in &self.stats.known_ips {
        // Too many ports accessed
        if info.ports_accessed.len() > 20 {
            return Some(NetworkAlert {
                device_id: self.device_id.clone(),
                alert_type: AlertType::Anomaly,
                severity: AlertSeverity::High,
                description: format!("Unusual port access pattern from IP: {}", ip),
                source_ip: Some(ip.to_string()),
                destination_ip: None,
                protocol: None,
                port: None,
                timestamp: Utc::now(),
            });
        }

        // Sudden burst of data
        if info.bytes_sent > 1_000_000 && // 1MB
           (Utc::now() - info.first_seen).num_seconds() < TIME_WINDOW {
            return Some(NetworkAlert {
                device_id: self.device_id.clone(),
                alert_type: AlertType::Anomaly,
                severity: AlertSeverity::High,
                description: format!("Data burst from IP: {} ({} bytes)", ip, info.bytes_sent),
                source_ip: Some(ip.to_string()),
                destination_ip: None,
                protocol: None,
                port: None,
                timestamp: Utc::now(),
            });
        }
    }

    // Check connection count
    if self.stats.connection_count > CONN_THRESHOLD {
        return Some(NetworkAlert {
            device_id: self.device_id.clone(),
            alert_type: AlertType::Anomaly,
            severity: AlertSeverity::Medium,
            description: format!("High connection count: {}", self.stats.connection_count),
            source_ip: None,
            destination_ip: None,
            protocol: None,
            port: None,
            timestamp: Utc::now(),
        });
    }

    None
  }
}

fn load_malware_signatures() -> Vec<Vec<u8>> {
    vec![
        // Common malware header patterns
        vec![0x4D, 0x5A], // DOS MZ header
        vec![0x7F, 0x45, 0x4C, 0x46], // ELF header
        
        // Known malicious patterns
        vec![0x68, 0x74, 0x74, 0x70, 0x3A, 0x2F, 0x2F], // "http://"
        vec![0x77, 0x73, 0x32, 0x5F], // WinSock API calls
        
        // Ransomware patterns
        vec![0x2E, 0x65, 0x6E, 0x63, 0x72, 0x79, 0x70, 0x74], // ".encrypt"
        vec![0x2E, 0x6C, 0x6F, 0x63, 0x6B, 0x65, 0x64], // ".locked"
        
        // Botnet command patterns
        vec![0x43, 0x4D, 0x44, 0x3A], // "CMD:"
        vec![0x42, 0x4F, 0x54, 0x3A]  // "BOT:"
    ]
}