use anyhow::Result;
use std::path::Path;

struct PublicRepo {
    name: &'static str,
    description: &'static str,
    cid: &'static str,
    owner: &'static str,
    stars: u32,
}

const DEMO_REGISTRY: &[PublicRepo] = &[
    PublicRepo {
        name: "dforge",
        description: "Serverless Git on IPFS + Blockchain",
        cid: "QmDecentraForge1111111111111111111111111111111",
        owner: "12D3KooW...",
        stars: 42,
    },
    PublicRepo {
        name: "rust-ipfs-demo",
        description: "Minimal IPFS node in Rust",
        cid: "QmRustIPFS22222222222222222222222222222222222",
        owner: "12D3KooWABC...",
        stars: 17,
    },
    PublicRepo {
        name: "sss-benchmark",
        description: "Shamir Secret Sharing benchmarks in GF(2^8)",
        cid: "QmSSS333333333333333333333333333333333333333",
        owner: "12D3KooWXYZ...",
        stars: 8,
    },
];

pub async fn cmd_explore(
    _cwd: &Path,
    target: Option<&str>,
    list: bool,
) -> Result<()> {
    if list || target.is_none() {
        println!("\x1b[1mPublic DecentraForge Repositories\x1b[0m");
        println!("{}", "─".repeat(70));
        println!("{:<30} {:<8} {}", "NAME", "STARS", "DESCRIPTION");
        println!("{}", "─".repeat(70));
        for repo in DEMO_REGISTRY {
            println!("{:<30} \x1b[33m{:>5}★\x1b[0m  {}",
                repo.name, repo.stars, repo.description);
            println!("  \x1b[90mCID: {} | Owner: {}\x1b[0m", repo.cid, repo.owner);
        }
        println!();
        println!("Use 'dforge explore <CID>' to browse a repo.");
        println!("Use 'dforge clone <CID>' to clone a repo locally.");
        println!();
        println!("\x1b[90mNote: In production, this queries Kademlia DHT for live repo listings.\x1b[0m");
        return Ok(());
    }

    let target = target.unwrap();
    println!("\x1b[1mBrowsing: {}\x1b[0m", target);
    println!("{}", "─".repeat(60));

    if let Some(repo) = DEMO_REGISTRY.iter().find(|r| r.cid == target || r.name == target) {
        println!("Name:   \x1b[1m{}\x1b[0m", repo.name);
        println!("Desc:   {}", repo.description);
        println!("Owner:  {}", repo.owner);
        println!("Stars:  \x1b[33m{}★\x1b[0m", repo.stars);
        println!("CID:    {}", repo.cid);
        println!();
        println!("To clone:  dforge clone {}", repo.cid);
    } else {
        println!("Repository '{}' not found in local registry.", target);
        println!();
        println!("To fetch from IPFS gateway (requires network):");
        println!("  dforge clone {} ./local-copy", target);
        println!();
        println!("Run 'dforge explore --list' to see known repos.");
    }

    Ok(())
}
