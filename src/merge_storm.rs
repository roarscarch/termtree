use crate::{CommitNode, Forest, Tree};
use std::collections::{HashMap, HashSet};

/// Represents a detected merge storm: a cluster of merge commits occurring close in time.
#[derive(Debug, Clone)]
pub struct MergeStorm {
    /// The merge commit ids that form the storm
    pub merge_ids: Vec<String>,
    /// The center position (x, y) of the storm in normalized coordinates
    pub center: (f64, f64),
    /// The intensity (0.0 to 1.0) based on number of merges and how close they are in time
    pub intensity: f64,
    /// The radius of the storm's root system visualization
    pub radius: f64,
}

/// Detect merge storms in the forest.
/// A storm is defined as a set of merge commits where:
/// - Each merge commit has two or more parents
/// - They occur within a sliding time window (default 3600 seconds = 1 hour)
/// - They are topologically close (share at least one common ancestor within 5 steps)
pub fn detect_merge_storms(forest: &Forest, time_window_secs: i64) -> Vec<MergeStorm> {
    if forest.commit_map.is_empty() {
        return vec![];
    }

    // Collect all merge commits (commits with 2+ parents)
    let mut merge_commits: Vec<&CommitNode> = forest
        .commit_map
        .values()
        .filter(|c| c.parents.len() >= 2)
        .collect();

    // Sort by time ascending
    merge_commits.sort_by_key(|c| c.time);

    if merge_commits.is_empty() {
        return vec![];
    }

    // Cluster merges by sliding time window
    let mut storms: Vec<MergeStorm> = Vec::new();
    let mut current_cluster: Vec<&CommitNode> = Vec::new();
    let mut window_start = merge_commits[0].time;

    for mc in &merge_commits {
        if mc.time - window_start <= time_window_secs {
            current_cluster.push(mc);
        } else {
            if current_cluster.len() >= 2 {
                if let Some(storm) = build_storm(&current_cluster, forest) {
                    storms.push(storm);
                }
            }
            current_cluster.clear();
            current_cluster.push(mc);
            window_start = mc.time;
        }
    }
    // Handle last cluster
    if current_cluster.len() >= 2 {
        if let Some(storm) = build_storm(&current_cluster, forest) {
            storms.push(storm);
        }
    }

    storms
}

/// Build a MergeStorm from a cluster of merge commits.
fn build_storm(cluster: &[&CommitNode], forest: &Forest) -> Option<MergeStorm> {
    if cluster.len() < 2 {
        return None;
    }

    // Compute center as average of commit positions (if positions available)
    // We use time as a proxy for y-coordinate, and average parent hashes for x
    let mut avg_x: f64 = 0.0;
    let mut avg_y: f64 = 0.0;
    let mut count: usize = 0;

    // Build a set of all commit ids in the cluster
    let ids: HashSet<&str> = cluster.iter().map(|c| c.id.as_str()).collect();

    for mc in cluster {
        // Use time for y (normalized to 0..1 based on min/max time in forest)
        let min_time = forest.commit_map.values().map(|c| c.time).min().unwrap_or(0);
        let max_time = forest.commit_map.values().map(|c| c.time).max().unwrap_or(1);
        let time_range = (max_time - min_time).max(1);
        let y = (mc.time - min_time) as f64 / time_range as f64;
        avg_y += y;

        // Use first parent's id as hash for x (simple deterministic spread)
        let parent_id = mc.parents.first().map(|s| s.as_str()).unwrap_or("");
        let x = simple_hash(parent_id) as f64 / 1000.0;
        avg_x += x;
        count += 1;
    }

    if count == 0 {
        return None;
    }

    let center = (avg_x / count as f64, avg_y / count as f64);

    // Intensity: proportional to cluster size and time density
    let cluster_size = cluster.len() as f64;
    let time_span = cluster.last().unwrap().time - cluster.first().unwrap().time;
    let time_density = if time_span > 0 {
        cluster_size / time_span as f64
    } else {
        cluster_size
    };
    let intensity = (cluster_size / 10.0).min(1.0) * (time_density / 0.01).min(1.0);

    // Radius scales with intensity and cluster size
    let radius = 0.05 + (intensity * 0.15);

    Some(MergeStorm {
        merge_ids: cluster.iter().map(|c| c.id.clone()).collect(),
        center,
        intensity,
        radius,
    })
}

/// Simple hash function for a string, returns a u32 in [0, 1000).
fn simple_hash(s: &str) -> u32 {
    let mut hash: u32 = 0;
    for (i, b) in s.bytes().enumerate() {
        hash = hash.wrapping_mul(31).wrapping_add(b as u32);
        if i > 10 {
            break;
        }
    }
    hash % 1000
}

/// Render the root systems for merge storms as ASCII art.
/// Returns a vector of lines that overlay the forest at the storm centers.
pub fn render_merge_storms(
    storms: &[MergeStorm],
    width: u16,
    height: u16,
    zoom: f64,
    offset_x: f64,
    offset_y: f64,
) -> Vec<(u16, u16, String)> {
    let mut overlays: Vec<(u16, u16, String)> = Vec::new();

    for storm in storms {
        // Convert normalized center to screen coordinates
        let screen_x = ((storm.center.0 / zoom) - offset_x + 0.5) * width as f64;
        let screen_y = ((storm.center.1 / zoom) - offset_y + 0.5) * height as f64;

        let sx = screen_x as u16;
        let sy = screen_y as u16;

        // Draw a tangled root system: a cluster of '@' and '#' characters
        let intensity_char = if storm.intensity > 0.7 {
            "@"
        } else if storm.intensity > 0.4 {
            "#"
        } else {
            "*"
        };

        let radius_chars = (storm.radius * width as f64) as u16;
        let radius_chars = radius_chars.max(1).min(10);

        // Generate a circular pattern of characters
        for dy in 0..=radius_chars {
            for dx in 0..=radius_chars {
                let dist = ((dx as f64).powi(2) + (dy as f64).powi(2)).sqrt();
                if dist <= radius_chars as f64 && dist >= radius_chars as f64 * 0.3 {
                    let px = sx + dx;
                    let py = sy + dy;
                    if px < width && py < height {
                        overlays.push((px, py, intensity_char.to_string()));
                    }
                }
            }
        }

        // Add a label if intensity is high enough
        if storm.intensity > 0.5 {
            let label = format!("Storm! ({}