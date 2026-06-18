use anyhow::Result;
use std::path::Path;
use dforge_core::Repo;

pub fn cmd_branch_list(cwd: &Path) -> Result<()> {
    let repo = Repo::open(cwd)?;
    let current = repo.refs.current_branch()?;
    let branches = repo.refs.list_branches()?;

    if branches.is_empty() {
        println!("No branches yet. Make a commit first.");
        return Ok(());
    }

    for branch in branches {
        if branch == current {
            println!("\x1b[32m* {}\x1b[0m", branch);
        } else {
            println!("  {}", branch);
        }
    }
    Ok(())
}

pub fn cmd_branch_new(cwd: &Path, name: &str) -> Result<()> {
    let repo = Repo::open(cwd)?;
    let head = repo.refs.head_cid()?
        .ok_or_else(|| anyhow::anyhow!("cannot create branch: no commits yet"))?;

    repo.refs.create_branch(name, &head)?;
    println!("Created branch '{}'", name);
    Ok(())
}

pub fn cmd_branch_delete(cwd: &Path, name: &str) -> Result<()> {
    let repo = Repo::open(cwd)?;
    repo.refs.delete_branch(name)?;
    println!("Deleted branch '{}'", name);
    Ok(())
}

pub fn cmd_checkout(cwd: &Path, branch: &str) -> Result<()> {
    let repo = Repo::open(cwd)?;
    repo.refs.checkout(branch)?;
    println!("Switched to branch '{}'", branch);
    Ok(())
}
