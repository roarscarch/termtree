use crate::{Forest, Tree, CommitNode, MergeNode};
use std::collections::{HashMap, VecDeque};

/// Represents a 2D position for a tree or commit node
#[derive(Debug, Clone)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

/// Layout configuration parameters
pub struct LayoutConfig {
    /// Horizontal spacing between trees (branches)
    pub tree_spacing: f64,
    /// Vertical spacing between commits on the same branch
    pub commit_spacing: f64,
    /// Gravitational pull strength toward parent commits
    pub gravity: f64,
    /// Number of layout iterations for convergence
    pub iterations: usize,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        LayoutConfig {
            tree_spacing: 6.0,
            commit_spacing: 1.0,
            gravity: 0.1,
            iterations: 50,
        }
    }
}

/// Compute 2D positions for all trees and commits in the forest.
/// Uses a topological sort then applies gravitational pull between related commits.
pub fn layout_forest(forest: &Forest, config: &LayoutConfig) -> HashMap<String, Position> {
    let mut positions: HashMap<String, Position> = HashMap::new();
    
    // Assign initial positions based on branch order and commit depth
    let branch_names: Vec<&String> = forest.trees.keys().collect();
    let mut x_offset = 0.0;
    for branch_name in &branch_names {
        if let Some(tree) = forest.trees.get(*branch_name) {
            let mut y_offset = 0.0;
            // Sort commits by timestamp (assuming they are stored in order)
            let mut commits: Vec<&CommitNode> = tree.commits.values().collect();
            commits.sort_by_key(|c| c.timestamp);
            for commit in &commits {
                positions.insert(commit.id.clone(), Position {
                    x: x_offset,
                    y: y_offset,
                });
                y_offset += config.commit_spacing;
            }
            x_offset += config.tree_spacing;
        }
    }
    
    // Apply gravitational pull toward parent commits (iteratively)
    for _ in 0..config.iterations {
        let mut new_positions = positions.clone();
        for (commit_id, pos) in &positions {
            // Find parent commits
            let parents = find_parents(forest, commit_id);
            if parents.is_empty() {
                continue;
            }
            // Compute average position of parents
            let mut avg_x = 0.0;
            let mut avg_y = 0.0;
            let mut count = 0.0;
            for parent_id in &parents {
                if let Some(parent_pos) = positions.get(parent_id) {
                    avg_x += parent_pos.x;
                    avg_y += parent_pos.y;
                    count += 1.0;
                }
            }
            if count > 0.0 {
                avg_x /= count;
                avg_y /= count;
                // Pull current commit toward parent average
                let dx = avg_x - pos.x;
                let dy = avg_y - pos.y;
                let new_x = pos.x + dx * config.gravity;
                let new_y = pos.y + dy * config.gravity;
                new_positions.insert(commit_id.clone(), Position { x: new_x, y: new_y });
            }
        }
        positions = new_positions;
    }
    
    // Normalize positions to avoid negative coordinates
    let min_x = positions.values().map(|p| p.x).fold(f64::INFINITY, f64::min);
    let min_y = positions.values().map(|p| p.y).fold(f64::INFINITY, f64::min);
    for pos in positions.values_mut() {
        pos.x -= min_x;
        pos.y -= min_y;
    }
    
    positions
}

/// Find parent commit IDs for a given commit ID.
/// Searches across all trees and merge nodes.
fn find_parents(forest: &Forest, commit_id: &str) -> Vec<String> {
    let mut parents = Vec::new();
    
    // Search in all trees
    for tree in forest.trees.values() {
        if let Some(commit) = tree.commits.get(commit_id) {
            for parent_id in &commit.parents {
                parents.push(parent_id.clone());
            }
            break;
        }
    }
    
    // Also check merge nodes
    for merge in &forest.merges {
        if merge.commit_id == commit_id {
            for parent_id in &merge.parents {
                parents.push(parent_id.clone());
            }
        }
    }
    
    parents
}

/// Build a tree structure from a set of commits (lineage).
/// Returns the trunk as a series of positions (bottom to top).
pub fn build_tree_trunk(
    positions: &HashMap<String, Position>,
    branch_commits: &[&CommitNode],
) -> Vec<Position> {
    let mut trunk: Vec<Position> = Vec::new();
    for commit in branch_commits {
        if let Some(pos) = positions.get(&commit.id) {
            trunk.push(pos.clone());
        }
    }
    // Sort by y ascending (bottom to top)
    trunk.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal));
    trunk
}

/// Compute branch taper for visual thickness (wider at bottom, narrower at top).
pub fn compute_branch_thickness(y: f64, max_y: f64, min_thickness: f64, max_thickness: f64) -> f64 {
    if max_y == 0.0 {
        return max_thickness;
    }
    let ratio = y / max_y;
    max_thickness - (max_thickness - min_thickness) * ratio
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Forest, Tree, CommitNode, MergeNode};
    
    fn make_test_forest() -> Forest {
        let mut forest = Forest {
            trees: HashMap::new(),
            merges: Vec::new(),
            commit_map: HashMap::new(),
        };
        
        let mut tree = Tree {
            branch_name: "main".to_string(),
            commits: HashMap::new(),
        };
        
        let commit1 = CommitNode {
            id: "aaa".to_string(),
            author: "alice".to_string(),
            message: "initial".to_string(),
            timestamp: 1000,
            parents: vec![],
            children: vec!["bbb".to_string()],
        };
        let commit2 = CommitNode {
            id: "bbb".to_string(),
            author: "alice".to_string(),
            message: "second".to_string(),
            timestamp: 1001,
            parents: vec!["aaa".to_string()],
            children: vec![],
        };
        
        tree.commits.insert("aaa".to_string(), commit1);
        tree.commits.insert("bbb".to_string(), commit2);
        
        forest.trees.insert("main".to_string(), tree);
        forest.commit_map.insert("alice".to_string(), vec!["aaa".to_string(), "bbb".to_string()]);
        
        forest
    }
    
    #[test]
    fn test_layout_simple() {
        let forest = make_test_forest();
        let config = LayoutConfig::default();
        let positions = layout_forest(&forest, &config);
        
        assert!(positions.contains_key("aaa"));
        assert!(positions.contains_key("bbb"));
        
        // bbb should be above aaa (higher y)
        assert!(positions["bbb"].y > positions["aaa"].y);
    }