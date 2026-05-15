use anyhow::{Context, Result};
use camino::Utf8Path;
use std::fs;

/// Write `contents` to `output_root/relpath`, creating parent directories.
#[allow(dead_code)]
pub fn write_html(output_root: &Utf8Path, relpath: &Utf8Path, contents: &str) -> Result<()> {
    let dest = output_root.join(relpath);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create parent dir: {parent}"))?;
    }
    fs::write(&dest, contents).with_context(|| format!("Failed to write: {dest}"))?;
    Ok(())
}

/// Copy `bytes` to `output_root/relpath`, creating parent directories.
#[allow(dead_code)]
pub fn write_bytes(output_root: &Utf8Path, relpath: &Utf8Path, bytes: &[u8]) -> Result<()> {
    let dest = output_root.join(relpath);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create parent dir: {parent}"))?;
    }
    fs::write(&dest, bytes).with_context(|| format!("Failed to write: {dest}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn write_html_creates_nested_dirs() {
        let tmp = TempDir::new().unwrap();
        let root = camino::Utf8Path::from_path(tmp.path()).unwrap();
        let rel = Utf8Path::new("a/b/c.html");
        write_html(root, rel, "<html></html>").unwrap();
        let contents = std::fs::read_to_string(root.join(rel)).unwrap();
        assert_eq!(contents, "<html></html>");
    }
}
