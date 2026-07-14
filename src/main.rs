use clap::{App, Arg};
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

mod animate;
mod interact;
mod layout;
mod merge_storm;
mod svg;

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
    pub time: i64,
    pub author: String,
    pub message: String,
}

/// Supported output modes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputMode {
    Interactive,
    Animated,
    Svg,
    Static,
}

fn main() -> Result<(), Box<dyn Error>> {
    let matches = App::new("git-forest")
        .version("0.1.0")
        .author("Git Forest Team")
        .about("See your git history grow as an ASCII-art forest")
        .arg(
            Arg::with_name("path")
                .help("Path to the git repository (default: current directory)")
                .default_value(".")
                .index(1),
        )
        .arg(
            Arg::with_name("mode")
                .short("m")
                .long("mode")
                .help("Output mode: interactive, animated, svg, static")
                .possible_values(&["interactive", "animated", "svg", "static"])
                .default_value("interactive"),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .help("Output file for SVG export (only used in svg mode)")
                .default_value("forest.svg"),
        )
        .get_matches();

    let repo_path = matches.value_of("path").unwrap();
    let mode_str = matches.value_of("mode").unwrap();
    let output_path = matches.value_of("output").unwrap();

    let mode = match mode_str {
        "interactive" => OutputMode::Interactive,
        "animated" => OutputMode::Animated,
        "svg" => OutputMode::Svg,
        "static" => OutputMode::Static,
        _ => unreachable!(),
    };

    eprintln!("Opening repository at: {}", repo_path);
    let repo = Repository::open(repo_path)?;

    let forest = build_forest(&repo)?;
    eprintln!("Built forest with {} trees and {} merges", forest.trees.len(), forest.merges.len());

    match mode {
        OutputMode::Interactive => {
            eprintln!("Entering interactive mode. Use arrow keys to scroll, +/- to zoom, q to quit.");
            interact::run_interactive(&forest)?;
        }
        OutputMode::Animated => {
            eprintln!("Starting animated forest (press Ctrl+C to stop)...");
            animate::run_animation(&forest)?;
        }
        OutputMode::Svg => {
            eprintln!("Exporting forest to SVG: {}", output_path);
            let svg_content = svg::render_forest_svg(&forest)?;
            std::fs::write(output_path, svg_content)?;
            eprintln!("SVG saved to {}", output_path);
        }
        OutputMode::Static => {
            eprintln!("Rendering static ASCII forest...");
            let ascii = render_static(&forest);
            println!("{}", ascii);
        }
    }

    Ok(())
}

/// Build a Forest from a git2 Repository.
pub fn build_forest(repo: &Repository) -> Result<Forest, Box<dyn Error>> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(git2::Sort::TIME)?;

    let mut commit_map: HashMap<String, CommitNode> = HashMap::new();
    let mut parent_counts: HashMap<String, usize> = HashMap::new();

    for oid_result in revwalk {
        let oid = oid_result?;
        let commit = repo.find_commit(oid)?;
        let id = oid.to_string();
        let author = commit.author().name().unwrap_or("unknown").to_string();
        let time = commit.time().seconds();
        let message = commit.message().unwrap_or("").to_string();
        let parents: Vec<String> = commit.parents().map(|p| p.id().to_string()).collect();

        commit_map.insert(id.clone(), CommitNode {
            id: id.clone(),
            author,
            time,
            message,
            parents: parents.clone(),
        });

        *parent_counts.entry(id).or_insert(0) += 0;
        for p in &parents {
            *parent_counts.entry(p.clone()).or_insert(0) += 1;
        }
    }

    // Identify merge commits (more than one parent)
    let mut merges: Vec<MergeNode> = Vec::new();
    for (id, node) in &commit_map {
        if node.parents.len() > 1 {
            let children: Vec<String> = commit_map.values()
                .filter(|c| c.parents.contains(id))
                .map(|c| c.id.clone())
                .collect();
            merges.push(MergeNode {
                id: id.clone(),
                parents: node.parents.clone(),
                children,
                time: node.time,
                author: node.author.clone(),
                message: node.message.clone(),
            });
        }
    }

    // Build trees by following linear chains (no merges)
    let mut visited: HashSet<String> = HashSet::new();
    let mut trees: Vec<Tree> = Vec::new();

    // Start from root commits (no children or all children visited)
    let root_candidates: Vec<String> = commit_map.keys()
        .filter(|id| {
            // A root is a commit that has no children (leaf) or is the initial commit
            let has_children = commit_map.values().any(|c| c.parents.contains(id));
            !has_children || parent_counts.get(*id).copied().unwrap_or(0) == 0
        })
        .cloned()
        .collect();

    for root in &root_candidates {
        if visited.contains(root) {
            continue;
        }
        let mut commits: Vec<String> = Vec::new();
        let mut current = root.clone();
        loop {
            if visited.contains(&current) {
                break;
            }