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
    author_colors: &HashMap<String, (u8, u8, u8)>,
) -> Option<CommitNode> {
    let world_x = (screen_x as f64 - view.offset_x) / view.zoom;
    let world_y = (screen_y as f64 - view.offset_y) / view.zoom;

    for (_, commit) in &forest.commit_map {
        let dx = commit.x - world_x;
        let dy = commit.y - world_y;
        let dist = (dx * dx + dy * dy).sqrt();
        // Click radius of 3 in world coordinates
        if dist < 3.0 {
            return Some(commit.clone());
        }
    }
    None
}

/// Run the interactive loop until user quits
pub fn run_interactive(
    forest: &Forest,
    author_colors: &HashMap<String, (u8, u8, u8)>,
) -> io::Result<()> {
    let stdout = stdout();
    let mut stdout = stdout.lock().into_raw_mode()?;
    let stdin = stdin();
    let mut keys = stdin.keys();

    let mut view = InteractiveView::default();

    write!(stdout, "{}{}", clear::All, cursor::Goto(1, 1))?;
    render_forest_interactive(&mut stdout, forest, &view, author_colors)?;
    stdout.flush()?;

    loop {
        if let Some(key) = keys.next() {
            match key? {
                Key::Char('q') => break,
                Key::Left => view.offset_x -= 5.0,
                Key::Right => view.offset_x += 5.0,
                Key::Up => view.offset_y -= 5.0,
                Key::Down => view.offset_y += 5.0,
                Key::Char('+') | Key::Char('=') => view.zoom *= 1.1,
                Key::Char('-') | Key::Char('_') => view.zoom /= 1.1,
                Key::Char('i') => view.info_visible = !view.info_visible,
                Key::Esc => break,
                _ => {}
            }

            write!(stdout, "{}{}", clear::All, cursor::Goto(1, 1))?;
            render_forest_interactive(&mut stdout, forest, &view, author_colors)?;
            stdout.flush()?;
        }
    }

    write!(stdout, "{}{}", clear::All, cursor::Goto(1, 1))?;
    writeln!(stdout, "Exited interactive mode.")?;
    stdout.flush()?;
    Ok(())
}

/// Render the forest to the terminal with current view state
fn render_forest_interactive<W: Write>(
    w: &mut W,
    forest: &Forest,
    view: &InteractiveView,
    author_colors: &HashMap<String, (u8, u8, u8)>,
) -> io::Result<()> {
    // Determine terminal size (approximate)
    let term_width = 80;
    let term_height = 24;

    // Draw trees
    for (tree_id, tree) in &forest.trees {
        let trunk_x = tree.trunk_x * view.zoom + view.offset_x;
        let trunk_y = tree.trunk_y * view.zoom + view.offset_y;

        // Draw trunk as vertical line
        for i in 0..tree.height as usize {
            let screen_x = trunk_x as u16;
            let screen_y = (trunk_y - i as f64 * view.zoom) as u16;
            if screen_x < term_width && screen_y < term_height {
                // Color by author of first commit in tree
                let author = &tree.commits.first().map(|c| c.author.clone()).unwrap_or_default();
                let color = author_colors.get(author).copied().unwrap_or((100, 180, 100));
                write!(
                    w,
                    "{}{}{}█{}",
                    cursor::Goto(screen_x + 1, screen_y + 1),
                    color::Fg(color::Rgb(color.0, color.1, color.2)),
                    color::Bg(color::Rgb(20, 40, 20)),
                    color::Fg(color::Reset)
                )?;
            }
        }

        // Draw leaves (commits) as colored dots along the trunk
        for (i, commit) in tree.commits.iter().enumerate() {
            let leaf_y = (trunk_y - i as f64 * view.zoom) as u16;
            let leaf_x = trunk_x as u16;
            if leaf_x < term_width && leaf_y < term_height {
                let color = author_colors.get(&commit.author).copied().unwrap_or((200, 200, 100));
                let symbol = if view.selected_commit.as_ref().map(|c| c.id == commit.id).unwrap_or(false) {
                    "*"
                } else {
                    "•"
                };
                write!(
                    w,
                    "{}{}{}{}{}",
                    cursor::Goto(leaf_x + 1, leaf_y + 1),
                    color::Fg(color::Rgb(color.0, color.1, color.2)),
                    color::Bg(color::Rgb(10, 30, 10)),
                    symbol,
                    color::Fg(color::Reset)
                )?;
            }
        }
    }

    // Draw merge roots
    for (_, merge) in &forest.merges {
        let mx = merge.x * view.zoom + view.offset_x;
        let my = merge.y * view.zoom + view.offset_y;
        let mx_u = mx as u16;
        let my_u = my as u16;
        if mx_u < term_width && my_u < term_height {
            write!(
                w,
                "{}{}⚡{}",
                cursor::Goto(mx_u + 1, my_u + 1),
                color::Fg(color::Rgb(255, 200, 50)),
                color::Fg(color::Reset)
            )?;
        }
    }

    // Draw info panel if visible
    if view.info_visible {
        if let Some(commit) = &view.selected_commit {
            let details = format_commit_details_interactive(commit, author_colors);
            let lines: Vec<&str> = details.lines().collect();
            for (i, line) in lines.iter().enumerate() {
                if i < term_height as usize {
                    write!(
                        w,
                        "{}{}",
                        cursor::Goto(1, (term_height - 5 + i as u16).min(term_height)),
                        line
                    )?;
                }
            }