use crate::{Forest, Tree, MergeNode, CommitNode};
use std::collections::HashMap;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::cursor;
use termion::clear;
use termion::color;
use std::io::{self, Write, stdin, stdout};

/// Represents the interactive view state
#[derive(Debug)]
pub struct InteractiveView {
    pub offset_x: f64,
    pub offset_y: f64,
    pub zoom: f64,
    pub selected_commit: Option<CommitNode>,
    pub selected_tree: Option<String>,
    pub info_visible: bool,
}

impl Default for InteractiveView {
    fn default() -> Self {
        InteractiveView {
            offset_x: 0.0,
            offset_y: 0.0,
            zoom: 1.0,
            selected_commit: None,
            selected_tree: None,
            info_visible: false,
        }
    }
}

/// Find the commit nearest to a screen position (leaf click)
pub fn find_commit_at_position(
    forest: &Forest,
    screen_x: u16,
    screen_y: u16,
    view: &InteractiveView,
    term_width: u16,
    term_height: u16,
) -> Option<CommitNode> {
    // Convert screen coordinates to normalized forest coordinates
    let (nx, ny) = screen_to_forest(screen_x, screen_y, view, term_width, term_height);
    // Search all commits for nearest within threshold
    let threshold = 0.02 / view.zoom;
    let mut best: Option<(CommitNode, f64)> = None;
    for tree in &forest.trees {
        let tree_x = tree_x_position(tree, forest, view.zoom);
        for commit_id in &tree.commits {
            if let Some(commit) = forest.commit_map.get(commit_id) {
                let cy = commit_y_position(commit, forest, view.zoom);
                let dx = nx - tree_x;
                let dy = ny - cy;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < threshold {
                    match &best {
                        Some((_, best_dist)) if dist < *best_dist => {
                            best = Some((commit.clone(), dist));
                        }
                        None => {
                            best = Some((commit.clone(), dist));
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    best.map(|(c, _)| c)
}

/// Convert screen coordinates to normalized forest coordinates (0..1)
fn screen_to_forest(
    screen_x: u16,
    screen_y: u16,
    view: &InteractiveView,
    term_width: u16,
    term_height: u16,
) -> (f64, f64) {
    let margin = 0.05;
    let usable_w = 1.0 - 2.0 * margin;
    let usable_h = 1.0 - 2.0 * margin;
    let fx = margin + (screen_x as f64 / term_width as f64) * usable_w;
    let fy = margin + (screen_y as f64 / term_height as f64) * usable_h;
    // Apply inverse zoom and offset
    let fx = (fx - 0.5) / view.zoom + 0.5 + view.offset_x;
    let fy = (fy - 0.5) / view.zoom + 0.5 + view.offset_y;
    (fx.clamp(0.0, 1.0), fy.clamp(0.0, 1.0))
}

/// Compute the x position of a tree center in normalized coordinates
fn tree_x_position(tree: &Tree, forest: &Forest, zoom: f64) -> f64 {
    let tree_count = forest.trees.len();
    if tree_count == 0 {
        return 0.5;
    }
    let spacing = 1.0 / (tree_count as f64 + 1.0);
    let idx = forest.trees.iter().position(|t| t.root == tree.root).unwrap_or(0);
    let base = spacing * (idx as f64 + 1.0);
    let root_node = forest.commit_map.get(&tree.root);
    let jitter = root_node.map_or(0.0, |c| (c.time as f64 % 1000.0) / 10000.0);
    (base + jitter * 0.1).clamp(0.02, 0.98)
}

/// Compute the y position of a commit (normalized)
fn commit_y_position(commit: &CommitNode, forest: &Forest, zoom: f64) -> f64 {
    // Simple: commits are ordered by time, normalized to 0..1
    let times: Vec<i64> = forest.commit_map.values().map(|c| c.time).collect();
    if times.is_empty() {
        return 0.5;
    }
    let min_time = *times.iter().min().unwrap_or(&0);
    let max_time = *times.iter().max().unwrap_or(&1);
    if max_time == min_time {
        return 0.5;
    }
    let t = (commit.time - min_time) as f64 / (max_time - min_time) as f64;
    t
}

/// Display commit info popup in the terminal
pub fn show_commit_info(commit: &CommitNode, stdout: &mut dyn Write) -> io::Result<()> {
    let info = format!(
        "Commit: {} | Author: {} | Time: {} | Message: {}",
        &commit.id[..8.min(commit.id.len())],
        commit.author,
        commit.time,
        commit.message.lines().next().unwrap_or("")
    );
    write!(stdout, "{}{}", cursor::Goto(1, 1), clear::CurrentLine)?;
    write!(stdout, "{}{}", color::Fg(color::Yellow), info)?;
    write!(stdout, "{}", color::Fg(color::Reset))?;
    stdout.flush()?;
    Ok(())
}

/// Clear info line
pub fn clear_info_line(stdout: &mut dyn Write) -> io::Result<()> {
    write!(stdout, "{}{}", cursor::Goto(1, 1), clear::CurrentLine)?;
    stdout.flush()?;
    Ok(())
}
