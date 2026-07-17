use crate::{Forest, Tree, CommitNode, MergeNode};
use std::collections::{HashMap, HashSet};

/// Statistics computed from a Forest
#[derive(Debug, Clone)]
pub struct ForestStats {
    /// Total number of commits
    pub total_commits: usize,
    /// Number of unique authors
    pub unique_authors: usize,
    /// Number of trees (branches)
    pub tree_count: usize,
    /// Number of merges
    pub merge_count: usize,
    /// Average commits per tree
    pub avg_commits_per_tree: f64,
    /// Most active author (commits)
    pub most_active_author: String,
    /// Most active author commit count
    pub most_active_author_commits: usize,
    /// Date range: earliest commit timestamp (seconds since epoch)
    pub earliest_commit: i64,
    /// Date range: latest commit timestamp
    pub latest_commit: i64,
    /// Number of merge storms detected
    pub merge_storm_count: usize,
    /// Total branch length (sum of all tree heights)
    pub total_branch_length: usize,
}

/// Compute statistics from the forest
pub fn compute_stats(forest: &Forest) -> ForestStats {
    let total_commits = forest.commit_map.len();

    // Collect unique authors
    let authors: HashSet<&String> = forest.commit_map.values().map(|c| &c.author).collect();
    let unique_authors = authors.len();

    // Tree count
    let tree_count = forest.trees.len();

    // Merge count from commit_map: commits with parent count > 1
    let merge_count = forest
        .commit_map
        .values()
        .filter(|c| c.parents.len() > 1)
        .count();

    // Average commits per tree
    let avg_commits_per_tree = if tree_count > 0 {
        total_commits as f64 / tree_count as f64
    } else {
        0.0
    };

    // Most active author
    let mut author_commit_counts: HashMap<&String, usize> = HashMap::new();
    for c in forest.commit_map.values() {
        *author_commit_counts.entry(&c.author).or_insert(0) += 1;
    }
    let most_active_author = author_commit_counts
        .iter()
        .max_by_key(|&(_, count)| count)
        .map(|(author, _)| (*author).clone())
        .unwrap_or_else(|| "Unknown".to_string());
    let most_active_author_commits = *author_commit_counts
        .get(&most_active_author)
        .unwrap_or(&0);

    // Date range (timestamps from commit_map)
    let timestamps: Vec<i64> = forest.commit_map.values().map(|c| c.timestamp).collect();
    let earliest_commit = timestamps.iter().cloned().min().unwrap_or(0);
    let latest_commit = timestamps.iter().cloned().max().unwrap_or(0);

    // Merge storm count (count trees that have merge_storm flag set)
    let merge_storm_count = forest
        .trees
        .iter()
        .filter(|t| t.merge_storm)
        .count();

    // Total branch length: sum of all tree heights (number of commits per tree)
    let total_branch_length = forest.trees.iter().map(|t| t.commits.len()).sum();

    ForestStats {
        total_commits,
        unique_authors,
        tree_count,
        merge_count,
        avg_commits_per_tree,
        most_active_author,
        most_active_author_commits,
        earliest_commit,
        latest_commit,
        merge_storm_count,
        total_branch_length,
    }
}

/// Format a timestamp (seconds since epoch) into a human-readable date string
fn format_timestamp(ts: i64) -> String {
    use std::time::{UNIX_EPOCH, Duration};
    let d = UNIX_EPOCH + Duration::from_secs(ts as u64);
    // Simple formatting: use chrono-like approach via system time
    // Since chrono is not a dependency, we'll use a basic approach
    let secs = ts;
    let days = secs / 86400;
    let year = 1970 + (days / 365) as i32;
    let month = ((days % 365) / 30) + 1;
    let day = (days % 30) + 1;
    format!("{:04}-{:02}-{:02}", year, month, day)
}

/// Display statistics in a formatted table
pub fn display_stats(stats: &ForestStats) -> String {
    let mut output = String::new();

    output.push_str("\r\n");
    output.push_str("═══════════════════════════════════════\r\n");
    output.push_str("        Git Forest Statistics\r\n");
    output.push_str("═══════════════════════════════════════\r\n");
    output.push_str(&format!(" Total commits:          {:>8}\r\n", stats.total_commits));
    output.push_str(&format!(" Unique authors:         {:>8}\r\n", stats.unique_authors));
    output.push_str(&format!(" Trees (branches):       {:>8}\r\n", stats.tree_count));
    output.push_str(&format!(" Merges:                 {:>8}\r\n", stats.merge_count));
    output.push_str(&format!(" Avg commits per tree:   {:>8.2}\r\n", stats.avg_commits_per_tree));
    output.push_str(&format!(" Most active author:     {} ({} commits)\r\n", stats.most_active_author, stats.most_active_author_commits));
    output.push_str(&format!(" Earliest commit:        {}\r\n", format_timestamp(stats.earliest_commit)));
    output.push_str(&format!(" Latest commit:          {}\r\n", format_timestamp(stats.latest_commit)));
    output.push_str(&format!(" Merge storms:           {:>8}\r\n", stats.merge_storm_count));
    output.push_str(&format!(" Total branch length:    {:>8}\r\n", stats.total_branch_length));
    output.push_str("═══════════════════════════════════════\r\n");

    output
}
