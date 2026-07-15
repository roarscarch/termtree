use crate::{Forest, Tree, LayoutResult, CommitNode};
use termion::color;
use std::collections::HashMap;

/// Renders the forest to a terminal string.
/// Each tree is drawn as a trunk (vertical lines) with leaves (commit dots) at positions.
pub fn render_forest(forest: &Forest, layout: &LayoutResult, width: u16, height: u16) -> String {
    let w = width as usize;
    let h = height as usize;
    if w == 0 || h == 0 {
        return String::new();
    }

    // Build a grid of characters (space by default)
    let mut grid: Vec<Vec<char>> = vec![vec![' '; w]; h];
    // Also store foreground color for each cell: (r, g, b) or None for default
    let mut colors: Vec<Vec<Option<(u8, u8, u8)>>> = vec![vec![None; w]; h];

    let tree_count = forest.trees.len();
    if tree_count == 0 {
        return String::new();
    }

    // Determine y-range of commits to map to grid rows
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    for (_, (_, y)) in &layout.positions {
        if *y < min_y { min_y = *y; }
        if *y > max_y { max_y = *y; }
    }
    for (_, (_, y)) in &layout.merge_positions {
        if *y < min_y { min_y = *y; }
        if *y > max_y { max_y = *y; }
    }
    if (max_y - min_y).abs() < 0.001 {
        max_y = min_y + 1.0;
    }

    // Map y from [min_y, max_y] to rows [0, h-1] (row 0 = top = max_y)
    let y_to_row = |y: f64| -> usize {
        let normalized = (y - min_y) / (max_y - min_y); // 0 at bottom, 1 at top
        let row = ((1.0 - normalized) * (h as f64 - 1.0)).round() as usize;
        row.min(h - 1)
    };

    // Map x from [0,1] to columns
    let x_to_col = |x: f64| -> usize {
        let col = (x * (w as f64 - 1.0)).round() as usize;
        col.min(w - 1)
    };

    // Draw trees: for each tree, draw trunk from root to topmost commit, then leaves
    for tree in &forest.trees {
        if tree.commits.is_empty() {
            continue;
        }
        // Collect positions of commits in this tree
        let mut commit_positions: Vec<(usize, usize)> = Vec::new();
        for cid in &tree.commits {
            if let Some(&(x, y)) = layout.positions.get(cid) {
                let row = y_to_row(y);
                let col = x_to_col(x);
                commit_positions.push((row, col));
            }
        }
        if commit_positions.is_empty() {
            continue;
        }
        // Sort by row (top to bottom)
        commit_positions.sort_by_key(|&(r, _)| r);

        // Find trunk column (use root commit column, or first commit column)
        let trunk_col = commit_positions[0].1;

        // Draw trunk from bottom row to topmost commit row
        let first_row = commit_positions[0].0;
        let last_row = commit_positions.last().unwrap().0;
        for row in first_row..=last_row {
            if row < h && trunk_col < w {
                let ch = grid[row][trunk_col];
                // Only draw if not already occupied by a leaf or trunk
                if ch == ' ' || ch == '│' || ch == '├' || ch == '┤' {
                    // Check if there is a commit at this (row, trunk_col)
                    let is_commit = commit_positions.iter().any(|&(r, c)| r == row && c == trunk_col);
                    if is_commit {
                        // Commit dot overrides trunk
                        grid[row][trunk_col] = '●';
                        colors[row][trunk_col] = Some(tree.color);
                    } else {
                        grid[row][trunk_col] = '│';
                        colors[row][trunk_col] = Some(tree.color);
                    }
                }
            }
        }

        // Draw leaves (commits) with their exact positions
        for &(row, col) in &commit_positions {
            if row < h && col < w {
                grid[row][col] = '●';
                colors[row][col] = Some(tree.color);
            }
        }
    }

    // Draw merge nodes
    for (_, &(x, y)) in &layout.merge_positions {
        let row = y_to_row(y);
        let col = x_to_col(x);
        if row < h && col < w {
            grid[row][col] = '◆';
            // Merge nodes get a default brownish color
            colors[row][col] = Some((139, 69, 19));
        }
    }

    // Build output string with ANSI color codes
    let mut output = String::new();
    for row in 0..h {
        for col in 0..w {
            let ch = grid[row][col];
            if let Some((r, g, b)) = colors[row][col] {
                output.push_str(&format!("{}{}{}", color::Fg(color::Rgb(r, g, b)), ch, color::Fg(color::Reset)));
            } else {
                output.push(ch);
            }
        }
        if row < h - 1 {
            output.push('\
');
        }
    }
    output
}

/// Render a single tree as a standalone ASCII art (for detail view).
pub fn render_tree(tree: &Tree, layout: &LayoutResult, width: u16, height: u16) -> String {
    let forest = Forest {
        trees: vec![tree.clone()],
        commit_map: HashMap::new(),
        merge_nodes: vec![],
    };
    // Build a layout that only includes positions for this tree's commits
    let mut positions = HashMap::new();
    for cid in &tree.commits {
        if let Some(pos) = layout.positions.get(cid) {
            positions.insert(cid.clone(), *pos);
        }
    }
    let sub_layout = LayoutResult {
        positions,
        merge_positions: HashMap::new(),
        tree_centers: layout.tree_centers.clone(),
    };
    render_forest(&forest, &sub_layout, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Forest, Tree, LayoutResult};
    use std::collections::HashMap;

    #[test]
    fn test_render_empty_forest() {
        let forest = Forest {
            trees: vec![],
            commit_map: HashMap::new(),
            merge_nodes: vec![],
        };
        let layout = LayoutResult {
            positions: HashMap::new(),
            merge_positions: HashMap::new(),
            tree_centers: vec![],
        };
        let result = render_forest(&forest, &layout, 10, 5);
        assert_eq!(result, "");
    }

    #[test]
    fn test_render_single_tree() {
        let tree = Tree {
            root: "r1".to_string(),
            commits: vec!["c1".to_string(), "c2".to_string()],
            color: (0, 255, 0),
            author: "test".to_string(),
        }