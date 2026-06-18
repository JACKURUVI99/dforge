use anyhow::Result;
use std::path::Path;
use dforge_crypto::{RepoKey, decrypt, reconstruct, Share};
use dforge_ipfs::{IpfsClient, MiddlemanClient};
use dforge_core::{Repo, ObjectStore};
use dforge_core::pack::PackFile;

pub async fn cmd_pull(cwd: &Path, cid: Option<&str>) -> Result<()> {
    let repo = Repo::open(cwd)?;

    let ipfs_cid = cid
        .map(|s| s.to_string())
        .or_else(|| repo.config.ipfs_cid.clone())
        .ok_or_else(|| anyhow::anyhow!("no CID specified and none in config. Use: dforge pull <cid>"))?;

    println!("Pulling CID: {}", &ipfs_cid[..16]);

    // Load owner's share (stored locally after push)
    let owner_share_path = repo.dforge_dir.join("owner_share");
    let owner_share_hex = std::fs::read_to_string(&owner_share_path)
        .map_err(|_| anyhow::anyhow!("owner share not found — are you the repo owner?"))?;
    let s_owner = Share::from_hex(owner_share_hex.trim())?;

    // Authoritative-first: try blockchain
    // Fallback: middleman
    let second_share = fetch_second_share(&ipfs_cid, &s_owner).await?;

    // Reconstruct AES key from 2 shares
    let secret = reconstruct(&[s_owner, second_share])?;
    let repo_key = RepoKey::from_secret_bytes(&secret)?;
    println!("Key reconstructed ✓");

    // Download encrypted pack from IPFS
    let pinata_jwt = std::env::var("PINATA_JWT").unwrap_or_else(|_| "DEMO_MODE".to_string());

    if pinata_jwt == "DEMO_MODE" {
        println!("\n\x1b[33mDEMO MODE\x1b[0m: Set PINATA_JWT env var for real IPFS pull.");
        println!("Would download from IPFS CID: {}", &ipfs_cid[..16]);
        println!("Would decrypt with reconstructed key ✓");
        println!("Would unpack objects into local store ✓");
        return Ok(());
    }

    println!("Downloading from IPFS...");
    let ipfs = IpfsClient::new(pinata_jwt);
    let encrypted_bytes = ipfs.download(&ipfs_cid).await?;

    // Decrypt
    let encrypted: dforge_crypto::EncryptedBlob = serde_json::from_slice(&encrypted_bytes)?;
    let pack_bytes = decrypt(&encrypted, &repo_key)?;
    println!("Decrypted ✓");

    // Unpack into object store
    let pack = PackFile::deserialize(&pack_bytes)?;
    let store = ObjectStore::new(&repo.dforge_dir);
    let count = pack.unpack_into(&store)?;
    println!("Unpacked {} objects ✓", count);

    println!("\n\x1b[32mPull complete!\x1b[0m");
    Ok(())
}

async fn fetch_second_share(ipfs_cid: &str, _owner_share: &Share) -> Result<Share> {
    // Step 1: Try authoritative on-chain share first
    println!("Checking blockchain for authoritative share...");
    match fetch_onchain_share(ipfs_cid).await {
        Ok(share) => {
            println!("  On-chain share retrieved ✓ (authoritative)");
            return Ok(share);
        }
        Err(e) => {
            println!("  Blockchain not ready ({}), falling back to middleman...", e);
        }
    }

    // Step 2: Optimistic fallback — middleman (fast path)
    let middleman_url = std::env::var("DFORGE_MIDDLEMAN_URL")
        .unwrap_or_else(|_| "http://localhost:3001".to_string());
    let middleman = MiddlemanClient::new(middleman_url);
    let share_hex = middleman.get_share(ipfs_cid).await?;
    println!("  Middleman share retrieved ✓ (fast path)");
    Share::from_hex(&share_hex)
}

async fn fetch_onchain_share(_ipfs_cid: &str) -> Result<Share> {
    // Real impl: call smart contract getOnChainShare(ipfs_cid)
    // Returns S_on-chain if transaction confirmed, else error
    anyhow::bail!("blockchain not yet integrated — use middleman path")
}
