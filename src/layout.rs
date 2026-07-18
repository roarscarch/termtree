use crate::{Forest, Tree, CommitNode, MergeNode, Branch, LayoutResult};
use std::collections::{HashMap, HashSet, VecDeque};

/// Compute layout for the forest: map commits to a 2D grid using topological order
/// and gravitational pull between related commits, then skeletonize into tree shapes.
pub fn compute_layout(forest: &Forest) -> LayoutResult {
    let mut layout = LayoutResult::default();

    // Step 1: Topological sort by commit timestamp (oldest first)
    let mut sorted: Vec<&CommitNode> = forest.commit_map.values().collect();
    sorted.sort_by_key(|c| c.timestamp);

    // Step 2: Assign grid positions (x = branching factor, y = depth from root)
    let mut depth_map: HashMap<String, usize> = HashMap::new();
    let mut x_map: HashMap<String, f64> = HashMap::new();
    let mut y_map: HashMap<String, usize> = HashMap::new();

    // Track branches per depth to spread out siblings
    let mut depth_counts: HashMap<usize, usize> = HashMap::new();

    for commit in &sorted {
        let depth = commit.depth;
        let count = depth_counts.entry(depth).or_insert(0);
        let x = *count as f64 * 2.5; // spacing factor
        *count += 1;

        x_map.insert(commit.id.clone(), x);
        y_map.insert(commit.id.clone(), depth);
        depth_map.insert(commit.id.clone(), depth);

        layout.grid_positions.insert(commit.id.clone(), (x, depth as f64));
    }

    // Step 3: Apply gravitational pull between related commits (parent-child)
    for _ in 0..3 {
        let mut adjustments: HashMap<String, (f64, f64)> = HashMap::new();
        for commit in &sorted {
            let pos = layout.grid_positions.get(&commit.id).copied().unwrap_or((0.0, 0.0));
            let mut dx = 0.0;
            let mut dy = 0.0;
            let mut count = 0;

            for parent_id in &commit.parents {
                if let Some(&parent_pos) = layout.grid_positions.get(parent_id) {
                    dx += parent_pos.0 - pos.0;
                    dy += parent_pos.1 - pos.1;
                    count += 1;
                }
            }
            for child_id in &commit.children {
                if let Some(&child_pos) = layout.grid_positions.get(child_id) {
                    dx += child_pos.0 - pos.0;
                    dy += child_pos.1 - pos.1;
                    count += 1;
                }
            }

            if count > 0 {
                let pull_strength = 0.3;
                let adjustment = (dx / count as f64 * pull_strength, dy / count as f64 * pull_strength);
                adjustments.insert(commit.id.clone(), adjustment);
            }
        }

        for (id, adj) in &adjustments {
            if let Some(pos) = layout.grid_positions.get_mut(id) {
                pos.0 += adj.0;
                pos.1 += adj.1;
            }
        }
    }

    // Step 4: Skeletonize into tree shapes (trunks and branches)
    for branch in &forest.branches {
        let mut tree = Tree::new(branch.name.clone());
        let mut segments: Vec<Vec<String>> = Vec::new();
        let mut current_segment: Vec<String> = Vec::new();

        for commit_id in &branch.commit_ids {
            if let Some(commit) = forest.commit_map.get(commit_id) {
                // Detect if this commit is a merge point (multiple parents)
                let is_merge = commit.parents.len() > 1;
                // Detect if this commit starts a new sub-branch (fork)
                let is_fork = commit.children.len() > 1;

                if is_merge || is_fork {
                    if !current_segment.is_empty() {
                        segments.push(current_segment.clone());
                        current_segment.clear();
                    }
                }
                current_segment.push(commit_id.clone());
            }
        }
        if !current_segment.is_empty() {
            segments.push(current_segment);
        }

        // Convert segments to tree skeleton
        for (i, segment) in segments.iter().enumerate() {
            if segment.len() >= 3 {
                // Trunk segment
                if i == 0 {
                    let trunk = TreeSkeleton {
                        start: *layout.grid_positions.get(&segment[0]).unwrap_or(&(0.0, 0.0)),
                        end: *layout.grid_positions.get(&segment[segment.len()-1]).unwrap_or(&(0.0, 0.0)),
                        thickness: 2.0,
                        commits: segment.clone(),
                    };
                    tree.trunks.push(trunk);
                } else {
                    // Branch segment
                    let branch_skel = TreeSkeleton {
                        start: *layout.grid_positions.get(&segment[0]).unwrap_or(&(0.0, 0.0)),
                        end: *layout.grid_positions.get(&segment[segment.len()-1]).unwrap_or(&(0.0, 0.0)),
                        thickness: 1.0,
                        commits: segment.clone(),
                    };
                    tree.branches.push(branch_skel);
                }
            }
        }

        layout.trees.push(tree);
    }

    // Step 5: Identify merge nodes for root system visualization
    for commit in &sorted {
        if commit.parents.len() > 1 {
            let merge_node = MergeNode {
                commit_id: commit.id.clone(),
                position: *layout.grid_positions.get(&commit.id).unwrap_or(&(0.0, 0.0)),
                parent_positions: commit.parents.iter()
                    .filter_map(|pid| layout.grid_positions.get(pid).copied())
                    .collect(),
                child_positions: commit.children.iter()
                    .filter_map(|cid| layout.grid_positions.get(cid).copied())
                    .collect(),
            };
            layout.merge_nodes.push(merge_node);
        }
    }

    // Finalize: compute bounding box
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    for (_, &(x, y)) in &layout.grid_positions {
        if x < min_x { min_x = x; }
        if x > max_x { max_x = x; }
        if y < min_y { min_y = y; }
        if y > max_y { max_y = y; }
    }
    layout.bounding_box = Some((min_x, max_x, min_y, max_y));

    layout
}

/// Internal skeleton structure for a tree
#[derive(Debug, Clone)]
pub struct TreeSkeleton {
    pub start: (f64, f64),
    pub end: (f64, f64),
    pub thickness: f64,
    pub commits: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Tree {
    pub name: String,
    pub trunks: Vec<TreeSkeleton>,
    pub branches: Vec<TreeSkeleton>,
}

impl Tree {
    pub fn new(name: String) -> Self {
        Tree {
            name,
            trunks: Vec::new(),
            branches: Vec::new(),
        }
    }
}

/// Layout result type containing grid positions, trees, and merge nodes
#[derive(Debug, Clone, Default)]
pub struct LayoutResult {
    pub grid_positions: HashMap<String, (f64, f64)>,
    pub trees: Vec<Tree>,
    pub merge_nodes: Vec<MergeNode>,
    pub bounding_box: Option<(f64, f64, f64, f64)>,
}