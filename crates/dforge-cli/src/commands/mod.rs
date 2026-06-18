mod init;
mod commit;
mod log;
mod diff;
mod branch;
mod status;
mod identity;
mod search;
mod push;
mod pull;
mod merge;
mod pr;
mod issue;
mod config;
mod explore;

pub use init::cmd_init;
pub use commit::cmd_commit;
pub use log::cmd_log;
pub use diff::cmd_diff;
pub use branch::{cmd_branch_list, cmd_branch_new, cmd_branch_delete, cmd_checkout};
pub use status::cmd_status;
pub use identity::cmd_identity;
pub use search::cmd_search;
pub use push::cmd_push;
pub use pull::cmd_pull;
pub use merge::cmd_merge;
pub use pr::{cmd_pr_create, cmd_pr_list, cmd_pr_show, cmd_pr_merge};
pub use issue::{cmd_issue_new, cmd_issue_list, cmd_issue_show, cmd_issue_close};
pub use config::{cmd_config_set, cmd_config_show};
pub use explore::cmd_explore;

pub async fn cmd_collab_add(_cwd: &std::path::Path, eth_address: &str) -> anyhow::Result<()> {
    println!("Adding collaborator {} to blockchain...", eth_address);
    println!("(Blockchain integration coming — requires Ethereum wallet config)");
    Ok(())
}

pub async fn cmd_clone(source: &str, dest: &std::path::PathBuf) -> anyhow::Result<()> {
    println!("Cloning from IPFS: {}", source);
    println!("Destination: {}", dest.display());
    println!("(Clone requires IPFS config — run: dforge init first)");
    Ok(())
}
