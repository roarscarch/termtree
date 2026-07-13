use crate::{Forest, Tree, MergeNode};
use std::error::Error;
use std::fs::File;
use std::io::Write;

/// Export the forest as an SVG string.
pub fn export_forest_svg(forest: &Forest, width: u32, height: u32) -> Result<String, Box<dyn Error>> {
    let mut svg = String::new();
    svg.push_str(&format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}">"##,
        width, height
    ));
    svg.push_str("<style>\n");
    svg.push_str(".tree-trunk { fill: none; stroke-width: 3; }\n");
    svg.push_str(".tree-branch { fill: none; stroke-width: 1.5; }\n");
    svg.push_str(".merge-node { fill: #8B4513; stroke: #5C2E0A; stroke-width: 2; }\n");
    svg.push_str(".leaf { fill: #228B22; opacity: 0.8; }\n");
    svg.push_str("</style>\n");
    svg.push_str("<rect width='100%' height='100%' fill='#1a1a2e'/>\n");

    let margin = 50.0;
    let usable_width = width as f64 - 2.0 * margin;
    let usable_height = height as f64 - 2.0 * margin;

    let tree_count = forest.trees.len();
    if tree_count == 0 {
        svg.push_str("</svg>");
        return Ok(svg);
    }

    // Layout trees horizontally
    let spacing = usable_width / (tree_count as f64 + 1.0);
    for (i, tree) in forest.trees.iter().enumerate() {
        let x_center = margin + spacing * (i as f64 + 1.0);
        let trunk_top_y = margin;
        let trunk_bottom_y = margin + usable_height * 0.7;
        // Draw trunk
        let trunk_color = format!("#{:02x}{:02x}{:02x}", tree.color.0, tree.color.1, tree.color.2);
        svg.push_str(&format!(
            r##"<line x1="{}" y1="{}" x2="{}" y2="{}" class="tree-trunk" stroke="{}"/>"##,
            x_center, trunk_top_y, x_center, trunk_bottom_y, trunk_color
        ));
        // Draw branches (commits)
        let commit_count = tree.commits.len();
        if commit_count > 1 {
            let step = (trunk_bottom_y - trunk_top_y) / (commit_count as f64 - 1.0);
            for (j, _commit_id) in tree.commits.iter().enumerate() {
                let y = trunk_top_y + step * j as f64;
                // branch offset
                let offset = (j as f64 * 0.3).sin() * 15.0;
                svg.push_str(&format!(
                    r##"<line x1="{}" y1="{}" x2="{}" y2="{}" class="tree-branch" stroke="{}"/>"##,
                    x_center, y, x_center + offset, y - 5.0, trunk_color
                ));
                // leaf at end of branch
                svg.push_str(&format!(
                    r##"<circle cx="{}" cy="{}" r="3" class="leaf"/>"##,
                    x_center + offset, y - 5.0
                ));
            }
        }
    }

    // Draw merge nodes
    for merge in &forest.merges {
        // Place merge nodes in lower area
        let merge_y = margin + usable_height * 0.85;
        let merge_x = margin + usable_width * (merge.id.len() as f64 % 10.0) / 10.0; // simplistic placement
        svg.push_str(&format!(
            r##"<circle cx="{}" cy="{}" r="8" class="merge-node"/>"##,
            merge_x, merge_y
        ));
        // Draw lines from parent trees to merge
        for parent_id in &merge.parents {
            // Find parent tree x position
            if let Some(parent_tree) = forest.trees.iter().find(|t| t.root == *parent_id || t.commits.contains(parent_id)) {
                let parent_idx = forest.trees.iter().position(|t| t.root == parent_tree.root).unwrap();
                let parent_x = margin + spacing * (parent_idx as f64 + 1.0);
                let parent_y = margin + usable_height * 0.7;
                svg.push_str(&format!(
                    r##"<line x1="{}" y1="{}" x2="{}" y2="{}" class="tree-branch" stroke="#8B4513"/>"##,
                    parent_x, parent_y, merge_x, merge_y
                ));
            }
        }
    }

    svg.push_str("</svg>");
    Ok(svg)
}

/// Write the SVG to a file.
pub fn export_forest_to_file(forest: &Forest, path: &str) -> Result<(), Box<dyn Error>> {
    let svg_content = export_forest_svg(forest, 800, 600)?;
    let mut file = File::create(path)?;
    file.write_all(svg_content.as_bytes())?;
    Ok(())
}
