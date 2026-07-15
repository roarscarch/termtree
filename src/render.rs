use crate::{Forest, Tree, CommitNode, MergeNode, LayoutResult};
use termion::color;
use std::collections::HashMap;

/// Render the forest as ASCII art for the terminal.
pub fn render_forest(forest: &Forest, layout: &LayoutResult, width: u16, height: u16, offset_x: f64, offset_y: f64, zoom: f64) -> String {
    let mut output = String::new();
    let tree_count = forest.trees.len();
    if tree_count == 0 {
        return output;
    }

    // Prepare a grid of characters
    let grid_rows = height as usize;
    let grid_cols = width as usize;
    let mut grid: Vec<Vec<char>> = vec![vec![' '; grid_cols]; grid_rows];

    // Draw trees
    for tree in &forest.trees {
        let tree_index = forest.trees.iter().position(|t| t.root == tree.root).unwrap_or(0);
        let center_x = if tree_index < layout.tree_centers.len() {
            layout.tree_centers[tree_index]
        } else {
            0.5
        };
        // Convert normalized x to screen x
        let screen_x = ((center_x * width as f64) as i32 + offset_x as i32) as usize;
        if screen_x >= grid_cols {
            continue;
        }

        // Draw trunk from bottom to top
        let trunk_length = tree.commits.len().min(grid_rows);
        for i in 0..trunk_length {
            let row = grid_rows - 1 - i;
            if row < grid_rows && screen_x < grid_cols {
                grid[row][screen_x] = '|';
            }
        }

        // Draw leaves (commits) along the trunk
        for (i, commit_id) in tree.commits.iter().enumerate() {
            let row = grid_rows - 1 - i;
            if row < grid_rows && screen_x < grid_cols {
                // Leaf density: if multiple commits near same row, cluster leaves
                let leaf_char = if i % 3 == 0 { '*' } else { '.' };
                grid[row][screen_x] = leaf_char;
            }
        }
    }

    // Draw merge nodes
    for (merge_id, merge_node) in &forest.merge_map {
        if let Some(&(mx, my)) = layout.merge_positions.get(merge_id) {
            let screen_x = ((mx * width as f64) as i32 + offset_x as i32) as usize;
            let screen_y = ((my * height as f64) as i32 + offset_y as i32) as usize;
            if screen_x < grid_cols && screen_y < grid_rows {
                // Merge node symbol: '#' for root systems
                grid[screen_y][screen_x] = '#';
            }
        }
    }

    // Convert grid to string
    for row in &grid {
        let line: String = row.iter().collect();
        output.push_str(&line);
        output.push('\n');
    }

    output
}

/// Render a colored version of the forest (each tree gets author color).
pub fn render_colored_forest(forest: &Forest, layout: &LayoutResult, width: u16, height: u16, offset_x: f64, offset_y: f64, zoom: f64) -> String {
    let mut output = String::new();
    let tree_count = forest.trees.len();
    if tree_count == 0 {
        return output;
    }

    let grid_rows = height as usize;
    let grid_cols = width as usize;
    // Each cell stores (char, color_tuple)
    let mut grid: Vec<Vec<(char, Option<(u8,u8,u8)>)>> = vec![vec![(' ', None); grid_cols]; grid_rows];

    for tree in &forest.trees {
        let tree_index = forest.trees.iter().position(|t| t.root == tree.root).unwrap_or(0);
        let center_x = if tree_index < layout.tree_centers.len() {
            layout.tree_centers[tree_index]
        } else {
            0.5
        };
        let screen_x = ((center_x * width as f64) as i32 + offset_x as i32) as usize;
        if screen_x >= grid_cols {
            continue;
        }

        let color_tuple = tree.color;

        let trunk_length = tree.commits.len().min(grid_rows);
        for i in 0..trunk_length {
            let row = grid_rows - 1 - i;
            if row < grid_rows && screen_x < grid_cols {
                grid[row][screen_x] = ('|', Some(color_tuple));
            }
        }

        for (i, commit_id) in tree.commits.iter().enumerate() {
            let row = grid_rows - 1 - i;
            if row < grid_rows && screen_x < grid_cols {
                let leaf_char = if i % 3 == 0 { '*' } else { '.' };
                grid[row][screen_x] = (leaf_char, Some(color_tuple));
            }
        }
    }

    // Draw merge nodes with a default color (white/gray)
    for (merge_id, merge_node) in &forest.merge_map {
        if let Some(&(mx, my)) = layout.merge_positions.get(merge_id) {
            let screen_x = ((mx * width as f64) as i32 + offset_x as i32) as usize;
            let screen_y = ((my * height as f64) as i32 + offset_y as i32) as usize;
            if screen_x < grid_cols && screen_y < grid_rows {
                grid[screen_y][screen_x] = ('#', Some((128, 128, 128)));
            }
        }
    }

    // Build output with ANSI color codes
    for row in &grid {
        let mut line = String::new();
        let mut current_color: Option<(u8,u8,u8)> = None;
        for (ch, color_opt) in row {
            if let Some(c) = color_opt {
                if current_color != Some(*c) {
                    // Set color
                    let (r, g, b) = c;
                    line.push_str(&format!("\x1b[38;2;{};{};{}m", r, g, b));
                    current_color = Some(*c);
                }
            } else if current_color.is_some() {
                // Reset color
                line.push_str("\x1b[0m");
                current_color = None;
            }
            line.push(*ch);
        }
        if current_color.is_some() {
            line.push_str("\x1b[0m");
        }
        output.push_str(&line);
        output.push('\n');
    }

    output
}