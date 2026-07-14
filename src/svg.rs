use crate::{Forest, Tree, MergeNode, CommitNode, LayoutResult};
use std::collections::HashMap;

/// Generate an SVG representation of the forest.
/// Returns the SVG string with proper dimensions and styling.
pub fn render_svg(forest: &Forest, layout: &LayoutResult) -> String {
    let width = 800.0;
    let height = 600.0;
    let padding = 50.0;
    let draw_width = width - 2.0 * padding;
    let draw_height = height - 2.0 * padding;

    let mut svg = String::new();
    svg.push_str(&format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}">
  <rect width="{w}" height="{h}" fill="#f0f5f0" rx="10"/>
  <g transform="translate({px}, {py})">
"##,
        w = width,
        h = height,
        px = padding,
        py = padding
    ));

    // Draw merge nodes (roots) first
    for (merge_id, pos) in &layout.merge_positions {
        let x = pos.0 * draw_width;
        let y = (1.0 - pos.1) * draw_height; // flip y so 0 is bottom
        svg.push_str(&format!(
            "    <circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"6\" fill=\"#8b4513\" stroke=\"#5c2d0a\" stroke-width=\"1.5\"/>\n",
            x, y
        ));
    }

    // Draw trees
    for tree in &forest.trees {
        let commits = &tree.commits;
        if commits.is_empty() {
            continue;
        }
        let color_str = format!("#{:02x}{:02x}{:02x}", tree.color.0, tree.color.1, tree.color.2);
        // Get positions for this tree's commits
        let mut points: Vec<(f64, f64)> = Vec::new();
        for cid in commits {
            if let Some(pos) = layout.positions.get(cid) {
                let x = pos.0 * draw_width;
                let y = (1.0 - pos.1) * draw_height;
                points.push((x, y));
            }
        }
        if points.is_empty() {
            continue;
        }
        // Sort by y (descending, so root at top? Actually we want trunk from bottom to top)
        // We'll draw from bottom (largest y) to top (smallest y)
        points.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Draw trunk as a thick line
        if points.len() >= 2 {
            let trunk_points: Vec<String> = points
                .iter()
                .map(|(x, y)| format!("{:.1},{:.1}", x, y))
                .collect::<Vec<_>>()
                .join(" ");
            svg.push_str(&format!(
                "    <polyline points=\"{}\" stroke=\"{}\" stroke-width=\"4\" fill=\"none\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>\n",
                trunk_points, color_str
            ));
        }

        // Draw leaves (commits) as circles with varying radius based on commit frequency
        // We'll use commit count on this branch as leaf density proxy
        let leaf_radius = 3.0 + (commits.len() as f64 * 0.5).min(8.0);
        for (x, y) in &points {
            svg.push_str(&format!(
                "    <circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" fill=\"{}\" stroke=\"#333\" stroke-width=\"1\" opacity=\"0.9\"/>\n",
                x, y, leaf_radius, color_str
            ));
        }
    }

    // Draw merge connections as curved lines (root systems)
    // For each merge node, connect to its parent commits
    for (merge_id, pos) in &layout.merge_positions {
        let mx = pos.0 * draw_width;
        let my = (1.0 - pos.1) * draw_height;
        if let Some(merge_node) = forest.merge_map.get(merge_id) {
            for parent_id in &merge_node.parents {
                if let Some(parent_pos) = layout.positions.get(parent_id) {
                    let px = parent_pos.0 * draw_width;
                    let py = (1.0 - parent_pos.1) * draw_height;
                    // Draw a bezier curve connecting merge node to parent
                    let ctrl_x = (mx + px) / 2.0;
                    let ctrl_y = (my + py) / 2.0 - 30.0; // pull up to create arch
                    svg.push_str(&format!(
                        "    <path d=\"M {:.1} {:.1} Q {:.1} {:.1} {:.1} {:.1}\" stroke=\"#8b4513\" stroke-width=\"2\" fill=\"none\" stroke-dasharray=\"4,2\" opacity=\"0.7\"/>\n",
                        mx, my, ctrl_x, ctrl_y, px, py
                    ));
                }
            }
        }
    }

    svg.push_str("  </g>\n");
    svg.push_str(&format!(
        "  <text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#666\" text-anchor=\"middle\">Git Forest - {} trees, {} merges</text>\n",
        width / 2.0,
        height - 10.0,
        forest.trees.len(),
        forest.merge_map.len()
    ));
    svg.push_str("</svg>");
    svg
}

/// Write SVG to a file path.
pub fn export_svg(forest: &Forest, layout: &LayoutResult, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let svg = render_svg(forest, layout);
    std::fs::write(path, svg.as_bytes())?;
    Ok(())
}
