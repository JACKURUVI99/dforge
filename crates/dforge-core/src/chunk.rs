// Content-Defined Chunking via Rabin Fingerprinting
// Finds natural split points in data using rolling hash
// Same content always chunks the same way → maximum deduplication
// across versions AND across different files
//
// Rolling hash: hash[i] = (hash[i-1] * BASE + data[i] - data[i-W] * BASE^W) mod M
// Split when hash & MASK == 0
// O(n) time, O(window_size) space — constant sliding window

const WINDOW_SIZE: usize = 48;
const BASE: u64 = 31;
const MASK: u64 = 0x1FFF; // split every ~8KB average
const MIN_CHUNK: usize = 2 * 1024;   // 2KB minimum
const MAX_CHUNK: usize = 64 * 1024;  // 64KB maximum

// Precomputed BASE^WINDOW_SIZE for rolling hash subtraction
const fn pow_mod(mut base: u64, mut exp: usize) -> u64 {
    let mut result = 1u64;
    while exp > 0 {
        if exp & 1 != 0 { result = result.wrapping_mul(base); }
        base = base.wrapping_mul(base);
        exp >>= 1;
    }
    result
}

const BASE_WINDOW: u64 = pow_mod(BASE, WINDOW_SIZE);

pub struct Chunk<'a> {
    pub data: &'a [u8],
    pub offset: usize,
}

// Iterator over content-defined chunks
pub struct ChunkIter<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> ChunkIter<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }
}

impl<'a> Iterator for ChunkIter<'a> {
    type Item = Chunk<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.data.len() { return None; }

        let start = self.pos;
        let remaining = &self.data[start..];

        // Find next split point using rolling Rabin fingerprint
        let end = find_split_point(remaining);
        self.pos = start + end;

        Some(Chunk {
            data: &self.data[start..self.pos],
            offset: start,
        })
    }
}

fn find_split_point(data: &[u8]) -> usize {
    if data.len() <= MIN_CHUNK {
        return data.len();
    }

    let mut window = [0u8; WINDOW_SIZE];
    let mut hash: u64 = 0;

    // Fill initial window
    let start = MIN_CHUNK.min(data.len());
    for i in 0..start.min(WINDOW_SIZE) {
        let b = data[i] as u64;
        hash = hash.wrapping_mul(BASE).wrapping_add(b);
        window[i % WINDOW_SIZE] = data[i];
    }

    // Roll window through data looking for split boundary
    for i in start..data.len().min(MAX_CHUNK) {
        let outgoing = window[i % WINDOW_SIZE] as u64;
        let incoming = data[i] as u64;

        // Roll: remove outgoing byte, add incoming byte
        hash = hash
            .wrapping_sub(BASE_WINDOW.wrapping_mul(outgoing))
            .wrapping_mul(BASE)
            .wrapping_add(incoming);

        window[i % WINDOW_SIZE] = data[i];

        if hash & MASK == 0 {
            return i + 1; // split here
        }
    }

    data.len().min(MAX_CHUNK)
}

// Chunk data and return all chunk boundaries
pub fn chunk_data(data: &[u8]) -> Vec<&[u8]> {
    ChunkIter::new(data).map(|c| c.data).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunks_reconstruct_original() {
        let data: Vec<u8> = (0..100_000u32).map(|i| (i * 31 + 7) as u8).collect();
        let chunks: Vec<&[u8]> = chunk_data(&data);
        let reconstructed: Vec<u8> = chunks.iter().flat_map(|c| c.iter().copied()).collect();
        assert_eq!(data, reconstructed);
    }

    #[test]
    fn same_content_same_chunks() {
        let data: Vec<u8> = (0..50_000u32).map(|i| (i % 256) as u8).collect();
        let c1 = chunk_data(&data);
        let c2 = chunk_data(&data);
        assert_eq!(c1.len(), c2.len());
        for (a, b) in c1.iter().zip(c2.iter()) {
            assert_eq!(a, b);
        }
    }

    #[test]
    fn small_change_preserves_most_chunks() {
        let mut data: Vec<u8> = (0..100_000u32).map(|i| (i % 256) as u8).collect();
        let chunks_before: Vec<Vec<u8>> = chunk_data(&data).iter().map(|c| c.to_vec()).collect();

        // Change 10 bytes in middle — should only affect ~2 nearby chunks
        for i in 50_000..50_010 { data[i] ^= 0xFF; }
        let chunks_after: Vec<Vec<u8>> = chunk_data(&data).iter().map(|c| c.to_vec()).collect();

        let shared = chunks_before.iter()
            .filter(|c| chunks_after.contains(c))
            .count();

        // Most chunks should be identical (deduplication works)
        assert!(shared as f64 / chunks_before.len() as f64 > 0.9,
            "too many chunks changed: {}/{}", chunks_before.len() - shared, chunks_before.len());
    }
}
