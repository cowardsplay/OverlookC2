use std::path::Path;
use std::fs;
use std::fs::OpenOptions;
use anyhow::{Result, anyhow};
use base64::{Engine as _, engine::general_purpose};
use std::io::Write;
use chrono::Utc;

/// Utility functions for the C2 framework

/// Read a file and return its contents as bytes
pub fn read_file(path: &str) -> Result<Vec<u8>> {
    fs::read(path).map_err(|e| anyhow!("Failed to read file {}: {}", path, e))
}

/// Write bytes to a file
pub fn write_file(path: &str, content: &[u8]) -> Result<()> {
    // Ensure directory exists
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    
    fs::write(path, content).map_err(|e| anyhow!("Failed to write file {}: {}", path, e))
}

/// Encode data to base64
pub fn encode_b64(data: &[u8]) -> String {
    general_purpose::STANDARD.encode(data)
}

/// Decode base64 data
pub fn decode_b64(data: &str) -> Result<Vec<u8>> {
    general_purpose::STANDARD.decode(data)
        .map_err(|e| anyhow!("Base64 decode failed: {}", e))
}

/// Generate a random string of specified length
pub fn random_string(length: usize) -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                            abcdefghijklmnopqrstuvwxyz\
                            0123456789)(*&^%$#@!~";
    let mut rng = rand::thread_rng();
    
    (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Check if a file exists
pub fn file_exists(path: &str) -> bool {
    Path::new(path).exists()
}

/// Get file size in bytes
pub fn get_file_size(path: &str) -> Result<u64> {
    let metadata = fs::metadata(path)
        .map_err(|e| anyhow!("Failed to get file metadata for {}: {}", path, e))?;
    Ok(metadata.len())
}

/// Create a temporary file path
pub fn temp_file_path(prefix: &str, extension: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| anyhow!("System time error"))
        .unwrap_or_default()
        .as_millis();
    
    format!("{}_{}.{}", prefix, timestamp, extension)
}

/// Validate file path (basic security check)
pub fn validate_file_path(path: &str) -> Result<()> {
    let path_obj = Path::new(path);
    
    // Check for path traversal attempts
    if path.contains("..") {
        return Err(anyhow!("Path traversal not allowed"));
    }
    
    // Check for absolute paths (optional security measure)
    if path_obj.is_absolute() {
        return Err(anyhow!("Absolute paths not allowed"));
    }
    
    Ok(())
}

/// Get current working directory
pub fn get_current_dir() -> Result<String> {
    std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| anyhow!("Failed to get current directory: {}", e))
}

/// Get environment variable with default
pub fn get_env_var(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

/// Set environment variable
pub fn set_env_var(key: &str, value: &str) -> Result<()> {
    std::env::set_var(key, value);
    Ok(())
}

/// Check if running with elevated privileges (admin/root) - simplified version
pub fn is_elevated() -> bool {
    #[cfg(target_os = "windows")]
    {
        // Simplified check - in production you'd want a more robust implementation
        std::env::var("USERNAME").unwrap_or_default() == "Administrator"
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        use std::process::Command;
        Command::new("id")
            .arg("-u")
            .output()
            .map(|output| {
                String::from_utf8_lossy(&output.stdout).trim() == "0"
            })
            .unwrap_or(false)
    }
}

/// Get system architecture
pub fn get_architecture() -> &'static str {
    std::env::consts::ARCH
}

/// Get operating system
pub fn get_os() -> &'static str {
    std::env::consts::OS
}

/// Format bytes to human readable format
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    format!("{:.2} {}", size, UNITS[unit_index])
}

/// Parse command line arguments into a vector
pub fn parse_command_args(command: &str) -> Vec<String> {
    // Simple command parsing - in production you might want a more robust parser
    command
        .split_whitespace()
        .map(|s| s.to_string())
        .collect()
}

/// Escape special characters in a string
pub fn escape_string(s: &str) -> String {
    s.replace("\\", "\\\\")
        .replace("\"", "\\\"")
        .replace("\n", "\\n")
        .replace("\r", "\\r")
        .replace("\t", "\\t")
}

/// Unescape special characters in a string
pub fn unescape_string(s: &str) -> String {
    s.replace("\\\\", "\\")
        .replace("\\\"", "\"")
        .replace("\\n", "\n")
        .replace("\\r", "\r")
        .replace("\\t", "\t")
}

/// Initialize the logs directory and ensure it exists
pub fn init_logs_directory() -> std::io::Result<()> {
    let logs_dir = Path::new("logs");
    if !logs_dir.exists() {
        fs::create_dir(logs_dir)?;
    }
    Ok(())
}

/// Write a log message to a specific log file in the logs directory
pub fn write_log(component: &str, message: &str) -> std::io::Result<()> {
    init_logs_directory()?;
    
    let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let log_message = format!("[{}] {}\n", timestamp, message);
    
    let log_file = format!("logs/{}.log", component);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)?;
    
    file.write_all(log_message.as_bytes())?;
    file.flush()?;
    Ok(())
}

/// Write a log message with a specific level (INFO, ERROR, DEBUG, etc.)
pub fn write_log_with_level(component: &str, level: &str, message: &str) -> std::io::Result<()> {
    let formatted_message = format!("[{}] {}", level, message);
    write_log(component, &formatted_message)
}

/// Log an info message
pub fn log_info(component: &str, message: &str) {
    if let Err(e) = write_log_with_level(component, "INFO", message) {
        eprintln!("Failed to write log: {}", e);
    }
}

/// Log an error message
pub fn log_error(component: &str, message: &str) {
    if let Err(e) = write_log_with_level(component, "ERROR", message) {
        eprintln!("Failed to write log: {}", e);
    }
}

/// Log a debug message
pub fn log_debug(component: &str, message: &str) {
    if let Err(e) = write_log_with_level(component, "DEBUG", message) {
        eprintln!("Failed to write log: {}", e);
    }
}

/// Log a warning message
pub fn log_warning(component: &str, message: &str) {
    if let Err(e) = write_log_with_level(component, "WARN", message) {
        eprintln!("Failed to write log: {}", e);
    }
}

/// Log a success message
pub fn log_success(component: &str, message: &str) {
    if let Err(e) = write_log_with_level(component, "SUCCESS", message) {
        eprintln!("Failed to write log: {}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_random_string() {
        let s1 = random_string(10);
        let s2 = random_string(10);
        
        assert_eq!(s1.len(), 10);
        assert_eq!(s2.len(), 10);
        assert_ne!(s1, s2);
    }
    
    #[test]
    fn test_encode_decode_b64() {
        let data = b"Hello, World!";
        let encoded = encode_b64(data);
        let decoded = decode_b64(&encoded).unwrap();
        
        assert_eq!(data, decoded.as_slice());
    }
    
    #[test]
    fn test_escape_unescape() {
        let original = "Hello\nWorld\t\"Test\"";
        let escaped = escape_string(original);
        let unescaped = unescape_string(&escaped);
        
        assert_eq!(original, unescaped);
    }
    
    #[test]
    fn test_validate_file_path() {
        assert!(validate_file_path("test.txt").is_ok());
        assert!(validate_file_path("folder/test.txt").is_ok());
        assert!(validate_file_path("../test.txt").is_err());
        assert!(validate_file_path("..\\test.txt").is_err());
    }
} 