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
    /// Depth (branch level) for each commit: 0 = main trunk, higher = side branches
    pub branch_levels: HashMap<String, usize>,
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
            branch_levels: HashMap::new(),
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
    let mut branch_levels: HashMap<String, usize> = HashMap::new();

    // 2. Compute topological order (BFS from roots)
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut children: HashMap<String, Vec<String>> = HashMap::new();
    for (id, _) in &forest.commit_map {
        in_degree.entry(id.clone()).or_insert(0);
        children.entry(id.clone()).or_insert_with(Vec::new);
    }
    for (id, node) in &forest.commit_map {
        for parent in &node.parents {
            children.entry(parent.clone()).or_insert_with(Vec::new).push(id.clone());
            *in_degree.entry(id.clone()).or_insert(0) += 1;
        }
    }

    // 3. Assign branch levels via DFS from root, tracking depth along each path
    // We process trees in order, assigning level 0 to the trunk (longest path from root)
    for tree in &forest.trees {
        let root_id = &tree.root;
        // First, find all commits in this tree
        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<String> = VecDeque::new();
        queue.push_back(root_id.clone());
        visited.insert(root_id.clone());
        while let Some(id) = queue.pop_front() {
            if let Some(node) = forest.commit_map.get(&id) {
                for child in children.get(&id).unwrap_or(&vec![]) {
                    if visited.insert(child.clone()) {
                        queue.push_back(child.clone());
                    }
                }
            }
        }

        // Now assign levels: root = 0, each child = parent's level + 1, but merges get min of parents
        let mut topo: Vec<String> = Vec::new();
        let mut in_deg_local: HashMap<String, usize> = HashMap::new();
        for id in &visited {
            in_deg_local.insert(id.clone(), 0);
        }
        for id in &visited {
            if let Some(node) = forest.commit_map.get(id) {
                for parent in &node.parents {
                    if visited.contains(parent) {
                        *in_deg_local.entry(id.clone()).or_insert(0) += 1;
                    }
                }
            }
        }
        let mut q: VecDeque<String> = VecDeque::new();
        for (id, deg) in &in_deg_local {
            if *deg == 0 {
                q.push_back(id.clone());
            }
        }
        while let Some(id) = q.pop_front() {
            topo.push(id.clone());
            if let Some(node) = forest.commit_map.get(&id) {
                for child in children.get(&id).unwrap_or(&vec![]) {
                    if visited.contains(child) {
                        if let Some(deg) = in_deg_local.get_mut(child) {
                            *deg -= 1;
                            if *deg == 0 {
                                q.push_back(child.clone());
                            }
                        }
                    }
                }
            }
        }

        // Assign levels: for each commit, level = max(parent levels) + 1, but for merges take min of parents
        for id in &topo {
            if id == root_id {
                branch_levels.insert(id.clone(), 0);
            } else if let Some(node) = forest.commit_map.get(id) {
                let parent_levels: Vec<usize> = node.parents.iter()
                    .filter(|p| visited.contains(*p))
                    .filter_map(|p| branch_levels.get(p))
                    .cloned()
                    .collect();
                if parent_levels.is_empty() {
                    branch_levels.insert(id.clone(), 0);
                } else if node.parents.len() > 1 {
                    // Merge: take minimum parent level (closer to trunk)
                    let min_level = parent_levels.iter().min().cloned().unwrap_or(0);
                    branch_levels.insert(id.clone(), min_level);
                } else {
                    let max_level = parent_levels.iter().max().cloned().unwrap_or(0);
                    branch_levels.insert(id.clone(), max_level + 1);
                }
            }
        }
    }

    // 4. Place commits vertically by topological order, horizontally by tree center + branch offset
    // Use a simple layered approach: assign each commit a 'layer' (depth from root via longest path)
    let mut commit_order: Vec<&String> = forest.commit_map.keys().collect();
    // Sort by time ascending (root earliest)
    commit_order.sort_by(|a, b| {
        let ca = forest.commit_map.get(a).map(|c| c.time).unwrap_or(0);
        let cb = forest.commit_map.get(b).map(|c| c.time).unwrap_or(0);
        ca.cmp(&cb)
    });

    // Group commits by their tree assignment
    let mut commit_to_tree: HashMap<String, usize> = HashMap::new();
    for (i, tree) in forest.trees.iter().enumerate() {
        let mut stack: Vec<String> = vec![tree.root.clone()];
        let mut visited: HashSet<String> = HashSet::new();
        while let Some(id) = stack.pop() {
            if visited.insert(id.clone()) {
                commit_to_tree.insert(id.clone(), i);
                if let Some(node) = forest.commit_map.get(&id) {
                    for child in &node.children {
                        stack.push(child.clone());
                    }
                }
            }
        }
    }