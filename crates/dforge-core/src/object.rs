// Merkle DAG object store
// Every object identified by Blake3(content) — O(1) deduplication
// Stored in: ~/.dforge/<repo>/objects/<2-char-prefix>/<rest-of-hash>
// Same layout as Git for compatibility

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use dforge_crypto::ContentId;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ObjectKind {
    Blob,
    Tree,
    Commit,
    PullRequest,
    Issue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Object {
    pub kind: ObjectKind,
    pub data: Vec<u8>,
}

impl Object {
    pub fn blob(data: Vec<u8>) -> Self {
        Self { kind: ObjectKind::Blob, data }
    }

    pub fn tree(entries: Vec<TreeEntry>) -> Result<Self> {
        let data = serde_json::to_vec(&entries)?;
        Ok(Self { kind: ObjectKind::Tree, data })
    }

    pub fn commit(c: &CommitData) -> Result<Self> {
        let data = serde_json::to_vec(c)?;
        Ok(Self { kind: ObjectKind::Commit, data })
    }

    pub fn content_id(&self) -> ContentId {
        // Prefix with kind byte for domain separation
        let mut hasher = dforge_crypto::StreamHasher::new();
        hasher.update(&[self.kind.as_byte()]);
        hasher.update(&self.data);
        hasher.finalize()
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(1 + self.data.len());
        buf.push(self.kind.as_byte());
        buf.extend_from_slice(&self.data);
        buf
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self> {
        if bytes.is_empty() { anyhow::bail!("empty object"); }
        let kind = ObjectKind::from_byte(bytes[0])?;
        Ok(Self { kind, data: bytes[1..].to_vec() })
    }
}

impl ObjectKind {
    fn as_byte(&self) -> u8 {
        match self {
            ObjectKind::Blob       => 1,
            ObjectKind::Tree       => 2,
            ObjectKind::Commit     => 3,
            ObjectKind::PullRequest => 4,
            ObjectKind::Issue      => 5,
        }
    }

    fn from_byte(b: u8) -> Result<Self> {
        match b {
            1 => Ok(ObjectKind::Blob),
            2 => Ok(ObjectKind::Tree),
            3 => Ok(ObjectKind::Commit),
            4 => Ok(ObjectKind::PullRequest),
            5 => Ok(ObjectKind::Issue),
            _ => anyhow::bail!("unknown object kind: {}", b),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeEntry {
    pub name: String,
    pub cid: String,       // hex ContentId
    pub is_dir: bool,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitData {
    pub tree_cid: String,
    pub parent_cids: Vec<String>, // empty for root, 2 for merge commits
    pub message: String,
    pub author: String,           // NodeId hex
    pub timestamp: u64,
    pub signature: String,        // Ed25519 sig over tree_cid+parents+message
}

// Object store — flat file storage with 2-char prefix (like Git)
pub struct ObjectStore {
    root: PathBuf,
}

impl ObjectStore {
    pub fn new(repo_path: &Path) -> Self {
        Self { root: repo_path.join("objects") }
    }

    pub fn init(&self) -> Result<()> {
        std::fs::create_dir_all(&self.root)?;
        Ok(())
    }

    // Write object — O(1) after Blake3 hash computed
    // Returns CID (content identifier)
    pub fn write(&self, obj: &Object) -> Result<ContentId> {
        let cid = obj.content_id();
        let hex = cid.to_hex();
        let dir = self.root.join(&hex[..2]);
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(&hex[2..]);
        if !path.exists() {
            std::fs::write(&path, obj.serialize())?;
        }
        Ok(cid)
    }

    // Read object — O(1) lookup by CID
    pub fn read(&self, cid: &ContentId) -> Result<Object> {
        let hex = cid.to_hex();
        let path = self.root.join(&hex[..2]).join(&hex[2..]);
        let bytes = std::fs::read(&path)
            .with_context(|| format!("object not found: {}", &hex[..12]))?;
        Object::deserialize(&bytes)
    }

    // Check existence without reading — uses filesystem O(1) stat
    pub fn exists(&self, cid: &ContentId) -> bool {
        let hex = cid.to_hex();
        self.root.join(&hex[..2]).join(&hex[2..]).exists()
    }

    // Write raw bytes, return CID
    pub fn write_blob(&self, data: &[u8]) -> Result<ContentId> {
        let obj = Object::blob(data.to_vec());
        self.write(&obj)
    }
}
