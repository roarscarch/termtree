use crate::{Forest, Tree, CommitNode, MergeNode};
use std::collections::HashMap;

/// Generate a structured summary report of the forest.
pub fn generate_summary(forest: &Forest) -> SummaryReport {
    let total_commits = forest.commit_map.len();
    let total_trees = forest.trees.len();
    let total_merges = forest.merge_nodes.len();
    let total_authors = forest.commit_map.values().map(|c| &c.author).collect::<std::collections::HashSet<_>>().len();
    let mut author_commit_counts: HashMap<String, usize> = HashMap::new();
    for commit in forest.commit_map.values() {
        *author_commit_counts.entry(commit.author.clone()).or_insert(0) += 1;
    }
    let mut author_counts: Vec<(String, usize)> = author_commit_counts.into_iter().collect();
    author_counts.sort_by(|a, b| b.1.cmp(&a.1));

    let mut branch_lengths: Vec<usize> = forest.trees.iter().map(|t| t.nodes.len()).collect();
    branch_lengths.sort();
    let median_branch_length = if branch_lengths.is_empty() {
        0
    } else {
        let mid = branch_lengths.len() / 2;
        if branch_lengths.len() % 2 == 0 {
            (branch_lengths[mid - 1] + branch_lengths[mid]) / 2
        } else {
            branch_lengths[mid]
        }
    };
    let max_branch_length = branch_lengths.last().copied().unwrap_or(0);
    let min_branch_length = branch_lengths.first().copied().unwrap_or(0);

    // Count merge storms (simultaneous merges within a small time window)
    let mut merge_storms = 0;
    let mut merge_timestamps: Vec<u64> = forest.merge_nodes.iter().filter_map(|m| m.timestamp).collect();
    merge_timestamps.sort();
    let storm_window = 3600; // 1 hour in seconds
    let mut i = 0;
    while i < merge_timestamps.len() {
        let window_start = merge_timestamps[i];
        let mut count = 0;
        while i < merge_timestamps.len() && merge_timestamps[i] <= window_start + storm_window {
            count += 1;
            i += 1;
        }
        if count >= 3 {
            merge_storms += 1;
        }
    }

    SummaryReport {
        total_commits,
        total_trees,
        total_merges,
        total_authors,
        author_counts,
        median_branch_length,
        max_branch_length,
        min_branch_length,
        merge_storms,
    }
}

#[derive(Debug, Clone)]
pub struct SummaryReport {
    pub total_commits: usize,
    pub total_trees: usize,
    pub total_merges: usize,
    pub total_authors: usize,
    pub author_counts: Vec<(String, usize)>,
    pub median_branch_length: usize,
    pub max_branch_length: usize,
    pub min_branch_length: usize,
    pub merge_storms: usize,
}

impl std::fmt::Display for SummaryReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Forest Summary")?;
        writeln!(f, "{}", "─".repeat(40))?;
        writeln!(f, "Total commits: {}", self.total_commits)?;
        writeln!(f, "Total trees (branches): {}", self.total_trees)?;
        writeln!(f, "Total merges: {}", self.total_merges)?;
        writeln!(f, "Total authors: {}", self.total_authors)?;
        writeln!(f, "Branch lengths (min/median/max): {}/{}/{}", self.min_branch_length, self.median_branch_length, self.max_branch_length)?;
        writeln!(f, "Merge storms: {}", self.merge_storms)?;
        writeln!(f, "\nAuthor contribution:")?;
        for (author, count) in &self.author_counts {
            let bar = "█".repeat((*count as f64 / self.total_commits as f64 * 40.0) as usize);
            writeln!(f, "  {:20} {:5} {}", author, count, bar)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Forest, Tree, CommitNode, MergeNode};

    #[test]
    fn test_summary_empty_forest() {
        let forest = Forest {
            trees: vec![],
            commit_map: std::collections::HashMap::new(),
            merge_nodes: vec![],
            layout: None,
        };
        let report = generate_summary(&forest);
        assert_eq!(report.total_commits, 0);
        assert_eq!(report.total_trees, 0);
        assert_eq!(report.total_merges, 0);
        assert_eq!(report.total_authors, 0);
        assert_eq!(report.median_branch_length, 0);
        assert_eq!(report.max_branch_length, 0);
        assert_eq!(report.min_branch_length, 0);
        assert_eq!(report.merge_storms, 0);
    }

    #[test]
    fn test_summary_with_data() {
        let mut commit_map = std::collections::HashMap::new();
        commit_map.insert("a".to_string(), CommitNode {
            id: "a".to_string(),
            author: "alice".to_string(),
            timestamp: Some(1000),
            message: "first".to_string(),
            parents: vec![],
            children: vec!["b".to_string()],
            x: 0.0,
            y: 0.0,
        });
        commit_map.insert("b".to_string(), CommitNode {
            id: "b".to_string(),
            author: "bob".to_string(),
            timestamp: Some(2000),
            message: "second".to_string(),
            parents: vec!["a".to_string()],
            children: vec![],
            x: 0.0,
            y: 1.0,
        });
        let tree = Tree {
            id: "main".to_string(),
            nodes: vec!["a".to_string(), "b".to_string()],
            color: (100, 150, 200),
            trunk_width: 1.0,
        };
        let merge_node = MergeNode {
            id: "merge1".to_string(),
            parents: vec!["a".to_string(), "b".to_string()],
            children: vec![],
            timestamp: Some(3000),
            x: 0.5,
            y: 0.5,
        };
        let forest = Forest {
            trees: vec![tree],
            commit_map,
            merge_nodes: vec![merge_node],
            layout: None,
        };
        let report = generate_summary(&forest);
        assert_eq!(report.total_commits, 2);
        assert_eq!(report.total_trees, 1);
        assert_eq!(report.total_merges, 1);
        assert_eq!(report.total_authors, 2);
        assert_eq!(report.median_branch_length, 2);
        assert_eq!(report.max_branch_length, 2);
        assert_eq!(report.min_branch_length, 2);
        assert_eq!(report.merge_storms, 0);
    }
}