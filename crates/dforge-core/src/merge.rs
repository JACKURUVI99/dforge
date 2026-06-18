// 3-way merge using LCA (Lowest Common Ancestor)
// merge(ours, theirs) → find base = LCA(ours, theirs)
//                      → diff(base, ours) + diff(base, theirs)
//                      → apply both diffs, detect conflicts

use crate::diff::{diff_lines, Edit};

#[derive(Debug, Clone)]
pub enum MergeLine {
    Clean(String),
    ConflictOurs(String),
    ConflictTheirs(String),
    ConflictMarker(String),
}

// 3-way merge on line arrays
// Returns merged lines and conflict count
pub fn merge_3way(base: &str, ours: &str, theirs: &str) -> (Vec<MergeLine>, usize) {
    let base_lines: Vec<&str> = base.lines().collect();
    let ours_lines: Vec<&str> = ours.lines().collect();
    let theirs_lines: Vec<&str> = theirs.lines().collect();

    let diff_ours = diff_lines(&base_lines, &ours_lines);
    let diff_theirs = diff_lines(&base_lines, &theirs_lines);

    let mut result = Vec::new();
    let mut conflicts = 0;

    // Walk both diffs in lockstep
    // Simple heuristic: if both diffs change the same region, it's a conflict
    let mut i = 0;
    let mut j = 0;

    let ours_ops: Vec<_> = diff_ours.iter().collect();
    let theirs_ops: Vec<_> = diff_theirs.iter().collect();

    while i < ours_ops.len() && j < theirs_ops.len() {
        match (ours_ops[i], theirs_ops[j]) {
            (Edit::Keep(a), Edit::Keep(b)) if a == b => {
                result.push(MergeLine::Clean(a.to_string()));
                i += 1;
                j += 1;
            }
            (Edit::Insert(line), Edit::Keep(_)) => {
                result.push(MergeLine::Clean(line.to_string()));
                i += 1;
            }
            (Edit::Keep(_), Edit::Insert(line)) => {
                result.push(MergeLine::Clean(line.to_string()));
                j += 1;
            }
            (Edit::Delete(_), Edit::Delete(_)) => {
                // Both deleted same line — agree
                i += 1;
                j += 1;
            }
            (Edit::Delete(_), Edit::Keep(_)) => {
                // Ours deleted, theirs kept — use ours (delete it)
                i += 1;
                j += 1;
            }
            (Edit::Keep(_), Edit::Delete(_)) => {
                // Theirs deleted, ours kept — use theirs (delete it)
                i += 1;
                j += 1;
            }
            // Conflict: both modified
            _ => {
                conflicts += 1;
                result.push(MergeLine::ConflictMarker("<<<<<<< ours".to_string()));
                // Collect ours changes
                while i < ours_ops.len() && !matches!(ours_ops[i], Edit::Keep(_)) {
                    if let Edit::Insert(l) | Edit::Keep(l) = ours_ops[i] {
                        result.push(MergeLine::ConflictOurs(l.to_string()));
                    }
                    i += 1;
                }
                result.push(MergeLine::ConflictMarker("=======".to_string()));
                // Collect theirs changes
                while j < theirs_ops.len() && !matches!(theirs_ops[j], Edit::Keep(_)) {
                    if let Edit::Insert(l) | Edit::Keep(l) = theirs_ops[j] {
                        result.push(MergeLine::ConflictTheirs(l.to_string()));
                    }
                    j += 1;
                }
                result.push(MergeLine::ConflictMarker(">>>>>>> theirs".to_string()));
            }
        }
    }

    // Append remaining
    while i < ours_ops.len() {
        if let Edit::Insert(l) | Edit::Keep(l) = ours_ops[i] {
            result.push(MergeLine::Clean(l.to_string()));
        }
        i += 1;
    }
    while j < theirs_ops.len() {
        if let Edit::Insert(l) | Edit::Keep(l) = theirs_ops[j] {
            result.push(MergeLine::Clean(l.to_string()));
        }
        j += 1;
    }

    (result, conflicts)
}

pub fn merge_lines_to_string(lines: &[MergeLine]) -> String {
    lines.iter().map(|l| match l {
        MergeLine::Clean(s) |
        MergeLine::ConflictOurs(s) |
        MergeLine::ConflictTheirs(s) |
        MergeLine::ConflictMarker(s) => format!("{}\n", s),
    }).collect()
}
