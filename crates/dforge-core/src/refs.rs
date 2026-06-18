// Reference management — branches, tags, HEAD
// Refs are just named pointers to commit CIDs
// Stored as plain files: refs/heads/<name>, refs/tags/<name>
// HEAD is a symref: "ref: refs/heads/main"

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::collections::HashMap;

pub struct RefStore {
    root: PathBuf, // repo/.dforge/
}

impl RefStore {
    pub fn new(repo_path: &Path) -> Self {
        Self { root: repo_path.to_path_buf() }
    }

    pub fn init(&self) -> Result<()> {
        std::fs::create_dir_all(self.root.join("refs/heads"))?;
        std::fs::create_dir_all(self.root.join("refs/tags"))?;
        std::fs::create_dir_all(self.root.join("refs/prs"))?;
        // HEAD points to main branch initially
        std::fs::write(self.root.join("HEAD"), "ref: refs/heads/main\n")?;
        Ok(())
    }

    // Resolve HEAD → CID string
    pub fn head_cid(&self) -> Result<Option<String>> {
        let head = std::fs::read_to_string(self.root.join("HEAD"))?;
        let head = head.trim();
        if let Some(refname) = head.strip_prefix("ref: ") {
            self.read_ref(refname)
        } else {
            Ok(Some(head.to_string())) // detached HEAD
        }
    }

    // Get current branch name
    pub fn current_branch(&self) -> Result<String> {
        let head = std::fs::read_to_string(self.root.join("HEAD"))?;
        let head = head.trim();
        if let Some(refname) = head.strip_prefix("ref: refs/heads/") {
            Ok(refname.to_string())
        } else {
            Ok("HEAD (detached)".to_string())
        }
    }

    // Read a ref → CID
    pub fn read_ref(&self, refname: &str) -> Result<Option<String>> {
        let path = self.root.join(refname);
        if !path.exists() { return Ok(None); }
        let cid = std::fs::read_to_string(&path)?.trim().to_string();
        Ok(if cid.is_empty() { None } else { Some(cid) })
    }

    // Write a ref → CID (create or update branch)
    pub fn write_ref(&self, refname: &str, cid: &str) -> Result<()> {
        let path = self.root.join(refname);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, format!("{}\n", cid))?;
        Ok(())
    }

    pub fn create_branch(&self, name: &str, from_cid: &str) -> Result<()> {
        // Validate branch name — no spaces, no special chars except - _ /
        if name.chars().any(|c| c == ' ' || c == '\0') {
            anyhow::bail!("invalid branch name: {}", name);
        }
        self.write_ref(&format!("refs/heads/{}", name), from_cid)
    }

    pub fn delete_branch(&self, name: &str) -> Result<()> {
        let current = self.current_branch()?;
        if current == name {
            anyhow::bail!("cannot delete current branch '{}'", name);
        }
        let path = self.root.join("refs/heads").join(name);
        std::fs::remove_file(path).with_context(|| format!("branch '{}' not found", name))?;
        Ok(())
    }

    pub fn list_branches(&self) -> Result<Vec<String>> {
        let dir = self.root.join("refs/heads");
        if !dir.exists() { return Ok(vec![]); }
        let mut branches = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            if let Some(name) = entry.file_name().to_str() {
                branches.push(name.to_string());
            }
        }
        branches.sort();
        Ok(branches)
    }

    pub fn checkout(&self, branch: &str) -> Result<()> {
        let refname = format!("refs/heads/{}", branch);
        let path = self.root.join(&refname);
        if !path.exists() {
            anyhow::bail!("branch '{}' not found", branch);
        }
        std::fs::write(self.root.join("HEAD"), format!("ref: {}\n", refname))?;
        Ok(())
    }

    // Update HEAD's ref to a new CID (after commit)
    pub fn advance_head(&self, cid: &str) -> Result<()> {
        let head = std::fs::read_to_string(self.root.join("HEAD"))?;
        let head = head.trim();
        if let Some(refname) = head.strip_prefix("ref: ") {
            self.write_ref(refname, cid)?;
        } else {
            // Detached HEAD — update directly
            std::fs::write(self.root.join("HEAD"), format!("{}\n", cid))?;
        }
        Ok(())
    }
}
