use crate::{Forest, Tree, CommitNode, MergeNode, LayoutResult};
use std::collections::HashMap;

/// Generate an SVG representation of the forest.
pub fn forest_to_svg(
    forest: &Forest,
    layout: &LayoutResult,
    author_colors: &HashMap<String, (u8, u8, u8)>,
) -> String {
    let mut svg = String::new();
    let width = 800;
    let height = 600;
    let padding = 50.0;

    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" width="{}" height="{}">"#,
        width, height, width, height
    ));
    svg.push_str("<defs>");
    // Define leaf gradient
    svg.push_str(r#"<radialGradient id="leafGrad" cx="50%" cy="30%" r="50%">"#);
    svg.push_str(r#"<stop offset="0%" stop-color="#88cc88" stop-opacity="0.8"/>"#);
    svg.push_str(r#"<stop offset="100%" stop-color="#226622" stop-opacity="0.4"/>"#);
    svg.push_str("</radialGradient>");
    svg.push_str("</defs>");
    svg.push_str(&format!(
        r#"<rect width="{}" height="{}" fill="#1a1a2e"/>"#,
        width, height
    ));

    // Draw merge roots (tangled root systems)
    for node in &forest.merge_nodes {
        if let Some(pos) = layout.node_positions.get(&node.id) {
            let cx = padding + (pos.x as f64 / 100.0) * (width as f64 - 2.0 * padding);
            let cy = height as f64 - padding - (pos.y as f64 / 100.0) * (height as f64 - 2.0 * padding);
            // Draw tangled roots
            let root_count = node.parents.len().max(2);
            for i in 0..root_count {
                let angle = std::f64::consts::PI * 2.0 * (i as f64) / (root_count as f64);
                let r = 10.0 + (i as f64 * 5.0);
                let ex = cx + angle.cos() * r;
                let ey = cy + angle.sin() * r;
                svg.push_str(&format!(
                    r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#8B4513" stroke-width="2" opacity="0.6"/>"#,
                    cx, cy, ex, ey
                ));
                // Add a small knot at each root end
                svg.push_str(&format!(
                    r#"<circle cx="{:.1}" cy="{:.1}" r="2" fill="#A0522D" opacity="0.8"/>"#,
                    ex, ey
                ));
            }
            // Merge point
            svg.push_str(&format!(
                r#"<circle cx="{:.1}" cy="{:.1}" r="6" fill="#DAA520" stroke="#B8860B" stroke-width="2"/>"#,
                cx, cy
            ));
        }
    }

    // Draw trees (branches)
    for tree in &forest.trees {
        let trunk_length = tree.commit_count as f64 * 0.8;
        // Determine trunk start/end from layout
        if let Some(start_pos) = layout.node_positions.get(&tree.start_commit) {
            if let Some(end_pos) = layout.node_positions.get(&tree.end_commit) {
                let x1 = padding + (start_pos.x as f64 / 100.0) * (width as f64 - 2.0 * padding);
                let y1 = height as f64 - padding - (start_pos.y as f64 / 100.0) * (height as f64 - 2.0 * padding);
                let x2 = padding + (end_pos.x as f64 / 100.0) * (width as f64 - 2.0 * padding);
                let y2 = height as f64 - padding - (end_pos.y as f64 / 100.0) * (height as f64 - 2.0 * padding);

                // Trunk (tapered)
                let dx = x2 - x1;
                let dy = y2 - y1;
                let len = (dx * dx + dy * dy).sqrt();
                if len > 0.0 {
                    let nx = -dy / len;
                    let ny = dx / len;
                    let base_width = 4.0;
                    let tip_width = 1.5;
                    // Polygon for tapered trunk
                    let p1x = x1 + nx * base_width;
                    let p1y = y1 + ny * base_width;
                    let p2x = x1 - nx * base_width;
                    let p2y = y1 - ny * base_width;
                    let p3x = x2 - nx * tip_width;
                    let p3y = y2 - ny * tip_width;
                    let p4x = x2 + nx * tip_width;
                    let p4y = y2 + ny * tip_width;
                    let author_color = author_colors.get(&tree.author).copied().unwrap_or((100, 180, 100));
                    svg.push_str(&format!(
                        r#"<polygon points="{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}" fill="rgb({},{},{})" opacity="0.9" stroke="#333" stroke-width="0.5"/>"#,
                        p1x, p1y, p2x, p2y, p3x, p3y, p4x, p4y,
                        author_color.0, author_color.1, author_color.2
                    ));
                }

                // Leaves (commit nodes along trunk)
                for commit_id in &tree.commits {
                    if let Some(pos) = layout.node_positions.get(commit_id) {
                        let lx = padding + (pos.x as f64 / 100.0) * (width as f64 - 2.0 * padding);
                        let ly = height as f64 - padding - (pos.y as f64 / 100.0) * (height as f64 - 2.0 * padding);
                        // Leaf size proportional to commit frequency (default small)
                        let leaf_r = 3.0;
                        svg.push_str(&format!(
                            r#"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" fill="url(#leafGrad)" stroke="#44aa44" stroke-width="0.5"/>"#,
                            lx, ly, leaf_r
                        ));
                    }
                }
            }
        }
    }

    svg.push_str("</svg>");
    svg
}

/// Write SVG to file.
pub fn export_svg(forest: &Forest, layout: &LayoutResult, author_colors: &HashMap<String, (u8, u8, u8)>, path: &str) -> Result<(), String> {
    let svg_content = forest_to_svg(forest, layout, author_colors);
    std::fs::write(path, &svg_content).map_err(|e| format!("Failed to write SVG: {}", e))
}