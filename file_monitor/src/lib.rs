use std::path::{Path, PathBuf};
use std::io::Read;
use std::error::Error;
use zip::ZipArchive;
use tokio::sync::mpsc;
use notify::Event;
use notify::RecursiveMode;
use notify::Watcher;
use calamine::{open_workbook, Reader, Xlsx};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Alert {
    pub device_id: String,
    pub file_path: String,
    pub pattern_type: String,
    pub matched_content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct Communication {
    alerts: std::sync::Arc<tokio::sync::Mutex<Vec<Alert>>>,
    device_id: String,
    api_endpoint: String,
    client: reqwest::Client,
}

impl Communication {
    pub fn new(device_id: String, api_endpoint: String) -> Self {
        Self {
            alerts: std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new())),
            device_id,
            api_endpoint,
            client: reqwest::Client::new(),
        }
    }

    pub async fn send_alert(&self, alert: Alert) -> Result<(), Box<dyn Error>> {
        let mut alerts = self.alerts.lock().await;
        println!("⚠️ Alert: Found {} in file {}: {}", 
            alert.pattern_type, 
            alert.file_path, 
            alert.matched_content
        );

        let response = self.client
            .post(&format!("{}/alerts", self.api_endpoint))
            .json(&alert)
            .send()
            .await?;

        if !response.status().is_success() {
            eprintln!("Failed to send alert to server: {}", response.status());
            self.store_failed_alert(&alert).await?;
        }

        alerts.push(alert);
        Ok(())
    }

    async fn store_failed_alert(&self, alert: &Alert) -> Result<(), Box<dyn Error>> {
        let failed_alerts_dir = Path::new("failed_alerts");
        tokio::fs::create_dir_all(failed_alerts_dir).await?;
        
        let filename = format!("alert_{}_{}.json", 
            self.device_id,
            chrono::Utc::now().timestamp()
        );
        let path = failed_alerts_dir.join(filename);
        
        tokio::fs::write(
            path,
            serde_json::to_string_pretty(&alert)?
        ).await?;

        Ok(())
    }
}

struct ContentScanner {
    infer: infer::Infer,
}

impl ContentScanner {
    fn new() -> Self {
        Self { infer: infer::Infer::new() }
    }

    async fn scan(&self, path: &Path) -> Result<Vec<String>, Box<dyn Error>> {
        let kind = self.infer.get_from_path(path)?;
        
        match kind.map(|k| k.mime_type()) {
            Some("application/pdf") => self.scan_pdf(path).await,
            Some("application/xlsx") => self.scan_excel(path).await,
            Some("application/zip") => self.scan_zip(path).await,
            _ => self.scan_text(path).await,
        }
    }

    async fn scan_zip(&self, path: &Path) -> Result<Vec<String>, Box<dyn Error>> {
        let file = std::fs::File::open(path)?;
        let mut archive = ZipArchive::new(file)?;
        let mut text = Vec::new();

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            text.push(content);
        }

        Ok(text)
    }

    async fn scan_excel(&self, path: &Path) -> Result<Vec<String>, Box<dyn Error>> {
        let mut workbook: Xlsx<_> = open_workbook(path)?;
        let mut text = Vec::new();
        
        let sheet_names = workbook.sheet_names().to_owned();
        for name in sheet_names {
            if let Some(Ok(range)) = workbook.worksheet_range(&name) {
                for row in range.rows() {
                    let row_text: String = row.iter()
                        .map(|cell| cell.to_string())
                        .collect::<Vec<String>>()
                        .join(" ");
                    text.push(row_text);
                }
            }
        }

        Ok(text)
    }

    async fn scan_pdf(&self, path: &Path) -> Result<Vec<String>, Box<dyn Error>> {
        let doc = lopdf::Document::load(path)?;
        let mut text = Vec::new();

        for page_num in doc.get_pages().keys() {
            if let Ok(content) = doc.extract_text(&[*page_num]) {
                text.push(content);
            }
        }

        Ok(text)
    }

    async fn scan_text(&self, path: &Path) -> Result<Vec<String>, Box<dyn Error>> {
        Ok(vec![std::fs::read_to_string(path)?])
    }
}

pub struct FileMonitor {
    comm: Communication,
    patterns: Vec<regex::Regex>,
    content_scanner: ContentScanner,
}

impl FileMonitor {
    pub fn new(comm: Communication) -> Self {
        let patterns = vec![
            regex::Regex::new(r"\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b").unwrap(),
            regex::Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap(),
            regex::Regex::new(r"\b\d{4}[- ]?\d{4}[- ]?\d{4}[- ]?\d{4}\b").unwrap(),
            regex::Regex::new(r"(?i)password.*=.*").unwrap(),
            regex::Regex::new(r"\b(?:\d[ -]*?){13,16}\b").unwrap(),
            regex::Regex::new(r"(?i)(api[_-]?key|secret[_-]?key).*=.*").unwrap(),
        ];

        Self {
            comm,
            patterns,
            content_scanner: ContentScanner::new(),
        }
    }

    pub async fn start_monitoring(&self, path: &Path) -> Result<(), Box<dyn Error>> {
        println!("Starting file monitor for device: {}", self.comm.device_id);
        let (tx, mut rx) = mpsc::channel(100);

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
            if let Ok(event) = res {
                let _ = tx.blocking_send(event);
            }
        })?;

        watcher.watch(path, RecursiveMode::Recursive)?;
        println!("Monitoring directory: {:?}", path);

        while let Some(event) = rx.recv().await {
            if let notify::EventKind::Create(_) | notify::EventKind::Modify(_) = event.kind {
                for path_buf in event.paths {
                    let path = path_buf.clone();
                    if let Err(e) = self.scan_file(&path_buf).await {
                        eprintln!("Error scanning file {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(())
    }

    async fn scan_file(&self, path: &PathBuf) -> Result<(), Box<dyn Error>> {
        println!("Scanning file: {:?}", path);

        let contents = self.content_scanner.scan(path).await?;
        
        for content in contents {
            for pattern in &self.patterns {
                if let Some(matched) = pattern.find(&content) {
                    self.comm.send_alert(Alert {
                        device_id: self.comm.device_id.clone(),
                        file_path: path.to_string_lossy().to_string(),
                        pattern_type: pattern.to_string(),
                        matched_content: matched.as_str().to_string(),
                        timestamp: chrono::Utc::now(),
                    }).await?;
                }
            }
        }
        
        Ok(())
    }
}