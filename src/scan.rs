use crate::{CommitNode, Forest, Tree, MergeNode};
use git2::{Repository, Oid, Sort};
use std::collections::{HashMap, HashSet};

/// Scan a git repository and build a Forest data structure.
pub fn scan_repository(path: &str) -> Result<Forest, String> {
    let repo = Repository::open(path).map_err(|e| format!("Failed to open repository: {}", e))?;
    let mut revwalk = repo.revwalk().map_err(|e| format!("Failed to create revwalk: {}", e))?;
    revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME).map_err(|e| format!("Failed to set sorting: {}", e))?;
    revwalk.push_head().map_err(|e| format!("Failed to push HEAD: {}", e))?;

    let mut commit_map: HashMap<String, CommitNode> = HashMap::new();
    let mut parent_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut child_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut branch_heads: HashMap<String, String> = HashMap::new(); // branch name -> commit id

    // Collect all commits
    for oid_result in revwalk {
        let oid = oid_result.map_err(|e| format!("Revwalk error: {}", e))?;
        let commit = repo.find_commit(oid).map_err(|e| format!("Failed to find commit: {}", e))?;
        let id = oid.to_string();
        let author = commit.author().name().unwrap_or("unknown").to_string();
        let message = commit.message().unwrap_or("").to_string();
        let time = commit.time().seconds() as u64;
        let parent_ids: Vec<String> = commit.parents().map(|p| p.id().to_string()).collect();

        let node = CommitNode {
            id: id.clone(),
            message,
            author,
            time,
            parent_ids: parent_ids.clone(),
            children: Vec::new(),
        };
        commit_map.insert(id.clone(), node);
        parent_map.insert(id.clone(), parent_ids.clone());

        for pid in &parent_ids {
            child_map.entry(pid.clone()).or_insert_with(Vec::new).push(id.clone());
        }
    }

    // Populate children in commit_map
    for (child_id, parents) in &parent_map {
        for pid in parents {
            if let Some(parent_node) = commit_map.get_mut(pid) {
                parent_node.children.push(child_id.clone());
            }
        }
    }

    // Identify branch heads (local branches)
    if let Ok(branches) = repo.branches(Some(git2::BranchType::Local)) {
        for branch_result in branches {
            if let Ok((branch, _)) = branch_result {
                if let Some(name) = branch.name().ok().flatten() {
                    if let Some(target) = branch.get().target() {
                        branch_heads.insert(name.to_string(), target.to_string());
                    }
                }
            }
        }
    }

    // Build trees: each branch head becomes a tree root, but we merge branches that share history
    let mut tree_roots: Vec<String> = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();

    // Start from branch heads and walk backwards, marking commits as belonging to that tree
    for (branch_name, head_id) in &branch_heads {
        if !visited.contains(head_id) {
            tree_roots.push(head_id.clone());
            // Mark all ancestors as visited for this tree
            let mut stack = vec![head_id.clone()];
            while let Some(current) = stack.pop() {
                if visited.contains(&current) {
                    continue;
                }
                visited.insert(current.clone());
                if let Some(parents) = parent_map.get(&current) {
                    for parent in parents {
                        stack.push(parent.clone());
                    }
                }
            }
        }
    }

    // If no branches found, use HEAD
    if tree_roots.is_empty() {
        if let Ok(head) = repo.head() {
            if let Some(target) = head.target() {
                tree_roots.push(target.to_string());
            }
        }
    }

    // Build trees
    let mut trees: Vec<Tree> = Vec::new();
    for root_id in &tree_roots {
        let mut commit_ids: Vec<String> = Vec::new();
        let mut stack = vec![root_id.clone()];
        let mut local_visited: HashSet<String> = HashSet::new();
        while let Some(current) = stack.pop() {
            if local_visited.contains(&current) {
                continue;
            }
            local_visited.insert(current.clone());
            commit_ids.push(current.clone());
            if let Some(parents) = parent_map.get(&current) {
                for parent in parents {
                    if !local_visited.contains(parent) {
                        stack.push(parent.clone());
                    }
                }
            }
        }
        trees.push(Tree {
            root: root_id.clone(),
            commits: commit_ids,
        });
    }

    // Detect merge nodes (commits with more than one parent)
    let mut merge_nodes: Vec<MergeNode> = Vec::new();
    for (id, parents) in &parent_map {
        if parents.len() > 1 {
            if let Some(commit) = commit_map.get(id) {
                merge_nodes.push(MergeNode {
                    id: id.clone(),
                    parent_ids: parents.clone(),
                    child_ids: commit.children.clone(),
                    timestamp: commit.time,
                });
            }
        }
    }

    Ok(Forest {
        commit_map,
        trees,
        merge_nodes,
    })
}
