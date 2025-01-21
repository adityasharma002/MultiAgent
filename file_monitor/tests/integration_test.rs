pub struct Alert {
    pub file_path: String,
    pub pattern_type: String,
    pub matched_content: String,
}

pub struct Communication {
    pub alerts: std::sync::Arc<tokio::sync::Mutex<Vec<Alert>>>,
}

impl Communication {
    pub fn new() -> Self {
        Self {
            alerts: std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new())),
        }
    }

    pub async fn send_alert(&self, alert: Alert) {
        let mut alerts = self.alerts.lock().await;
        println!("⚠️ Alert: Found {} in file {}: {}", 
            alert.pattern_type, 
            alert.file_path, 
            alert.matched_content
        );
        alerts.push(alert);
    }
}

#[tokio::test]
async fn test_file_monitor_alerts() {
    let test_dir = Path::new("test_files");
    fs::create_dir_all(test_dir).unwrap();

    // Create test file with sensitive data
    fs::write(
        test_dir.join("sensitive.txt"),
        "Email: test@example.com\nSSN: 123-45-6789\nAPI_KEY=secretkey123"
    ).unwrap();

    let comm = Communication::new();
    let alerts_handle = comm.alerts.clone();
    let file_monitor = FileMonitor::new(comm);

    let monitor_handle = tokio::spawn(async move {
        file_monitor.start_monitoring(test_dir).await.unwrap();
    });

    // Wait briefly for processing
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Check alerts
    let alerts = alerts_handle.lock().await;
    assert!(!alerts.is_empty(), "No alerts were generated!");

    // Cleanup
    fs::remove_dir_all(test_dir).unwrap();
    monitor_handle.abort();
}