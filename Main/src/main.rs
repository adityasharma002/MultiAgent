use communication::registration::{RegistrationService, RegistrationRequest, AgentConfig};
use file_monitor::FileMonitor;
use std::io::{self, Write};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if !RegistrationService::is_registered() {
        println!("Agent needs to be registered. Starting registration process...");
        register_agent().await?;
    }
    
    let config = RegistrationService::load_config().await?;
    start_monitors(&config).await?;
    Ok(())
}

async fn register_agent() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let registration_service = RegistrationService::new(
        "https://backend-security-solution.onrender.com/api/agents/register".to_string()
    );
    
    let request = get_registration_info()?;
    
    match registration_service.register(request).await {
        Ok(response) => {
            println!("Registration successful!");
            println!("Agent ID: {}", response.agent.id);
            println!("Status: {}", response.agent.status);
            println!("Message: {}", response.message);
            Ok(())
        }
        Err(e) => {
            println!("Registration failed: {}", e);
            Err(e)
        }
    }
}

fn get_registration_info() -> Result<RegistrationRequest, Box<dyn std::error::Error + Send + Sync>> {
    let mut request = RegistrationRequest {
        name: "Test-agent".to_string(),
        os: "Windows".to_string(),
        features: vec!["DLP".to_string(), "EDR".to_string()],
        device_name: String::new(),
        organization: String::new(),
        environment: String::new(),
        location: String::new(),
        admin_email: String::new(),
        policy_group: String::new(),
        license_key: String::new(),
    };

    print!("Enter device name: ");
    io::stdout().flush()?;
    io::stdin().read_line(&mut request.device_name)?;
    request.device_name = request.device_name.trim().to_string();

    print!("Enter organization: ");
    io::stdout().flush()?;
    io::stdin().read_line(&mut request.organization)?;
    request.organization = request.organization.trim().to_string();

    print!("Enter environment (production/staging/development): ");
    io::stdout().flush()?;
    io::stdin().read_line(&mut request.environment)?;
    request.environment = request.environment.trim().to_string();

    print!("Enter location: ");
    io::stdout().flush()?;
    io::stdin().read_line(&mut request.location)?;
    request.location = request.location.trim().to_string();

    print!("Enter admin email: ");
    io::stdout().flush()?;
    io::stdin().read_line(&mut request.admin_email)?;
    request.admin_email = request.admin_email.trim().to_string();

    print!("Enter policy group: ");
    io::stdout().flush()?;
    io::stdin().read_line(&mut request.policy_group)?;
    request.policy_group = request.policy_group.trim().to_string();

    print!("Enter license key: ");
    io::stdout().flush()?;
    io::stdin().read_line(&mut request.license_key)?;
    request.license_key = request.license_key.trim().to_string();

    Ok(request)
}

async fn start_monitors(config: &AgentConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let comm = file_monitor::Communication::new(
        config.device_id.clone(),
        "https://backend-security-solution.onrender.com/api/alerts".to_string()
    );
    
    let file_monitor = FileMonitor::new(comm);
    let path = Path::new("file_monitor/tests");
    
    let file_monitor_handle = tokio::spawn(async move {
        if let Err(e) = file_monitor.start_monitoring(path).await {
            eprintln!("File monitor error: {}", e);
        }
    });

    println!("File monitor started successfully.");
    
    tokio::select! {
        _ = file_monitor_handle => println!("File monitor stopped"),
        _ = tokio::signal::ctrl_c() => {
            println!("Received shutdown signal");
        }
    }

    println!("Shutting down monitors...");
    Ok(())
}