// Commit creation and DAG traversal
// Commit graph is a DAG (directed acyclic graph)
// Each commit → parent(s) → forms the version history

use anyhow::Result;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::object::{CommitData, Object, ObjectStore, TreeEntry};
use dforge_crypto::{ContentId, Identity};

pub struct CommitGraph {
    store: ObjectStore,
}

impl CommitGraph {
    pub fn new(store: ObjectStore) -> Self {
        Self { store }
    }

    // Create a new commit from the current working directory state
    pub fn create_commit(
        &self,
        repo_path: &Path,
        message: &str,
        parent_cids: Vec<String>,
        identity: &Identity,
    ) -> Result<ContentId> {
        // Build tree from working directory
        let tree_cid = self.build_tree(repo_path)?;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        let author = identity.node_id.to_hex();

        // Sign the commit data
        let sign_payload = format!("{}|{}|{}|{}",
            tree_cid, parent_cids.join(","), message, timestamp);
        let signature = hex::encode(identity.sign(sign_payload.as_bytes()));

        let commit_data = CommitData {
            tree_cid,
            parent_cids,
            message: message.to_string(),
            author,
            timestamp,
            signature,
        };

        let obj = Object::commit(&commit_data)?;
        self.store.write(&obj)
    }

    // Recursively build tree object from directory
    fn build_tree(&self, dir: &Path) -> Result<String> {
        let mut entries: Vec<TreeEntry> = Vec::new();

        let read_dir = std::fs::read_dir(dir)?;
        let mut paths: Vec<_> = read_dir
            .filter_map(|e| e.ok())
            .collect();
        paths.sort_by_key(|e| e.file_name());

        for entry in paths {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip .dforge directory and hidden files
            if name.starts_with(".dforge") || name.starts_with(".git") {
                continue;
            }

            let metadata = std::fs::metadata(&path)?;

            if metadata.is_dir() {
                let subtree_cid = self.build_tree(&path)?;
                entries.push(TreeEntry {
                    name,
                    cid: subtree_cid,
                    is_dir: true,
                    size: 0,
                });
            } else if metadata.is_file() {
                let data = std::fs::read(&path)?;
                let size = data.len() as u64;
                let blob_cid = self.store.write_blob(&data)?;
                entries.push(TreeEntry {
                    name,
                    cid: blob_cid.to_hex(),
                    is_dir: false,
                    size,
                });
            }
        }

        let tree_obj = Object::tree(entries)?;
        let cid = self.store.write(&tree_obj)?;
        Ok(cid.to_hex())
    }

    // Walk commit history — BFS from tip, returns (cid_hex, commit) pairs
    pub fn log(&self, tip_cid: &str, limit: usize) -> Result<Vec<(String, CommitData)>> {
        let mut result = Vec::new();
        let mut queue = std::collections::VecDeque::new();
        let mut visited = std::collections::HashSet::new();

        queue.push_back(tip_cid.to_string());

        while let Some(cid_hex) = queue.pop_front() {
            if visited.contains(&cid_hex) || result.len() >= limit { break; }
            visited.insert(cid_hex.clone());

            let cid_bytes = hex::decode(&cid_hex)?;
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&cid_bytes);
            let cid = ContentId(arr);

            let obj = self.store.read(&cid)?;
            let commit: CommitData = serde_json::from_slice(&obj.data)?;

            for parent in &commit.parent_cids {
                queue.push_back(parent.clone());
            }
            result.push((cid_hex, commit));
        }

        Ok(result)
    }

    // Find the Lowest Common Ancestor of two commits
    // Used for 3-way merge base computation
    // BFS from both commits simultaneously — O(n) where n = reachable commits
    pub fn find_merge_base(&self, cid_a: &str, cid_b: &str) -> Result<Option<String>> {
        let ancestors_a = self.ancestors(cid_a)?;
        // Walk b's ancestors until we find one in a's ancestor set
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(cid_b.to_string());
        let mut visited = std::collections::HashSet::new();

        while let Some(cid) = queue.pop_front() {
            if visited.contains(&cid) { continue; }
            visited.insert(cid.clone());

            if ancestors_a.contains(&cid) {
                return Ok(Some(cid));
            }

            if let Ok(commit) = self.read_commit(&cid) {
                for parent in commit.parent_cids {
                    queue.push_back(parent);
                }
            }
        }
        Ok(None)
    }

    fn ancestors(&self, tip: &str) -> Result<std::collections::HashSet<String>> {
        let mut set = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(tip.to_string());
        while let Some(cid) = queue.pop_front() {
            if set.contains(&cid) { continue; }
            set.insert(cid.clone());
            if let Ok(commit) = self.read_commit(&cid) {
                for parent in commit.parent_cids {
                    queue.push_back(parent);
                }
            }
        }
        Ok(set)
    }

    fn read_commit(&self, cid_hex: &str) -> Result<CommitData> {
        let bytes = hex::decode(cid_hex)?;
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        let obj = self.store.read(&ContentId(arr))?;
        Ok(serde_json::from_slice(&obj.data)?)
    }
}
