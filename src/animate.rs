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
) -> io::Result<String> {
    let mut output = String::new();
    let now = Instant::now();
    let dt = now.duration_since(state.last_frame).as_secs_f64();
    state.last_frame = now;

    // Update growth
    if state.growth < 1.0 {
        state.growth += dt * 0.3; // grow over ~3 seconds
        if state.growth > 1.0 {
            state.growth = 1.0;
        }
    }

    // Update season (cycle every 10 seconds)
    state.sway_time += dt;
    let season_duration = 10.0;
    let cycle = (state.sway_time / season_duration) as u8 % 4;
    state.season = cycle;

    // Generate sway amplitudes for each tree
    if state.sway_amplitudes.len() != layout.trees.len() {
        state.sway_amplitudes = layout.trees.iter().map(|_| rand::random::<f64>() * 2.0).collect();
    }

    // Build frame
    output.push_str(&format!("{}", termion::clear::All));
    output.push_str(&format!("{}", termion::cursor::Goto(1, 1)));

    // Render trees with growth and sway
    for (i, tree) in layout.trees.iter().enumerate() {
        let base_x = tree.position.0 as f64;
        let base_y = tree.position.1 as f64;
        let sway = if state.growth > 0.5 {
            (state.sway_time * 1.5 + state.sway_amplitudes[i]).sin() * 0.5
        } else {
            0.0
        };
        let visible_height = (tree.height as f64 * state.growth) as usize;
        let trunk_lines = render_tree_trunk(tree, visible_height, sway, state.season);
        for line in trunk_lines {
            output.push_str(&format!("{}\
", line));
        }
    }

    // Render commits as leaves (with season color)
    for commit_node in &layout.commit_nodes {
        let x = commit_node.position.0 as f64;
        let y = commit_node.position.1 as f64;
        let growth_factor = if commit_node.position.1 as f64 <= layout.max_y as f64 * state.growth {
            1.0
        } else {
            0.0
        };
        if growth_factor > 0.0 {
            let leaf_char = match state.season {
                0 => '@', // spring: fresh leaves
                1 => '%', // summer: dense
                2 => '&', // autumn: turning
                _ => '.', // winter: bare
            };
            let color_code = match state.season {
                0 => color::Fg(color::Green).to_string(),
                1 => color::Fg(color::LightGreen).to_string(),
                2 => color::Fg(color::Red).to_string(),
                _ => color::Fg(color::White).to_string(),
            };
            output.push_str(&format!(
                "{}{}{}",
                termion::cursor::Goto(x as u16 + 1, y as u16 + 1),
                color_code,
                leaf_char
            ));
        }
    }

    // Render merge storms as tangled roots (only when fully grown)
    if state.growth >= 1.0 {
        for storm in &layout.merge_storms {
            let color_code = color::Fg(color::Magenta).to_string();
            for merge in &storm.merges {
                let x = merge.position.0 as u16;
                let y = merge.position.1 as u16;
                output.push_str(&format!(
                    "{}{}{}",
                    termion::cursor::Goto(x + 1, y + 1),
                    color_code,
                    '#'
                ));
            }
        }
    }

    // Add info overlay
    output.push_str(&format!(
        "{}{}growth: {:.0}% | season: {}",
        termion::cursor::Goto(1, 1),
        color::Fg(color::Yellow),
        state.growth * 100.0,
        match state.season {
            0 => "spring",
            1 => "summer",
            2 => "autumn",
            _ => "winter",
        }
    ));

    Ok(output)
}

/// Render the trunk of a tree with taper effect
fn render_tree_trunk(tree: &Tree, visible_height: usize, sway: f64, season: u8) -> Vec<String> {
    let mut lines = Vec::new();
    let base_width = 3;
    let top_width = 1;
    let trunk_char = match season {
        0 => '|',
        1 => '║',
        2 => '╚',
        _ => '╙',
    };
    for level in 0..visible_height {
        let progress = level as f64 / visible_height.max(1) as f64;
        let width = (base_width as f64 * (1.0 - progress) + top_width as f64 * progress) as usize;
        let x_offset = (sway * progress * 2.0) as i32;
        let mut line = String::new();
        for w in 0..width {
            let pos = tree.position.0 as i32 + w as i32 + x_offset;
            if pos >= 0 {
                line.push(trunk_char);
            }
        }
        lines.push(line);
    }
    lines
}

/// Run the animation loop until user quits
pub fn run_animation(forest: &Forest, layout: &LayoutResult) -> io::Result<()> {
    let mut state = AnimationState::default();
    let mut stdout = stdout();
    write!(stdout, "{}", termion::raw::IntoRawMode::into_raw_mode)?;
    loop {
        let frame = render_animated_frame(forest, layout, &mut state)?;
        write!(stdout, "{}", frame)?;
        stdout.flush()?;
        thread::sleep(Duration::from_millis(50));
        if state.growth >= 1.0 && state.sway_time > 20.0 {
            break; // exit after 20 seconds of full growth
        }
    }
    write!(stdout, "{}", termion::clear::All)?;
    write!(stdout, "{}