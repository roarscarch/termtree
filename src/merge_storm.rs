use crate::{Forest, CommitNode, MergeNode};
use std::collections::{HashMap, HashSet};

/// Represents a detected merge storm — a period with many simultaneous merges.
#[derive(Debug, Clone)]
pub struct MergeStorm {
    /// The central commit ID where the storm occurs
    pub epicenter: String,
    /// Number of branches involved in the storm
    pub branch_count: usize,
    /// Total commits in the storm window
    pub commit_count: usize,
    /// The time window (in seconds) during which the storm occurred
    pub time_window_secs: f64,
    /// Intensity score (0.0 to 1.0) based on branch density
    pub intensity: f64,
    /// List of branch names involved
    pub branches: Vec<String>,
    /// Commit IDs of all commits in the storm
    pub commit_ids: Vec<String>,
}

/// Detect merge storms in the forest.
/// A merge storm is defined as a commit that has more than `threshold` parents
/// or a cluster of merges within a short time window.
pub fn detect_merge_storms(
    forest: &Forest,
    threshold: usize,
    time_window: f64,
) -> Vec<MergeStorm> {
    let mut storms = Vec::new();
    let mut visited = HashSet::new();

    // Build a map from commit ID to its timestamp
    let timestamps: HashMap<&String, f64> = forest
        .commit_map
        .iter()
        .map(|(id, node)| (id, node.timestamp))
        .collect();

    // Find merge points (commits with >1 parent)
    let merges: Vec<&CommitNode> = forest
        .commit_map
        .values()
        .filter(|node| node.parents.len() > 1)
        .collect();

    // Cluster merges by time proximity
    let mut merge_clusters: Vec<Vec<&CommitNode>> = Vec::new();

    for merge in &merges {
        if visited.contains(&merge.id) {
            continue;
        }

        let mut cluster = vec![*merge];
        visited.insert(merge.id.clone());

        // Find nearby merges within the time window
        for other in &merges {
            if visited.contains(&other.id) {
                continue;
            }
            if let (Some(t1), Some(t2)) = (timestamps.get(&merge.id), timestamps.get(&other.id)) {
                if (t1 - t2).abs() <= time_window {
                    cluster.push(*other);
                    visited.insert(other.id.clone());
                }
            }
        }

        if cluster.len() >= threshold {
            merge_clusters.push(cluster);
        }
    }

    // Convert clusters to MergeStorm objects
    for cluster in merge_clusters {
        let mut branch_set = HashSet::new();
        let mut all_ids = Vec::new();
        let mut total_parents = 0;

        for node in &cluster {
            all_ids.push(node.id.clone());
            // Collect branch names from parents
            for parent_id in &node.parents {
                if let Some(parent) = forest.commit_map.get(parent_id) {
                    branch_set.insert(parent.branch.clone());
                }
            }
            total_parents += node.parents.len();
        }

        let branches: Vec<String> = branch_set.into_iter().collect();
        let branch_count = branches.len();
        let commit_count = all_ids.len();

        // Intensity: ratio of branches to commits, normalized
        let intensity = if commit_count > 0 {
            (branch_count as f64 / commit_count as f64).min(1.0)
        } else {
            0.0
        };

        // Calculate time window
        let timestamps_cluster: Vec<f64> = cluster
            .iter()
            .filter_map(|node| timestamps.get(&node.id))
            .copied()
            .collect();
        let time_window_secs = if timestamps_cluster.len() >= 2 {
            let min_t = timestamps_cluster.iter().cloned().fold(f64::INFINITY, f64::min);
            let max_t = timestamps_cluster.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            (max_t - min_t).max(0.0)
        } else {
            0.0
        };

        // Use the first merge as epicenter
        let epicenter = cluster[0].id.clone();

        storms.push(MergeStorm {
            epicenter,
            branch_count,
            commit_count,
            time_window_secs,
            intensity,
            branches,
            commit_ids: all_ids,
        });
    }

    storms
}

/// Generate a tangled root system visualization for a merge storm.
/// Returns a vector of (x, y) coordinates representing root curves.
pub fn generate_tangled_roots(
    storm: &MergeStorm,
    center_x: f64,
    center_y: f64,
    spread: f64,
) -> Vec<(f64, f64)> {
    let mut roots = Vec::new();
    let num_roots = storm.branch_count.max(2);
    let angle_step = std::f64::consts::PI * 2.0 / num_roots as f64;

    for i in 0..num_roots {
        let base_angle = angle_step * i as f64 + storm.intensity * 0.5;
        let length = spread * (0.5 + storm.intensity * 0.5);
        let segments = 10;

        for j in 0..=segments {
            let t = j as f64 / segments as f64;
            let angle = base_angle + t * storm.intensity * 1.5;
            let radius = t * length * (1.0 + 0.3 * (t * 3.0).sin());
            let x = center_x + radius * angle.cos();
            let y = center_y + radius * angle.sin();
            roots.push((x, y));
        }
    }

    roots
}

/// Highlight merge storms in the forest by adding visual markers.
/// Returns a map of commit ID -> intensity for colored rendering.
pub fn highlight_merge_storms(
    forest: &Forest,
    threshold: usize,
    time_window: f64,
) -> HashMap<String, f64> {
    let storms = detect_merge_storms(forest, threshold, time_window);
    let mut highlights = HashMap::new();

    for storm in &storms {
        for id in &storm.commit_ids {
            highlights.insert(id.clone(), storm.intensity);
        }
    }

    highlights
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Forest, CommitNode};
    use std::collections::HashMap;

    fn create_test_forest() -> Forest {
        let mut commit_map = HashMap::new();
        // Create a merge storm: commit 'c' has 3 parents
        commit_map.insert(
            "a".to_string(),
            CommitNode {
                id: "a".to_string(),
                author: "alice".to_string(),
                timestamp: 1000.0,
                parents: vec![],
                branch: "main".to_string(),
                message: "initial".to_string(),
            },
        );
        commit_map.insert(
            "b".to_string(),
            CommitNode {
                id: "b".to_string(),
                author: "bob".to_string(),
                timestamp: 1001.0,
                parents: vec!["a".to_string()],
                branch: "feature".to_string(),
                message: "work".to_string(),
            }