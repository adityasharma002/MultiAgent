use serde::{Deserialize, Serialize};
use reqwest::Client;
use tokio::fs;
use std::error::Error;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RegistrationRequest {
    pub name: String,
    pub os: String,
    pub features: Vec<String>,
    pub device_name: String,
    pub organization: String,
    pub environment: String,
    pub location: String,
    pub admin_email: String,
    pub policy_group: String,
    pub license_key: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RegistrationResponse {
    pub message: String,
    pub agent: AgentData
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentData {
    pub id: i32,
    pub name: String,
    pub os: String,
    pub status: String,
    pub features: Vec<String>,
    pub device_name: String,
    pub organization: String,
    pub environment: String,
    pub location: String,
    pub admin_email: String,
    pub policy_group: String,
    pub license_key: String,
    pub last_seen: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentConfig {
    pub device_id: String,
    pub api_key: String,
    pub registration_data: RegistrationRequest,
}

pub struct RegistrationService {
    client: Client,
    api_endpoint: String,
}

impl RegistrationService {
    pub fn new(api_endpoint: String) -> Self {
        Self {
            client: Client::new(),
            api_endpoint,
        }
    }

    pub async fn register(&self, request: RegistrationRequest) -> Result<RegistrationResponse, Box<dyn Error + Send + Sync>> {
        println!("Attempting registration with payload:");
        println!("{}", serde_json::to_string_pretty(&request)?);
        
        let response = self.client
            .post(&self.api_endpoint)
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        let response_text = response.text().await?;
        println!("Raw API Response: {}", response_text);

        if !status.is_success() {
            return Err(format!("Registration failed: {} - {}", status, response_text).into());
        }

        let reg_response: RegistrationResponse = serde_json::from_str(&response_text)?;
        self.save_config(&request, &reg_response).await?;
        
        Ok(reg_response)
    }

    async fn save_config(&self, request: &RegistrationRequest, response: &RegistrationResponse) -> Result<(), Box<dyn Error + Send + Sync>> {
        let config = AgentConfig {
            device_id: response.agent.id.to_string(),
            api_key: format!("agent_{}", response.agent.id), // Using id as api_key since it's not in response
            registration_data: request.clone(),
        };

        fs::write(
            "agent_config.json",
            serde_json::to_string_pretty(&config)?
        ).await?;

        Ok(())
    }

    pub fn is_registered() -> bool {
        std::path::Path::new("agent_config.json").exists()
    }

    pub async fn load_config() -> Result<AgentConfig, Box<dyn Error + Send + Sync>> {
        let config_str = fs::read_to_string("agent_config.json").await?;
        let config: AgentConfig = serde_json::from_str(&config_str)?;
        Ok(config)
    }
}