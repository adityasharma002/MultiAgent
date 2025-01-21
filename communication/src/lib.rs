pub mod registration; 
use std::error::Error;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Alert {
   pub severity: AlertLevel,
   pub message: String,
   pub source: String,
   pub timestamp: DateTime<Utc>
}


#[derive(Debug, Serialize, Deserialize)]
pub enum AlertLevel {
   Low,
   Medium, 
   High,
   Critical
}

pub struct Communication {
   pub device_id: String,
   pub api_endpoint: String,
}

impl Communication {
   pub fn new(device_id: String, api_endpoint: String) -> Self {
        Self {
            device_id,
            api_endpoint,
        }
    }

   pub fn log_alert(&self, alert: Alert) {
       println!("[ALERT] {:?}: {}", alert.severity, alert.message);
   }
}