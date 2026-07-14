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
) -> Option<CommitNode> {
    // Convert screen coordinates to world coordinates
    let world_x = (screen_x as f64 - view.offset_x) / view.zoom;
    let world_y = (screen_y as f64 - view.offset_y) / view.zoom;

    // Search threshold (in world units)
    let threshold = 0.5 / view.zoom;

    let mut closest: Option<(f64, &CommitNode)> = None;

    // Check all commits
    for commit in forest.commit_map.values() {
        // For simplicity, we assume each commit has a stored position from layout
        // In a full implementation, we'd have a position map. Here we use a heuristic:
        // approximate position based on tree index and time ordering
        if let Some(pos) = approximate_commit_position(forest, commit) {
            let dx = pos.0 - world_x;
            let dy = pos.1 - world_y;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist < threshold {
                match closest {
                    None => closest = Some((dist, commit)),
                    Some((best_dist, _)) if dist < best_dist => closest = Some((dist, commit)),
                    _ => {}
                }
            }
        }
    }

    closest.map(|(_, c)| c.clone())
}

/// Approximate commit position based on tree index and time
fn approximate_commit_position(forest: &Forest, commit: &CommitNode) -> Option<(f64, f64)> {
    let tree_count = forest.trees.len();
    if tree_count == 0 {
        return None;
    }

    // Find which tree this commit belongs to
    for (i, tree) in forest.trees.iter().enumerate() {
        if tree.commits.contains(&commit.id) {
            let idx = tree.commits.iter().position(|id| id == &commit.id).unwrap_or(0);
            let x = (i as f64 + 0.5) / tree_count as f64;
            let y = idx as f64 / tree.commits.len().max(1) as f64;
            return Some((x, y));
        }
    }

    // Check merge nodes
    for merge in &forest.merges {
        if merge.id == commit.id {
            let x = 0.5;
            let y = 0.5;
            return Some((x, y));
        }
    }

    None
}

/// Render the commit detail panel on screen
pub fn render_commit_info<W: Write>(
    stdout: &mut W,
    commit: &CommitNode,
    tree: Option<&Tree>,
    x: u16,
    y: u16,
) -> io::Result<()> {
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!(" Commit: {}", &commit.id[..8.min(commit.id.len())]));
    lines.push(format!(" Author: {}", commit.author));
    lines.push(format!(" Date:   {}", commit.time));
    lines.push(format!(" Message: {}", commit.message));
    if let Some(t) = tree {
        lines.push(format!(" Branch: {}", t.author));
        lines.push(format!(" Color:  RGB({},{},{})", t.color.0, t.color.1, t.color.2));
    }
    if !commit.parents.is_empty() {
        lines.push(format!(" Parents: {}", commit.parents.join(", ")));
    }

    // Draw a bordered box
    let width = lines.iter().map(|l| l.len()).max().unwrap_or(20) + 4;
    let height = lines.len() + 2;

    // Ensure we don't go off screen
    let start_x = x.min(80u16.saturating_sub(width as u16));
    let start_y = y.min(24u16.saturating_sub(height as u16));

    write!(stdout, "{}", cursor::Goto(start_x, start_y))?;
    write!(stdout, "{}{}", color::Fg(color::White), color::Bg(color::Blue))?;
    for i in 0..height {
        write!(stdout, "{}", cursor::Goto(start_x, start_y + i as u16))?;
        if i == 0 || i == height - 1 {
            write!(stdout, "{}", " ".repeat(width))?;
        } else {
            let line = &lines[i - 1];
            write!(stdout, " {} {}", line, " ".repeat(width.saturating_sub(line.len() + 3)))?;
        }
    }
    write!(stdout, "{}{}", color::Fg(color::Reset), color::Bg(color::Reset))?;
    Ok(())
}

/// Run the interactive terminal session
pub fn run_interactive(forest: &Forest) -> io::Result<()> {
    let stdin = stdin();
    let mut stdout = stdout().into_raw_mode()?;
    let mut view = InteractiveView::default();

    write!(stdout, "{}", clear::All)?;
    loop {
        // Read a key
        let key = stdin.keys().next();
        match key {
            Some(Ok(Key::Char('q'))) => break,
            Some(Ok(Key::Left)) => view.offset_x -= 2.0,
            Some(Ok(Key::Right)) => view.offset_x += 2.0,
            Some(Ok(Key::Up)) => view.offset_y -= 2.0,
            Some(Ok(Key::Down)) => view.offset_y += 2.0,
            Some(Ok(Key::Char('+'))) => view.zoom *= 1.2,
            Some(Ok(Key::Char('-'))) => view.zoom /= 1.2,
            Some(Ok(Key::Char('i'))) => view.info_visible = !view.info_visible,
            Some(Ok(Key::Char(' '))) => {
                // Click on a leaf at center of screen
                let screen_center_x = 40;
                let screen_center_y = 12;
                if let Some(commit) = find_commit_at_position(forest, screen_center_x, screen_center_y, &view) {
                    view.selected_commit = Some(commit);
                    view.info_visible = true;
                }
            }
            _ => {}
        }

        // Render forest (placeholder)
        write!(stdout, "{}", cursor::Goto(1, 1))?;
        write!(stdout, "Forest view - Zoom: {:.2}, Offset: ({:.1}, {:.1}