use crate::{Forest, Tree, CommitNode, MergeNode, LayoutResult};
use std::collections::{HashMap, HashSet};

/// Statistics about the forest
#[derive(Debug, Clone, Default)]
pub struct ForestStats {
    /// Total number of commits
    pub total_commits: usize,
    /// Number of branches (trees)
    pub total_branches: usize,
    /// Number of merge nodes
    pub total_merges: usize,
    /// Number of merge storms
    pub total_storms: usize,
    /// Commits per author
    pub commits_per_author: HashMap<String, usize>,
    /// Branch length distribution (number of commits per branch)
    pub branch_lengths: Vec<usize>,
    /// Average commits per branch
    pub avg_commits_per_branch: f64,
    /// Max branch length
    pub max_branch_length: usize,
    /// Min branch length
    pub min_branch_length: usize,
    /// Date range (earliest, latest timestamp)
    pub date_range: Option<(i64, i64)>,
    /// Number of authors
    pub total_authors: usize,
    /// Number of commits with no parent (root commits)
    pub root_commits: usize,
}

/// Compute statistics from the forest and layout
pub fn compute_stats(forest: &Forest, layout: &LayoutResult) -> ForestStats {
    let mut stats = ForestStats::default();
    
    stats.total_commits = forest.commit_map.len();
    stats.total_branches = forest.trees.len();
    stats.total_merges = forest.merge_nodes.len();
    stats.total_storms = layout.merge_storms.len();
    
    // Author analysis
    let mut author_count: HashMap<String, usize> = HashMap::new();
    let mut earliest: Option<i64> = None;
    let mut latest: Option<i64> = None;
    let mut root_count = 0;
    
    for (_hash, commit) in &forest.commit_map {
        *author_count.entry(commit.author.clone()).or_insert(0) += 1;
        
        if commit.parents.is_empty() {
            root_count += 1;
        }
        
        if let Some(ts) = commit.timestamp {
            if earliest.map_or(true, |e| ts < e) {
                earliest = Some(ts);
            }
            if latest.map_or(true, |l| ts > l) {
                latest = Some(ts);
            }
        }
    }
    
    stats.commits_per_author = author_count;
    stats.total_authors = stats.commits_per_author.len();
    stats.root_commits = root_count;
    
    if let (Some(e), Some(l)) = (earliest, latest) {
        stats.date_range = Some((e, l));
    }
    
    // Branch length analysis
    let mut branch_lengths: Vec<usize> = Vec::new();
    for tree in &forest.trees {
        let len = tree.nodes.len();
        branch_lengths.push(len);
    }
    
    branch_lengths.sort_unstable();
    stats.branch_lengths = branch_lengths.clone();
    
    if let Some(max) = branch_lengths.last() {
        stats.max_branch_length = *max;
    }
    if let Some(min) = branch_lengths.first() {
        stats.min_branch_length = *min;
    }
    
    let total: usize = branch_lengths.iter().sum();
    if !branch_lengths.is_empty() {
        stats.avg_commits_per_branch = total as f64 / branch_lengths.len() as f64;
    }
    
    stats
}

/// Format stats as a human-readable string
pub fn format_stats(stats: &ForestStats) -> String {
    let mut output = String::new();
    
    output.push_str(&format!("Forest Statistics\n"));
    output.push_str(&format!("{:-^40}\n", ""));
    output.push_str(&format!("Total commits: {}\n", stats.total_commits));
    output.push_str(&format!("Total branches: {}\n", stats.total_branches));
    output.push_str(&format!("Total merges: {}\n", stats.total_merges));
    output.push_str(&format!("Merge storms: {}\n", stats.total_storms));
    output.push_str(&format!("Root commits: {}\n", stats.root_commits));
    output.push_str(&format!("Total authors: {}\n", stats.total_authors));
    
    if let Some((earliest, latest)) = stats.date_range {
        output.push_str(&format!("Date range: {} to {}\n", earliest, latest));
    }
    
    output.push_str(&format!("\nBranch length distribution:\n"));
    output.push_str(&format!("  Min: {}\n", stats.min_branch_length));
    output.push_str(&format!("  Max: {}\n", stats.max_branch_length));
    output.push_str(&format!("  Avg: {:.2}\n", stats.avg_commits_per_branch));
    
    output.push_str(&format!("\nCommits per author:\n"));
    let mut authors: Vec<(&String, &usize)> = stats.commits_per_author.iter().collect();
    authors.sort_by(|a, b| b.1.cmp(a.1));
    for (author, count) in &authors {
        output.push_str(&format!("  {}: {}\n", author, count));
    }
    
    output
}

/// Print stats to stdout
pub fn print_stats(stats: &ForestStats) {
    println!("{}", format_stats(stats));
}
