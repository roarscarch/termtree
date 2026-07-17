use crate::{Forest, Tree, MergeNode, CommitNode, LayoutResult};
use termion::color;
use std::io::{self, Write, stdout};
use std::thread;
use std::time::{Duration, Instant};

/// Animation state for the forest
#[derive(Debug, Clone)]
pub struct AnimationState {
    /// Current growth progress (0.0 to 1.0)
    pub growth: f64,
    /// Season cycle (0..=3: spring, summer, autumn, winter)
    pub season: u8,
    /// Whether animation is running
    pub running: bool,
    /// Timestamp of last frame
    pub last_frame: Instant,
    /// Elapsed time for idle sway (in seconds)
    pub sway_time: f64,
    /// Sway amplitude per tree (index -> amplitude)
    pub sway_amplitudes: Vec<f64>,
}

impl Default for AnimationState {
    fn default() -> Self {
        AnimationState {
            growth: 0.0,
            season: 0,
            running: false,
            last_frame: Instant::now(),
            sway_time: 0.0,
            sway_amplitudes: Vec::new(),
        }
    }
}

/// Render a single frame of the forest with animation effects.
/// Trees grow from bottom to top, leaves change color with seasons.
pub fn render_animated_frame(
    forest: &Forest,
    layout: &LayoutResult,
    state: &mut AnimationState,
    width: u16,
    height: u16,
    delta: f64,
) -> String {
    state.sway_time += delta;
    let mut output = String::new();
    output.push_str(&format!("{}{}", termion::clear::All, termion::cursor::Goto(1, 1)));

    // background
    let (bg_r, bg_g, bg_b) = match state.season {
        0 => (135, 206, 235), // spring: sky blue
        1 => (100, 180, 255), // summer: deeper blue
        2 => (200, 180, 100), // autumn: golden
        3 => (220, 220, 255), // winter: pale
        _ => (135, 206, 235),
    };
    for y in 0..height {
        for x in 0..width {
            output.push_str(&format!(
                "{}{} ",
                color::Bg(color::Rgb(bg_r, bg_g, bg_b)),
                color::Fg(color::Reset)
            ));
        }
        if y < height - 1 {
            output.push_str("\r\
");
        }
    }
    output.push_str(&format!("{} ", color::Bg(color::Reset)));

    // Ensure sway amplitudes allocated
    let tree_count = forest.trees.len();
    if state.sway_amplitudes.len() != tree_count {
        state.sway_amplitudes = vec![0.0; tree_count];
        for i in 0..tree_count {
            state.sway_amplitudes[i] = 0.5 + (i as f64 * 0.3).fract(); // pseudo-random
        }
    }

    // Draw trees with sway
    for (tree_idx, tree) in forest.trees.iter().enumerate() {
        let sway_offset = (state.sway_time * 1.5 + state.sway_amplitudes[tree_idx] * 6.28).sin()
            * state.sway_amplitudes[tree_idx]
            * 2.0;
        let base_x = tree.x as f64 + sway_offset;
        let base_y = tree.y as f64;
        let trunk_height = (tree.branches.len() as f64) * 2.0;
        let visible_height = trunk_height * state.growth;
        let start_y = base_y.floor() as u16;
        let end_y = (base_y - visible_height).max(0.0).floor() as u16;

        // Draw trunk
        let trunk_char = '║';
        for y in end_y..start_y {
            if y < height {
                let x_pos = (base_x.round() as u16).min(width.saturating_sub(1));
                let (trunk_r, trunk_g, trunk_b) = (101, 67, 33); // brown
                output.push_str(&format!(
                    "{}{}{}",
                    termion::cursor::Goto(x_pos + 1, y + 1),
                    color::Fg(color::Rgb(trunk_r, trunk_g, trunk_b)),
                    trunk_char
                ));
            }
        }

        // Draw branches with sway
        for (branch_idx, branch) in tree.branches.iter().enumerate() {
            let branch_sway = (state.sway_time * 2.0 + branch_idx as f64 * 0.7).sin()
                * state.sway_amplitudes[tree_idx]
                * 1.5;
            let branch_x = base_x + branch.offset_x as f64 + branch_sway;
            let branch_y = base_y - (branch_idx as f64) * 2.0 - 1.0;
            if branch_y >= 0.0 && branch_y < height as f64 {
                let x_pos = (branch_x.round() as u16).min(width.saturating_sub(1));
                let y_pos = branch_y as u16;
                let branch_char = match branch.direction {
                    crate::BranchDirection::Left => '/',
                    crate::BranchDirection::Right => '\\',
                    crate::BranchDirection::Straight => '|',
                };
                let (branch_r, branch_g, branch_b) = (139, 90, 43);
                output.push_str(&format!(
                    "{}{}{}",
                    termion::cursor::Goto(x_pos + 1, y_pos + 1),
                    color::Fg(color::Rgb(branch_r, branch_g, branch_b)),
                    branch_char
                ));
            }
        }

        // Draw leaves with season color and leaf density
        let leaf_color = match state.season {
            0 => (34, 139, 34),   // spring green
            1 => (0, 200, 0),     // summer green
            2 => (255, 140, 0),   // autumn orange
            3 => (200, 200, 200), // winter gray
            _ => (34, 139, 34),
        };
        let leaf_density = (tree.commit_count as f64 / 50.0).min(1.0) * state.growth;
        let leaf_count = (leaf_density * 20.0) as u16;
        for i in 0..leaf_count {
            let leaf_angle = (state.sway_time * 3.0 + i as f64 * 1.1) * 0.3;
            let leaf_radius = 2.0 + (i as f64 * 0.5).fract() * 3.0;
            let leaf_x = base_x + leaf_angle.cos() * leaf_radius;
            let leaf_y = base_y - trunk_height * state.growth - 2.0 + leaf_angle.sin() * leaf_radius;
            if leaf_y >= 0.0 && leaf_y < height as f64 && leaf_x >= 0.0 && leaf_x < width as f64 {
                let x_pos = leaf_x as u16;
                let y_pos = leaf_y as u16;
                let leaf_char = if state.season == 3 { '.' } else { '*' };
                output.push_str(&format!(
                    "{}{}{}",
                    termion::cursor::Goto(x_pos + 1, y_pos + 1),
                    color::Fg(color::Rgb(leaf_color.0, leaf_color.1, leaf_color.2)),
                    leaf_char
                ));
            }
        }
    }