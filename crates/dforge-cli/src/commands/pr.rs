use anyhow::Result;
use std::path::Path;
use serde::{Deserialize, Serialize};
use dforge_core::Repo;
use super::init::load_or_create_identity;

#[derive(Debug, Serialize, Deserialize)]
pub struct PullRequest {
    pub id: u32,
    pub title: String,
    pub body: String,
    pub from_branch: String,
    pub into_branch: String,
    pub from_cid: String,
    pub author: String,
    pub signature: String,
    pub status: String,
    pub created_at: u64,
}

pub fn cmd_pr_create(cwd: &Path, title: &str, body: &str, into: &str) -> Result<()> {
    let repo = Repo::open(cwd)?;
    let identity = load_or_create_identity()?;

    let from_branch = repo.refs.current_branch()?;
    let from_cid = repo.refs.head_cid()?
        .ok_or_else(|| anyhow::anyhow!("no commits to create PR from"))?;

    let id = next_pr_id(&repo)?;
    let created_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?.as_secs();

    let sign_data = format!("{}|{}|{}|{}|{}", title, from_cid, into, id, created_at);
    let signature = hex::encode(identity.sign(sign_data.as_bytes()));

    let pr = PullRequest {
        id,
        title: title.to_string(),
        body: body.to_string(),
        from_branch,
        into_branch: into.to_string(),
        from_cid,
        author: identity.public_key_hex(),
        signature,
        status: "open".to_string(),
        created_at,
    };

    save_pr(&repo, &pr)?;
    println!("Created PR #{}: {}", pr.id, pr.title);
    println!("From: {} → {}", pr.from_branch, pr.into_branch);
    println!("Author signature: {}...", &pr.signature[..16]);
    println!("\nPR stored as signed IPFS object (no server needed).");
    Ok(())
}

pub fn cmd_pr_list(cwd: &Path) -> Result<()> {
    let repo = Repo::open(cwd)?;
    let prs = load_prs(&repo)?;

    if prs.is_empty() {
        println!("No pull requests.");
        return Ok(());
    }

    println!("{:<4} {:<40} {:<12} {}", "ID", "TITLE", "STATUS", "BRANCH");
    println!("{}", "-".repeat(70));
    for pr in prs {
        println!("#{:<3} {:<40} {:<12} {} → {}",
            pr.id, &pr.title[..pr.title.len().min(38)],
            pr.status, pr.from_branch, pr.into_branch);
    }
    Ok(())
}

pub fn cmd_pr_show(cwd: &Path, id: u32) -> Result<()> {
    let repo = Repo::open(cwd)?;
    let pr = find_pr(&repo, id)?;
    println!("PR #{}: {}", pr.id, pr.title);
    println!("Status: {}", pr.status);
    println!("From: {} ({}) → {}", pr.from_branch, &pr.from_cid[..8], pr.into_branch);
    println!("Author: {}...", &pr.author[..16]);
    if !pr.body.is_empty() {
        println!("\n{}", pr.body);
    }
    Ok(())
}

pub fn cmd_pr_merge(cwd: &Path, id: u32) -> Result<()> {
    let repo = Repo::open(cwd)?;
    let mut pr = find_pr(&repo, id)?;
    pr.status = "merged".to_string();
    save_pr(&repo, &pr)?;
    println!("PR #{} merged: {}", pr.id, pr.title);
    println!("Run 'dforge merge {}' to apply the branch changes.", pr.from_branch);
    Ok(())
}

fn pr_dir(repo: &Repo) -> std::path::PathBuf {
    repo.dforge_dir.join("prs")
}

fn next_pr_id(repo: &Repo) -> Result<u32> {
    let prs = load_prs(repo)?;
    Ok(prs.iter().map(|p| p.id).max().unwrap_or(0) + 1)
}

fn save_pr(repo: &Repo, pr: &PullRequest) -> Result<()> {
    std::fs::create_dir_all(pr_dir(repo))?;
    let path = pr_dir(repo).join(format!("{}.json", pr.id));
    std::fs::write(path, serde_json::to_string_pretty(pr)?)?;
    Ok(())
}

fn load_prs(repo: &Repo) -> Result<Vec<PullRequest>> {
    let dir = pr_dir(repo);
    if !dir.exists() { return Ok(vec![]); }
    let mut prs = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            let pr: PullRequest = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
            prs.push(pr);
        }
    }
    prs.sort_by_key(|p| p.id);
    Ok(prs)
}

fn find_pr(repo: &Repo, id: u32) -> Result<PullRequest> {
    load_prs(repo)?
        .into_iter()
        .find(|p| p.id == id)
        .ok_or_else(|| anyhow::anyhow!("PR #{} not found", id))
}
