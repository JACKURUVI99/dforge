use anyhow::Result;
use std::path::Path;
use dforge_core::diff::{diff_colored, LineKind};

pub fn cmd_diff(cwd: &Path, file: Option<&Path>) -> Result<()> {
    // Show diff between working directory and last commit
    // For now: show all modified text files
    let search_path = file.map(|f| cwd.join(f)).unwrap_or_else(|| cwd.to_path_buf());

    if search_path.is_file() {
        show_file_diff(&search_path)?;
    } else {
        // Walk directory
        walk_and_diff(cwd, &search_path)?;
    }
    Ok(())
}

fn walk_and_diff(repo_root: &Path, dir: &Path) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') { continue; }
        if path.is_dir() {
            walk_and_diff(repo_root, &path)?;
        } else if is_text(&name) {
            show_file_diff(&path)?;
        }
    }
    Ok(())
}

fn show_file_diff(path: &Path) -> Result<()> {
    // For a real diff: compare working file against last committed version
    // Here we show the file as entirely new (no previous commit comparison yet)
    let content = std::fs::read_to_string(path)?;
    if content.is_empty() { return Ok(()); }

    let lines = diff_colored("", &content);
    let has_changes = lines.iter().any(|l| l.kind != LineKind::Context);
    if !has_changes { return Ok(()); }

    println!("\x1b[1mdiff -- {}\x1b[0m", path.display());
    for line in lines {
        match line.kind {
            LineKind::Added   => println!("\x1b[32m+{}\x1b[0m", line.text),
            LineKind::Removed => println!("\x1b[31m-{}\x1b[0m", line.text),
            LineKind::Context => println!(" {}", line.text),
        }
    }
    Ok(())
}

fn is_text(name: &str) -> bool {
    let exts = ["rs","py","js","ts","go","md","txt","toml","json","sh","c","cpp","h","sol"];
    if let Some(ext) = name.rsplit('.').next() {
        exts.contains(&ext)
    } else { false }
}
