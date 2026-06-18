// Trigram index for O(query + results) code search
// Build: every 3-char sequence → posting list of (file, line)
// Query: intersect posting lists of all trigrams in the search term
// Space: ~3x file size for index (3 trigrams per char on average)
//
// This is how GitHub's code search works internally

use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Location {
    pub file: String,  // file path or CID
    pub line: u32,
}

// The trigram index: 3-char string → set of locations
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TrigramIndex {
    // trigram → set of file locations containing it
    index: HashMap<String, HashSet<Location>>,
    // file → list of lines (for verification after trigram match)
    files: HashMap<String, Vec<String>>,
}

impl TrigramIndex {
    pub fn new() -> Self {
        Self::default()
    }

    // Index a file — O(n) where n = file length
    // Extracts all 3-char sequences and records their locations
    pub fn add_file(&mut self, path: &str, content: &str) {
        let lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();

        for (line_num, line) in lines.iter().enumerate() {
            let lower = line.to_lowercase();
            let chars: Vec<char> = lower.chars().collect();

            // Extract all trigrams from this line
            for i in 0..chars.len().saturating_sub(2) {
                let trigram: String = chars[i..i+3].iter().collect();
                // Only index printable ASCII trigrams
                if trigram.chars().all(|c| c.is_ascii() && !c.is_control()) {
                    self.index
                        .entry(trigram)
                        .or_default()
                        .insert(Location {
                            file: path.to_string(),
                            line: line_num as u32,
                        });
                }
            }
        }
        self.files.insert(path.to_string(), lines);
    }

    // Search — O(|query| + |results|)
    // 1. Extract trigrams from query
    // 2. Intersect posting lists — only files with ALL trigrams are candidates
    // 3. Verify candidates contain exact query string
    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        if query.len() < 3 {
            return self.search_short(query);
        }

        let lower = query.to_lowercase();
        let chars: Vec<char> = lower.chars().collect();

        // Extract all trigrams from query
        let query_trigrams: Vec<String> = (0..chars.len().saturating_sub(2))
            .map(|i| chars[i..i+3].iter().collect())
            .collect();

        if query_trigrams.is_empty() {
            return vec![];
        }

        // Start with posting list of first trigram
        let mut candidates: HashSet<Location> = self.index
            .get(&query_trigrams[0])
            .cloned()
            .unwrap_or_default();

        // Intersect with remaining trigrams — shrinks candidate set fast
        for trigram in &query_trigrams[1..] {
            if let Some(postings) = self.index.get(trigram) {
                candidates = candidates.intersection(postings).cloned().collect();
            } else {
                return vec![]; // trigram not in any file
            }
        }

        // Verify candidates contain the actual query string
        let mut results = Vec::new();
        for loc in candidates {
            if let Some(lines) = self.files.get(&loc.file) {
                if let Some(line) = lines.get(loc.line as usize) {
                    if line.to_lowercase().contains(&lower) {
                        let col = line.to_lowercase().find(&lower).unwrap_or(0) as u32;
                        results.push(SearchResult {
                            file: loc.file,
                            line: loc.line + 1, // 1-indexed for display
                            col,
                            content: line.trim().to_string(),
                            matched_query: query.to_string(),
                        });
                    }
                }
            }
        }

        results.sort_by(|a, b| a.file.cmp(&b.file).then(a.line.cmp(&b.line)));
        results
    }

    // For short queries (< 3 chars), fall back to linear scan
    fn search_short(&self, query: &str) -> Vec<SearchResult> {
        let lower = query.to_lowercase();
        let mut results = Vec::new();
        for (path, lines) in &self.files {
            for (i, line) in lines.iter().enumerate() {
                if line.to_lowercase().contains(&lower) {
                    results.push(SearchResult {
                        file: path.clone(),
                        line: (i + 1) as u32,
                        col: 0,
                        content: line.trim().to_string(),
                        matched_query: query.to_string(),
                    });
                }
            }
        }
        results
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    pub fn trigram_count(&self) -> usize {
        self.index.len()
    }

    // Serialize index to bytes for storage on IPFS
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(self)?)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        Ok(serde_json::from_slice(bytes)?)
    }
}

// Build index from a directory recursively
pub fn index_directory(root: &std::path::Path) -> Result<TrigramIndex> {
    let mut index = TrigramIndex::new();
    index_dir_recursive(&mut index, root, root)?;
    Ok(index)
}

fn index_dir_recursive(
    index: &mut TrigramIndex,
    root: &std::path::Path,
    dir: &std::path::Path,
) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if name.starts_with('.') { continue; }

        if path.is_dir() {
            index_dir_recursive(index, root, &path)?;
        } else if is_text_file(&name) {
            if let Ok(content) = std::fs::read_to_string(&path) {
                let rel_path = path.strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string();
                index.add_file(&rel_path, &content);
            }
        }
    }
    Ok(())
}

fn is_text_file(name: &str) -> bool {
    let text_exts = [
        "rs", "py", "js", "ts", "go", "java", "c", "cpp", "h",
        "md", "txt", "toml", "json", "yaml", "yml", "sh", "html",
        "css", "sol", "rb", "swift", "kt", "scala", "hs",
    ];
    if let Some(ext) = name.rsplit('.').next() {
        text_exts.contains(&ext.to_lowercase().as_str())
    } else {
        false
    }
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub content: String,
    pub matched_query: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_search() {
        let mut idx = TrigramIndex::new();
        idx.add_file("src/main.rs", "fn main() {\n    println!(\"hello\");\n}\n");
        idx.add_file("src/lib.rs", "pub fn helper() -> String {\n    String::from(\"world\")\n}\n");

        let results = idx.search("println");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].file, "src/main.rs");
        assert_eq!(results[0].line, 2);
    }

    #[test]
    fn no_false_positives() {
        let mut idx = TrigramIndex::new();
        idx.add_file("a.rs", "let x = 42;\n");
        let results = idx.search("println");
        assert!(results.is_empty());
    }

    #[test]
    fn case_insensitive() {
        let mut idx = TrigramIndex::new();
        idx.add_file("a.rs", "fn MyFunction() {}\n");
        let results = idx.search("myfunction");
        assert_eq!(results.len(), 1);
    }
}
