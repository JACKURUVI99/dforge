use anyhow::Result;
use std::path::Path;
use dforge_crypto::Identity;
use dforge_core::Repo;

pub fn cmd_init(path: &Path, name: &str) -> Result<()> {
    // Generate or load identity
    let identity = load_or_create_identity()?;

    println!("Identity: {}", identity.node_id);
    println!("Public key: {}", &identity.public_key_hex()[..16]);

    Repo::init(path, name, &identity.public_key_hex())?;

    println!("\nRepo '{}' ready.", name);
    println!("Run 'dforge commit -m \"initial commit\"' to make your first commit.");
    Ok(())
}

pub fn load_or_create_identity() -> Result<Identity> {
    let identity_path = dirs_path();
    if identity_path.exists() {
        let bytes = std::fs::read(&identity_path)?;
        Identity::from_bytes(&bytes)
    } else {
        let id = Identity::generate();
        if let Some(parent) = identity_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&identity_path, id.to_bytes())?;
        println!("Generated new identity: {}", id.node_id);
        Ok(id)
    }
}

fn dirs_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home).join(".dforge").join("identity")
}
