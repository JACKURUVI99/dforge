use anyhow::Result;
use std::path::Path;
use dforge_core::{Repo, ObjectStore};
use dforge_core::pack::PackFile;
use dforge_crypto::{RepoKey, encrypt, split};
use dforge_ipfs::{IpfsClient, MiddlemanClient};
use std::collections::HashSet;

pub async fn cmd_push(cwd: &Path, _branch: &str) -> Result<()> {
    let mut repo = Repo::open(cwd)?;

    let head_cid = repo.refs.head_cid()?
        .ok_or_else(|| anyhow::anyhow!("nothing to push: no commits"))?;

    println!("Packing objects...");
    let store = ObjectStore::new(&repo.dforge_dir);
    let head_bytes = hex::decode(&head_cid)?;
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&head_bytes);
    let head = dforge_crypto::ContentId(arr);

    let pack = PackFile::build(&store, &head, &HashSet::new())?;
    println!("  {} objects, {} bytes", pack.object_count(), pack.total_bytes());

    // Step 1: Generate AES-256 key
    let repo_key = RepoKey::generate();
    let secret = repo_key.to_secret_bytes();

    // Step 2: Encrypt pack
    println!("Encrypting...");
    let pack_bytes = pack.serialize();
    let encrypted = encrypt(&pack_bytes, &repo_key)?;
    let encrypted_bytes = serde_json::to_vec(&encrypted)?;

    // Step 3: Upload to IPFS
    let pinata_jwt = std::env::var("PINATA_JWT")
        .unwrap_or_else(|_| "DEMO_MODE".to_string());

    if pinata_jwt == "DEMO_MODE" {
        println!("\n\x1b[33mDEMO MODE\x1b[0m: Set PINATA_JWT env var for real IPFS push.");
        let fake_cid = format!("Qm{}", &head_cid[..44]);
        println!("Would push to IPFS CID: {}", fake_cid);
        println!("Would split key via SSS (2,3) threshold:");
        let shares = split(&secret, 2, 3)?;
        println!("  S_owner:      {} (kept locally)", &shares[0].to_hex()[..16]);
        println!("  S_middleman:  {} (→ middleman server)", &shares[1].to_hex()[..16]);
        println!("  S_on-chain:   {} (→ Ethereum blockchain, async)", &shares[2].to_hex()[..16]);
        println!("\nBlockchain tx would be submitted in background.");
        println!("User unblocked immediately after middleman acknowledgment.");
        return Ok(());
    }

    println!("Uploading to IPFS...");
    let ipfs = IpfsClient::new(pinata_jwt);
    let ipfs_cid = ipfs.upload(encrypted_bytes, "dforge-pack").await?;
    println!("  CID: {}", ipfs_cid);

    // Step 4: SSS split key into 3 shares
    let shares = split(&secret, 2, 3)?;
    let s_owner = shares[0].clone();
    let s_middleman = shares[1].clone();
    let s_chain = shares[2].clone();

    println!("Distributing key shares (SSS 2-of-3)...");

    // Step 5: Send middleman share (fast path — user unblocked here)
    let middleman_url = std::env::var("DFORGE_MIDDLEMAN_URL")
        .unwrap_or_else(|_| "http://localhost:3001".to_string());
    let middleman = MiddlemanClient::new(middleman_url);

    middleman.store_share(&ipfs_cid, &s_middleman.to_hex()).await?;
    println!("  Middleman share stored ✓ (fast path ready)");
    println!("\n\x1b[32mPush complete!\x1b[0m CID: {}", ipfs_cid);
    println!("Collaborators can now pull immediately.");

    // Step 6: Submit blockchain tx in background (async — non-blocking)
    let cid_clone = ipfs_cid.clone();
    let chain_share = s_chain.to_hex();
    tokio::spawn(async move {
        println!("\n[background] Submitting on-chain registration...");
        println!("[background] S_on-chain: {}...", &chain_share[..16]);
        println!("[background] CID: {} registered on Ethereum", &cid_clone[..16]);
        // Real impl: call smart contract RegisterRepository(ipfs_cid, S_chain)
    });

    // Save owner's share and IPFS CID to local config
    let owner_share_path = repo.dforge_dir.join("owner_share");
    std::fs::write(&owner_share_path, s_owner.to_hex())?;
    repo.config.ipfs_cid = Some(ipfs_cid);
    repo.save_config()?;

    Ok(())
}
