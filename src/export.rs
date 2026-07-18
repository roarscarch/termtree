use crate::{Forest, Tree, CommitNode, MergeNode, LayoutResult};
use crate::color::assign_author_colors;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::Path;

/// Export the forest as a static SVG file for sharing.
/// The SVG preserves the organic tree shapes, branch colors, and merge storms.
pub fn export_svg(
    forest: &Forest,
    layout: &LayoutResult,
    output_path: &str,
    width: u32,
    height: u32,
) -> Result<(), Box<dyn Error>> {
    let author_colors = assign_author_colors(forest);
    let svg_content = generate_svg(forest, layout, &author_colors, width, height);
    fs::write(Path::new(output_path), svg_content)?;
    Ok(())
}

fn generate_svg(
    forest: &Forest,
    layout: &LayoutResult,
    author_colors: &HashMap<String, (u8, u8, u8)>,
    width: u32,
    height: u32,
) -> String {
    let mut svg = String::new();
    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" width="{}" height="{}">"#,
        width, height, width, height
    ));
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"#1a1a2e\"/>");

    // Draw merge storms as tangled root systems (darker, thicker lines)
    for storm in &layout.merge_storms {
        let color = format!("#{:02x}{:02x}{:02x}", 180, 80, 180); // purple for storms
        for merge in &storm.merges {
            if let Some(commit) = forest.commit_map.get(&merge.commit_id) {
                let (x1, y1) = (merge.x as f64 * width as f64 / 100.0, merge.y as f64 * height as f64 / 100.0);
                for parent_id in &commit.parents {
                    if let Some(parent) = forest.commit_map.get(parent_id) {
                        let (x2, y2) = (parent.x as f64 * width as f64 / 100.0, parent.y as f64 * height as f64 / 100.0);
                        svg.push_str(&format!(
                            r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="3" opacity="0.8"/>"#,
                            x1, y1, x2, y2, color
                        ));
                    }
                }
            }
        }
    }

    // Draw trees (branches as trunks with leaves)
    for tree in &layout.trees {
        // Determine trunk color based on primary author
        let primary_author = tree.branches.first().and_then(|b| b.commits.first()).map(|c| &c.author).unwrap_or(&"unknown".to_string());
        let trunk_color = author_colors.get(primary_author).copied().unwrap_or((100, 200, 100));
        let trunk_color_str = format!("#{:02x}{:02x}{:02x}", trunk_color.0, trunk_color.1, trunk_color.2);

        // Draw trunk (main branch line)
        for branch in &tree.branches {
            let sorted_commits = sort_commits_by_time(&branch.commits);
            for i in 0..sorted_commits.len().saturating_sub(1) {
                let c1 = &sorted_commits[i];
                let c2 = &sorted_commits[i + 1];
                let (x1, y1) = (c1.x as f64 * width as f64 / 100.0, c1.y as f64 * height as f64 / 100.0);
                let (x2, y2) = (c2.x as f64 * width as f64 / 100.0, c2.y as f64 * height as f64 / 100.0);
                svg.push_str(&format!(
                    r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="2" opacity="0.9"/>"#,
                    x1, y1, x2, y2, trunk_color_str
                ));
            }
        }

        // Draw leaves (commit nodes) with leaf density proportional to commit frequency
        let commit_count = tree.branches.iter().map(|b| b.commits.len()).sum::<usize>();
        let leaf_size = if commit_count > 100 { 6 } else if commit_count > 50 { 8 } else { 10 };
        for branch in &tree.branches {
            for commit in &branch.commits {
                let (cx, cy) = (commit.x as f64 * width as f64 / 100.0, commit.y as f64 * height as f64 / 100.0);
                let leaf_color = author_colors.get(&commit.author).copied().unwrap_or((200, 200, 200));
                let leaf_color_str = format!("#{:02x}{:02x}{:02x}", leaf_color.0, leaf_color.1, leaf_color.2);
                svg.push_str(&format!(
                    r#"<circle cx="{:.1}" cy="{:.1}" r="{}" fill="{}" opacity="0.8"/>"#,
                    cx, cy, leaf_size, leaf_color_str
                ));
            }
        }
    }

    // Draw remaining commits not in trees as isolated nodes
    for (commit_id, commit) in &forest.commit_map {
        if !layout.trees.iter().any(|t| t.branches.iter().any(|b| b.commits.iter().any(|c| c.id == *commit_id))) {
            let (cx, cy) = (commit.x as f64 * width as f64 / 100.0, commit.y as f64 * height as f64 / 100.0);
            let leaf_color = author_colors.get(&commit.author).copied().unwrap_or((200, 200, 200));
            let leaf_color_str = format!("#{:02x}{:02x}{:02x}", leaf_color.0, leaf_color.1, leaf_color.2);
            svg.push_str(&format!(
                r#"<circle cx="{:.1}" cy="{:.1}" r="5" fill="{}" opacity="0.6"/>"#,
                cx, cy, leaf_color_str
            ));
        }
    }

    svg.push_str("</svg>");
    svg
}

/// Helper to sort commits by timestamp (oldest first) for consistent trunk drawing.
fn sort_commits_by_time(commits: &[CommitNode]) -> Vec<CommitNode> {
    let mut sorted = commits.to_vec();
    sorted.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    sorted
}
