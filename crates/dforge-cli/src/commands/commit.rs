use anyhow::Result;
use std::path::Path;
use dforge_core::{Repo, CommitGraph, ObjectStore};
use super::init::load_or_create_identity;

pub fn cmd_commit(cwd: &Path, message: &str) -> Result<()> {
    let repo = Repo::open(cwd)?;
    let identity = load_or_create_identity()?;

    // Get current HEAD as parent
    let parent_cids = repo.refs.head_cid()?
        .map(|c| vec![c])
        .unwrap_or_default();

    let graph = CommitGraph::new(ObjectStore::new(&repo.dforge_dir));
    let commit_cid = graph.create_commit(&repo.path, message, parent_cids, &identity)?;

    // Advance HEAD ref to new commit
    repo.refs.advance_head(&commit_cid.to_hex())?;

    let branch = repo.refs.current_branch()?;
    println!("[{}] {}", &commit_cid.to_hex()[..8], message);
    println!("Branch: {}", branch);
    Ok(())
}
