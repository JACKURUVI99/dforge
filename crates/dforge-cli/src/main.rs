use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod commands;
use commands::*;

#[derive(Parser)]
#[command(
    name = "dforge",
    about = "DecentraForge — Serverless Git on IPFS + Blockchain",
    version = "0.1.0",
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new repository
    Init {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Clone a repository from IPFS
    Clone {
        /// IPFS CID or IPNS path
        source: String,
        #[arg(default_value = ".")]
        dest: PathBuf,
    },
    /// Stage and commit changes
    Commit {
        #[arg(short, long)]
        message: String,
    },
    /// Push to IPFS + register on blockchain
    Push {
        #[arg(long, default_value = "main")]
        branch: String,
    },
    /// Pull from IPFS (authoritative-first, middleman fallback)
    Pull {
        /// IPFS CID to pull
        cid: Option<String>,
    },
    /// Show commit history
    Log {
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
    /// Show diff of working directory
    Diff {
        file: Option<PathBuf>,
    },
    /// Branch management
    Branch {
        #[command(subcommand)]
        action: BranchAction,
    },
    /// Switch to a branch
    Checkout {
        branch: String,
    },
    /// Merge a branch
    Merge {
        branch: String,
    },
    /// Pull request management
    Pr {
        #[command(subcommand)]
        action: PrAction,
    },
    /// Issue management
    Issue {
        #[command(subcommand)]
        action: IssueAction,
    },
    /// Add collaborator (blockchain tx)
    Collab {
        eth_address: String,
    },
    /// Search code with trigram index
    Search {
        query: String,
        #[arg(short, long)]
        path: Option<PathBuf>,
    },
    /// Show repo status
    Status,
    /// Show identity (NodeId, public key)
    Identity,
    /// Launch TUI (lazygit-style interface)
    Tui,
    /// Edit repo config (name, description)
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Explore public repositories (by IPFS CID or NodeId)
    Explore {
        /// IPFS CID or NodeId of the repo to browse
        target: Option<String>,
        #[arg(long)]
        list: bool,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Set repo name
    Name { value: String },
    /// Set repo description
    Desc { value: String },
    /// Show current config
    Show,
}

#[derive(Subcommand)]
enum BranchAction {
    /// List branches
    List,
    /// Create new branch
    New { name: String },
    /// Delete branch
    Delete { name: String },
}

#[derive(Subcommand)]
enum PrAction {
    /// Create a pull request
    Create {
        #[arg(short, long)]
        title: String,
        #[arg(short, long, default_value = "")]
        body: String,
        #[arg(long, default_value = "main")]
        into: String,
    },
    /// List pull requests
    List,
    /// Show PR details
    Show { id: u32 },
    /// Merge a PR
    Merge { id: u32 },
}

#[derive(Subcommand)]
enum IssueAction {
    /// Create an issue
    New {
        title: String,
        #[arg(short, long, default_value = "")]
        body: String,
    },
    /// List issues
    List,
    /// Show issue details
    Show { id: u32 },
    /// Close an issue
    Close { id: u32 },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let cwd = std::env::current_dir()?;

    match cli.command {
        Commands::Init { path, name } => {
            let target = if path == PathBuf::from(".") { cwd } else { path };
            let repo_name = name.unwrap_or_else(|| {
                target.file_name().unwrap_or_default()
                    .to_string_lossy().to_string()
            });
            cmd_init(&target, &repo_name)?;
        }
        Commands::Commit { message } => {
            cmd_commit(&cwd, &message)?;
        }
        Commands::Log { limit } => {
            cmd_log(&cwd, limit)?;
        }
        Commands::Diff { file } => {
            cmd_diff(&cwd, file.as_deref())?;
        }
        Commands::Branch { action } => {
            match action {
                BranchAction::List => cmd_branch_list(&cwd)?,
                BranchAction::New { name } => cmd_branch_new(&cwd, &name)?,
                BranchAction::Delete { name } => cmd_branch_delete(&cwd, &name)?,
            }
        }
        Commands::Checkout { branch } => {
            cmd_checkout(&cwd, &branch)?;
        }
        Commands::Status => {
            cmd_status(&cwd)?;
        }
        Commands::Identity => {
            cmd_identity(&cwd)?;
        }
        Commands::Search { query, path } => {
            let search_path = path.unwrap_or_else(|| cwd.clone());
            cmd_search(&search_path, &query)?;
        }
        Commands::Push { branch } => {
            cmd_push(&cwd, &branch).await?;
        }
        Commands::Pull { cid } => {
            cmd_pull(&cwd, cid.as_deref()).await?;
        }
        Commands::Merge { branch } => {
            cmd_merge(&cwd, &branch)?;
        }
        Commands::Pr { action } => {
            match action {
                PrAction::Create { title, body, into } => {
                    cmd_pr_create(&cwd, &title, &body, &into)?;
                }
                PrAction::List => cmd_pr_list(&cwd)?,
                PrAction::Show { id } => cmd_pr_show(&cwd, id)?,
                PrAction::Merge { id } => cmd_pr_merge(&cwd, id)?,
            }
        }
        Commands::Issue { action } => {
            match action {
                IssueAction::New { title, body } => cmd_issue_new(&cwd, &title, &body)?,
                IssueAction::List => cmd_issue_list(&cwd)?,
                IssueAction::Show { id } => cmd_issue_show(&cwd, id)?,
                IssueAction::Close { id } => cmd_issue_close(&cwd, id)?,
            }
        }
        Commands::Collab { eth_address } => {
            cmd_collab_add(&cwd, &eth_address).await?;
        }
        Commands::Clone { source, dest } => {
            cmd_clone(&source, &dest).await?;
        }
        Commands::Tui => {
            dforge_tui::run()?;
        }
        Commands::Config { action } => {
            match action {
                ConfigAction::Name { value } => cmd_config_set(&cwd, "name", &value)?,
                ConfigAction::Desc { value } => cmd_config_set(&cwd, "description", &value)?,
                ConfigAction::Show => cmd_config_show(&cwd)?,
            }
        }
        Commands::Explore { target, list } => {
            cmd_explore(&cwd, target.as_deref(), list).await?;
        }
    }
    Ok(())
}
