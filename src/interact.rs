use crate::{Forest, Tree, CommitNode, MergeNode, LayoutResult};
use termion::{color, cursor};
use std::collections::HashMap;

/// View state for interactive forest browsing.
#[derive(Debug, Clone)]
pub struct InteractiveView {
    /// Horizontal scroll offset in grid units
    pub offset_x: f64,
    /// Vertical scroll offset in grid units
    pub offset_y: f64,
    /// Zoom level (1.0 = normal)
    pub zoom: f64,
    /// Currently selected commit id (if any)
    pub selected_commit: Option<String>,
    /// Currently selected tree index (if any)
    pub selected_tree: Option<usize>,
    /// Whether the info panel is visible
    pub info_visible: bool,
    /// Terminal dimensions cached for rendering
    pub term_width: u16,
    pub term_height: u16,
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
            term_width: 80,
            term_height: 24,
        }
    }
}

/// Find the commit at a given screen position.
/// Returns the commit id if one is found within a small radius.
pub fn find_commit_at_position(
    forest: &Forest,
    layout: &LayoutResult,
    view: &InteractiveView,
    screen_x: u16,
    screen_y: u16,
) -> Option<String> {
    // Convert screen coordinates to world coordinates
    let world_x = (screen_x as f64 / view.zoom) + view.offset_x;
    let world_y = (screen_y as f64 / view.zoom) + view.offset_y;

    // Check all commits in the forest
    for (commit_id, commit_node) in &forest.commit_map {
        if let Some(pos) = layout.grid_positions.get(commit_id) {
            let dx = pos.x - world_x;
            let dy = pos.y - world_y;
            let distance = (dx * dx + dy * dy).sqrt();
            // Within a small radius (0.5 grid units)
            if distance < 0.5 {
                return Some(commit_id.clone());
            }
        }
    }

    // Check merge nodes
    for merge_node in &forest.merge_nodes {
        if let Some(pos) = layout.grid_positions.get(&merge_node.id) {
            let dx = pos.x - world_x;
            let dy = pos.y - world_y;
            let distance = (dx * dx + dy * dy).sqrt();
            if distance < 0.5 {
                return Some(merge_node.id.clone());
            }
        }
    }

    None
}

/// Convert a screen coordinate to world coordinate given the view state.
pub fn screen_to_world(view: &InteractiveView, screen_x: f64, screen_y: f64) -> (f64, f64) {
    let world_x = (screen_x / view.zoom) + view.offset_x;
    let world_y = (screen_y / view.zoom) + view.offset_y;
    (world_x, world_y)
}

/// Convert a world coordinate back to screen coordinate.
pub fn world_to_screen(view: &InteractiveView, world_x: f64, world_y: f64) -> (f64, f64) {
    let screen_x = (world_x - view.offset_x) * view.zoom;
    let screen_y = (world_y - view.offset_y) * view.zoom;
    (screen_x, screen_y)
}

/// Compute the visible world rectangle based on the view state.
pub fn visible_world_rect(view: &InteractiveView) -> (f64, f64, f64, f64) {
    let left = view.offset_x;
    let top = view.offset_y;
    let right = left + (view.term_width as f64 / view.zoom);
    let bottom = top + (view.term_height as f64 / view.zoom);
    (left, top, right, bottom)
}

/// Render a simple cursor position indicator for debugging.
pub fn render_cursor_indicator(
    screen_x: u16,
    screen_y: u16,
    commit_id: Option<&str>,
) -> String {
    let mut output = String::new();
    output.push_str(&cursor::Goto(screen_x, screen_y).to_string());
    output.push_str(&color::Fg(color::LightYellow).to_string());
    if let Some(id) = commit_id {
        output.push_str(&format!("[{}]", &id[..id.len().min(7)]));
    } else {
        output.push_str("[   ]");
    }
    output.push_str(&color::Fg(color::Reset).to_string());
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_to_world_identity() {
        let view = InteractiveView::default();
        let (wx, wy) = screen_to_world(&view, 0.0, 0.0);
        assert!((wx - 0.0).abs() < 1e-6);
        assert!((wy - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_screen_to_world_with_offset() {
        let mut view = InteractiveView::default();
        view.offset_x = 5.0;
        view.offset_y = 10.0;
        let (wx, wy) = screen_to_world(&view, 0.0, 0.0);
        assert!((wx - 5.0).abs() < 1e-6);
        assert!((wy - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_screen_to_world_with_zoom() {
        let mut view = InteractiveView::default();
        view.zoom = 2.0;
        let (wx, wy) = screen_to_world(&view, 10.0, 20.0);
        assert!((wx - 5.0).abs() < 1e-6);
        assert!((wy - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_world_to_screen() {
        let mut view = InteractiveView::default();
        view.offset_x = 3.0;
        view.offset_y = 4.0;
        view.zoom = 0.5;
        let (sx, sy) = world_to_screen(&view, 7.0, 8.0);
        assert!((sx - 2.0).abs() < 1e-6);
        assert!((sy - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_visible_world_rect() {
        let mut view = InteractiveView::default();
        view.offset_x = 10.0;
        view.offset_y = 20.0;
        view.term_width = 80;
        view.term_height = 24;
        view.zoom = 2.0;
        let (left, top, right, bottom) = visible_world_rect(&view);
        assert!((left - 10.0).abs() < 1e-6);
        assert!((top - 20.0).abs() < 1e-6);
        assert!((right - 50.0).abs() < 1e-6);
        assert!((bottom - 32.0).abs() < 1e-6);
    }
}