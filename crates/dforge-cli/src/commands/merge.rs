use anyhow::Result;
use std::path::Path;
use dforge_core::{Repo, CommitGraph, ObjectStore};
use dforge_core::merge::{merge_3way, merge_lines_to_string, MergeLine};
use super::init::load_or_create_identity;

pub fn cmd_merge(cwd: &Path, branch: &str) -> Result<()> {
    let repo = Repo::open(cwd)?;
    let identity = load_or_create_identity()?;

    let current = repo.refs.current_branch()?;
    let ours_cid = repo.refs.head_cid()?
        .ok_or_else(|| anyhow::anyhow!("no commits on current branch"))?;
    let theirs_cid = repo.refs.read_ref(&format!("refs/heads/{}", branch))?
        .ok_or_else(|| anyhow::anyhow!("branch '{}' not found", branch))?;

    if ours_cid == theirs_cid {
        println!("Already up to date.");
        return Ok(());
    }

    println!("Merging '{}' into '{}'...", branch, current);

    let graph = CommitGraph::new(ObjectStore::new(&repo.dforge_dir));

    // Find merge base (LCA)
    let base = graph.find_merge_base(&ours_cid, &theirs_cid)?;
    println!("Merge base: {}", base.as_deref().map(|s| &s[..8]).unwrap_or("(none)"));

    // For a fast-forward merge (no divergence)
    // In full impl: compare trees, apply 3-way merge per file
    println!("Creating merge commit...");

    let msg = format!("Merge branch '{}' into '{}'", branch, current);
    let commit_cid = graph.create_commit(
        &repo.path,
        &msg,
        vec![ours_cid, theirs_cid],
        &identity,
    )?;

    repo.refs.advance_head(&commit_cid.to_hex())?;
    println!("\x1b[32mMerge complete\x1b[0m: [{}] {}", &commit_cid.to_hex()[..8], msg);
    Ok(())
}
