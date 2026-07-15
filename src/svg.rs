use crate::{Forest, Tree, MergeNode, CommitNode, LayoutResult};
use std::collections::HashMap;

/// Export the forest as a static SVG string.
/// Renders trees with trunks and leaves, merge storms as tangled root systems.
pub fn export_svg(forest: &Forest, layout: &LayoutResult) -> String {
    let width = 800;
    let height = 600;
    let mut svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" width="{}" height="{}">"##,
        width, height, width, height
    );
    svg.push_str("<rect width='100%' height='100%' fill='#1a1a2e'/>");
    svg.push_str("<defs><style>text{font-family:monospace;font-size:10px;fill:#e0e0e0;}</style></defs>");

    // Collect all commit positions and authors
    let mut commit_info: Vec<(&CommitNode, f64, f64)> = Vec::new();
    for (id, pos) in &layout.positions {
        if let Some(node) = forest.commit_map.get(id) {
            let x = pos.0 * width as f64;
            let y = pos.1 * height as f64;
            commit_info.push((node, x, y));
        }
    }

    // Group commits by tree
    let mut tree_commits: HashMap<&String, Vec<(&CommitNode, f64, f64)>> = HashMap::new();
    for tree in &forest.trees {
        let mut commits = Vec::new();
        for (node, x, y) in &commit_info {
            // Check if this commit belongs to the tree by traversing parent chain
            if belongs_to_tree(node, tree, forest) {
                commits.push((*node, *x, *y));
            }
        }
        tree_commits.insert(&tree.root, commits);
    }

    // Render each tree as trunk + leaves
    for (root_id, commits) in &tree_commits {
        if commits.is_empty() {
            continue;
        }
        let trunk_x = commits[0].1;
        let trunk_top_y = commits.iter().map(|c| c.2).fold(f64::MAX, f64::min);
        let trunk_bottom_y = commits.iter().map(|c| c.2).fold(f64::MIN, f64::max);
        let trunk_color = "#5c4033"; // brown
        // Draw trunk
        svg.push_str(&format!(
            "<line x1='{}' y1='{}' x2='{}' y2='{}' stroke='{}' stroke-width='4' stroke-linecap='round'/>",
            trunk_x, trunk_top_y - 10.0, trunk_x, trunk_bottom_y + 10.0, trunk_color
        ));
        // Draw leaves
        for (node, x, y) in commits {
            let author_color = author_to_svg_color(&node.author);
            // Leaf density based on commit frequency (simulated: all leaves same size for now)
            let r = 4.0;
            svg.push_str(&format!(
                "<circle cx='{}' cy='{}' r='{}' fill='{}' opacity='0.8'/>",
                *x, *y, r, author_color
            ));
            // Add commit id label on hover
            svg.push_str(&format!(
                "<title>{}</title>",
                node.id
            ));
            // Add small commit hash text
            let short_hash = if node.id.len() > 7 { &node.id[..7] } else { &node.id };
            svg.push_str(&format!(
                "<text x='{}' y='{}' dx='6' dy='3'>{}</text>",
                *x, *y, short_hash
            ));
        }
    }

    // Render merge nodes as tangled root systems
    for (id, pos) in &layout.merge_positions {
        let x = pos.0 * width as f64;
        let y = pos.1 * height as f64;
        // Draw a small cluster of lines to represent tangled roots
        let root_count = 5;
        for i in 0..root_count {
            let angle = (i as f64 / root_count as f64) * std::f64::consts::PI * 2.0;
            let length = 15.0 + (i as f64 * 3.0);
            let x2 = x + angle.cos() * length;
            let y2 = y + angle.sin() * length;
            svg.push_str(&format!(
                "<line x1='{}' y1='{}' x2='{}' y2='{}' stroke='#8b4513' stroke-width='2' stroke-linecap='round' opacity='0.7'/>",
                x, y, x2, y2
            ));
        }
        // Add a small dot at center
        svg.push_str(&format!(
            "<circle cx='{}' cy='{}' r='3' fill='#cd853f'/>",
            x, y
        ));
        // Add merge node label
        svg.push_str(&format!(
            "<text x='{}' y='{}' dx='8' dy='0'>merge</text>",
            x, y
        ));
    }

    svg.push_str("</svg>");
    svg
}

/// Check if a commit belongs to a tree by traversing parents up to root.
fn belongs_to_tree(node: &CommitNode, tree: &Tree, forest: &Forest) -> bool {
    if node.id == tree.root {
        return true;
    }
    for parent_id in &node.parents {
        if let Some(parent_node) = forest.commit_map.get(parent_id) {
            if belongs_to_tree(parent_node, tree, forest) {
                return true;
            }
        }
    }
    false
}

/// Convert an author name to an SVG color string (simple hash-based).
fn author_to_svg_color(author: &str) -> String {
    let hash: u64 = author.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
    let r = (hash & 0xFF) as u8;
    let g = ((hash >> 8) & 0xFF) as u8;
    let b = ((hash >> 16) & 0xFF) as u8;
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}