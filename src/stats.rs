use crate::{Forest, Tree, CommitNode, MergeNode};
use std::collections::HashMap;

/// Compute and display summary statistics for a forest.
pub fn forest_stats(forest: &Forest) -> String {
    let mut output = String::new();

    // Count total commits
    let total_commits = forest.commit_map.len();
    output.push_str(&format!("Total commits: {}\n", total_commits));

    // Count trees (branches)
    let total_trees = forest.trees.len();
    output.push_str(&format!("Total branches (trees): {}\n", total_trees));

    // Count merge nodes
    let total_merges = forest.merge_nodes.len();
    output.push_str(&format!("Total merge nodes: {}\n", total_merges));

    // Average commits per tree
    if total_trees > 0 {
        let avg = total_commits as f64 / total_trees as f64;
        output.push_str(&format!("Average commits per branch: {:.2}\n", avg));
    }

    // Count unique authors
    let mut authors: Vec<&String> = forest.commit_map.keys().collect();
    authors.sort();
    let unique_authors = authors.len();
    output.push_str(&format!("Unique authors: {}\n", unique_authors));

    // Author commit counts
    output.push_str("\nCommits per author:\n");
    let mut author_counts: HashMap<&String, usize> = HashMap::new();
    for (_, commit) in &forest.commit_map {
        for c in commit {
            *author_counts.entry(&c.author).or_insert(0) += 1;
        }
    }
    let mut author_vec: Vec<(&String, usize)> = author_counts.into_iter().collect();
    author_vec.sort_by(|a, b| b.1.cmp(&a.1));
    for (author, count) in author_vec {
        output.push_str(&format!("  {}: {}\n", author, count));
    }

    // Longest branch (tree with most commits)
    if let Some(max_tree) = forest.trees.iter().max_by_key(|t| t.commits.len()) {
        output.push_str(&format!("\nLongest branch: {} ({} commits)\n", max_tree.name, max_tree.commits.len()));
    }

    // Merge storm count
    let storm_count = forest.merge_nodes.iter().filter(|m| m.is_storm).count();
    output.push_str(&format!("Merge storms detected: {}\n", storm_count));

    output
}
