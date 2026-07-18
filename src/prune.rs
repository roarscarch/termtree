use crate::{Forest, Tree, CommitNode, MergeNode};
use std::collections::{HashSet, HashMap};

/// Options for pruning the forest.
#[derive(Debug, Clone)]
pub struct PruneOptions {
    /// Remove branches with fewer than this many commits (default: 2)
    pub min_branch_commits: usize,
    /// Remove stubs (commits with no children and no parent? actually stubs are short dead ends)
    pub remove_stubs: bool,
    /// Remove isolated commits (no parent, no children)
    pub remove_isolated: bool,
    /// Maximum depth of dead branch to keep (0 = keep none)
    pub max_dead_depth: usize,
}

impl Default for PruneOptions {
    fn default() -> Self {
        PruneOptions {
            min_branch_commits: 2,
            remove_stubs: true,
            remove_isolated: true,
            max_dead_depth: 0,
        }
    }
}

/// Result of pruning.
#[derive(Debug, Clone)]
pub struct PruneResult {
    /// Number of commits removed
    pub commits_removed: usize,
    /// Number of trees removed
    pub trees_removed: usize,
    /// Number of merge nodes removed
    pub merges_removed: usize,
    /// Number of stubs removed
    pub stubs_removed: usize,
}

/// Prune dead branches and stubs from the forest.
/// Dead branches are those that have no commits beyond a certain depth from the mainline.
/// Stubs are short branches with few commits.
pub fn prune_forest(forest: &mut Forest, options: &PruneOptions) -> PruneResult {
    let mut result = PruneResult::default();

    // Identify commits that are part of long-lived branches (mainline or active branches)
    let active_commits = identify_active_commits(forest, options);

    // Collect commits to remove: those not in active set and meeting pruning criteria
    let mut to_remove: HashSet<String> = HashSet::new();
    for (id, commit) in &forest.commit_map {
        if !active_commits.contains(id) {
            // Check if this commit is part of a stub (short branch)
            if options.remove_stubs && is_stub_commit(forest, id, options) {
                to_remove.insert(id.clone());
                result.stubs_removed += 1;
            } else if options.remove_isolated && commit.parents.is_empty() && forest.children_of(id).is_empty() {
                to_remove.insert(id.clone());
                result.commits_removed += 1;
            }
        }
    }

    // Remove commits from commit_map
    for id in &to_remove {
        forest.commit_map.remove(id);
    }
    result.commits_removed += to_remove.len();

    // Remove trees that have no commits left
    let trees_before = forest.trees.len();
    forest.trees.retain(|tree| {
        let mut has_commits = false;
        for commit_id in &tree.commit_ids {
            if forest.commit_map.contains_key(commit_id) {
                has_commits = true;
                break;
            }
        }
        has_commits
    });
    result.trees_removed = trees_before - forest.trees.len();

    // Remove merge nodes that reference removed commits
    let merges_before = forest.merges.len();
    forest.merges.retain(|merge| {
        let mut valid = true;
        for commit_id in &merge.commit_ids {
            if !forest.commit_map.contains_key(commit_id) {
                valid = false;
                break;
            }
        }
        valid
    });
    result.merges_removed = merges_before - forest.merges.len();

    result
}

/// Identify commits that are part of active branches (mainline or branches with sufficient commits).
fn identify_active_commits(forest: &Forest, options: &PruneOptions) -> HashSet<String> {
    let mut active = HashSet::new();

    // First, find the mainline: the longest chain of commits (like the trunk)
    let mainline = find_mainline(forest);
    for id in &mainline {
        active.insert(id.clone());
    }

    // For each tree, if it has at least min_branch_commits commits, mark all its commits as active
    for tree in &forest.trees {
        if tree.commit_ids.len() >= options.min_branch_commits {
            for id in &tree.commit_ids {
                active.insert(id.clone());
            }
        }
    }

    // Also keep commits that are ancestors of active commits (to preserve merge structure)
    let mut changed = true;
    while changed {
        changed = false;
        let current_ids: Vec<String> = active.iter().cloned().collect();
        for id in &current_ids {
            if let Some(commit) = forest.commit_map.get(id) {
                for parent_id in &commit.parents {
                    if !active.contains(parent_id) {
                        active.insert(parent_id.clone());
                        changed = true;
                    }
                }
            }
        }
    }

    active
}

/// Find the mainline: the longest chain of commits from any root to any leaf.
fn find_mainline(forest: &Forest) -> Vec<String> {
    // Find all root commits (no parents)
    let roots: Vec<&String> = forest.commit_map.iter()
        .filter(|(_, c)| c.parents.is_empty())
        .map(|(id, _)| id)
        .collect();

    if roots.is_empty() {
        return Vec::new();
    }

    // DFS to find longest path
    let mut longest_path: Vec<String> = Vec::new();
    for root_id in roots {
        let mut path = Vec::new();
        dfs_longest_path(forest, root_id, &mut path, &mut longest_path);
    }

    longest_path
}

fn dfs_longest_path(forest: &Forest, current_id: &str, path: &mut Vec<String>, longest: &mut Vec<String>) {
    path.push(current_id.to_string());

    let commit = match forest.commit_map.get(current_id) {
        Some(c) => c,
        None => {
            path.pop();
            return;
        }
    };

    // Find children (commits that have this as parent)
    let children = forest.children_of(current_id);
    if children.is_empty() {
        // Leaf: check if this path is longer
        if path.len() > longest.len() {
            *longest = path.clone();
        }
    } else {
        for child_id in &children {
            dfs_longest_path(forest, child_id, path, longest);
        }
    }

    path.pop();
}

/// Check if a commit is part of a stub (short branch with few commits).
fn is_stub_commit(forest: &Forest, commit_id: &str, options: &PruneOptions) -> bool {
    // A stub is a commit that is on a branch with fewer than min_branch_commits commits
    // and is not on the mainline.
    for tree in &forest.trees {
        if tree.commit_ids.contains(&commit_id.to_string()) {
            return tree.commit_ids.len() < options.min_branch_commits;
        }
    }
    false
}

/// Helper trait to get children of a commit.
pub trait ForestChildren {
    fn children_of(&self, commit_id: &str) -> Vec<String>;
}

impl ForestChildren for Forest {
    fn children_of(&self, commit_id: &str) -> Vec<String> {
        let mut children = Vec::new();
        for (id, commit) in &self.commit_map {
            if commit.parents.contains(&commit_id.to_string()) {
                children.push(id.clone());
            }
        }
        children
    }
}