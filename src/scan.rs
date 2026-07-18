use git2::{Repository, Oid, SortMode};
use crate::types::{Forest, CommitNode, MergeNode, Tree, BranchInfo};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Scan a git repository and extract the commit graph into a Forest structure.
pub fn scan_repository<P: AsRef<Path>>(path: P) -> Result<Forest, Box<dyn std::error::Error>> {
    let repo = Repository::open(path)?;
    
    let mut forest = Forest {
        trees: Vec::new(),
        commit_map: HashMap::new(),
        merge_nodes: Vec::new(),
        root_commits: Vec::new(),
        branches: Vec::new(),
        stats: Default::default(),
    };

    // Collect all references (branches)
    let branches = collect_branches(&repo)?;
    forest.branches = branches.clone();

    // Walk all commits from all branch tips
    let mut visited: HashSet<Oid> = HashSet::new();
    let mut commit_list: Vec<CommitNode> = Vec::new();
    let mut merge_list: Vec<MergeNode> = Vec::new();
    let mut parent_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut child_map: HashMap<String, Vec<String>> = HashMap::new();

    for branch in &branches {
        let oid = Oid::from_str(&branch.target_oid)?;
        if visited.contains(&oid) {
            continue;
        }
        let mut revwalk = repo.revwalk()?;
        revwalk.push(oid)?;
        revwalk.set_sorting(SortMode::TOPOLOGICAL | SortMode::TIME)?;

        for oid_result in revwalk {
            let oid = oid_result?;
            if visited.contains(&oid) {
                continue;
            }
            visited.insert(oid);

            let commit = repo.find_commit(oid)?;
            let id = oid.to_string();
            let author = commit.author().name().unwrap_or("unknown").to_string();
            let time = commit.time().seconds();
            let message = commit.message().unwrap_or("").to_string();

            let parent_ids: Vec<String> = commit.parent_ids().map(|p| p.to_string()).collect();
            parent_map.insert(id.clone(), parent_ids.clone());

            for parent_id in &parent_ids {
                child_map.entry(parent_id.clone()).or_default().push(id.clone());
            }

            let is_merge = parent_ids.len() > 1;
            if is_merge {
                merge_list.push(MergeNode {
                    id: id.clone(),
                    parents: parent_ids.clone(),
                    time,
                    author: author.clone(),
                });
            }

            commit_list.push(CommitNode {
                id: id.clone(),
                author,
                time,
                message: message.lines().next().unwrap_or("").to_string(),
                parent_ids: parent_ids.clone(),
                is_merge,
                children: Vec::new(),
                branch_hint: branch.name.clone(),
                tree_index: None,
            });
        }
    }

    // Build children lists
    for commit in &mut commit_list {
        if let Some(children) = child_map.get(&commit.id) {
            commit.children = children.clone();
        }
    }

    // Identify root commits (no parents)
    for commit in &commit_list {
        if commit.parent_ids.is_empty() {
            forest.root_commits.push(commit.id.clone());
        }
    }

    // Build commit map
    for commit in commit_list.into_iter() {
        forest.commit_map.insert(commit.id.clone(), commit);
    }

    forest.merge_nodes = merge_list;

    // Build trees from branches
    forest.trees = build_trees(&forest, &branches);

    // Compute stats
    forest.stats = crate::stats::compute_stats(&forest);

    Ok(forest)
}

/// Collect all branches and their current tip OIDs.
fn collect_branches(repo: &Repository) -> Result<Vec<BranchInfo>, Box<dyn std::error::Error>> {
    let mut branches = Vec::new();
    let branch_iter = repo.branches(Some(git2::BranchType::Local))?;

    for branch_result in branch_iter {
        let (branch, _type) = branch_result?;
        let name = branch.name()?.unwrap_or("unknown").to_string();
        if let Some(target) = branch.get().target() {
            branches.push(BranchInfo {
                name,
                target_oid: target.to_string(),
            });
        }
    }

    // Also collect remote branches
    let remote_branch_iter = repo.branches(Some(git2::BranchType::Remote))?;
    for branch_result in remote_branch_iter {
        let (branch, _type) = branch_result?;
        let name = branch.name()?.unwrap_or("unknown").to_string();
        if let Some(target) = branch.get().target() {
            branches.push(BranchInfo {
                name,
                target_oid: target.to_string(),
            });
        }
    }

    Ok(branches)
}

/// Build tree structures from branch information and commit graph.
fn build_trees(forest: &Forest, branches: &[BranchInfo]) -> Vec<Tree> {
    let mut trees: Vec<Tree> = Vec::new();
    let mut assigned_commits: HashSet<String> = HashSet::new();

    for branch in branches {
        let mut tree_commits: Vec<String> = Vec::new();
        let mut current_oid = branch.target_oid.clone();

        // Walk backwards from branch tip until we hit an already assigned commit or root
        loop {
            if assigned_commits.contains(&current_oid) {
                break;
            }
            if let Some(commit) = forest.commit_map.get(&current_oid) {
                tree_commits.push(current_oid.clone());
                assigned_commits.insert(current_oid.clone());

                if commit.parent_ids.is_empty() {
                    break;
                }
                // Follow first parent (main line)
                current_oid = commit.parent_ids[0].clone();
            } else {
                break;
            }
        }

        if !tree_commits.is_empty() {
            trees.push(Tree {
                root: tree_commits.last().cloned().unwrap_or_default(),
                trunk_commits: tree_commits.clone(),
                branch_tip: branch.target_oid.clone(),
                leaf_commits: Vec::new(),
                color: (100, 180, 100), // default green, will be updated later
                label: branch.name.clone(),
                sway_offset: 0.0,
            });
        }
    }

    // Assign remaining unassigned commits to nearest tree
    for (id, commit) in &forest.commit_map {
        if !assigned_commits.contains(id) {
            // Find closest tree by checking common ancestors
            if let Some(tree_idx) = find_nearest_tree(forest, &trees, id) {
                if let Some(tree) = trees.get_mut(tree_idx) {
                    tree.leaf_commits.push(id.clone());
                }
            }
        }
    }

    trees
}

/// Find the tree that is most closely related to a given commit.
fn find_nearest_tree(forest: &Forest, trees: &[Tree], commit_id: &str) -> Option<usize> {
    let mut best_score = 0usize;
    let mut best_idx = None;

    for (idx, tree) in trees.iter().enumerate() {
        let mut score = 0;
        // Count ancestors in common
        let mut current = commit_id.to_string();
        loop {
            if tree.trunk_commits.contains(&current) || tree.leaf_commits.contains(&current) {
                score += 10;
            }