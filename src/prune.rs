use crate::{Forest, Tree, CommitNode, MergeNode};
use std::collections::{HashSet, HashMap};

/// Prune the forest by removing dead branches (branches with no recent commits)
/// and stubs (very short branches that don't add visual value).
/// Returns a new Forest with the pruned trees.
pub fn prune_forest(forest: &Forest, min_commits: usize, max_age_days: Option<u64>) -> Forest {
    let mut pruned_trees: Vec<Tree> = Vec::new();
    let mut pruned_commit_map: HashMap<String, CommitNode> = HashMap::new();
    let mut pruned_merge_nodes: Vec<MergeNode> = Vec::new();

    for tree in &forest.trees {
        let branch_id = &tree.branch_id;
        let commits_on_branch: Vec<&String> = forest.commit_map.keys()
            .filter(|id| {
                if let Some(node) = forest.commit_map.get(*id) {
                    node.branch_id == *branch_id
                } else {
                    false
                }
            })
            .collect();

        // Check minimum commit count
        if commits_on_branch.len() < min_commits {
            continue;
        }

        // Optionally check max age (if any commit is too old, prune the whole branch?)
        // For simplicity, we keep the branch if at least one commit is recent enough.
        if let Some(max_age) = max_age_days {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let max_age_secs = max_age * 86400;
            let has_recent = commits_on_branch.iter().any(|id| {
                if let Some(node) = forest.commit_map.get(*id) {
                    (now - node.timestamp) < max_age_secs
                } else {
                    false
                }
            });
            if !has_recent {
                continue;
            }
        }

        // Keep the tree
        pruned_trees.push(tree.clone());
        for id in &commits_on_branch {
            if let Some(node) = forest.commit_map.get(*id) {
                pruned_commit_map.insert(id.clone(), node.clone());
            }
        }
    }

    // Also keep merge nodes that reference surviving commits
    for merge in &forest.merge_nodes {
        let parent_survives = merge.parent_ids.iter().any(|pid| pruned_commit_map.contains_key(pid));
        let child_survives = pruned_commit_map.contains_key(&merge.child_id);
        if parent_survives || child_survives {
            pruned_merge_nodes.push(merge.clone());
        }
    }

    Forest {
        trees: pruned_trees,
        commit_map: pruned_commit_map,
        merge_nodes: pruned_merge_nodes,
    }
}

/// Remove stub branches (branches with very few commits and no merges).
pub fn remove_stubs(forest: &Forest, min_commits: usize) -> Forest {
    let mut pruned = forest.clone();
    loop {
        let before = pruned.trees.len();
        pruned = prune_forest(&pruned, min_commits, None);
        if pruned.trees.len() == before {
            break;
        }
    }
    pruned
}

/// Clean up orphaned commits (commits referenced by no tree and no merge node).
pub fn remove_orphans(forest: &Forest) -> Forest {
    let mut referenced: HashSet<String> = HashSet::new();
    for tree in &forest.trees {
        // We assume tree has a root_commit_id
        // For now, we add all commits from commit_map that belong to this tree
        for (id, node) in &forest.commit_map {
            if node.branch_id == tree.branch_id {
                referenced.insert(id.clone());
            }
        }
    }
    for merge in &forest.merge_nodes {
        referenced.insert(merge.child_id.clone());
        for pid in &merge.parent_ids {
            referenced.insert(pid.clone());
        }
    }
    let mut pruned_commit_map: HashMap<String, CommitNode> = HashMap::new();
    for (id, node) in &forest.commit_map {
        if referenced.contains(id) {
            pruned_commit_map.insert(id.clone(), node.clone());
        }
    }
    Forest {
        trees: forest.trees.clone(),
        commit_map: pruned_commit_map,
        merge_nodes: forest.merge_nodes.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prune_forest_removes_short_branches() {
        let mut forest = Forest::default();
        // Add a tree with 1 commit (should be pruned if min_commits=2)
        let tree1 = Tree {
            branch_id: "branch1".to_string(),
            trunk_length: 1.0,
            branch_points: vec![],
            leaf_density: 0.5,
        };
        let commit1 = CommitNode {
            id: "abc123".to_string(),
            branch_id: "branch1".to_string(),
            author: "author1".to_string(),
            message: "first commit".to_string(),
            timestamp: 1000,
            parent_ids: vec![],
            child_ids: vec![],
        };
        forest.trees.push(tree1);
        forest.commit_map.insert("abc123".to_string(), commit1);

        let pruned = prune_forest(&forest, 2, None);
        assert!(pruned.trees.is_empty());
        assert!(pruned.commit_map.is_empty());
    }

    #[test]
    fn test_remove_orphans() {
        let mut forest = Forest::default();
        let tree = Tree {
            branch_id: "main".to_string(),
            trunk_length: 5.0,
            branch_points: vec![],
            leaf_density: 0.8,
        };
        forest.trees.push(tree);
        let commit_main = CommitNode {
            id: "main1".to_string(),
            branch_id: "main".to_string(),
            author: "author".to_string(),
            message: "main commit".to_string(),
            timestamp: 100,
            parent_ids: vec![],
            child_ids: vec![],
        };
        let orphan_commit = CommitNode {
            id: "orphan".to_string(),
            branch_id: "nonexistent".to_string(),
            author: "orphan".to_string(),
            message: "orphan".to_string(),
            timestamp: 200,
            parent_ids: vec![],
            child_ids: vec![],
        };
        forest.commit_map.insert("main1".to_string(), commit_main);
        forest.commit_map.insert("orphan".to_string(), orphan_commit);

        let cleaned = remove_orphans(&forest);
        assert!(cleaned.commit_map.contains_key("main1"));
        assert!(!cleaned.commit_map.contains_key("orphan"));
    }
}