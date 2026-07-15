use crate::{Forest, Tree, MergeNode, LayoutResult};
use termion::color;
use std::io::{self, Write, stdout};

/// Render the forest to a terminal string using ASCII art.
/// Trunks are drawn as vertical lines, branches as slashes, leaves as dots.
/// Merge nodes are drawn as asterisks.
pub fn render_forest_to_terminal(
    forest: &Forest,
    layout: &LayoutResult,
    width: u16,
    height: u16,
) -> String {
    let mut output = String::new();
    let w = width as usize;
    let h = height as usize;
    let mut grid: Vec<Vec<char>> = vec![vec![' '; w]; h];
    let mut colors: Vec<Vec<Option<(u8, u8, u8)>>> = vec![vec![None; w]; h];

    // Draw trees
    for tree in &forest.trees {
        if tree.commits.is_empty() {
            continue;
        }
        let center_x = layout
            .tree_centers
            .get(forest.trees.iter().position(|t| t.root == tree.root).unwrap_or(0))
            .copied()
            .unwrap_or(0.5);
        let x = (center_x * (w as f64 - 1.0)) as usize;
        // Draw trunk from bottom up
        let trunk_start = (h as f64 * 0.1) as usize;
        let trunk_end = (h as f64 * 0.8) as usize;
        for y in trunk_start..=trunk_end {
            if y < h && x < w {
                grid[y][x] = '|';
                colors[y][x] = Some(tree.color);
            }
        }
        // Draw leaves at top
        let leaf_y = trunk_start.saturating_sub(1);
        if leaf_y < h && x < w {
            grid[leaf_y][x] = '@';
            colors[leaf_y][x] = Some(tree.color);
        }
        // Draw leaves proportional to commit frequency
        let leaf_density = (tree.commits.len() as f64 / forest.trees.len().max(1) as f64).min(1.0);
        let leaf_count = (leaf_density * 5.0) as usize + 1;
        for i in 0..leaf_count {
            let leaf_x = x as isize + (i as isize - leaf_count as isize / 2);
            let leaf_y = trunk_end + 1 + i % 2;
            if leaf_x >= 0 && leaf_x < w as isize && leaf_y < h {
                grid[leaf_y][leaf_x as usize] = '.';
                colors[leaf_y][leaf_x as usize] = Some(tree.color);
            }
        }
    }

    // Draw merge nodes
    for merge in &forest.merges {
        if let Some(&(mx, my)) = layout.merge_positions.get(&merge.id) {
            let x = (mx * (w as f64 - 1.0)) as usize;
            let y = (my * (h as f64 - 1.0)) as usize;
            if y < h && x < w {
                grid[y][x] = '*';
                colors[y][x] = Some((255, 215, 0)); // gold for merge nodes
            }
        }
    }

    // Build output string with ANSI colors
    for y in 0..h {
        for x in 0..w {
            let ch = grid[y][x];
            if let Some((r, g, b)) = colors[y][x] {
                let color_str = format!("\x1b[38;2;{};{};{}m", r, g, b);
                output.push_str(&color_str);
                output.push(ch);
                output.push_str("\x1b[0m");
            } else {
                output.push(ch);
            }
        }
        if y < h - 1 {
            output.push('\n');
        }
    }
    output
}