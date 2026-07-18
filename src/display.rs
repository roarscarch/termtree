use crate::{Forest, Tree, CommitNode, MergeNode, LayoutResult};
use crate::color::assign_author_colors;
use crate::interact::{InteractiveView, find_commit_at_position};
use crate::inspect::format_commit_details;
use crate::render::render_forest;
use std::collections::HashMap;
use std::io::{self, Write, stdout};
use termion::{clear, cursor, color, terminal_size};
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;

/// Main display loop for interactive forest viewer.
pub fn run_display(forest: &Forest, layout: &LayoutResult) -> io::Result<()> {
    let mut view = InteractiveView::default();
    let author_colors = assign_author_colors(forest);
    let stdin = stdin();
    let mut stdout = stdout().into_raw_mode()?;

    // Initial render
    render_frame(&mut stdout, forest, layout, &view, &author_colors)?;

    // Event loop
    for key in stdin.keys() {
        match key? {
            Key::Char('q') | Key::Esc => break,
            Key::Left => view.offset_x -= 1.0,
            Key::Right => view.offset_x += 1.0,
            Key::Up => view.offset_y -= 1.0,
            Key::Down => view.offset_y += 1.0,
            Key::Char('+') | Key::Char('=') => view.zoom *= 1.1,
            Key::Char('-') | Key::Char('_') => view.zoom /= 1.1,
            Key::Char('i') => view.info_visible = !view.info_visible,
            Key::Char('r') => {
                view.offset_x = 0.0;
                view.offset_y = 0.0;
                view.zoom = 1.0;
                view.selected_commit = None;
                view.selected_tree = None;
                view.info_visible = false;
            }
            _ => {}
        }
        render_frame(&mut stdout, forest, layout, &view, &author_colors)?;
    }

    // Restore terminal
    write!(stdout, "{}", cursor::Show)?;
    stdout.flush()?;
    Ok(())
}

/// Render a single frame to the terminal.
pub fn render_frame(
    stdout: &mut impl Write,
    forest: &Forest,
    layout: &LayoutResult,
    view: &InteractiveView,
    author_colors: &HashMap<String, (u8, u8, u8)>,
) -> io::Result<()> {
    let (term_width, term_height) = terminal_size()?;
    let width = term_width as f64;
    let height = term_height as f64;

    // Clear screen and move cursor home
    write!(stdout, "{}{}", clear::All, cursor::Goto(1, 1))?;

    // Draw the forest
    let rendered = render_forest(forest, layout, view, author_colors);
    for (y, line) in rendered.lines().enumerate() {
        if y >= term_height as usize {
            break;
        }
        write!(stdout, "{}{}", cursor::Goto(1, (y + 1) as u16), line)?;
    }

    // Draw info panel if visible
    if view.info_visible {
        if let Some(ref commit) = view.selected_commit {
            let details = format_commit_details(forest, commit, author_colors);
            // Draw at bottom-right corner
            let panel_x = width as u16 - 45;
            let panel_y = height as u16 - 15;
            write!(stdout, "{}", cursor::Goto(panel_x, panel_y))?;
            for (i, line) in details.lines().enumerate() {
                if i >= 14 {
                    break;
                }
                write!(stdout, "{}{}", cursor::Goto(panel_x, panel_y + i as u16), line)?;
            }
        }
    }

    // Draw status bar
    let status = format!(
        " Forest: {} trees, {} commits | Zoom: {:.2}x | Offset: ({:.0}, {:.0}) | [arrows: move, +/-: zoom, i: info, r: reset, q: quit]",
        forest.trees.len(),
        forest.commit_map.len(),
        view.zoom,
        view.offset_x,
        view.offset_y
    );
    write!(
        stdout,
        "{}{}{}",
        cursor::Goto(1, term_height),
        color::Fg(color::Rgb(100, 200, 100)),
        &status[..status.len().min(term_width as usize - 1)]
    )?;
    write!(stdout, "{}", color::Fg(color::Reset))?;

    stdout.flush()?;
    Ok(())
}