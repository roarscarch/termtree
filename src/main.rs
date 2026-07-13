use git2::Repository;
use std::collections::{HashMap, HashSet, VecDeque};
use std::env;
use std::error::Error;
use std::io::{self, Write, stdin, stdout};
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::cursor;
use termion::clear;
use termion::color;

/// A single commit in our forest representation.
#[derive(Debug, Clone)]
pub struct CommitNode {
    pub id: String,
    pub author: String,
    pub time: i64,
    pub message: String,
    pub parents: Vec<String>,
}

/// A tree represents a linear branch (a chain of commits with no merges).
#[derive(Debug, Clone)]
pub struct Tree {
    pub root: String,
    pub commits: Vec<String>,
    pub color: (u8, u8, u8),
    pub author: String,
}

/// The entire forest – a collection of trees and merge points.
#[derive(Debug)]
pub struct Forest {
    pub trees: Vec<Tree>,
    pub merges: Vec<MergeNode>,
    pub commit_map: HashMap<String, CommitNode>,
}

/// A merge node – where multiple trees join.
#[derive(Debug, Clone)]
pub struct MergeNode {
    pub id: String,
    pub parent_ids: Vec<String>,
    pub child_ids: Vec<String>,
}

/// Load the git repository and build the forest structure.
pub fn load_forest(repo_path: &str) -> Result<Forest, Box<dyn Error>> {
    let repo = Repository::open(repo_path)?;
    let mut commit_map = HashMap::new();
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(git2::Sort::TIME)?;

    for oid_result in revwalk {
        let oid = oid_result?;
        let commit = repo.find_commit(oid)?;
        let id = oid.to_string();
        let author = commit.author().name().unwrap_or("unknown").to_string();
        let time = commit.time().seconds();
        let message = commit.message().unwrap_or("").to_string();
        let parents: Vec<String> = commit.parents().map(|p| p.id().to_string()).collect();
        commit_map.insert(
            id.clone(),
            CommitNode {
                id,
                author,
                time,
                message,
                parents,
            },
        );
    }

    // Build trees and merges
    let mut trees = Vec::new();
    let mut merges = Vec::new();
    let mut visited = HashSet::new();
    let mut in_tree = HashSet::new();

    // Collect all commit ids sorted by time
    let mut all_ids: Vec<String> = commit_map.keys().cloned().collect();
    all_ids.sort_by(|a, b| {
        let ca = &commit_map[a];
        let cb = &commit_map[b];
        cb.time.cmp(&ca.time)
    });

    for id in &all_ids {
        if visited.contains(id) {
            continue;
        }
        let commit = &commit_map[id];
        if commit.parents.len() > 1 {
            // This is a merge commit itself – treat as merge node
            let child_ids = find_children(id, &commit_map, &in_tree);
            merges.push(MergeNode {
                id: id.clone(),
                parent_ids: commit.parents.clone(),
                child_ids,
            });
            visited.insert(id.clone());
            continue;
        }
        // Start a new tree from this commit (it's a leaf or start of a branch)
        let mut branch_commits = Vec::new();
        let mut current = id.clone();
        loop {
            if visited.contains(&current) {
                break;
            }
            visited.insert(current.clone());
            in_tree.insert(current.clone());
            branch_commits.push(current.clone());
            let node = &commit_map[&current];
            if node.parents.is_empty() {
                break;
            }
            let parent = &node.parents[0];
            if !commit_map.contains_key(parent) {
                break;
            }
            let parent_commit = &commit_map[parent];
            if parent_commit.parents.len() > 1 {
                // Parent is a merge – stop here, parent will be a merge node
                break;
            }
            current = parent.clone();
        }
        if !branch_commits.is_empty() {
            let author = commit_map[&branch_commits[0]].author.clone();
            let color = simple_hash_color(&author);
            trees.push(Tree {
                root: branch_commits[0].clone(),
                commits: branch_commits,
                color,
                author,
            });
        }
    }

    // Find any remaining merge nodes (commits with >1 parent not yet visited)
    for id in &all_ids {
        if visited.contains(id) {
            continue;
        }
        let commit = &commit_map[id];
        if commit.parents.len() > 1 {
            let child_ids = find_children(id, &commit_map, &in_tree);
            merges.push(MergeNode {
                id: id.clone(),
                parent_ids: commit.parents.clone(),
                child_ids,
            });
            visited.insert(id.clone());
        }
    }

    Ok(Forest {
        trees,
        merges,
        commit_map,
    })
}

/// Find child commits of a given commit id (commits that list this as parent).
fn find_children(
    id: &str,
    commit_map: &HashMap<String, CommitNode>,
    in_tree: &HashSet<String>,
) -> Vec<String> {
    let mut children = Vec::new();
    for (_, node) in commit_map {
        if node.parents.contains(&id.to_string()) && !in_tree.contains(&node.id) {
            children.push(node.id.clone());
        }
    }
    children
}

/// Simple hash-based color assignment for an author string.
pub fn simple_hash_color(author: &str) -> (u8, u8, u8) {
    let hash: u32 = author.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
    let r = ((hash >> 16) & 0xFF) as u8;
    let g = ((hash >> 8) & 0xFF) as u8;
    let b = (hash & 0xFF) as u8;
    // Ensure reasonable brightness
    let brightness = (r as u16 + g as u16 + b as u16) / 3;
    if brightness < 60 {
        (r + 60, g + 60, b + 60)
    } else {
        (r, g, b)
    }
}

/// Render the forest as ASCII art.
pub fn render_forest(forest: &Forest) -> String {
    let mut output = String::new();
    output.push_str(&format!("\x1b[1mGit Forest: {} trees, {} merges\x1b[0m\
\
", forest.trees.len(), forest.merges.len()));

    // Assign a column for each tree
    let mut tree_col = HashMap::new();
    for (i, tree) in forest.trees.iter().enumerate() {
        tree_col.insert(tree.root.clone(), i);
    }

    // Draw trees
    for (i, tree) in forest.trees.iter().enumerate() {
        let col = i;
        let (r, g, b) = tree.color;
        let color_code = format!("\x1b[38;2;{};{};{}