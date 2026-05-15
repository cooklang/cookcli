use anyhow::{Context, Result};
use camino::Utf8Path;
use std::fs;

/// Write `contents` to `output_root/relpath`, creating parent directories.
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
pub fn write_bytes(output_root: &Utf8Path, relpath: &Utf8Path, bytes: &[u8]) -> Result<()> {
    let dest = output_root.join(relpath);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create parent dir: {parent}"))?;
    }
    fs::write(&dest, bytes).with_context(|| format!("Failed to write: {dest}"))?;
    Ok(())
}

/// Copy every file in the rust-embed `StaticFiles` to `output_root/static/<path>`.
pub fn copy_static_assets(output_root: &Utf8Path) -> Result<usize> {
    let mut count = 0;
    for path in crate::server::StaticFiles::iter() {
        let rel = Utf8Path::new("static").join(path.as_ref());
        let file = crate::server::StaticFiles::get(path.as_ref())
            .with_context(|| format!("Embedded file vanished: {path}"))?;
        write_bytes(output_root, &rel, &file.data)?;
        count += 1;
    }
    Ok(count)
}

/// Copy a single source file into `output_root/api/static/<relpath>`.
pub fn copy_image(
    output_root: &Utf8Path,
    source_root: &Utf8Path,
    abs_image: &Utf8Path,
) -> Result<()> {
    let rel = abs_image
        .strip_prefix(source_root)
        .with_context(|| format!("Image {abs_image} not under source {source_root}"))?;
    let dest = output_root.join("api/static").join(rel);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create parent dir: {parent}"))?;
    }
    fs::copy(abs_image, &dest).with_context(|| format!("Failed to copy {abs_image} -> {dest}"))?;
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

    #[test]
    fn copy_static_assets_writes_known_file() {
        let tmp = TempDir::new().unwrap();
        let root = camino::Utf8Path::from_path(tmp.path()).unwrap();
        let count = copy_static_assets(root).unwrap();
        assert!(count > 0, "should copy at least one static asset");
        assert!(root.join("static/css/output.css").is_file());
    }
}
