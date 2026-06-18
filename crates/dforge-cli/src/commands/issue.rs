use anyhow::Result;
use std::path::Path;
use serde::{Deserialize, Serialize};
use dforge_core::Repo;
use super::init::load_or_create_identity;

#[derive(Debug, Serialize, Deserialize)]
pub struct Issue {
    pub id: u32,
    pub title: String,
    pub body: String,
    pub author: String,
    pub status: String,
    pub created_at: u64,
}

pub fn cmd_issue_new(cwd: &Path, title: &str, body: &str) -> Result<()> {
    let repo = Repo::open(cwd)?;
    let identity = load_or_create_identity()?;
    let id = next_issue_id(&repo)?;
    let created_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?.as_secs();

    let issue = Issue {
        id,
        title: title.to_string(),
        body: body.to_string(),
        author: identity.public_key_hex(),
        status: "open".to_string(),
        created_at,
    };
    save_issue(&repo, &issue)?;
    println!("Created issue #{}: {}", issue.id, issue.title);
    Ok(())
}

pub fn cmd_issue_list(cwd: &Path) -> Result<()> {
    let repo = Repo::open(cwd)?;
    let issues = load_issues(&repo)?;
    if issues.is_empty() { println!("No issues."); return Ok(()); }
    println!("{:<4} {:<45} {}", "ID", "TITLE", "STATUS");
    println!("{}", "-".repeat(60));
    for i in &issues {
        let icon = if i.status == "open" { "\x1b[32m●\x1b[0m" } else { "\x1b[31m✓\x1b[0m" };
        println!("{} #{:<3} {}", icon, i.id, &i.title[..i.title.len().min(44)]);
    }
    Ok(())
}

pub fn cmd_issue_show(cwd: &Path, id: u32) -> Result<()> {
    let repo = Repo::open(cwd)?;
    let issue = find_issue(&repo, id)?;
    println!("Issue #{}: {}", issue.id, issue.title);
    println!("Status: {}  Author: {}...", issue.status, &issue.author[..12]);
    if !issue.body.is_empty() { println!("\n{}", issue.body); }
    Ok(())
}

pub fn cmd_issue_close(cwd: &Path, id: u32) -> Result<()> {
    let repo = Repo::open(cwd)?;
    let mut issue = find_issue(&repo, id)?;
    issue.status = "closed".to_string();
    save_issue(&repo, &issue)?;
    println!("Closed issue #{}: {}", issue.id, issue.title);
    Ok(())
}

fn issue_dir(repo: &Repo) -> std::path::PathBuf { repo.dforge_dir.join("issues") }

fn next_issue_id(repo: &Repo) -> Result<u32> {
    let issues = load_issues(repo)?;
    Ok(issues.iter().map(|i| i.id).max().unwrap_or(0) + 1)
}

fn save_issue(repo: &Repo, issue: &Issue) -> Result<()> {
    std::fs::create_dir_all(issue_dir(repo))?;
    let path = issue_dir(repo).join(format!("{}.json", issue.id));
    std::fs::write(path, serde_json::to_string_pretty(issue)?)?;
    Ok(())
}

fn load_issues(repo: &Repo) -> Result<Vec<Issue>> {
    let dir = issue_dir(repo);
    if !dir.exists() { return Ok(vec![]); }
    let mut issues = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            let i: Issue = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
            issues.push(i);
        }
    }
    issues.sort_by_key(|i| i.id);
    Ok(issues)
}

fn find_issue(repo: &Repo, id: u32) -> Result<Issue> {
    load_issues(repo)?
        .into_iter()
        .find(|i| i.id == id)
        .ok_or_else(|| anyhow::anyhow!("issue #{} not found", id))
}
