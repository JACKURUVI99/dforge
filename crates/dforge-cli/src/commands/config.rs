use anyhow::Result;
use std::path::Path;
use dforge_core::Repo;

pub fn cmd_config_set(cwd: &Path, key: &str, value: &str) -> Result<()> {
    let repo = Repo::open(cwd)?;
    let config_path = repo.dforge_dir.join("config.json");

    let mut config: serde_json::Value = if config_path.exists() {
        let data = std::fs::read(&config_path)?;
        serde_json::from_slice(&data)?
    } else {
        serde_json::json!({})
    };

    config[key] = serde_json::Value::String(value.to_string());
    std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;

    println!("Set {}: {}", key, value);
    Ok(())
}

pub fn cmd_config_show(cwd: &Path) -> Result<()> {
    let repo = Repo::open(cwd)?;
    println!("Repo: \x1b[1m{}\x1b[0m", repo.config.name);
    println!("Desc: {}", repo.config.description);
    println!("Owner: {}", repo.config.owner_pubkey);
    if let Some(cid) = &repo.config.ipfs_cid {
        println!("IPFS CID: {}", cid);
    }
    if let Some(eth) = &repo.config.eth_address {
        println!("ETH: {}", eth);
    }
    Ok(())
}
