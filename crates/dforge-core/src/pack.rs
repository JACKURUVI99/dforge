// Pack file — bundles all objects for a push into one encrypted blob
// Uses Rabin CDC chunking for deduplication before encryption
// Format: [object_count: u32][objects: (cid_32bytes + len_4bytes + data)*]

use anyhow::Result;
use std::collections::HashSet;
use crate::object::{ObjectStore, Object};
use dforge_crypto::ContentId;

pub struct PackFile {
    pub objects: Vec<(ContentId, Vec<u8>)>, // (cid, serialized_object)
}

impl PackFile {
    // Build pack from all objects reachable from a commit CID
    // that are NOT already known to the remote (tracked in remote_cids)
    pub fn build(
        store: &ObjectStore,
        tip_cid: &ContentId,
        known_remote: &HashSet<String>,
    ) -> Result<Self> {
        let mut objects = Vec::new();
        let mut visited: HashSet<String> = HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(tip_cid.to_hex());

        while let Some(hex) = queue.pop_front() {
            if visited.contains(&hex) || known_remote.contains(&hex) {
                continue;
            }
            visited.insert(hex.clone());

            let bytes = hex::decode(&hex)?;
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            let cid = ContentId(arr);

            if let Ok(obj) = store.read(&cid) {
                // Queue linked objects (tree entries, parent commits)
                queue_links(&obj, &mut queue)?;
                objects.push((cid, obj.serialize()));
            }
        }

        Ok(Self { objects })
    }

    // Serialize all objects into a single byte buffer
    // Format: magic(4) + count(4) + [cid(32) + len(4) + data(len)]*
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"DFRG"); // magic
        buf.extend_from_slice(&(self.objects.len() as u32).to_le_bytes());

        for (cid, data) in &self.objects {
            buf.extend_from_slice(cid.as_bytes());
            buf.extend_from_slice(&(data.len() as u32).to_le_bytes());
            buf.extend_from_slice(data);
        }
        buf
    }

    // Deserialize pack file bytes
    pub fn deserialize(buf: &[u8]) -> Result<Self> {
        if buf.len() < 8 { anyhow::bail!("pack too small"); }
        if &buf[..4] != b"DFRG" { anyhow::bail!("invalid pack magic"); }

        let count = u32::from_le_bytes(buf[4..8].try_into()?) as usize;
        let mut objects = Vec::with_capacity(count);
        let mut pos = 8;

        for _ in 0..count {
            if pos + 36 > buf.len() { anyhow::bail!("pack truncated"); }
            let mut cid_bytes = [0u8; 32];
            cid_bytes.copy_from_slice(&buf[pos..pos+32]);
            pos += 32;
            let len = u32::from_le_bytes(buf[pos..pos+4].try_into()?) as usize;
            pos += 4;
            if pos + len > buf.len() { anyhow::bail!("pack data truncated"); }
            let data = buf[pos..pos+len].to_vec();
            pos += len;
            objects.push((ContentId(cid_bytes), data));
        }

        Ok(Self { objects })
    }

    // Unpack into object store
    pub fn unpack_into(&self, store: &ObjectStore) -> Result<usize> {
        let mut count = 0;
        for (cid, data) in &self.objects {
            if !store.exists(cid) {
                let obj = Object::deserialize(data)?;
                store.write(&obj)?;
                count += 1;
            }
        }
        Ok(count)
    }

    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    pub fn total_bytes(&self) -> usize {
        self.objects.iter().map(|(_, d)| d.len()).sum()
    }
}

fn queue_links(obj: &Object, queue: &mut std::collections::VecDeque<String>) -> Result<()> {
    use crate::object::{ObjectKind, TreeEntry, CommitData};

    match obj.kind {
        ObjectKind::Tree => {
            let entries: Vec<TreeEntry> = serde_json::from_slice(&obj.data)?;
            for e in entries {
                queue.push_back(e.cid);
            }
        }
        ObjectKind::Commit => {
            let commit: CommitData = serde_json::from_slice(&obj.data)?;
            queue.push_back(commit.tree_cid);
            for p in commit.parent_cids {
                queue.push_back(p);
            }
        }
        _ => {}
    }
    Ok(())
}
