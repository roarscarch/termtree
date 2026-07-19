use crate::{Forest, Tree, CommitNode, MergeNode, LayoutResult};
use crate::inspect::format_commit_details;
use std::collections::HashMap;

/// State for interactive viewport.
#[derive(Debug, Clone)]
pub struct InteractiveView {
    /// Horizontal scroll offset (in grid units)
    pub offset_x: f64,
    /// Vertical scroll offset
    pub offset_y: f64,
    /// Zoom factor (1.0 = default)
    pub zoom: f64,
    /// Currently selected commit hash, if any
    pub selected_commit: Option<String>,
    /// Currently selected tree index, if any
    pub selected_tree: Option<usize>,
    /// Whether commit info panel is visible
    pub info_visible: bool,
}

impl Default for InteractiveView {
    fn default() -> Self {
        Self {
            offset_x: 0.0,
            offset_y: 0.0,
            zoom: 1.0,
            selected_commit: None,
            selected_tree: None,
            info_visible: false,
        }
    }
}

/// Find a commit at the given (x, y) position in the layout grid.
/// Returns Some(commit_hash) if found, None otherwise.
pub fn find_commit_at_position(
    x: f64,
    y: f64,
    layout: &LayoutResult,
    view: &InteractiveView,
) -> Option<String> {
    let effective_x = (x + view.offset_x) / view.zoom;
    let effective_y = (y + view.offset_y) / view.zoom;
    // Iterate over all commit positions in layout and find closest within threshold
    for (hash, pos) in &layout.commit_positions {
        let dx = pos.x - effective_x;
        let dy = pos.y - effective_y;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist < 1.5 {
            return Some(hash.clone());
        }
    }
    None
}

/// Prepare commit info panel text for a selected commit.
pub fn get_commit_info_panel(
    forest: &Forest,
    commit_hash: &str,
) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(node) = forest.commit_map.get(commit_hash) {
        lines.push(format!("Commit: {}", &commit_hash[..8.min(commit_hash.len())]));
        lines.push(format!("Author: {}", node.author));
        lines.push(format!("Date:   {}", node.timestamp));
        lines.push(format!("Message: {}", node.message));
        lines.push(format!("Branch:  {}", node.branch));
        lines.push(format!("Children: {}", node.children.len()));
        lines.push(format!("Parents:  {}", node.parents.len()));
        lines.push(String::new());
        lines.push("---".to_string());
        lines.push("Press 'i' to toggle this panel.".to_string());
    } else {
        lines.push("Commit not found.".to_string());
    }
    lines
}

/// Apply zoom and pan to a layout position, returning screen coordinates.
pub fn world_to_screen(
    world_x: f64,
    world_y: f64,
    view: &InteractiveView,
) -> (f64, f64) {
    let screen_x = (world_x - view.offset_x) * view.zoom;
    let screen_y = (world_y - view.offset_y) * view.zoom;
    (screen_x, screen_y)
}

/// Apply inverse transformation for mouse clicks.
pub fn screen_to_world(
    screen_x: f64,
    screen_y: f64,
    view: &InteractiveView,
) -> (f64, f64) {
    let world_x = screen_x / view.zoom + view.offset_x;
    let world_y = screen_y / view.zoom + view.offset_y;
    (world_x, world_y)
}