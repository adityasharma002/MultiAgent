
# File Monitoring and Alert System

This is a file monitoring system that scans files in a specified directory for specific patterns 
(e.g., emails, credit card numbers, API keys). When a pattern is detected, the system generates an alert and sends it to a remote API endpoint. 
If the alert fails to send, it is stored locally for later retry.

## Key Features

1. **File Monitoring**: Watches a directory for file creation and modification events.
2. **Content Scanning**: Scans files of various types (PDF, Excel, ZIP, plain text) for sensitive information.
3. **Pattern Matching**: Uses regular expressions to detect sensitive data like emails, credit card numbers, and API keys.
4. **Alert System**: Sends alerts to a remote API endpoint when sensitive data is detected.
5. **Error Handling**: Stores failed alerts locally for retry.

## Components

### 1. **Communication Module**
- **Purpose**: Handles sending alerts to a remote API and stores failed alerts locally.
- **Key Methods**:
  - `send_alert`: Sends an alert to the API endpoint.
  - `store_failed_alert`: Stores failed alerts in a local directory.

### 2. **Content Scanner**
- **Purpose**: Scans files of different formats (PDF, Excel, ZIP, plain text) and extracts their content.
- **Key Methods**:
  - `scan`: Determines the file type and delegates to the appropriate scanner.
  - `scan_pdf`, `scan_excel`, `scan_zip`, `scan_text`: Extract content from specific file types.

### 3. **File Monitor**
- **Purpose**: Monitors a directory for file changes and triggers scans when files are created or modified.
- **Key Methods**:
  - `start_monitoring`: Starts monitoring the specified directory.
  - `scan_file`: Scans a file for sensitive data using predefined patterns.

## Flow

1. **Initialization**:
   - The `FileMonitor` is initialized with a `Communication` instance and a set of predefined patterns.
   - The `ContentScanner` is initialized to handle file content extraction.

2. **Monitoring**:
   - The `FileMonitor` starts watching the specified directory for file creation and modification events.
   - When a file event is detected, the file is scanned for sensitive data.

3. **Scanning**:
   - The `ContentScanner` extracts the content of the file based on its type.
   - The extracted content is matched against predefined patterns.

4. **Alerting**:
   - If a pattern match is found, an `Alert` is generated and sent to the remote API via the `Communication` module.
   - If the alert fails to send, it is stored locally for later retry.

## Usage

1. **Setup**:
   - Ensure all dependencies are installed (`tokio`, `notify`, `calamine`, `lopdf`, `reqwest`, `serde`, etc.).
   - Configure the `api_endpoint` and `device_id` in the `Communication` module.

2. **Run**:
   - Start the file monitor by calling `start_monitoring` with the directory path to monitor.

```rust
let comm = Communication::new("device_id".to_string(), "http://api.example.com".to_string());
let monitor = FileMonitor::new(comm);
monitor.start_monitoring(Path::new("/path/to/monitor")).await?;
```

## Error Handling

- Failed alerts are stored in a `failed_alerts` directory for later retry.
- Errors during file scanning or alert sending are logged to the console.

## Dependencies

- `tokio`: Asynchronous runtime.
- `notify`: File system event monitoring.
- `calamine`: Excel file parsing.
- `lopdf`: PDF file parsing.
- `reqwest`: HTTP client for sending alerts.
- `serde`: Serialization and deserialization of alerts.

---

