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
    author_colors: &std::collections::HashMap<String, (u8, u8, u8)>,
    anim: &AnimationState,
    layout: &LayoutResult,
) -> String {
    let mut output = String::new();
    output.push_str(&format!("{}{}", termion::clear::All, termion::cursor::Goto(1, 1)));
    
    // Title bar
    output.push_str(&format!(
        "{}╔══════════════════════ GIT FOREST ══════════════════════╗{}\r\
",
        color::Fg(color::Rgb(100, 200, 100)),
        color::Fg(color::Reset)
    ));
    
    // Season indicator
    let season_name = match anim.season {
        0 => "Spring 🌸",
        1 => "Summer ☀️",
        2 => "Autumn 🍂",
        3 => "Winter ❄️",
        _ => "Unknown",
    };
    output.push_str(&format!(
        "{}Season: {} | Growth: {:.0}%{}  \r\
",
        color::Fg(color::Rgb(255, 200, 100)),
        season_name,
        anim.growth * 100.0,
        color::Fg(color::Reset)
    ));
    
    // Draw each tree
    for (i, tree) in forest.trees.iter().enumerate() {
        let tree_lines = render_tree_with_growth(tree, anim, layout, i, author_colors);
        for line in &tree_lines {
            output.push_str(line);
            output.push_str("\r\
");
        }
    }
    
    // Draw merge root systems
    for merge in &forest.merges {
        let merge_lines = render_merge_with_growth(merge, anim, layout);
        for line in &merge_lines {
            output.push_str(line);
            output.push_str("\r\
");
        }
    }
    
    // Legend
    output.push_str(&format!(
        "{}\r\
{}Controls: ↑↓ scroll | +/- zoom | q quit | i inspect | a toggle animation{}\r\
",
        "─".repeat(60),
        color::Fg(color::Rgb(150, 150, 150)),
        color::Fg(color::Reset)
    ));
    
    output
}

/// Render a single tree with growth animation: trunks grow upward, leaves appear gradually.
fn render_tree_with_growth(
    tree: &Tree,
    anim: &AnimationState,
    layout: &LayoutResult,
    tree_index: usize,
    author_colors: &std::collections::HashMap<String, (u8, u8, u8)>,
) -> Vec<String> {
    // Determine how many lines of trunk to show based on growth
    let total_trunk_height = tree.trunk_height as f64;
    let visible_height = (total_trunk_height * anim.growth).ceil() as usize;
    
    // Sway offset for this tree
    let sway_amp = if tree_index < anim.sway_amplitudes.len() {
        anim.sway_amplitudes[tree_index]
    } else {
        0.0
    };
    let sway = (anim.sway_time * 2.0).sin() * sway_amp;
    
    // Determine color based on author
    let author_color = tree.commits.first()
        .and_then(|c| author_colors.get(&c.author))
        .copied()
        .unwrap_or((100, 180, 100));
    
    let mut lines = Vec::new();
    
    // Trunk (bottom to top)
    for y in 0..visible_height.min(tree.trunk_lines.len()) {
        let raw_line = &tree.trunk_lines[y];
        // Apply sway to trunk: shift characters horizontally
        let shift = (sway * (1.0 - (y as f64 / total_trunk_height))).round() as i32;
        let shifted_line = apply_horizontal_shift(raw_line, shift);
        
        // Color the trunk
        let colored = format!(
            "{}{}{}",
            color::Fg(color::Rgb(author_color.0, author_color.1, author_color.2)),
            shifted_line,
            color::Fg(color::Reset)
        );
        lines.push(colored);
    }
    
    // Leaves (topmost part, only appear after growth > 0.7)
    if anim.growth > 0.7 && !tree.leaf_lines.is_empty() {
        let leaf_alpha = ((anim.growth - 0.7) / 0.3).min(1.0);
        let leaf_count = (tree.leaf_lines.len() as f64 * leaf_alpha).ceil() as usize;
        
        // Season-based leaf color
        let leaf_color = match anim.season {
            0 => (120, 255, 120), // spring green
            1 => (50, 200, 50),   // summer green
            2 => (255, 160, 50),  // autumn orange
            3 => (200, 200, 220), // winter pale
            _ => (100, 255, 100),
        };
        
        for y in 0..leaf_count.min(tree.leaf_lines.len()) {
            let raw_line = &tree.leaf_lines[y];
            let shift = (sway * 0.3 * (1.0 - (y as f64 / tree.leaf_lines.len() as f64))).round() as i32;
            let shifted_line = apply_horizontal_shift(raw_line, shift);
            
            let colored = format!(
                "{}{}{}",
                color::Fg(color::Rgb(leaf_color.0, leaf_color.1, leaf_color.2)),
                shifted_line,
                color::Fg(color::Reset)
            );
            lines.push(colored);
        }
    }
    
    lines
}

/// Render merge nodes as root systems with growth animation.
fn render_merge_with_growth(
    merge: &MergeNode,
    anim: &AnimationState,
    layout: &LayoutResult,
) -> Vec<String> {
    let mut lines = Vec::new();
    
    // Merge roots appear once growth > 0.3
    if anim.growth <= 0.3 {
        return lines;
    }