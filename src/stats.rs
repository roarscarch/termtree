use crate::{Forest, CommitNode, MergeNode, Tree};
use std::collections::HashMap;

/// Statistics about a forest's commit graph.
#[derive(Debug, Clone)]
pub struct ForestStats {
    /// Total number of commits
    pub total_commits: usize,
    /// Number of branches (tips)
    pub branch_count: usize,
    /// Number of merge commits
    pub merge_count: usize,
    /// Number of unique authors
    pub author_count: usize,
    /// Commits per author (author name -> count)
    pub commits_per_author: HashMap<String, usize>,
    /// Commits per day (YYYY-MM-DD -> count)
    pub commits_per_day: HashMap<String, usize>,
    /// Average commits per branch
    pub avg_commits_per_branch: f64,
    /// Longest branch length (in commits)
    pub longest_branch: usize,
    /// Shortest branch length (in commits)
    pub shortest_branch: usize,
    /// Merge frequency (merges per 100 commits)
    pub merge_frequency: f64,
    /// Number of merge storms detected
    pub merge_storm_count: usize,
    /// Total lines of code changed (additions + deletions) across all commits
    pub total_lines_changed: i64,
    /// Date range (earliest and latest commit timestamps)
    pub date_range: Option<(String, String)>,
}

/// Compute statistics from a forest.
pub fn compute_stats(forest: &Forest) -> ForestStats {
    let total_commits = forest.commit_map.len();
    let mut authors: HashMap<String, usize> = HashMap::new();
    let mut days: HashMap<String, usize> = HashMap::new();
    let mut merge_count = 0;
    let mut branch_lengths: Vec<usize> = Vec::new();
    let mut total_lines: i64 = 0;
    let mut timestamps: Vec<i64> = Vec::new();

    // Build reverse map: children for each commit
    let mut children: HashMap<String, Vec<String>> = HashMap::new();
    for (id, node) in &forest.commit_map {
        let author = &node.author;
        *authors.entry(author.clone()).or_insert(0) += 1;

        // Day from timestamp
        if let Some(ts) = node.timestamp {
            let secs = ts as i64;
            timestamps.push(secs);
            // Convert to date string (simple: use chrono if available, else approximate)
            let days_since_epoch = secs / 86400;
            let date = format_date_from_epoch(days_since_epoch);
            *days.entry(date).or_insert(0) += 1;
        }

        if node.parents.len() > 1 {
            merge_count += 1;
        }

        // Accumulate lines changed if available
        if let Some(lines) = node.lines_changed {
            total_lines += lines;
        }

        // Build children mapping
        for parent_id in &node.parents {
            children.entry(parent_id.clone()).or_default().push(id.clone());
        }
    }

    // Compute branch lengths: walk from root commits to leaves
    // Find root commits (commits with no children)
    let mut roots: Vec<&String> = Vec::new();
    for id in forest.commit_map.keys() {
        if !children.contains_key(id) || children[id].is_empty() {
            roots.push(id);
        }
    }

    // DFS from each root to compute branch lengths
    for root in roots {
        let mut stack = vec![(root.clone(), 1usize)];
        while let Some((node_id, depth)) = stack.pop() {
            branch_lengths.push(depth);
            if let Some(child_ids) = children.get(&node_id) {
                for child in child_ids {
                    stack.push((child.clone(), depth + 1));
                }
            }
        }
    }

    let branch_count = branch_lengths.len();
    let avg_commits_per_branch = if branch_count > 0 {
        branch_lengths.iter().sum::<usize>() as f64 / branch_count as f64
    } else {
        0.0
    };
    let longest_branch = branch_lengths.iter().cloned().max().unwrap_or(0);
    let shortest_branch = branch_lengths.iter().cloned().min().unwrap_or(0);
    let merge_frequency = if total_commits > 0 {
        merge_count as f64 / total_commits as f64 * 100.0
    } else {
        0.0
    };

    // Merge storms: we can count from layout if available, otherwise estimate
    let merge_storm_count = forest.merge_storms.len();

    // Date range
    let date_range = if timestamps.is_empty() {
        None
    } else {
        let min_ts = timestamps.iter().min().unwrap();
        let max_ts = timestamps.iter().max().unwrap();
        let min_days = min_ts / 86400;
        let max_days = max_ts / 86400;
        Some((
            format_date_from_epoch(min_days),
            format_date_from_epoch(max_days),
        ))
    };

    ForestStats {
        total_commits,
        branch_count,
        merge_count,
        author_count: authors.len(),
        commits_per_author: authors,
        commits_per_day: days,
        avg_commits_per_branch,
        longest_branch,
        shortest_branch,
        merge_frequency,
        merge_storm_count,
        total_lines_changed: total_lines,
        date_range,
    }
}

/// Format a date from days since epoch (simple: no external crate dependency).
fn format_date_from_epoch(days_since_epoch: i64) -> String {
    // Approximate: Jan 1 1970 = 0 days
    // Use a simple algorithm
    let mut y = 1970i64;
    let mut remaining = days_since_epoch;
    loop {
        let days_in_year = if is_leap_year(y) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }
    let months = [31, if is_leap_year(y) { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut m = 0usize;
    for (i, &days) in months.iter().enumerate() {
        if remaining < days {
            m = i + 1;
            break;
        }
        remaining -= days;
    }
    let d = remaining + 1;
    format!("{:04}-{:02}-{:02}", y, m, d)
}

fn is_leap_year(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Forest, CommitNode, MergeStorm};

    #[test]
    fn test_compute_stats_empty() {
        let forest = Forest {
            commit_map: HashMap::new(),
            merge_storms: Vec::new(),
        };
        let stats = compute_stats(&forest);
        assert_eq!(stats.total_commits, 0);
        assert_eq!(stats.branch_count, 0);
        assert_eq!(stats.merge_count, 0);
        assert_eq!(stats.author_count, 0);
        assert!(stats.date_range.is_none());
    }

    #[test]
    fn test_compute_stats_single_commit() {
        let mut forest = Forest {
            commit_map: HashMap::new(),
            merge_storms: Vec::new(),
        }