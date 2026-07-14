use crate::{CommitNode, Forest, Tree, MergeNode};
use git2::Repository;
use std::collections::{HashMap, HashSet, VecDeque};
use std::error::Error;

/// Scan a git repository and build a Forest data structure.
pub fn scan_repository(repo_path: &str) -> Result<Forest, Box<dyn Error>> {
    let repo = Repository::open(repo_path)?;
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(git2::Sort::TIME)?;

    let mut commit_map: HashMap<String, CommitNode> = HashMap::new();
    let mut children_map: HashMap<String, Vec<String>> = HashMap::new();

    // Collect all commits
    for oid_result in revwalk {
        let oid = oid_result?;
        let commit = repo.find_commit(oid)?;
        let id = oid.to_string();
        let author = commit.author().name().unwrap_or("unknown").to_string();
        let time = commit.time().seconds();
        let message = commit.message().unwrap_or("").to_string();
        let parents: Vec<String> = commit.parents().map(|p| p.id().to_string()).collect();

        let node = CommitNode {
            id: id.clone(),
            author,
            time,
            message,
            parents: parents.clone(),
        };
        commit_map.insert(id.clone(), node);

        for parent in &parents {
            children_map.entry(parent.clone()).or_default().push(id.clone());
        }
    }

    // Find root commits (no parents in the repository)
    let roots: Vec<String> = commit_map.keys()
        .filter(|id| {
            if let Some(node) = commit_map.get(*id) {
                node.parents.is_empty() || node.parents.iter().all(|p| !commit_map.contains_key(p))
            } else {
                false
            }
        })
        .cloned()
        .collect();

    // Build trees by following linear chains from roots
    let mut visited: HashSet<String> = HashSet::new();
    let mut trees: Vec<Tree> = Vec::new();
    let mut merge_points: Vec<MergeNode> = Vec::new();

    for root in &roots {
        let mut current = root.clone();
        let mut commits_in_tree: Vec<String> = Vec::new();

        loop {
            if visited.contains(&current) {
                break;
            }
            visited.insert(current.clone());
            commits_in_tree.push(current.clone());

            let node = match commit_map.get(&current) {
                Some(n) => n,
                None => break,
            };

            // Find child commits
            let children = children_map.get(&current).cloned().unwrap_or_default();

            if children.len() == 1 {
                // Continue linear chain
                current = children[0].clone();
            } else if children.len() > 1 {
                // This is a merge point: multiple children (branches originating here)
                merge_points.push(MergeNode {
                    id: current.clone(),
                    parents: node.parents.clone(),
                    children: children.clone(),
                });
                // For each child except the first, start a new tree
                for child in children.iter().skip(1) {
                    if !visited.contains(child) {
                        let mut subtree_commits: Vec<String> = Vec::new();
                        let mut sub_current = child.clone();
                        loop {
                            if visited.contains(&sub_current) {
                                break;
                            }
                            visited.insert(sub_current.clone());
                            subtree_commits.push(sub_current.clone());
                            let sub_children = children_map.get(&sub_current).cloned().unwrap_or_default();
                            if sub_children.len() == 1 {
                                sub_current = sub_children[0].clone();
                            } else {
                                break;
                            }
                        }
                        if !subtree_commits.is_empty() {
                            let author = commit_map.get(&subtree_commits[0])
                                .map(|c| c.author.clone())
                                .unwrap_or_default();
                            trees.push(Tree {
                                root: subtree_commits[0].clone(),
                                commits: subtree_commits,
                                color: (0, 0, 0), // will be assigned later
                                author,
                            });
                        }
                    }
                }
                // Continue with the first child
                if !children.is_empty() {
                    current = children[0].clone();
                } else {
                    break;
                }
            } else {
                // No children: leaf
                break;
            }
        }

        if !commits_in_tree.is_empty() {
            let author = commit_map.get(&commits_in_tree[0])
                .map(|c| c.author.clone())
                .unwrap_or_default();
            trees.push(Tree {
                root: commits_in_tree[0].clone(),
                commits: commits_in_tree,
                color: (0, 0, 0),
                author,
            });
        }
    }

    // Assign colors to trees based on author (simple hash-based)
    let mut author_color_index: HashMap<String, usize> = HashMap::new();
    let color_palette: [(u8, u8, u8); 8] = [
        (34, 139, 34),   // forest green
        (139, 69, 19),   // saddle brown
        (70, 130, 180),  // steel blue
        (218, 165, 32),  // goldenrod
        (255, 69, 0),    // red-orange
        (75, 0, 130),    // indigo
        (0, 139, 139),   // dark cyan
        (139, 0, 139),   // dark magenta
    ];

    for tree in &mut trees {
        let count = author_color_index.len();
        let idx = *author_color_index.entry(tree.author.clone())
            .or_insert(count % color_palette.len());
        tree.color = color_palette[idx];
    }

    // Collect all merge commits that are not already in merge_points
    for node in commit_map.values() {
        if node.parents.len() > 1 {
            // This is a merge commit (multiple parents)
            if !merge_points.iter().any(|m| m.id == node.id) {
                let children = children_map.get(&node.id).cloned().unwrap_or_default();
                merge_points.push(MergeNode {
                    id: node.id.clone(),
                    parents: node.parents.clone(),
                    children,
                });
            }
        }
    }

    let forest = Forest {
        commit_map,
        trees,
        merge_points,
    };

    Ok(forest)
}
