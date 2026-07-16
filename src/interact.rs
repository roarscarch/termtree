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
    let (term_w, term_h) = termion::terminal_size().ok()?;
    // Convert screen coordinates to world coordinates
    let world_x = (screen_x as f64 - term_w as f64 / 2.0) / view.zoom + view.offset_x;
    let world_y = (screen_y as f64 - term_h as f64 / 2.0) / view.zoom + view.offset_y;

    // Search through all commits for the closest one within a radius
    let threshold = 3.0 / view.zoom; // click radius in world units
    let mut closest: Option<(f64, CommitNode)> = None;

    for tree in &forest.trees {
        for commit_id in &tree.commits {
            if let Some(pos) = forest.positions.get(commit_id) {
                let dx = pos.0 - world_x;
                let dy = pos.1 - world_y;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < threshold {
                    match &closest {
                        Some((best_dist, _)) if dist < *best_dist => {
                            if let Some(node) = forest.commit_map.get(commit_id) {
                                closest = Some((dist, node.clone()));
                            }
                        }
                        None => {
                            if let Some(node) = forest.commit_map.get(commit_id) {
                                closest = Some((dist, node.clone()));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    closest.map(|(_, node)| node)
}

/// Run the interactive viewer: handles keyboard input and renders the forest.
/// Returns when the user presses 'q' or Ctrl-C.
pub fn run_interactive(forest: &Forest) -> Result<(), Box<dyn std::error::Error>> {
    let mut view = InteractiveView::default();
    let mut stdout = stdout().into_raw_mode()?;
    let stdin = stdin();

    // Initial render
    render_interactive_frame(&mut stdout, forest, &view)?;

    // Handle input
    for c in stdin.keys() {
        match c? {
            Key::Char('q') | Key::Ctrl('c') => break,
            Key::Left => {
                view.offset_x -= 0.1 / view.zoom;
                render_interactive_frame(&mut stdout, forest, &view)?;
            }
            Key::Right => {
                view.offset_x += 0.1 / view.zoom;
                render_interactive_frame(&mut stdout, forest, &view)?;
            }
            Key::Up => {
                view.offset_y -= 0.1 / view.zoom;
                render_interactive_frame(&mut stdout, forest, &view)?;
            }
            Key::Down => {
                view.offset_y += 0.1 / view.zoom;
                render_interactive_frame(&mut stdout, forest, &view)?;
            }
            Key::Char('+') | Key::Char('=') => {
                view.zoom *= 1.2;
                render_interactive_frame(&mut stdout, forest, &view)?;
            }
            Key::Char('-') | Key::Char('_') => {
                view.zoom /= 1.2;
                if view.zoom < 0.1 {
                    view.zoom = 0.1;
                }
                render_interactive_frame(&mut stdout, forest, &view)?;
            }
            Key::Char('i') => {
                view.info_visible = !view.info_visible;
                render_interactive_frame(&mut stdout, forest, &view)?;
            }
            Key::Char(' ') => {
                // Toggle selection (could be used for clicking)
                // For now, select the first commit of the first tree as a demo
                if let Some(tree) = forest.trees.first() {
                    if let Some(first_id) = tree.commits.first() {
                        if let Some(node) = forest.commit_map.get(first_id) {
                            view.selected_commit = Some(node.clone());
                            view.selected_tree = Some(tree.name.clone());
                        }
                    }
                }
                render_interactive_frame(&mut stdout, forest, &view)?;
            }
            _ => {}
        }
    }

    // Reset terminal
    write!(stdout, "{}{}{}", clear::All, cursor::Show, cursor::Goto(1, 1))?;
    stdout.flush()?;
    Ok(())
}

/// Render the current interactive frame to the terminal.
fn render_interactive_frame(
    stdout: &mut dyn Write,
    forest: &Forest,
    view: &InteractiveView,
) -> Result<(), Box<dyn std::error::Error>> {
    let (term_w, term_h) = termion::terminal_size()?;

    // Clear screen and hide cursor
    write!(stdout, "{}{}{}", clear::All, cursor::Goto(1, 1), cursor::Hide)?;

    // Draw forest elements
    for tree in &forest.trees {
        // Draw trunk (vertical line)
        if let Some(root_pos) = forest.positions.get(&tree.root) {
            let screen_x = ((root_pos.0 - view.offset_x) * view.zoom + term_w as f64 / 2.0) as u16;
            let screen_y = ((root_pos.1 - view.offset_y) * view.zoom + term_h as f64 / 2.0) as u16;
            if screen_x > 0 && screen_x <= term_w && screen_y > 0 && screen_y <= term_h {
                write!(stdout, "{}", cursor::Goto(screen_x, screen_y))?;
                // Use a tree symbol
                if view.selected_tree.as_deref() == Some(&tree.name) {
                    write!(stdout, "{}♥{}", color::Fg(color::Red), color::Fg(color::Reset))?;
                } else {
                    write!(stdout, "♣")?;
                }
            }
        }

        // Draw leaves for each commit
        for commit_id in &tree.commits {
            if let Some(pos) = forest.positions.get(commit_id) {
                let screen_x = ((pos.0 - view.offset_x) * view.zoom + term_w as f64 / 2.0) as u16;
                let screen_y = ((pos.1 - view.offset_y) * view.zoom + term_h as f64 / 2.0) as u16;
                if screen_x > 0 && screen_x <= term_w && screen_y > 0 && screen_y <= term_h {
                    write!(stdout, "{}", cursor::Goto(screen_x, screen_y))?;
                    if let Some(node) = forest.commit_map.get(commit_id) {
                        // Color by author if available
                        if let Some(color) = forest.author_colors.get(&node.author) {
                            write!(stdout, "{}·{}