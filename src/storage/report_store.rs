use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn save_markdown_report(directory: &str, filename: &str, content: &str) -> Result<String> {
    let dir_path = Path::new(directory);
    if !dir_path.exists() {
        fs::create_dir_all(dir_path)
            .context(format!("Failed to create reports directory: {}", directory))?;
    }

    let file_path = dir_path.join(filename);
    fs::write(&file_path, content)
        .context(format!("Failed to write report file: {:?}", file_path))?;

    Ok(file_path.to_string_lossy().to_string())
}
