use crate::{Forest, Tree, CommitNode, MergeNode, LayoutResult};
use std::collections::HashMap;

/// Viewport state for interactive forest exploration.
#[derive(Debug, Clone)]
pub struct InteractiveView {
    /// Horizontal offset (in character cells)
    pub offset_x: f64,
    /// Vertical offset (in character cells)
    pub offset_y: f64,
    /// Zoom factor (1.0 = default)
    pub zoom: f64,
    /// Currently selected commit hash, if any
    pub selected_commit: Option<String>,
    /// Currently selected tree index, if any
    pub selected_tree: Option<usize>,
    /// Whether info panel is visible
    pub info_visible: bool,
    /// Whether autorotate is enabled (slowly pans the view)
    pub autorotate: bool,
    /// Autorotate angle in radians (used for circular panning)
    pub autorotate_angle: f64,
    /// Automatically zoom to fit the entire forest
    pub zoom_to_fit: bool,
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
            autorotate: false,
            autorotate_angle: 0.0,
            zoom_to_fit: true,
        }
    }
}

/// Find the commit at a given (x, y) position in the terminal.
/// Returns the commit hash if a leaf or trunk cell is within a threshold.
pub fn find_commit_at_position(
    x: u16,
    y: u16,
    forest: &Forest,
    layout: &LayoutResult,
    view: &InteractiveView,
) -> Option<String> {
    // Convert terminal coordinates to forest coordinates
    let fx = (x as f64 - view.offset_x) / view.zoom;
    let fy = (y as f64 - view.offset_y) / view.zoom;
    let threshold = 2.0 / view.zoom;

    // Search through all trees
    for (tree_idx, tree) in forest.trees.iter().enumerate() {
        // Check commit nodes
        for commit in &tree.commits {
            if let Some(ref pos) = layout.positions.get(&commit.hash) {
                let dx = fx - pos.x;
                let dy = fy - pos.y;
                if (dx * dx + dy * dy) < threshold * threshold {
                    return Some(commit.hash.clone());
                }
            }
        }
        // Check merge nodes
        if let Some(ref merge_node) = layout.merge_nodes.get(tree_idx) {
            let dx = fx - merge_node.x;
            let dy = fy - merge_node.y;
            if (dx * dx + dy * dy) < threshold * threshold {
                return Some(merge_node.hash.clone());
            }
        }
    }
    None
}

/// Compute a zoom factor that fits the entire forest within the terminal dimensions.
pub fn compute_fit_zoom(
    terminal_width: u16,
    terminal_height: u16,
    forest: &Forest,
    layout: &LayoutResult,
) -> f64 {
    if forest.trees.is_empty() {
        return 1.0;
    }

    // Find bounding box of all positions
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;

    for tree in &forest.trees {
        for commit in &tree.commits {
            if let Some(pos) = layout.positions.get(&commit.hash) {
                min_x = min_x.min(pos.x);
                max_x = max_x.max(pos.x);
                min_y = min_y.min(pos.y);
                max_y = max_y.max(pos.y);
            }
        }
    }
    for merge_node in layout.merge_nodes.values() {
        min_x = min_x.min(merge_node.x);
        max_x = max_x.max(merge_node.x);
        min_y = min_y.min(merge_node.y);
        max_y = max_y.max(merge_node.y);
    }

    let forest_width = (max_x - min_x).max(1.0);
    let forest_height = (max_y - min_y).max(1.0);

    let zoom_x = (terminal_width as f64 - 4.0) / forest_width; // leave margin
    let zoom_y = (terminal_height as f64 - 4.0) / forest_height;
    zoom_x.min(zoom_y).max(0.1) // clamp to avoid extreme zoom
}

/// Update the viewport for autorotate: slowly pan in a circular pattern.
pub fn update_autorotate(view: &mut InteractiveView, terminal_width: u16, terminal_height: u16) {
    if !view.autorotate {
        return;
    }
    // Increment angle
    view.autorotate_angle += 0.002; // radians per frame
    if view.autorotate_angle > std::f64::consts::TAU {
        view.autorotate_angle -= std::f64::consts::TAU;
    }
    // Compute offset as a circle around the center
    let radius = (terminal_width as f64).min(terminal_height as f64) * 0.15;
    let center_x = 0.0;
    let center_y = 0.0;
    view.offset_x = center_x + radius * view.autorotate_angle.cos();
    view.offset_y = center_y + radius * view.autorotate_angle.sin();
}

/// Apply zoom-to-fit: compute zoom and offset to center the forest.
pub fn apply_zoom_to_fit(
    view: &mut InteractiveView,
    terminal_width: u16,
    terminal_height: u16,
    forest: &Forest,
    layout: &LayoutResult,
) {
    if !view.zoom_to_fit {
        return;
    }
    let zoom = compute_fit_zoom(terminal_width, terminal_height, forest, layout);
    view.zoom = zoom;
    // Center the forest
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    for tree in &forest.trees {
        for commit in &tree.commits {
            if let Some(pos) = layout.positions.get(&commit.hash) {
                min_x = min_x.min(pos.x);
                max_x = max_x.max(pos.x);
                min_y = min_y.min(pos.y);
                max_y = max_y.max(pos.y);
            }
        }
    }
    for merge_node in layout.merge_nodes.values() {
        min_x = min_x.min(merge_node.x);
        max_x = max_x.max(merge_node.x);
        min_y = min_y.min(merge_node.y);
        max_y = max_y.max(merge_node.y);
    }
    let center_x = (min_x + max_x) / 2.0;
    let center_y = (min_y + max_y) / 2.0;
    let term_center_x = terminal_width as f64 / 2.0;
    let term_center_y = terminal_height as f64 / 2.0;
    view.offset_x = term_center_x - center_x * zoom;
    view.offset_y = term_center_y - center_y * zoom;
    // Reset flag so it only runs once
    view.zoom_to_fit = false;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_commit_at_position_none() {
        let forest = Forest { trees: vec![] };
        let layout = LayoutResult {
            positions: HashMap::new(),
            merge_nodes: HashMap::new(),
            merge_storms: vec![],
        }