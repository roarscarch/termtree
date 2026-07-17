use crate::{Forest, Tree, CommitNode, LayoutResult};
use std::collections::{HashMap, HashSet};

/// Configuration for forest pruning
#[derive(Debug, Clone)]
pub struct PruneConfig {
    /// Minimum number of commits for a branch to be kept (0 = keep all)
    pub min_commits: usize,
    /// Maximum age in days for a branch tip to be considered alive (0 = no age limit)
    pub max_tip_age_days: u64,
    /// Whether to remove orphaned merge nodes (merges whose parents are both pruned)
    pub remove_orphan_merges: bool,
    /// Whether to reattach orphaned commits to nearest alive ancestor
    pub reattach_orphans: bool,
}

impl Default for PruneConfig {
    fn default() -> Self {
        PruneConfig {
            min_commits: 2,
            max_tip_age_days: 0,
            remove_orphan_merges: true,
            reattach_orphans: false,
        }
    }
}

/// Result of pruning operation
#[derive(Debug, Clone)]
pub struct PruneResult {
    /// Number of branches removed
    pub branches_removed: usize,
    /// Number of commits removed
    pub commits_removed: usize,
    /// Number of merges removed
    pub merges_removed: usize,
    /// IDs of removed branches
    pub removed_branches: Vec<String>,
    /// Whether the forest was modified
    pub modified: bool,
}

/// Prune dead branches from the forest according to configuration.
/// Returns the pruned forest and a result summary.
pub fn prune_forest(
    forest: &Forest,
    config: &PruneConfig,
    layout: &LayoutResult,
) -> (Forest, PruneResult) {
    let mut result = PruneResult {
        branches_removed: 0,
        commits_removed: 0,
        merges_removed: 0,
        removed_branches: Vec::new(),
        modified: false,
    };

    // Identify branches to keep
    let branches_to_keep = identify_branches_to_keep(forest, config);

    if branches_to_keep.len() == forest.trees.len() {
        // No pruning needed
        return (forest.clone(), result);
    }

    // Build set of commit IDs to keep (from kept branches and their ancestors)
    let commits_to_keep = collect_commits_to_keep(forest, &branches_to_keep);

    // Build new forest
    let mut new_forest = Forest {
        trees: Vec::new(),
        commit_map: HashMap::new(),
        merge_nodes: Vec::new(),
        ..forest.clone()
    };

    // Filter trees
    for tree in &forest.trees {
        if branches_to_keep.contains(&tree.branch_name) {
            new_forest.trees.push(tree.clone());
        } else {
            result.branches_removed += 1;
            result.removed_branches.push(tree.branch_name.clone());
        }
    }

    // Filter commits
    for (id, commit) in &forest.commit_map {
        if commits_to_keep.contains(id) {
            new_forest.commit_map.insert(id.clone(), commit.clone());
        } else {
            result.commits_removed += 1;
        }
    }

    // Filter merge nodes
    if config.remove_orphan_merges {
        for merge in &forest.merge_nodes {
            let parents_alive = merge.parents.iter()
                .all(|p| commits_to_keep.contains(p));
            if parents_alive || !config.remove_orphan_merges {
                new_forest.merge_nodes.push(merge.clone());
            } else {
                result.merges_removed += 1;
            }
        }
    } else {
        new_forest.merge_nodes = forest.merge_nodes.clone();
    }

    // Optionally reattach orphans
    if config.reattach_orphans {
        new_forest = reattach_orphaned_commits(&new_forest, &commits_to_keep, layout);
    }

    result.modified = result.branches_removed > 0 || result.commits_removed > 0 || result.merges_removed > 0;

    (new_forest, result)
}

/// Determine which branches to keep based on config
fn identify_branches_to_keep(forest: &Forest, config: &PruneConfig) -> HashSet<String> {
    let mut keep: HashSet<String> = HashSet::new();

    for tree in &forest.trees {
        let mut should_keep = true;

        // Check minimum commits
        if config.min_commits > 0 && tree.commits.len() < config.min_commits {
            should_keep = false;
        }

        // Check max tip age (if age info available in commits)
        if should_keep && config.max_tip_age_days > 0 {
            if let Some(tip_commit) = tree.commits.last() {
                if let Some(timestamp) = forest.commit_map.get(&tip_commit.id).and_then(|c| c.timestamp) {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    let age_seconds = now.saturating_sub(timestamp);
                    let age_days = age_seconds / 86400;
                    if age_days > config.max_tip_age_days {
                        should_keep = false;
                    }
                }
            }
        }

        if should_keep {
            keep.insert(tree.branch_name.clone());
        }
    }

    keep
}

/// Collect all commit IDs that should be kept (from kept branches and their ancestors)
fn collect_commits_to_keep(forest: &Forest, branches_to_keep: &HashSet<String>) -> HashSet<String> {
    let mut keep: HashSet<String> = HashSet::new();

    for tree in &forest.trees {
        if !branches_to_keep.contains(&tree.branch_name) {
            continue;
        }
        for commit in &tree.commits {
            keep.insert(commit.id.clone());
        }
    }

    // Also include parents of kept commits (ancestors)
    let mut changed = true;
    while changed {
        changed = false;
        let current_keep = keep.clone();
        for id in &current_keep {
            if let Some(commit) = forest.commit_map.get(id) {
                for parent in &commit.parents {
                    if !keep.contains(parent) {
                        keep.insert(parent.clone());
                        changed = true;
                    }
                }
            }
        }
    }

    keep
}

/// Reattach orphaned commits (commits that lost their branch but are still kept)
/// to the nearest alive ancestor branch.
fn reattach_orphaned_commits(
    forest: &Forest,
    commits_to_keep: &HashSet<String>,
    layout: &LayoutResult,
) -> Forest {
    let mut new_forest = forest.clone();

    // Find commits that are in commit_map but not in any tree
    let mut orphaned: Vec<String> = Vec::new();
    let mut all_tree_commits: HashSet<String> = HashSet::new();
    for tree in &new_forest.trees {
        for commit in &tree.commits {
            all_tree_commits.insert(commit.id.clone());
        }
    }
    for id in commits_to_keep {
        if !all_tree_commits.contains(id) {
            orphaned.push(id.clone());
        }
    }

    if orphaned.is_empty() {
        return new_forest;
    }