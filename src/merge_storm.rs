use crate::{Forest, Tree, MergeNode, CommitNode};
use std::collections::{HashMap, HashSet};

/// Represents a detected merge storm: a cluster of simultaneous merges.
#[derive(Debug, Clone)]
pub struct MergeStorm {
    /// The merge nodes that are part of this storm.
    pub merge_nodes: Vec<String>,
    /// The central position of the storm (x, y) normalized.
    pub center: (f64, f64),
    /// The intensity of the storm (0.0 to 1.0), based on number of merges and proximity.
    pub intensity: f64,
    /// The radius of the storm's root system.
    pub radius: f64,
}

/// Detects merge storms in the forest by clustering merge nodes that are close in time.
/// A storm is defined as a group of 3 or more merge commits within a short time window.
pub fn detect_merge_storms(forest: &Forest, time_window_seconds: u64) -> Vec<MergeStorm> {
    let mut storms: Vec<MergeStorm> = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();

    // Collect all merge nodes with their timestamps
    let mut merge_times: Vec<(String, u64)> = Vec::new();
    for (id, _) in &forest.merge_map {
        if let Some(node) = forest.commit_map.get(id) {
            merge_times.push((id.clone(), node.time));
        }
    }

    // Sort by time
    merge_times.sort_by(|a, b| a.1.cmp(&b.1));

    // Sliding window to find clusters
    let mut window_start = 0;
    for window_end in 0..merge_times.len() {
        let current_time = merge_times[window_end].1;
        // Advance window start until within time window
        while window_start < window_end && current_time - merge_times[window_start].1 > time_window_seconds {
            window_start += 1;
        }
        let window_size = window_end - window_start + 1;
        if window_size >= 3 {
            // Check if we have already formed a storm for this window
            let storm_nodes: Vec<String> = merge_times[window_start..=window_end]
                .iter()
                .map(|(id, _)| id.clone())
                .collect();
            // Only create a new storm if at least 3 nodes are not already in a storm
            let unvisited_count = storm_nodes.iter().filter(|id| !visited.contains(*id)).count();
            if unvisited_count >= 3 {
                let center_time = (merge_times[window_start].1 + merge_times[window_end].1) / 2;
                // Compute an approximate center position: average of tree centers
                let mut x_sum = 0.0;
                let mut y_sum = 0.0;
                let mut count = 0;
                for id in &storm_nodes {
                    if let Some(pos) = forest.merge_map.get(id) {
                        x_sum += pos.0;
                        y_sum += pos.1;
                        count += 1;
                    }
                }
                let center = if count > 0 {
                    (x_sum / count as f64, y_sum / count as f64)
                } else {
                    (0.5, 0.5)
                };
                let intensity = (window_size as f64 / 10.0).min(1.0);
                let radius = 0.05 + (window_size as f64 * 0.02).min(0.3);
                let storm = MergeStorm {
                    merge_nodes: storm_nodes.clone(),
                    center,
                    intensity,
                    radius,
                };
                for id in &storm_nodes {
                    visited.insert(id.clone());
                }
                storms.push(storm);
            }
        }
    }

    // Merge overlapping storms (if centers are close)
    merge_overlapping_storms(&mut storms);

    storms
}

/// Merges storms whose centers are within a small distance of each other.
fn merge_overlapping_storms(storms: &mut Vec<MergeStorm>) {
    let mut merged = true;
    while merged {
        merged = false;
        let mut i = 0;
        while i < storms.len() {
            let mut j = i + 1;
            while j < storms.len() {
                let dx = storms[i].center.0 - storms[j].center.0;
                let dy = storms[i].center.1 - storms[j].center.1;
                let dist = (dx * dx + dy * dy).sqrt();
                let threshold = (storms[i].radius + storms[j].radius) * 0.5;
                if dist < threshold {
                    // Merge j into i
                    let storm_j = storms.remove(j);
                    storms[i].merge_nodes.extend(storm_j.merge_nodes);
                    storms[i].intensity = (storms[i].intensity + storm_j.intensity).min(1.0);
                    storms[i].radius = storms[i].radius.max(storm_j.radius);
                    // Recompute center as weighted average by intensity
                    let total_intensity = storms[i].intensity + storm_j.intensity;
                    if total_intensity > 0.0 {
                        storms[i].center.0 = (storms[i].center.0 * storms[i].intensity + storm_j.center.0 * storm_j.intensity) / total_intensity;
                        storms[i].center.1 = (storms[i].center.1 * storms[i].intensity + storm_j.center.1 * storm_j.intensity) / total_intensity;
                    }
                    merged = true;
                } else {
                    j += 1;
                }
            }
            i += 1;
        }
    }
}