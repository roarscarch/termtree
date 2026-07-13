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
    pub parents: Vec<String>,
    pub children: Vec<String>,
    pub author: String,
    pub time: i64,
    pub message: String,
}

/// Represents the visible viewport into the forest.
#[derive(Debug, Clone)]
pub struct Viewport {
    pub offset_x: usize,
    pub offset_y: usize,
    pub width: usize,
    pub height: usize,
    pub zoom: f64,
}

impl Viewport {
    pub fn new(width: usize, height: usize) -> Self {
        Viewport {
            offset_x: 0,
            offset_y: 0,
            width,
            height,
            zoom: 1.0,
        }
    }

    pub fn scroll(&mut self, dx: isize, dy: isize) {
        if dx > 0 {
            self.offset_x = self.offset_x.saturating_add(dx as usize);
        } else {
            self.offset_x = self.offset_x.saturating_sub((-dx) as usize);
        }
        if dy > 0 {
            self.offset_y = self.offset_y.saturating_add(dy as usize);
        } else {
            self.offset_y = self.offset_y.saturating_sub((-dy) as usize);
        }
    }

    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom * 1.2).min(5.0);
    }

    pub fn zoom_out(&mut self) {
        self.zoom = (self.zoom / 1.2).max(0.2);
    }
}

/// Load the git repository and build the forest.
pub fn load_forest(path: &str) -> Result<Forest, Box<dyn Error>> {
    let repo = Repository::open(path)?;
    let mut commit_map: HashMap<String, CommitNode> = HashMap::new();
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

    // Build trees from linear commit chains
    let mut visited: HashSet<String> = HashSet::new();
    let mut trees: Vec<Tree> = Vec::new();
    let mut merges: Vec<MergeNode> = Vec::new();
    let mut merge_ids: HashSet<String> = HashSet::new();

    // First pass: identify merge commits (more than one parent)
    for (id, node) in &commit_map {
        if node.parents.len() > 1 {
            merge_ids.insert(id.clone());
        }
    }

    // Second pass: build trees from non-merge commits
    for (id, node) in &commit_map {
        if visited.contains(id) {
            continue;
        }
        if merge_ids.contains(id) {
            continue;
        }

        // Walk backwards to find the root of this chain
        let mut chain = Vec::new();
        let mut current = id.clone();
        loop {
            if visited.contains(&current) {
                break;
            }
            visited.insert(current.clone());
            chain.push(current.clone());
            if let Some(node) = commit_map.get(&current) {
                if node.parents.len() == 1 && !merge_ids.contains(&node.parents[0]) {
                    current = node.parents[0].clone();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        if !chain.is_empty() {
            let author = commit_map.get(&chain[0]).map(|c| c.author.clone()).unwrap_or_default();
            let root = chain.last().cloned().unwrap_or_default();
            trees.push(Tree {
                root,
                commits: chain,
                color: simple_hash_color(&author),
                author,
            });
        }
    }

    // Build merge nodes
    for id in &merge_ids {
        if let Some(node) = commit_map.get(id) {
            merges.push(MergeNode {
                id: id.clone(),
                parents: node.parents.clone(),
                children: Vec::new(), // will be filled later
                author: node.author.clone(),
                time: node.time,
                message: node.message.clone(),
            });
        }
    }

    // Fill children for merges
    let mut children_map: HashMap<String, Vec<String>> = HashMap::new();
    for (_, node) in &commit_map {
        for parent in &node.parents {
            children_map.entry(parent.clone()).or_default().push(node.id.clone());
        }
    }
    for merge in &mut merges {
        if let Some(children) = children_map.get(&merge.id) {
            merge.children = children.clone();
        }
    }

    Ok(Forest {
        trees,
        merges,
        commit_map,
    })
}

/// Simple hash-based color generation.
fn simple_hash_color(s: &str) -> (u8, u8, u8) {
    let hash: u64 = s.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
    let r = (hash & 0xFF) as u8;
    let g = ((hash >> 8) & 0xFF) as u8;
    let b = ((hash >> 16) & 0xFF) as u8;
    // Ensure colors are bright enough
    let r = r.max(80);
    let g = g.max(80);
    let b = b.max(80);
    (r, g, b)
}