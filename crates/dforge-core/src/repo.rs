// Repo — top-level struct that ties everything together
// A repo lives at: <path>/.dforge/

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::object::ObjectStore;
use crate::refs::RefStore;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoConfig {
    pub name: String,
    pub description: String,
    pub owner_pubkey: String, // Ed25519 public key hex
    pub ipfs_cid: Option<String>, // latest pushed CID
    pub eth_address: Option<String>, // owner's Ethereum address
}

pub struct Repo {
    pub path: PathBuf,
    pub dforge_dir: PathBuf,
    pub objects: ObjectStore,
    pub refs: RefStore,
    pub config: RepoConfig,
}

impl Repo {
    // Initialize a new repo at path
    pub fn init(path: &Path, name: &str, owner_pubkey: &str) -> Result<Self> {
        let dforge_dir = path.join(".dforge");
        std::fs::create_dir_all(&dforge_dir)?;

        let objects = ObjectStore::new(&dforge_dir);
        objects.init()?;

        let refs = RefStore::new(&dforge_dir);
        refs.init()?;

        let config = RepoConfig {
            name: name.to_string(),
            description: String::new(),
            owner_pubkey: owner_pubkey.to_string(),
            ipfs_cid: None,
            eth_address: None,
        };

        let config_path = dforge_dir.join("config.json");
        std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;

        println!("Initialized empty DecentraForge repo in {}", dforge_dir.display());

        Ok(Self {
            path: path.to_path_buf(),
            dforge_dir,
            objects,
            refs,
            config,
        })
    }

    // Open existing repo — walks up directory tree to find .dforge
    pub fn open(start: &Path) -> Result<Self> {
        let dforge_dir = find_repo_root(start)
            .context("not a dforge repository (no .dforge directory found)")?;

        let config_path = dforge_dir.join("config.json");
        let config: RepoConfig = serde_json::from_str(&std::fs::read_to_string(&config_path)?)?;

        let objects = ObjectStore::new(&dforge_dir);
        let refs = RefStore::new(&dforge_dir);

        Ok(Self {
            path: dforge_dir.parent().unwrap().to_path_buf(),
            dforge_dir,
            objects,
            refs,
            config,
        })
    }

    pub fn save_config(&self) -> Result<()> {
        let config_path = self.dforge_dir.join("config.json");
        std::fs::write(&config_path, serde_json::to_string_pretty(&self.config)?)?;
        Ok(())
    }

    pub fn status(&self) -> Result<RepoStatus> {
        let branch = self.refs.current_branch()?;
        let head_cid = self.refs.head_cid()?;
        let branches = self.refs.list_branches()?;

        Ok(RepoStatus {
            branch,
            head_cid,
            branches,
            name: self.config.name.clone(),
        })
    }
}

#[derive(Debug)]
pub struct RepoStatus {
    pub branch: String,
    pub head_cid: Option<String>,
    pub branches: Vec<String>,
    pub name: String,
}

// Walk up directories looking for .dforge
fn find_repo_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        let candidate = current.join(".dforge");
        if candidate.exists() && candidate.is_dir() {
            return Some(candidate);
        }
        if !current.pop() { return None; }
    }
}
