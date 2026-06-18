// Diff Algorithm — LCS-based O(ND) shortest edit script
// Uses DP table for LCS then backtrack for edits
// Correct and fast for typical code diffs where D << N

#[derive(Debug, Clone, PartialEq)]
pub enum Edit<T> {
    Keep(T),
    Insert(T),
    Delete(T),
}

// Diff on line arrays using LCS backtrack — O(n*m) space, O(n*m) time
// For typical code: n,m < 10000, so 100M ops max — still fast
pub fn diff_lines<'a>(old: &'a [&str], new: &'a [&str]) -> Vec<Edit<&'a str>> {
    let n = old.len();
    let m = new.len();

    if n == 0 {
        return new.iter().map(|&l| Edit::Insert(l)).collect();
    }
    if m == 0 {
        return old.iter().map(|&l| Edit::Delete(l)).collect();
    }

    // LCS DP table: dp[i][j] = LCS length of old[..i] and new[..j]
    // Space: O(n*m) — optimize to O(m) rolling array
    let mut dp = vec![vec![0u16; m + 1]; n + 1];

    for i in 1..=n {
        for j in 1..=m {
            dp[i][j] = if old[i-1] == new[j-1] {
                dp[i-1][j-1] + 1
            } else {
                dp[i-1][j].max(dp[i][j-1])
            };
        }
    }

    // Backtrack to build edit script
    let mut edits = Vec::new();
    let mut i = n;
    let mut j = m;

    while i > 0 || j > 0 {
        if i > 0 && j > 0 && old[i-1] == new[j-1] {
            edits.push(Edit::Keep(old[i-1]));
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j-1] >= dp[i-1][j]) {
            edits.push(Edit::Insert(new[j-1]));
            j -= 1;
        } else {
            edits.push(Edit::Delete(old[i-1]));
            i -= 1;
        }
    }

    edits.reverse();
    edits
}

// Render diff as unified diff format (like git diff)
pub fn render_unified(old_name: &str, new_name: &str, edits: &[Edit<&str>]) -> String {
    let mut out = String::new();
    out.push_str(&format!("--- {}\n", old_name));
    out.push_str(&format!("+++ {}\n", new_name));

    let mut old_line = 1usize;
    let mut new_line = 1usize;
    let mut hunk: Vec<String> = Vec::new();
    let mut hunk_old_start = 1usize;
    let mut hunk_new_start = 1usize;
    let mut in_hunk = false;
    let context = 3usize;

    let mut context_buf: Vec<String> = Vec::new();

    for edit in edits {
        match edit {
            Edit::Keep(line) => {
                if in_hunk {
                    hunk.push(format!(" {}", line));
                    if hunk.iter().rev().take(context + 1).all(|l| l.starts_with(' ')) {
                        if hunk.len() > context {
                            // flush hunk
                            let old_count: usize = hunk.iter().filter(|l| !l.starts_with('+')).count();
                            let new_count: usize = hunk.iter().filter(|l| !l.starts_with('-')).count();
                            out.push_str(&format!("@@ -{},{} +{},{} @@\n",
                                hunk_old_start, old_count, hunk_new_start, new_count));
                            for line in &hunk { out.push_str(line); out.push('\n'); }
                            hunk.clear();
                            in_hunk = false;
                        }
                    }
                } else {
                    context_buf.push(format!(" {}", line));
                    if context_buf.len() > context {
                        context_buf.remove(0);
                    }
                }
                old_line += 1;
                new_line += 1;
            }
            Edit::Delete(line) => {
                if !in_hunk {
                    hunk_old_start = old_line.saturating_sub(context_buf.len());
                    hunk_new_start = new_line.saturating_sub(context_buf.len());
                    hunk.extend(context_buf.drain(..));
                    in_hunk = true;
                }
                hunk.push(format!("-{}", line));
                old_line += 1;
            }
            Edit::Insert(line) => {
                if !in_hunk {
                    hunk_old_start = old_line.saturating_sub(context_buf.len());
                    hunk_new_start = new_line.saturating_sub(context_buf.len());
                    hunk.extend(context_buf.drain(..));
                    in_hunk = true;
                }
                hunk.push(format!("+{}", line));
                new_line += 1;
            }
        }
    }

    // Flush remaining hunk
    if in_hunk && !hunk.is_empty() {
        let old_count: usize = hunk.iter().filter(|l| !l.starts_with('+')).count();
        let new_count: usize = hunk.iter().filter(|l| !l.starts_with('-')).count();
        out.push_str(&format!("@@ -{},{} +{},{} @@\n",
            hunk_old_start, old_count, hunk_new_start, new_count));
        for line in &hunk { out.push_str(line); out.push('\n'); }
    }

    out
}

// Simple line-level diff with colored output for TUI
pub fn diff_colored(old: &str, new: &str) -> Vec<DiffLine> {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    let edits = diff_lines(&old_lines, &new_lines);

    edits.iter().map(|e| match e {
        Edit::Keep(l)   => DiffLine { kind: LineKind::Context, text: l.to_string() },
        Edit::Insert(l) => DiffLine { kind: LineKind::Added,   text: l.to_string() },
        Edit::Delete(l) => DiffLine { kind: LineKind::Removed, text: l.to_string() },
    }).collect()
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub kind: LineKind,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LineKind {
    Context,
    Added,
    Removed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_diff() {
        let edits = diff_lines(&["a", "b", "c"], &["a", "b", "c"]);
        assert!(edits.iter().all(|e| matches!(e, Edit::Keep(_))));
    }

    #[test]
    fn insertion() {
        let old = vec!["a", "b", "c"];
        let new = vec!["a", "X", "b", "c"];
        let edits = diff_lines(&old, &new);
        assert!(edits.contains(&Edit::Insert("X")));
    }

    #[test]
    fn deletion() {
        let old = vec!["a", "b", "c"];
        let new = vec!["a", "c"];
        let edits = diff_lines(&old, &new);
        assert!(edits.contains(&Edit::Delete("b")));
    }

    #[test]
    fn reconstruct_new_from_edits() {
        let old = ["hello", "world"];
        let new = ["hello", "earth"];
        let edits = diff_lines(&old, &new);
        // Keep "hello", delete "world", insert "earth"
        assert!(edits.contains(&Edit::Keep("hello")));
        assert!(edits.contains(&Edit::Delete("world")));
        assert!(edits.contains(&Edit::Insert("earth")));
        // Reconstructed new = keeps + inserts, no deletes
        let reconstructed: Vec<&str> = edits.iter().filter_map(|e| match e {
            Edit::Keep(l) | Edit::Insert(l) => Some(*l),
            Edit::Delete(_) => None,
        }).collect();
        assert_eq!(reconstructed, new);
    }
}
