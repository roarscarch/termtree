use crate::{Forest, MergeNode};
use std::collections::HashMap;

/// Represents a detected merge storm — a region with many simultaneous merges.
#[derive(Debug, Clone)]
pub struct MergeStorm {
    /// The merge nodes that form the storm
    pub merges: Vec<MergeNode>,
    /// Normalized center position (x, y)
    pub center: (f64, f64),
    /// Intensity: number of merges / time window
    pub intensity: f64,
    /// Time range of the storm
    pub time_start: i64,
    pub time_end: i64,
}

/// Detect merge storms in the forest.
/// A storm is defined as 3 or more merge nodes within a time window of 1 hour (3600 seconds)
/// and within a normalized horizontal distance of 0.2.
pub fn detect_merge_storms(forest: &Forest) -> Vec<MergeStorm> {
    if forest.merges.is_empty() {
        return vec![];
    }

    let mut storms: Vec<MergeStorm> = Vec::new();
    let mut used: Vec<bool> = vec![false; forest.merges.len()];

    // Sort merges by time for window-based clustering
    let mut merges_sorted: Vec<(usize, &MergeNode)> = forest.merges.iter().enumerate().collect();
    merges_sorted.sort_by_key(|(_, m)| m.time);

    let time_window: i64 = 3600; // 1 hour in seconds
    let distance_threshold: f64 = 0.2;

    for i in 0..merges_sorted.len() {
        if used[i] {
            continue;
        }
        let (idx_i, m_i) = merges_sorted[i];
        let mut cluster: Vec<MergeNode> = Vec::new();
        cluster.push(m_i.clone());
        used[i] = true;

        let mut time_start = m_i.time;
        let mut time_end = m_i.time;
        let mut sum_x = m_i.x;
        let mut sum_y = m_i.y;

        for j in (i + 1)..merges_sorted.len() {
            if used[j] {
                continue;
            }
            let (idx_j, m_j) = merges_sorted[j];
            // Check time window
            if m_j.time - time_start > time_window {
                break; // since sorted, later merges will also be outside window
            }
            // Check distance to cluster center (approximate)
            let cx = sum_x / cluster.len() as f64;
            let cy = sum_y / cluster.len() as f64;
            let dx = m_j.x - cx;
            let dy = m_j.y - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist < distance_threshold {
                cluster.push(m_j.clone());
                used[j] = true;
                sum_x += m_j.x;
                sum_y += m_j.y;
                if m_j.time < time_start {
                    time_start = m_j.time;
                }
                if m_j.time > time_end {
                    time_end = m_j.time;
                }
            }
        }

        // Only consider clusters of size >= 3 as storms
        if cluster.len() >= 3 {
            let cx = sum_x / cluster.len() as f64;
            let cy = sum_y / cluster.len() as f64;
            let intensity = cluster.len() as f64 / (time_end - time_start + 1) as f64;
            storms.push(MergeStorm {
                merges: cluster,
                center: (cx, cy),
                intensity,
                time_start,
                time_end,
            });
        }
    }

    storms
}

/// Highlight merge storms in the forest by modifying merge node colors and styles.
pub fn highlight_storms(forest: &mut Forest, storms: &[MergeStorm]) {
    // Create a set of merge indices that are part of storms
    let mut storm_merge_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    for storm in storms {
        for merge in &storm.merges {
            storm_merge_ids.insert(merge.id.clone());
        }
    }

    // Modify merge nodes in place to indicate storm status
    for merge in &mut forest.merges {
        if storm_merge_ids.contains(&merge.id) {
            // Mark as storm: set a special color or flag
            merge.storm = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MergeNode;

    #[test]
    fn test_detect_no_storms() {
        let forest = Forest {
            trees: vec![],
            merges: vec![],
            commit_map: std::collections::HashMap::new(),
        };
        let storms = detect_merge_storms(&forest);
        assert!(storms.is_empty());
    }

    #[test]
    fn test_detect_single_storm() {
        let merges = vec![
            MergeNode {
                id: "m1".to_string(),
                parents: vec!["a".to_string()],
                children: vec!["b".to_string()],
                time: 1000,
                x: 0.5,
                y: 0.5,
                storm: false,
            },
            MergeNode {
                id: "m2".to_string(),
                parents: vec!["c".to_string()],
                children: vec!["d".to_string()],
                time: 1200,
                x: 0.55,
                y: 0.52,
                storm: false,
            },
            MergeNode {
                id: "m3".to_string(),
                parents: vec!["e".to_string()],
                children: vec!["f".to_string()],
                time: 1400,
                x: 0.48,
                y: 0.49,
                storm: false,
            },
        ];
        let forest = Forest {
            trees: vec![],
            merges,
            commit_map: std::collections::HashMap::new(),
        };
        let storms = detect_merge_storms(&forest);
        assert_eq!(storms.len(), 1);
        assert_eq!(storms[0].merges.len(), 3);
        assert!(storms[0].intensity > 0.0);
    }

    #[test]
    fn test_highlight_storms() {
        let mut merges = vec![
            MergeNode {
                id: "m1".to_string(),
                parents: vec!["a".to_string()],
                children: vec!["b".to_string()],
                time: 1000,
                x: 0.5,
                y: 0.5,
                storm: false,
            },
            MergeNode {
                id: "m2".to_string(),
                parents: vec!["c".to_string()],
                children: vec!["d".to_string()],
                time: 1200,
                x: 0.55,
                y: 0.52,
                storm: false,
            },
            MergeNode {
                id: "m3".to_string(),
                parents: vec!["e".to_string()],
                children: vec!["f".to_string()],
                time: 1400,
                x: 0.48,
                y: 0.49,
                storm: false,
            },
        ];
        let mut forest = Forest {
            trees: vec![],
            merges,
            commit_map: std::collections::HashMap::new(),
        }