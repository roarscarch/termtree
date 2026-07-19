use crate::{Forest, Tree, CommitNode, MergeNode, LayoutResult};
use std::collections::{HashMap, HashSet, VecDeque};

/// Configuration for forest pruning.
#[derive(Debug, Clone)]
pub struct PruneConfig {
    /// Maximum depth of tree trunks to keep (number of commits from tip).
    pub max_trunk_depth: usize,
    /// Minimum leaf density (commits per branch) to keep a branch alive.
    pub min_leaf_density: f64,
    /// Whether to remove isolated commits (no parents, no children except root).
    pub remove_isolated: bool,
    /// Whether to compress linear segments into single nodes.
    pub compress_linear: bool,
    /// Whether to prune merge storms (keep only storm indicators).
    pub prune_storms: bool,
}

impl Default for PruneConfig {
    fn default() -> Self {
        PruneConfig {
            max_trunk_depth: 100,
            min_leaf_density: 0.1,
            remove_isolated: true,
            compress_linear: true,
            prune_storms: false,
        }
    }
}

/// Prune the forest according to the given configuration.
/// Returns a new forest with pruned trees and updated layout.
pub fn prune_forest(forest: &Forest, config: &PruneConfig) -> (Forest, LayoutResult) {
    let mut pruned_forest = forest.clone();

    // Step 1: Identify and remove isolated commits (commits with no children and no parents except root)
    if config.remove_isolated {
        let isolated = find_isolated_commits(&pruned_forest);
        for commit_id in &isolated {
            pruned_forest.commit_map.remove(commit_id);
            pruned_forest.merge_nodes.retain(|m| m.commit_id != *commit_id);
            for tree in &mut pruned_forest.trees {
                tree.commits.retain(|c| c.id != *commit_id);
            }
        }
        // Update parent/child references after removal
        for commit in pruned_forest.commit_map.values_mut() {
            commit.parents.retain(|p| !isolated.contains(p));
        }
    }

    // Step 2: Compress linear segments (chains of commits with single parent and single child)
    if config.compress_linear {
        compress_linear_segments(&mut pruned_forest);
    }

    // Step 3: Trim branches by depth
    if config.max_trunk_depth < usize::MAX {
        trim_by_depth(&mut pruned_forest, config.max_trunk_depth);
    }

    // Step 4: Remove branches with low leaf density
    if config.min_leaf_density > 0.0 {
        remove_low_density_branches(&mut pruned_forest, config.min_leaf_density);
    }

    // Step 5: Optionally prune merge storms (keep only storm markers)
    if config.prune_storms {
        prune_merge_storms(&mut pruned_forest);
    }

    // Recompute layout from pruned forest (using existing layout logic, here we create a minimal layout)
    let layout = recompute_layout(&pruned_forest);

    (pruned_forest, layout)
}

/// Find commits that have no children and only one parent (or none) and are not tips of any tree.
fn find_isolated_commits(forest: &Forest) -> HashSet<String> {
    let mut child_counts: HashMap<String, usize> = HashMap::new();
    for commit in forest.commit_map.values() {
        for parent in &commit.parents {
            *child_counts.entry(parent.clone()).or_insert(0) += 1;
        }
    }
    let mut isolated = HashSet::new();
    for (id, commit) in &forest.commit_map {
        let children = child_counts.get(id).copied().unwrap_or(0);
        if children == 0 && commit.parents.len() <= 1 {
            // Check if this commit is a tip of any tree
            let is_tip = forest.trees.iter().any(|t| {
                t.commits.iter().any(|c| c.id == *id && c.is_tip)
            });
            if !is_tip {
                isolated.insert(id.clone());
            }
        }
    }
    isolated
}

/// Compress linear chains of commits into single nodes.
fn compress_linear_segments(forest: &mut Forest) {
    // Build a map of child -> parent(s) and parent -> children
    let mut children_map: HashMap<String, Vec<String>> = HashMap::new();
    for commit in forest.commit_map.values() {
        for parent in &commit.parents {
            children_map.entry(parent.clone()).or_default().push(commit.id.clone());
        }
    }

    // Find nodes that have exactly one parent and one child (linear)
    let linear_nodes: Vec<String> = forest.commit_map.iter()
        .filter(|(id, commit)| {
            let parents = &commit.parents;
            let children = children_map.get(*id).map(|v| v.len()).unwrap_or(0);
            parents.len() == 1 && children == 1
        })
        .map(|(id, _)| id.clone())
        .collect();

    if linear_nodes.is_empty() {
        return;
    }

    // For each linear node, merge it into its parent (skip if parent also linear? we handle iteratively)
    let linear_set: HashSet<String> = linear_nodes.into_iter().collect();
    for id in &linear_set {
        if let Some(commit) = forest.commit_map.get(id) {
            if commit.parents.is_empty() {
                continue;
            }
            let parent_id = commit.parents[0].clone();
            // Find children of this node
            if let Some(children) = children_map.get(id) {
                for child_id in children {
                    if let Some(child) = forest.commit_map.get_mut(child_id) {
                        // Replace parent reference
                        if let Some(pos) = child.parents.iter().position(|p| p == id) {
                            child.parents[pos] = parent_id.clone();
                        }
                    }
                }
            }
            // Remove the linear node
            forest.commit_map.remove(id);
            // Also remove from merge_nodes if present
            forest.merge_nodes.retain(|m| m.commit_id != *id);
            // Update trees
            for tree in &mut forest.trees {
                tree.commits.retain(|c| c.id != *id);
            }
        }
    }
}

/// Trim branches to a maximum depth from the tip.
fn trim_by_depth(forest: &mut Forest, max_depth: usize) {
    // For each tree, keep only commits within max_depth from the tip
    for tree in &mut forest.trees {
        let mut to_remove: Vec<String> = Vec::new();
        // Compute depth from tip (assuming tip is last commit in tree.commits? we need to know tip)
        // We'll use a simple BFS from tips
        let tips: Vec<String> = tree.commits.iter()
            .filter(|c| c.is_tip)
            .map(|c| c.id.clone())
            .collect();
        if tips.is_empty() {
            continue;
        }
        // Build adjacency for this tree
        let mut adj: HashMap<String, Vec<String>> = HashMap::new();
        for commit in &tree.commits {
            for parent in &commit.parents {
                adj.entry(commit.id.clone()).or_default().push(parent.clone());
            }
        }
        // BFS from tips, tracking depth
        let mut depth_map: HashMap<String, usize> = HashMap::new();
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();
        for tip in &tips {
            queue.push_back((tip.clone(), 0));
        }
        while let Some((node, depth)) = queue.pop_front() {
            if depth_map.contains_key(&node) {
                continue;
            }