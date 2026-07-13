use git2::Repository;
use std::collections::{HashMap, HashSet, VecDeque};
use std::env;
use std::error::Error;

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

/// Load all commits from a git repository.
pub fn load_commits(repo_path: &str) -> Result<Vec<CommitNode>, Box<dyn Error>> {
    let repo = Repository::open(repo_path)?;
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(git2::Sort::TIME)?;

    let mut commits = Vec::new();
    for oid_result in revwalk {
        let oid = oid_result?;
        let commit = repo.find_commit(oid)?;
        let parents: Vec<String> = commit
            .parents()
            .map(|p| p.id().to_string())
            .collect();
        let node = CommitNode {
            id: oid.to_string(),
            author: commit.author().name().unwrap_or("unknown").to_string(),
            time: commit.time().seconds(),
            message: commit.message().unwrap_or("").to_string(),
            parents,
        };
        commits.push(node);
    }
    Ok(commits)
}

/// Build a forest from a list of commits.
pub fn build_forest(commits: &[CommitNode]) -> Forest {
    let mut commit_map: HashMap<String, CommitNode> = HashMap::new();
    for c in commits {
        commit_map.insert(c.id.clone(), c.clone());
    }

    // Identify root commits (no parents) and merge commits (multiple parents).
    let mut roots: Vec<String> = Vec::new();
    let mut merges: Vec<MergeNode> = Vec::new();
    for c in commits {
        if c.parents.is_empty() {
            roots.push(c.id.clone());
        } else if c.parents.len() > 1 {
            merges.push(MergeNode {
                id: c.id.clone(),
                parent_ids: c.parents.clone(),
                child_ids: Vec::new(),
            });
        }
    }

    // Build adjacency from children to parents.
    let mut children_of: HashMap<String, Vec<String>> = HashMap::new();
    for c in commits {
        for p in &c.parents {
            children_of.entry(p.clone()).or_default().push(c.id.clone());
        }
    }
    // For merges, also record children.
    for m in &mut merges {
        if let Some(children) = children_of.get(&m.id) {
            m.child_ids = children.clone();
        }
    }

    // Simple greedy tree extraction: start from each root, follow first-parent chain.
    let mut trees: Vec<Tree> = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();
    // Assign a simple color per author (hash-based).
    let mut author_color: HashMap<String, (u8, u8, u8)> = HashMap::new();
    let mut color_idx = 0u32;
    for c in commits {
        author_color.entry(c.author.clone()).or_insert_with(|| {
            let idx = color_idx;
            color_idx = color_idx.wrapping_add(1);
            let r = ((idx * 37) % 256) as u8;
            let g = ((idx * 79) % 256) as u8;
            let b = ((idx * 131) % 256) as u8;
            (r, g, b)
        });
    }

    for root in &roots {
        if visited.contains(root) {
            continue;
        }
        let mut chain = Vec::new();
        let mut current = Some(root.clone());
        while let Some(id) = current {
            if visited.contains(&id) {
                break;
            }
            visited.insert(id.clone());
            chain.push(id.clone());
            let node = &commit_map[&id];
            // Follow first parent unless it leads to a merge or is already visited.
            if let Some(first_parent) = node.parents.first() {
                if !visited.contains(first_parent) {
                    let parent_node = &commit_map[first_parent];
                    if parent_node.parents.len() <= 1 {
                        current = Some(first_parent.clone());
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        if !chain.is_empty() {
            let author = commit_map[&chain[0]].author.clone();
            let color = author_color[&author];
            trees.push(Tree {
                root: root.clone(),
                commits: chain,
                color,
                author,
            });
        }
    }

    Forest {
        trees,
        merges,
        commit_map,
    }
}

/// Render the forest as a simple ASCII diagram.
pub fn render_forest(forest: &Forest) -> String {
    let mut output = String::new();
    output.push_str("🌳 Git Forest\
");
    output.push_str(&format!("{} trees, {} merges\
", forest.trees.len(), forest.merges.len()));
    output.push_str("\
");
    for (i, tree) in forest.trees.iter().enumerate() {
        output.push_str(&format!("Tree {} ({}):\
", i + 1, tree.author));
        for (j, commit_id) in tree.commits.iter().enumerate() {
            let node = &forest.commit_map[commit_id];
            let indent = if j == 0 { "" } else { "  " };
            let prefix = if j == 0 { "🌱 " } else { "🌿 " };
            let short = &node.id[..7];
            output.push_str(&format!("{}{}{} {}\
", indent, prefix, short, node.message.lines().next().unwrap_or("")));
        }
        output.push_str("\
");
    }
    if !forest.merges.is_empty() {
        output.push_str("Merge points:\
");
        for m in &forest.merges {
            let short = &m.id[..7];
            output.push_str(&format!("  🌲 {} (parents: {})\
", short, m.parent_ids.len()));
        }
    }
    output
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let repo_path = if args.len() > 1 {
        &args[1]
    } else {
        "."
    };
    println!("Loading git forest from: {}", repo_path);
    let commits = load_commits(repo_path)?;
    println!("Loaded {}