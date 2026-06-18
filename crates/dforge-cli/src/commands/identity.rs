use anyhow::Result;
use std::path::Path;
use super::init::load_or_create_identity;

pub fn cmd_identity(_cwd: &Path) -> Result<()> {
    let id = load_or_create_identity()?;
    println!("NodeId:     {}", id.node_id);
    println!("Public key: {}", id.public_key_hex());
    println!("\nShare your public key to receive collaborator access.");
    Ok(())
}
