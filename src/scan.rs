use crate::types::{CommitNode, MergeNode, Forest, Tree};
use git2::{Repository, Oid, SortMode};
use std::collections::HashMap;

/// Result of scanning a git repository
#[derive(Debug, Clone)]
pub struct ScanResult {
    pub forest: Forest,
    pub trees: Vec<Tree>,
    pub commit_count: usize,
    pub branch_count: usize,
}

/// Scan a git repository at the given path and build the forest data structure.
/// Returns a ScanResult containing the parsed commit graph and derived trees.
pub fn scan_repository(path: &str) -> Result<ScanResult, Box<dyn std::error::Error>> {
    let repo = Repository::open(path)?;
    let mut revwalk = repo.revwalk()?;
    revwalk.set_sorting(SortMode::TOPOLOGICAL | SortMode::TIME)?;
    revwalk.push_head()?;

    let mut commits: Vec<CommitNode> = Vec::new();
    let mut commit_map: HashMap<Oid, usize> = HashMap::new();
    let mut author_count: HashMap<String, usize> = HashMap::new();

    for oid_result in revwalk {
        let oid = oid_result?;
        let commit = repo.find_commit(oid)?;
        let author = commit.author().name().unwrap_or("unknown").to_string();
        let message = commit.message().unwrap_or("").to_string();
        let time = commit.time().seconds() as u64;
        let parent_ids: Vec<Oid> = commit.parents().map(|p| p.id()).collect();

        let idx = commits.len();
        commit_map.insert(oid, idx);

        let merge_base = if parent_ids.len() > 1 {
            // attempt to find merge base among parents
            let mut base_oid = None;
            for i in 0..parent_ids.len() {
                for j in (i+1)..parent_ids.len() {
                    if let Ok(merge_base) = repo.merge_base(parent_ids[i], parent_ids[j]) {
                        base_oid = Some(merge_base);
                        break;
                    }
                }
                if base_oid.is_some() {
                    break;
                }
            }
            base_oid
        } else {
            None
        };

        let node = CommitNode {
            id: oid.to_string(),
            author: author.clone(),
            message,
            timestamp: time,
            parent_ids: parent_ids.iter().map(|o| o.to_string()).collect(),
            children: Vec::new(),
            is_merge: parent_ids.len() > 1,
            merge_base: merge_base.map(|o| o.to_string()),
            depth: 0,
        };
        *author_count.entry(author).or_insert(0) += 1;
        commits.push(node);
    }

    // Build children lists
    let oid_to_idx: HashMap<String, usize> = commit_map
        .iter()
        .map(|(oid, &idx)| (oid.to_string(), idx))
        .collect();
    for i in 0..commits.len() {
        let parent_ids = commits[i].parent_ids.clone();
        for pid in &parent_ids {
            if let Some(&pidx) = oid_to_idx.get(pid) {
                commits[pidx].children.push(i);
            }
        }
    }

    // Assign depth via BFS from roots (commits with no parents in our set)
    let roots: Vec<usize> = commits
        .iter()
        .enumerate()
        .filter(|(_, c)| c.parent_ids.is_empty())
        .map(|(i, _)| i)
        .collect();

    let mut queue: Vec<(usize, usize)> = roots.iter().map(|&r| (r, 0)).collect();
    while let Some((idx, depth)) = queue.pop() {
        commits[idx].depth = depth;
        for &child in &commits[idx].children {
            if commits[child].depth <= depth {
                commits[child].depth = depth + 1;
                queue.push((child, depth + 1));
            }
        }
    }

    // Build merge nodes
    let mut merge_nodes: Vec<MergeNode> = Vec::new();
    for (i, commit) in commits.iter().enumerate() {
        if commit.is_merge && commit.parent_ids.len() >= 2 {
            let parent_indices: Vec<usize> = commit
                .parent_ids
                .iter()
                .filter_map(|pid| oid_to_idx.get(pid))
                .copied()
                .collect();
            merge_nodes.push(MergeNode {
                commit_index: i,
                parent_indices,
                merge_base: commit.merge_base.clone(),
            });
        }
    }

    // Build trees (group commits by first-parent lineage)
    let mut trees: Vec<Tree> = Vec::new();
    let mut visited: Vec<bool> = vec![false; commits.len()];
    for i in 0..commits.len() {
        if visited[i] {
            continue;
        }
        // Start a new tree from this commit, follow first parent chain
        let mut trunk: Vec<usize> = Vec::new();
        let mut current = i;
        loop {
            if visited[current] {
                break;
            }
            visited[current] = true;
            trunk.push(current);
            // If has parents, follow first parent (index 0)
            let parent_ids = &commits[current].parent_ids;
            if parent_ids.is_empty() {
                break;
            }
            if let Some(&next) = oid_to_idx.get(&parent_ids[0]) {
                current = next;
            } else {
                break;
            }
        }
        if !trunk.is_empty() {
            let branch_count = trunk.len();
            let leaf_count = trunk.iter().map(|&idx| commits[idx].children.len()).sum();
            let author = commits[trunk[0]].author.clone();
            trees.push(Tree {
                trunk_nodes: trunk,
                branch_count,
                leaf_count,
                author,
            });
        }
    }

    let forest = Forest {
        commits,
        commit_map: HashMap::new(), // populated below
        merge_nodes,
        trees: trees.clone(),
        author_commit_counts: author_count,
    };

    // Build commit_map from author string to list of indices
    let mut commit_map: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, c) in forest.commits.iter().enumerate() {
        commit_map.entry(c.author.clone()).or_default().push(i);
    }

    let forest = Forest {
        commit_map,
        ..forest
    };

    let commit_count = forest.commits.len();
    let branch_count = trees.len();

    Ok(ScanResult {
        forest,
        trees,
        commit_count,
        branch_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_current_repo() {
        // This test runs against the current repo; assumes it's a git repo with commits
        let result = scan_repository(".");
        assert!(result.is_ok());
        let scan = result.unwrap();
        assert!(scan.commit_count > 0);
        assert!(scan.branch_count > 0);
        assert!(!scan.forest.commits.is_empty());
        assert!(!scan.forest.trees.is_empty());
    }
}
