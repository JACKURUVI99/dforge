// Blake3 — 4x faster than SHA-256, SIMD-parallel, same 256-bit security
// Used for: object IDs, content addressing, CID generation

use anyhow::Result;

pub struct ContentId(pub [u8; 32]);

impl ContentId {
    pub fn from_bytes(data: &[u8]) -> Self {
        // O(n) — single pass, SIMD-accelerated
        let hash = blake3::hash(data);
        Self(*hash.as_bytes())
    }

    pub fn from_reader(mut reader: impl std::io::Read) -> Result<Self> {
        let mut hasher = blake3::Hasher::new();
        let mut buf = [0u8; 65536]; // 64KB chunks for optimal SIMD
        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 { break; }
            hasher.update(&buf[..n]);
        }
        Ok(Self(*hasher.finalize().as_bytes()))
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Display for ContentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl std::fmt::Debug for ContentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CID({})", &self.to_hex()[..12])
    }
}

// Incremental hasher for streaming large objects
pub struct StreamHasher(blake3::Hasher);

impl StreamHasher {
    pub fn new() -> Self {
        Self(blake3::Hasher::new())
    }

    pub fn update(&mut self, data: &[u8]) {
        self.0.update(data);
    }

    pub fn finalize(self) -> ContentId {
        ContentId(*self.0.finalize().as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_content_same_cid() {
        let a = ContentId::from_bytes(b"hello world");
        let b = ContentId::from_bytes(b"hello world");
        assert_eq!(a.0, b.0);
    }

    #[test]
    fn different_content_different_cid() {
        let a = ContentId::from_bytes(b"hello");
        let b = ContentId::from_bytes(b"world");
        assert_ne!(a.0, b.0);
    }
}
