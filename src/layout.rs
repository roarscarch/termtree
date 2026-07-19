use crate::{Forest, Tree, CommitNode, MergeNode, MergeStorm, LayoutResult, Position};
use std::collections::{HashMap, VecDeque};

/// Compute the layout of the forest using a custom topological algorithm.
/// Maps the DAG to a 2D grid with gravitational pull between related commits,
/// then applies a recursive tree-skeletonizer to convert linear segments into
/// organic tree shapes with realistic branch taper.
pub fn compute_layout(forest: &Forest) -> LayoutResult {
    let mut layout = LayoutResult::default();
    let mut positions: HashMap<String, Position> = HashMap::new();
    let mut tree_positions: HashMap<String, Vec<Position>> = HashMap::new();

    // Step 1: assign initial positions using topological order
    let topo_order = topological_sort(forest);
    let mut x: f64 = 0.0;
    let mut y: f64 = 0.0;
    let spacing_x = 6.0;
    let spacing_y = 2.0;

    for commit_id in &topo_order {
        if let Some(node) = forest.commit_map.get(commit_id) {
            let pos = Position { x, y };
            positions.insert(commit_id.clone(), pos);
            y += spacing_y;
            if y > 40.0 {
                y = 0.0;
                x += spacing_x;
            }
        }
    }

    // Step 2: apply gravitational pull between related commits (children pulled toward parents)
    for _ in 0..10 {
        for commit_id in &topo_order {
            if let Some(node) = forest.commit_map.get(commit_id) {
                if let Some(pos) = positions.get_mut(commit_id) {
                    let mut dx = 0.0;
                    let mut dy = 0.0;
                    let mut count = 0;
                    for parent_id in &node.parents {
                        if let Some(parent_pos) = positions.get(parent_id) {
                            dx += parent_pos.x - pos.x;
                            dy += parent_pos.y - pos.y;
                            count += 1;
                        }
                    }
                    if count > 0 {
                        let pull_strength = 0.1;
                        pos.x += dx * pull_strength / count as f64;
                        pos.y += dy * pull_strength / count as f64;
                    }
                }
            }
        }
    }

    // Step 3: build trees from branches
    // Group commits by branch (using first parent as trunk)
    let mut branch_heads: Vec<&String> = forest
        .commit_map
        .keys()
        .filter(|id| {
            let node = &forest.commit_map[id.as_str()];
            node.parents.len() > 1 || node.children.is_empty()
        })
        .collect();
    branch_heads.sort();

    for head_id in branch_heads {
        let mut trunk_positions = Vec::new();
        let mut current = head_id.clone();
        loop {
            if let Some(pos) = positions.get(&current) {
                trunk_positions.push(*pos);
            }
            // Walk down first parent (trunk)
            if let Some(node) = forest.commit_map.get(&current) {
                if let Some(first_parent) = node.parents.first() {
                    current = first_parent.clone();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        if !trunk_positions.is_empty() {
            // Skeletonize: convert to tree shape with branch taper
            let tree = skeletonize_tree(&trunk_positions, head_id);
            layout.trees.push(tree);
            tree_positions.insert(head_id.clone(), trunk_positions);
        }
    }

    // Step 4: detect merge storms
    layout.merge_storms = detect_merge_storms(forest);

    // Step 5: store layout positions
    layout.positions = positions;

    layout
}

/// Topological sort of commits (parents before children)
fn topological_sort(forest: &Forest) -> Vec<String> {
    let mut in_degree: HashMap<&String, usize> = HashMap::new();
    let mut queue: VecDeque<&String> = VecDeque::new();
    let mut order = Vec::new();

    // Initialize in-degree
    for (id, node) in &forest.commit_map {
        in_degree.entry(id).or_insert(0);
        for parent_id in &node.parents {
            *in_degree.entry(parent_id).or_insert(0) += 1;
        }
    }

    // Start with nodes that have no parents (roots)
    for (id, _) in &forest.commit_map {
        let node = &forest.commit_map[id.as_str()];
        if node.parents.is_empty() {
            queue.push_back(id);
        }
    }

    while let Some(id) = queue.pop_front() {
        order.push(id.clone());
        if let Some(node) = forest.commit_map.get(id) {
            for child_id in &node.children {
                if let Some(deg) = in_degree.get_mut(child_id) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(child_id);
                    }
                }
            }
        }
    }

    order
}

/// Convert a list of trunk positions into a tree structure with branches
fn skeletonize_tree(positions: &[Position], head_id: &str) -> Tree {
    let mut tree = Tree::default();
    tree.head_id = head_id.to_string();
    tree.trunk = positions.to_vec();

    // Compute branch taper: each segment gets slightly thinner
    let num_segments = positions.len().max(1);
    tree.taper = 1.0 - (1.0 / num_segments as f64);

    // Compute leaf density based on branch length
    tree.leaf_density = (positions.len() as f64).sqrt();

    tree
}

/// Detect merge storms: areas where many simultaneous merges occur
fn detect_merge_storms(forest: &Forest) -> Vec<MergeStorm> {
    let mut storms = Vec::new();

    // Group merges by time window (here we use a simple heuristic: commits with same parent count)
    let mut merge_groups: HashMap<usize, Vec<MergeNode>> = HashMap::new();

    for (id, node) in &forest.commit_map {
        if node.parents.len() >= 2 {
            let key = node.parents.len();
            merge_groups.entry(key).or_default().push(MergeNode {
                id: id.clone(),
                parents: node.parents.clone(),
                children: node.children.clone(),
                timestamp: node.timestamp,
                author: node.author.clone(),
                message: node.message.clone(),
            });
        }
    }

    // Create storms for groups with many merges
    for (_, merges) in merge_groups {
        if merges.len() >= 3 {
            storms.push(MergeStorm {
                merges,
                intensity: merges.len() as f64 / 10.0,
            });
        }
    }

    storms
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Forest, CommitNode};
    use std::collections::HashMap;

    fn make_forest() -> Forest {
        let mut forest = Forest::default();
        // Add some commits
        forest.commit_map.insert("a".to_string(), CommitNode {
            id: "a".to_string(),
            parents: vec![],
            children: vec!["b".to_string()],
            timestamp: 1,
            author: "alice".to_string(),
            message: "initial".to_string(),
            branch: "main".to_string(),
        }