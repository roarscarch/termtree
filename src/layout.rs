use crate::{CommitNode, Forest, Tree, MergeNode};
use std::collections::{HashMap, HashSet, VecDeque};

/// Result of layout: positions of all commits and merge nodes.
#[derive(Debug, Clone)]
pub struct LayoutResult {
    /// Commit positions keyed by commit id (x:0..1, y:0..1)
    pub positions: HashMap<String, (f64, f64)>,
    /// Merge node positions
    pub merge_positions: HashMap<String, (f64, f64)>,
    /// For each tree, its horizontal center (x coordinate)
    pub tree_centers: Vec<f64>,
}

/// Layout the forest on a 2D grid with gravitational pull between related commits.
/// Returns positions normalized to [0,1] range.
pub fn layout_forest(forest: &Forest, width: f64, height: f64) -> LayoutResult {
    let tree_count = forest.trees.len();
    if tree_count == 0 {
        return LayoutResult {
            positions: HashMap::new(),
            merge_positions: HashMap::new(),
            tree_centers: vec![],
        };
    }

    // 1. Assign each tree a horizontal position (evenly spaced with some jitter based on age)
    let mut tree_centers: Vec<f64> = Vec::with_capacity(tree_count);
    let spacing = 1.0 / (tree_count as f64 + 1.0);
    for i in 0..tree_count {
        let base = spacing * (i as f64 + 1.0);
        // Add small jitter based on root commit time to avoid perfect alignment
        let root_id = &forest.trees[i].root;
        let root_node = forest.commit_map.get(root_id);
        let jitter = root_node.map_or(0.0, |c| (c.time as f64 % 1000.0) / 10000.0);
        tree_centers.push((base + jitter * 0.1).clamp(0.02, 0.98));
    }

    let mut positions: HashMap<String, (f64, f64)> = HashMap::new();
    let mut merge_positions: HashMap<String, (f64, f64)> = HashMap::new();

    // 2. Position commits within each tree vertically, with gravitational pull toward related trees
    // We'll process trees in order of depth (longest branch first) for stability
    let mut tree_order: Vec<usize> = (0..tree_count).collect();
    tree_order.sort_by_key(|&i| std::cmp::Reverse(forest.trees[i].commits.len()));

    // Track vertical progress for each tree (y coordinate from 0 top to 1 bottom)
    let mut tree_y_progress: Vec<f64> = vec![0.0; tree_count];
    // Track maximum y used in each tree (for spacing)
    let mut tree_max_y: Vec<f64> = vec![0.0; tree_count];

    // First pass: place commits in each tree linearly
    for &idx in &tree_order {
        let tree = &forest.trees[idx];
        let x = tree_centers[idx];
        let commit_count = tree.commits.len();
        if commit_count == 0 { continue; }
        // Reserve space: each commit gets a vertical slice, with small padding
        let vertical_step = 0.8 / (commit_count as f64 + 1.0);
        let mut y = 0.1 + vertical_step; // start a bit from top
        for commit_id in &tree.commits {
            positions.insert(commit_id.clone(), (x, y));
            y += vertical_step;
        }
        tree_max_y[idx] = y;
    }

    // 3. Apply gravitational pull: for each merge, pull parent commits toward the merge point
    // We'll run several iterations to stabilize
    for _iteration in 0..5 {
        for merge in &forest.merges {
            let merge_x = tree_centers.iter().sum::<f64>() / tree_centers.len() as f64; // center
            let merge_y = 0.5; // middle
            // Find the merge's position (approximate)
            let merge_pos = (merge_x, merge_y);
            merge_positions.insert(merge.id.clone(), merge_pos);

            // Pull parents toward this merge point
            for parent_id in &merge.parents {
                if let Some(pos) = positions.get_mut(parent_id) {
                    // Gravitational pull: shift x toward merge_x
                    let dx = merge_x - pos.0;
                    pos.0 += dx * 0.1;
                    // Also slightly adjust y to align horizontally
                    let dy = merge_y - pos.1;
                    pos.1 += dy * 0.05;
                }
            }
        }

        // Also pull within same tree: commits should form a smooth curve
        for tree in &forest.trees {
            let x_target = tree_centers.iter().position(|&c| c == tree_centers[0]).unwrap_or(0);
            let x_center = tree_centers[x_target];
            for window in tree.commits.windows(2) {
                let id1 = &window[0];
                let id2 = &window[1];
                if let (Some(&(x1, y1)), Some(&(x2, y2))) = (positions.get(id1), positions.get(id2)) {
                    // Attract consecutive commits to maintain trunk line
                    let mid_x = (x1 + x2) / 2.0;
                    let mid_y = (y1 + y2) / 2.0;
                    if let Some(pos1) = positions.get_mut(id1) {
                        pos1.0 += (mid_x - pos1.0) * 0.05;
                        pos1.1 += (mid_y - pos1.1) * 0.05;
                    }
                    if let Some(pos2) = positions.get_mut(id2) {
                        pos2.0 += (mid_x - pos2.0) * 0.05;
                        pos2.1 += (mid_y - pos2.1) * 0.05;
                    }
                }
            }
        }
    }

    // 4. Normalize positions to [0,1] range and ensure no overlap
    let mut all_x: Vec<f64> = positions.values().map(|&(x, _)| x).collect();
    let mut all_y: Vec<f64> = positions.values().map(|&(_, y)| y).collect();
    for &(x, y) in merge_positions.values() {
        all_x.push(x);
        all_y.push(y);
    }
    let min_x = all_x.iter().cloned().fold(f64::MAX, f64::min);
    let max_x = all_x.iter().cloned().fold(f64::MIN, f64::max);
    let min_y = all_y.iter().cloned().fold(f64::MAX, f64::min);
    let max_y = all_y.iter().cloned().fold(f64::MIN, f64::max);
    let range_x = if (max_x - min_x).abs() < 0.001 { 1.0 } else { max_x - min_x };
    let range_y = if (max_y - min_y).abs() < 0.001 { 1.0 } else { max_y - min_y };

    for (_, pos) in positions.iter_mut() {
        pos.0 = (pos.0 - min_x) / range_x;
        pos.1 = (pos.1 - min_y) / range_y;
    }
    for (_, pos) in merge_positions.iter_mut() {
        pos.0 = (pos.0 - min_x) / range_x;
        pos.1 = (pos.1 - min_y) / range_y;
    }