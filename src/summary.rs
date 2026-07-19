use crate::{Forest, Tree, CommitNode, MergeNode, LayoutResult};
use std::collections::HashMap;

/// Summary statistics for a git forest.
#[derive(Debug, Clone)]
pub struct ForestSummary {
    /// Total number of commits in the repository
    pub total_commits: usize,
    /// Number of unique authors
    pub unique_authors: usize,
    /// Number of branches (long-lived lines)
    pub branch_count: usize,
    /// Number of merge commits
    pub merge_count: usize,
    /// Number of merge storms detected
    pub merge_storm_count: usize,
    /// Total number of trees in the forest
    pub tree_count: usize,
    /// Average commits per tree
    pub avg_commits_per_tree: f64,
    /// Maximum depth (height) of the forest
    pub max_depth: usize,
    /// Commit frequency per author: author -> count
    pub author_commit_counts: HashMap<String, usize>,
    /// Number of commits in the longest single branch
    pub longest_branch_length: usize,
    /// Number of commits in the shortest single branch (non-zero)
    pub shortest_branch_length: usize,
    /// Total number of merge storms
    pub storm_count: usize,
}

impl ForestSummary {
    /// Compute summary from forest and layout.
    pub fn from_forest(forest: &Forest, layout: &LayoutResult) -> Self {
        let total_commits = forest.commit_map.len();
        let mut author_counts: HashMap<String, usize> = HashMap::new();
        for node in forest.commit_map.values() {
            *author_counts.entry(node.author.clone()).or_insert(0) += 1;
        }
        let unique_authors = author_counts.len();
        let merge_count = forest.merge_nodes.len();
        let tree_count = forest.trees.len();
        let branch_count = forest.trees.iter().filter(|t| t.is_long_lived).count();
        let avg_commits_per_tree = if tree_count > 0 {
            total_commits as f64 / tree_count as f64
        } else {
            0.0
        };
        let max_depth = forest.trees.iter().map(|t| t.depth).max().unwrap_or(0);
        let longest_branch_length = forest.trees.iter().map(|t| t.commit_count).max().unwrap_or(0);
        let shortest_branch_length = forest.trees.iter().map(|t| t.commit_count).min().unwrap_or(0);
        let storm_count = layout.merge_storms.len();
        ForestSummary {
            total_commits,
            unique_authors,
            branch_count,
            merge_count,
            merge_storm_count: storm_count,
            tree_count,
            avg_commits_per_tree,
            max_depth,
            author_commit_counts: author_counts,
            longest_branch_length,
            shortest_branch_length,
            storm_count,
        }
    }

    /// Render the summary as a formatted string for terminal output.
    pub fn render(&self) -> String {
        let mut output = String::new();
        output.push_str("🌲 Git Forest Summary\n");
        output.push_str("━━━━━━━━━━━━━━━━━━━━━\n");
        output.push_str(&format!("Total commits: {}\n", self.total_commits));
        output.push_str(&format!("Unique authors: {}\n", self.unique_authors));
        output.push_str(&format!("Trees (branches): {}\n", self.tree_count));
        output.push_str(&format!("Long-lived branches: {}\n", self.branch_count));
        output.push_str(&format!("Merge commits: {}\n", self.merge_count));
        output.push_str(&format!("Merge storms: {}\n", self.merge_storm_count));
        output.push_str(&format!("Average commits per tree: {:.2}\n", self.avg_commits_per_tree));
        output.push_str(&format!("Max depth: {}\n", self.max_depth));
        output.push_str(&format!("Longest branch: {} commits\n", self.longest_branch_length));
        output.push_str(&format!("Shortest branch: {} commits\n", self.shortest_branch_length));
        output.push_str("\nAuthor breakdown:\n");
        let mut authors: Vec<(&String, &usize)> = self.author_commit_counts.iter().collect();
        authors.sort_by(|a, b| b.1.cmp(a.1));
        for (author, count) in authors {
            let bar_width = (*count as f64 / self.total_commits as f64 * 30.0).ceil() as usize;
            let bar = "█".repeat(bar_width);
            output.push_str(&format!("  {}: {} {}\n", author, count, bar));
        }
        output
    }
}

/// Compute and display the forest summary.
pub fn display_summary(forest: &Forest, layout: &LayoutResult) {
    let summary = ForestSummary::from_forest(forest, layout);
    println!("{}", summary.render());
}
