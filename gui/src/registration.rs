use eframe::egui;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct RegistrationForm {
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
    registration_status: Option<RegistrationStatus>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RegistrationResponse {
    pub device_id: String,
    pub api_key: String,
    pub status: String,
}

#[derive(Serialize, Deserialize, Clone)]
enum RegistrationStatus {
    Success(String),  // device_id
    Error(String),    // error message
    InProgress,
}

impl RegistrationForm {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            os: std::env::consts::OS.to_string(),
            features: vec!["DLP".to_string(), "EDR".to_string()],
            device_name: String::new(),
            organization: String::new(),
            environment: String::new(),
            location: String::new(),
            admin_email: String::new(),
            policy_group: String::new(),
            license_key: String::new(),
            registration_status: None,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> bool {
        let mut completed = false;
        
        ui.heading("Agent Registration");
        self.show_essential_fields(ui);
        self.show_optional_fields(ui);

        if let Some(status) = &self.registration_status {
            match status {
                RegistrationStatus::Success(device_id) => {
                    ui.colored_label(egui::Color32::GREEN, 
                        format!("Registration successful! Device ID: {}", device_id));
                    completed = true;
                }
                RegistrationStatus::Error(msg) => {
                    ui.colored_label(egui::Color32::RED, 
                        format!("Registration failed: {}", msg));
                }
                RegistrationStatus::InProgress => {
                    ui.spinner();
                    ui.label("Registering...");
                }
            }
        }

        if ui.button("Register").clicked() {
            let form_data = self.clone();
            self.registration_status = Some(RegistrationStatus::InProgress);

            let task = async move {
                match form_data.submit_registration().await {
                    Ok(response) => {
                        if let Err(e) = form_data.save_credentials(&response).await {
                            RegistrationStatus::Error(e.to_string())
                        } else {
                            RegistrationStatus::Success(response.device_id)
                        }
                    }
                    Err(e) => RegistrationStatus::Error(e.to_string()),
                }
            };

            let _ = tokio::spawn(task);
        }

        completed
    }

    fn show_essential_fields(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading("Essential Information");
            
            ui.add(egui::TextEdit::singleline(&mut self.name)
                .hint_text("Agent Name")
                .desired_width(f32::INFINITY));
            
            ui.add(egui::TextEdit::singleline(&mut self.device_name)
                .hint_text("Device Name")
                .desired_width(f32::INFINITY));
            
            ui.add(egui::TextEdit::singleline(&mut self.organization)
                .hint_text("Organization Name")
                .desired_width(f32::INFINITY));
            
            ui.add(egui::TextEdit::singleline(&mut self.environment)
                .hint_text("Environment (production/staging/development)")
                .desired_width(f32::INFINITY));
            
            ui.add(egui::TextEdit::singleline(&mut self.admin_email)
                .hint_text("Admin Email")
                .desired_width(f32::INFINITY));
            
            ui.add(egui::TextEdit::singleline(&mut self.license_key)
                .hint_text("License Key")
                .desired_width(f32::INFINITY));
        });
    }

    fn show_optional_fields(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading("Optional Information");
            
            ui.add(egui::TextEdit::singleline(&mut self.location)
                .hint_text("Location")
                .desired_width(f32::INFINITY));
            
            ui.add(egui::TextEdit::singleline(&mut self.policy_group)
                .hint_text("Policy Group")
                .desired_width(f32::INFINITY));
        });
    }

    async fn submit_registration(&self) -> Result<RegistrationResponse, Box<dyn Error + Send + Sync>> {
        let client = reqwest::Client::new();
        let response = client
            .post("https://backend-security-solution.onrender.com/api/agents/register")
            .json(&self)
            .send()
            .await?
            .json::<RegistrationResponse>()
            .await?;

        Ok(response)
    }

    async fn save_credentials(&self, response: &RegistrationResponse) -> Result<(), Box<dyn Error + Send + Sync>> {
        let config = serde_json::json!({
            "device_id": response.device_id,
            "api_key": response.api_key,
            "registration_data": self
        });

        tokio::fs::write(
            "agent_config.json",
            serde_json::to_string_pretty(&config)?
        ).await?;

        Ok(())
    }

    pub fn is_registered() -> bool {
        std::path::Path::new("agent_config.json").exists()
    }
}