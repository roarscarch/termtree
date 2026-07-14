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
}

impl Default for AnimationState {
    fn default() -> Self {
        AnimationState {
            growth: 0.0,
            season: 0,
            running: false,
            last_frame: Instant::now(),
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
    // Advance growth
    state.growth = (state.growth + delta * 0.3).min(1.0);
    if state.growth >= 1.0 {
        state.growth = 0.0;
        state.season = (state.season + 1) % 4;
    }

    let mut output = String::new();
    let effective_height = (height as f64 * state.growth) as u16;
    let effective_height = effective_height.max(1);

    // Clear screen and move cursor home
    output.push_str(&format!("{}{}", termion::clear::All, termion::cursor::Goto(1, 1)));

    // Draw ground line
    output.push_str(&format!("{}{}\
", termion::cursor::Goto(1, effective_height + 1), "═".repeat(width as usize)));

    // For each tree, draw its trunk and canopy
    for (tree_idx, tree) in forest.trees.iter().enumerate() {
        let center_x = if tree_idx < layout.tree_centers.len() {
            layout.tree_centers[tree_idx] * width as f64
        } else {
            width as f64 / 2.0
        };
        let center_x = center_x as u16;
        let trunk_height = (effective_height as f64 * 0.6) as u16;
        let canopy_height = effective_height.saturating_sub(trunk_height).max(2);

        // Draw trunk (vertical line)
        let trunk_char = match state.season {
            1 => '║', // summer: thick
            3 => '║', // winter: bare
            _ => '│',
        };
        let trunk_color = match state.season {
            3 => color::Fg(color::White),
            _ => color::Fg(color::Rgb(tree.color.0, tree.color.1, tree.color.2)),
        };

        for y in (effective_height - trunk_height)..effective_height {
            if y < height && center_x < width {
                output.push_str(&format!(
                    "{}{}{}{}",
                    termion::cursor::Goto(center_x, y + 1),
                    trunk_color,
                    trunk_char,
                    color::Fg(color::Reset),
                ));
            }
        }

        // Draw canopy (triangular shape formed by leaves)
        let leaf_color = match state.season {
            0 => color::Fg(color::Green),   // spring
            1 => color::Fg(color::Rgb(0, 200, 0)), // summer
            2 => color::Fg(color::Rgb(255, 165, 0)), // autumn
            3 => color::Fg(color::White),    // winter (snow)
            _ => color::Fg(color::Green),
        };

        let mut leaf_density = tree.commits.len() as f64;
        // Increase leaf density for more active branches
        if leaf_density > 10.0 {
            leaf_density = 10.0;
        }
        let leaf_char = match state.season {
            3 => '*', // snowflake
            _ => '@',
        };

        for row in 0..canopy_height {
            let half_width = (canopy_height - row) / 2;
            let start_x = center_x.saturating_sub(half_width);
            let end_x = (center_x + half_width).min(width.saturating_sub(1));
            for x in start_x..=end_x {
                // Only place leaves with probability based on density
                if (x as f64 * leaf_density * 0.1).fract() < 0.5 {
                    let y = effective_height - trunk_height - row;
                    if y < height && x < width {
                        output.push_str(&format!(
                            "{}{}{}{}",
                            termion::cursor::Goto(x, y + 1),
                            leaf_color,
                            leaf_char,
                            color::Fg(color::Reset),
                        ));
                    }
                }
            }
        }
    }

    // Draw merge nodes as root systems (tangled lines below ground)
    for (merge_idx, merge) in forest.merges.iter().enumerate() {
        if merge_idx >= layout.merge_positions.len() {
            continue;
        }
        let (mx, my) = layout.merge_positions[merge_idx];
        let screen_x = (mx * width as f64) as u16;
        let screen_y = (my * effective_height as f64) as u16;
        if screen_x < width && screen_y < height {
            output.push_str(&format!(
                "{}{}{}{}",
                termion::cursor::Goto(screen_x, screen_y + 1),
                color::Fg(color::Rgb(139, 69, 19)),
                '&',
                color::Fg(color::Reset),
            ));
        }
    }

    // Draw season indicator at top
    let season_name = match state.season {
        0 => "Spring",
        1 => "Summer",
        2 => "Autumn",
        3 => "Winter",
        _ => "Unknown",
    };
    output.push_str(&format!(
        "{}{}Season: {} | Growth: {:.0}%{}",
        termion::cursor::Goto(1, 1),
        color::Fg(color::Yellow),
        season_name,
        state.growth * 100.0,
        color::Fg(color::Reset),
    ));

    output
}

/// Run the animation loop for a given number of frames (0 for infinite).
pub fn run_animation(
    forest: &Forest,
    layout: &LayoutResult,
    frames: u64,
) -> io::Result<()> {
    let mut state = AnimationState::default();
    let mut stdout = stdout();
    let (width, height) = termion::terminal_size()?;

    write!(stdout, "{}", termion::raw::IntoRawMode::into_raw_mode(stdout.lock())?)?;

    let mut frame_count = 0;
    while frames == 0 || frame_count < frames {
        let start = Instant::now();
        let delta = start.duration_since(state.last_frame).as_secs_f64();
        state.last_frame = start;

        let frame = render_animated_frame(forest, layout, &mut state, width, height, delta);
        write!(stdout, "{}