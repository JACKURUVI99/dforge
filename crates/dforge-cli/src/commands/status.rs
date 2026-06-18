use anyhow::Result;
use std::path::Path;
use dforge_core::Repo;

pub fn cmd_status(cwd: &Path) -> Result<()> {
    let repo = Repo::open(cwd)?;
    let status = repo.status()?;

    println!("On branch \x1b[1m{}\x1b[0m", status.branch);
    match &status.head_cid {
        Some(cid) => println!("HEAD: {}", &cid[..12]),
        None => println!("No commits yet"),
    }

    if let Some(cid) = &repo.config.ipfs_cid {
        println!("IPFS: {}", &cid[..16]);
    } else {
        println!("IPFS: not pushed yet");
    }

    println!("\nBranches: {}", status.branches.join(", "));
    println!("\nRun 'dforge diff' to see changes");
    println!("Run 'dforge commit -m \"message\"' to commit");
    println!("Run 'dforge push' to push to IPFS");
    Ok(())
}
