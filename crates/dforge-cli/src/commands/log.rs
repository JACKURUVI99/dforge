use anyhow::Result;
use std::path::Path;
use dforge_core::{Repo, CommitGraph, ObjectStore};

pub fn cmd_log(cwd: &Path, limit: usize) -> Result<()> {
    let repo = Repo::open(cwd)?;
    let head = match repo.refs.head_cid()? {
        Some(cid) => cid,
        None => {
            println!("No commits yet.");
            return Ok(());
        }
    };

    let graph = CommitGraph::new(ObjectStore::new(&repo.dforge_dir));
    let commits = graph.log(&head, limit)?;

    for (cid_hex, commit) in commits {
        let short_cid = &cid_hex[..8];
        let date = format_timestamp(commit.timestamp);
        println!("\x1b[33mcommit {}\x1b[0m", short_cid);
        println!("Author: {}", &commit.author[..16.min(commit.author.len())]);
        println!("Date:   {}", date);
        println!();
        println!("    {}", commit.message);
        println!();
    }
    Ok(())
}

fn format_timestamp(ts: u64) -> String {
    let secs = ts % 60;
    let mins = (ts / 60) % 60;
    let hours = (ts / 3600) % 24;
    let days = ts / 86400;
    format!("day {} {:02}:{:02}:{:02} UTC", days, hours, mins, secs)
}
