use std::collections::HashMap;

/// Represents a single commit in the git history.
#[derive(Debug, Clone)]
pub struct CommitNode {
    pub id: String,
    pub author: String,
    pub message: String,
    pub time: i64,
    pub parent_ids: Vec<String>,
    pub children_ids: Vec<String>,
    pub branch: Option<String>,
}

/// Represents a tree (long-lived branch) in the forest.
#[derive(Debug, Clone)]
pub struct Tree {
    pub root: String,
    pub tip: String,
    pub branch: String,
    pub commits: Vec<String>,
    pub author: String,
    pub commit_count: u64,
}

/// Represents a merge node where two or more branches join.
#[derive(Debug, Clone)]
pub struct MergeNode {
    pub id: String,
    pub parent_ids: Vec<String>,
    pub child_ids: Vec<String>,
    pub commit: CommitNode,
    pub degree: usize,
}

/// The entire forest structure built from a git repository.
#[derive(Debug, Clone)]
pub struct Forest {
    pub trees: Vec<Tree>,
    pub merge_nodes: Vec<MergeNode>,
    pub commit_map: HashMap<String, CommitNode>,
    pub branch_map: HashMap<String, Vec<String>>,
}

impl Forest {
    pub fn new() -> Self {
        Forest {
            trees: Vec::new(),
            merge_nodes: Vec::new(),
            commit_map: HashMap::new(),
            branch_map: HashMap::new(),
        }
    }
}

impl Default for Forest {
    fn default() -> Self {
        Self::new()
    }
}
