use std::fs;
use std::path::Path;

use super::parser;
use super::Config;

pub fn load_config(path: &Path) -> Result<Config, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let base_dir = path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
    parser::parse_config(&content, &base_dir)
}