use crate::{Forest, Tree, CommitNode, MergeNode};
use std::collections::HashMap;

/// Statistics for a single tree (branch)
#[derive(Debug, Clone)]
pub struct TreeStats {
    pub name: String,
    pub commit_count: usize,
    pub merge_count: usize,
    pub depth: usize,
    pub width: usize,
    pub author: String,
    pub first_commit_time: Option<i64>,
    pub last_commit_time: Option<i64>,
    pub leaf_count: usize,
}

/// Overall forest statistics
#[derive(Debug, Clone)]
pub struct ForestStats {
    pub total_trees: usize,
    pub total_commits: usize,
    pub total_merges: usize,
    pub total_authors: usize,
    pub average_depth: f64,
    pub average_width: f64,
    pub bus_factor: usize,
    pub tree_stats: Vec<TreeStats>,
    pub author_commit_counts: HashMap<String, usize>,
    pub merge_storms: Vec<MergeStorm>,
}

/// Information about a merge storm (many simultaneous merges)
#[derive(Debug, Clone)]
pub struct MergeStorm {
    pub time_range: (i64, i64),
    pub merge_count: usize,
    pub involved_branches: Vec<String>,
}

/// Compute statistics for the entire forest
pub fn compute_forest_stats(forest: &Forest) -> ForestStats {
    let mut total_commits = 0;
    let mut total_merges = 0;
    let mut author_commit_counts: HashMap<String, usize> = HashMap::new();
    let mut tree_stats = Vec::new();
    let mut all_commit_times: Vec<(i64, &str)> = Vec::new(); // (timestamp, branch_name)

    for tree in &forest.trees {
        let mut commit_count = 0;
        let mut merge_count = 0;
        let mut depth = 0;
        let mut width = 0;
        let mut first_time: Option<i64> = None;
        let mut last_time: Option<i64> = None;
        let mut leaf_count = 0;

        // Traverse all nodes in the tree
        for node in &tree.nodes {
            match node {
                CommitNode::Regular(c) => {
                    commit_count += 1;
                    *author_commit_counts.entry(c.author.clone()).or_insert(0) += 1;
                    if let Some(ts) = c.timestamp {
                        if first_time.map_or(true, |ft| ts < ft) {
                            first_time = Some(ts);
                        }
                        if last_time.map_or(true, |lt| ts > lt) {
                            last_time = Some(ts);
                        }
                        all_commit_times.push((ts, &tree.name));
                    }
                    if c.is_leaf {
                        leaf_count += 1;
                    }
                }
                CommitNode::Merge(m) => {
                    merge_count += 1;
                    total_merges += 1;
                    *author_commit_counts.entry(m.author.clone()).or_insert(0) += 1;
                    if let Some(ts) = m.timestamp {
                        if first_time.map_or(true, |ft| ts < ft) {
                            first_time = Some(ts);
                        }
                        if last_time.map_or(true, |lt| ts > lt) {
                            last_time = Some(ts);
                        }
                        all_commit_times.push((ts, &tree.name));
                    }
                }
            }
        }

        // Compute depth (max chain length) and width (max branches at any level)
        // Simple approximation: depth = number of levels in tree
        depth = forest.layout.as_ref().map(|l| l.depths.get(&tree.name).copied().unwrap_or(0)).unwrap_or(0);
        width = forest.layout.as_ref().map(|l| l.widths.get(&tree.name).copied().unwrap_or(0)).unwrap_or(0);

        total_commits += commit_count;

        let author = if !tree.nodes.is_empty() {
            match &tree.nodes[0] {
                CommitNode::Regular(c) => c.author.clone(),
                CommitNode::Merge(m) => m.author.clone(),
            }
        } else {
            String::from("unknown")
        };

        tree_stats.push(TreeStats {
            name: tree.name.clone(),
            commit_count,
            merge_count,
            depth,
            width,
            author,
            first_commit_time: first_time,
            last_commit_time: last_time,
            leaf_count,
        });
    }

    // Sort tree stats by commit count descending
    tree_stats.sort_by(|a, b| b.commit_count.cmp(&a.commit_count));

    // Detect merge storms: clusters of merges close in time
    let merge_storms = detect_merge_storms(&all_commit_times, 60); // 60-second window

    let total_authors = author_commit_counts.len();
    let average_depth = if !tree_stats.is_empty() {
        tree_stats.iter().map(|t| t.depth as f64).sum::<f64>() / tree_stats.len() as f64
    } else {
        0.0
    };
    let average_width = if !tree_stats.is_empty() {
        tree_stats.iter().map(|t| t.width as f64).sum::<f64>() / tree_stats.len() as f64
    } else {
        0.0
    };

    // Bus factor: number of authors responsible for 50% of commits
    let mut sorted_author_counts: Vec<usize> = author_commit_counts.values().copied().collect();
    sorted_author_counts.sort_by(|a, b| b.cmp(a));
    let half_commits = total_commits / 2;
    let mut cumulative = 0;
    let mut bus_factor = 0;
    for count in &sorted_author_counts {
        cumulative += count;
        bus_factor += 1;
        if cumulative >= half_commits {
            break;
        }
    }

    ForestStats {
        total_trees: forest.trees.len(),
        total_commits,
        total_merges,
        total_authors,
        average_depth,
        average_width,
        bus_factor,
        tree_stats,
        author_commit_counts,
        merge_storms,
    }
}

/// Detect merge storms: clusters of merges within a time window
fn detect_merge_storms(commits: &[(i64, &str)], window_seconds: i64) -> Vec<MergeStorm> {
    if commits.is_empty() {
        return Vec::new();
    }

    let mut sorted_commits: Vec<(i64, &str)> = commits.to_vec();
    sorted_commits.sort_by_key(|k| k.0);

    let mut storms = Vec::new();
    let mut i = 0;
    while i < sorted_commits.len() {
        let start_time = sorted_commits[i].0;
        let end_time = start_time + window_seconds;
        let mut cluster: Vec<(i64, &str)> = Vec::new();
        while i < sorted_commits.len() && sorted_commits[i].0 <= end_time {
            cluster.push(sorted_commits[i]);
            i += 1;
        }
        if cluster.len() >= 3 {
            let mut branches: Vec<String> = cluster.iter().map(|(_, b)| b.to_string()).collect();
            branches.sort();
            branches.dedup();
            storms.push(MergeStorm {
                time_range: (cluster[0].0, cluster[cluster.len() - 1].0),
                merge_count: cluster.len(),
                involved_branches: branches,
            });
        }
    }
    storms
}