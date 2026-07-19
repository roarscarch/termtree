use crate::{Forest, CommitNode, MergeNode, LayoutResult};
use std::collections::HashMap;

/// Represents a detected merge storm — a period of many simultaneous merges.
#[derive(Debug, Clone)]
pub struct MergeStorm {
    /// The time window (commit timestamp range) for this storm
    pub time_range: (i64, i64),
    /// List of merge commits that are part of this storm
    pub merges: Vec<String>,
    /// Number of distinct branches involved
    pub branch_count: usize,
    /// Intensity score (0.0 to 1.0) based on merge density and branch count
    pub intensity: f64,
}

/// Detect merge storms in a forest layout.
/// A merge storm is defined as a time window where the number of merge commits
/// exceeds a dynamic threshold based on average merge frequency.
pub fn detect_merge_storms(forest: &Forest, layout: &LayoutResult) -> Vec<MergeStorm> {
    let merge_commits: Vec<&String> = forest
        .merge_nodes
        .iter()
        .map(|n| &n.commit_id)
        .collect();

    if merge_commits.is_empty() {
        return Vec::new();
    }

    // Collect timestamps for merge commits
    let mut merge_times: Vec<(i64, &String)> = merge_commits
        .iter()
        .filter_map(|id| {
            forest.commit_map.get(*id).map(|c| (c.timestamp, *id))
        })
        .collect();
    merge_times.sort_by_key(|(t, _)| *t);

    // Use a sliding window to find high-density periods
    let window_size: i64 = 3600; // 1 hour in seconds
    let min_storm_merges: usize = 3;
    let mut storms: Vec<MergeStorm> = Vec::new();

    let mut i = 0;
    while i < merge_times.len() {
        let window_start = merge_times[i].0;
        let window_end = window_start + window_size;
        let mut window_merges: Vec<&String> = Vec::new();
        for j in i..merge_times.len() {
            if merge_times[j].0 <= window_end {
                window_merges.push(merge_times[j].1);
            } else {
                break;
            }
        }

        if window_merges.len() >= min_storm_merges {
            // Compute branch count: unique parent branches for these merges
            let mut parent_branches: Vec<&String> = Vec::new();
            for merge_id in &window_merges {
                if let Some(merge_node) = forest
                    .merge_nodes
                    .iter()
                    .find(|n| &&n.commit_id == merge_id)
                {
                    for parent in &merge_node.parents {
                        if !parent_branches.contains(parent) {
                            parent_branches.push(parent);
                        }
                    }
                }
            }

            let branch_count = parent_branches.len();
            let intensity = (window_merges.len() as f64 / 10.0).min(1.0)
                * (branch_count as f64 / 10.0).min(1.0);

            storms.push(MergeStorm {
                time_range: (window_start, window_end),
                merges: window_merges.iter().map(|s| (*s).clone()).collect(),
                branch_count,
                intensity,
            });

            // Skip ahead to avoid overlapping storms
            i += window_merges.len();
        } else {
            i += 1;
        }
    }

    // Merge overlapping storms
    merge_overlapping_storms(&mut storms);

    storms
}

fn merge_overlapping_storms(storms: &mut Vec<MergeStorm>) {
    if storms.is_empty() {
        return;
    }
    storms.sort_by_key(|s| s.time_range.0);

    let mut merged: Vec<MergeStorm> = Vec::new();
    let mut current = storms[0].clone();

    for storm in storms.iter().skip(1) {
        if storm.time_range.0 <= current.time_range.1 {
            // Overlap: merge
            current.time_range.1 = current.time_range.1.max(storm.time_range.1);
            for merge_id in &storm.merges {
                if !current.merges.contains(merge_id) {
                    current.merges.push(merge_id.clone());
                }
            }
            current.branch_count = current.branch_count.max(storm.branch_count);
            current.intensity = current.intensity.max(storm.intensity);
        } else {
            merged.push(current);
            current = storm.clone();
        }
    }
    merged.push(current);

    *storms = merged;
}

/// Assign visual styling for merge storms in the render.
/// Returns a tuple (color_r, color_g, color_b) based on intensity.
pub fn storm_color(intensity: f64) -> (u8, u8, u8) {
    // From dark purple (low intensity) to bright red (high intensity)
    let r = (128.0 + intensity * 127.0) as u8;
    let g = (0.0 + (1.0 - intensity) * 80.0) as u8;
    let b = (128.0 - intensity * 128.0) as u8;
    (r, g, b)
}

/// Return a label for the storm severity based on intensity.
pub fn storm_label(intensity: f64) -> &'static str {
    if intensity >= 0.8 {
        "severe merge storm"
    } else if intensity >= 0.5 {
        "moderate merge storm"
    } else if intensity >= 0.3 {
        "minor merge storm"
    } else {
        "gentle merge breeze"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Forest, CommitNode, MergeNode};

    fn make_forest_with_merges(merge_timestamps: &[i64]) -> Forest {
        let mut forest = Forest {
            trees: Vec::new(),
            commit_map: HashMap::new(),
            merge_nodes: Vec::new(),
            branch_names: Vec::new(),
        };

        for (i, ts) in merge_timestamps.iter().enumerate() {
            let commit_id = format!("merge{}", i);
            forest.commit_map.insert(
                commit_id.clone(),
                CommitNode {
                    id: commit_id.clone(),
                    message: format!("Merge {}", i),
                    author: "test".to_string(),
                    timestamp: *ts,
                    parents: vec!["parent1".to_string(), "parent2".to_string()],
                    children: Vec::new(),
                },
            );
            forest.merge_nodes.push(MergeNode {
                commit_id: commit_id.clone(),
                parents: vec!["parent1".to_string(), "parent2".to_string()],
            });
        }

        forest
    }

    #[test]
    fn test_no_merges() {
        let forest = Forest {
            trees: Vec::new(),
            commit_map: HashMap::new(),
            merge_nodes: Vec::new(),
            branch_names: Vec::new(),
        };
        let layout = LayoutResult {
            grid: Vec::new(),
            tree_lines: Vec::new(),
            merge_storms: Vec::new(),
        };
        let storms = detect_merge_storms(&forest, &layout);
        assert!(storms.is_empty());
    }

    #[test]
    fn test_single_merge_no_storm() {
        let forest = make_forest_with_merges(&[1000]);
        let layout = LayoutResult {
            grid: Vec::new(),
            tree_lines: Vec::new(),
            merge_storms: Vec::new(),
        }