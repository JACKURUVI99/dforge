// IPFS client — upload/download encrypted pack files
// Uses Pinata API for pinning (ensures persistence)
// Falls back to public IPFS gateway for downloads

use anyhow::{Context, Result};
use reqwest::multipart;

pub struct IpfsClient {
    pinata_jwt: String,
    client: reqwest::Client,
}

impl IpfsClient {
    pub fn new(pinata_jwt: String) -> Self {
        Self {
            pinata_jwt,
            client: reqwest::Client::new(),
        }
    }

    // Upload encrypted bytes to IPFS via Pinata → returns CID
    pub async fn upload(&self, data: Vec<u8>, name: &str) -> Result<String> {
        let part = multipart::Part::bytes(data)
            .file_name(name.to_string())
            .mime_str("application/octet-stream")?;

        let form = multipart::Form::new()
            .part("file", part);

        let resp = self.client
            .post("https://api.pinata.cloud/pinning/pinFileToIPFS")
            .bearer_auth(&self.pinata_jwt)
            .multipart(form)
            .send()
            .await
            .context("IPFS upload failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Pinata upload error {}: {}", status, body);
        }

        let json: serde_json::Value = resp.json().await?;
        let cid = json["IpfsHash"]
            .as_str()
            .context("no IpfsHash in Pinata response")?
            .to_string();

        Ok(cid)
    }

    // Download from IPFS via public gateway
    pub async fn download(&self, cid: &str) -> Result<Vec<u8>> {
        let gateways = [
            format!("https://gateway.pinata.cloud/ipfs/{}", cid),
            format!("https://ipfs.io/ipfs/{}", cid),
            format!("https://cloudflare-ipfs.com/ipfs/{}", cid),
        ];

        // Try gateways in order, use first success
        let mut last_err = None;
        for url in &gateways {
            match self.client.get(url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    return Ok(resp.bytes().await?.to_vec());
                }
                Ok(resp) => {
                    last_err = Some(anyhow::anyhow!("gateway {} returned {}", url, resp.status()));
                }
                Err(e) => {
                    last_err = Some(e.into());
                }
            }
        }
        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("all gateways failed")))
    }

    // Check if content is available on IPFS (HEAD request)
    pub async fn exists(&self, cid: &str) -> bool {
        let url = format!("https://gateway.pinata.cloud/ipfs/{}", cid);
        self.client
            .head(&url)
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}

// Middleman client — fast-path key share cache
pub struct MiddlemanClient {
    base_url: String,
    client: reqwest::Client,
}

impl MiddlemanClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
        }
    }

    // Store middleman's SSS share for a repo CID
    pub async fn store_share(&self, ipfs_cid: &str, share_hex: &str) -> Result<()> {
        let resp = self.client
            .post(format!("{}/share", self.base_url))
            .json(&serde_json::json!({
                "cid": ipfs_cid,
                "share": share_hex
            }))
            .send()
            .await
            .context("middleman store failed")?;

        if !resp.status().is_success() {
            anyhow::bail!("middleman error: {}", resp.status());
        }
        Ok(())
    }

    // Retrieve middleman's share for a repo CID (fast path)
    pub async fn get_share(&self, ipfs_cid: &str) -> Result<String> {
        let resp = self.client
            .get(format!("{}/share/{}", self.base_url, ipfs_cid))
            .send()
            .await
            .context("middleman fetch failed")?;

        if !resp.status().is_success() {
            anyhow::bail!("middleman returned {}", resp.status());
        }

        let json: serde_json::Value = resp.json().await?;
        json["share"]
            .as_str()
            .map(|s| s.to_string())
            .context("no share in middleman response")
    }
}
