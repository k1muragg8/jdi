use anyhow::Result;
use std::path::Path;

pub fn save_markdown_report(directory: &str, filename: &str, content: &str) -> Result<String> {
    let file_path = Path::new(directory).join(filename);
    crate::storage::safe_write(&file_path, content)?;
    Ok(file_path.to_string_lossy().to_string())
}
