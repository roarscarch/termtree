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
    let mut output = String::new();
    let w = width as usize;
    let h = height as usize;
    let mut screen = vec![vec![' '; w]; h];
    let mut colors = vec![vec![(255u8, 255u8, 255u8); w]; h];

    // Update growth progress
    state.growth += delta * 0.3;
    if state.growth > 1.0 {
        state.growth = 1.0;
    }

    // Update season (roughly every 5 seconds at 60fps)
    state.season = ((state.last_frame.elapsed().as_secs_f64() / 5.0) as u8) % 4;

    for (i, tree) in forest.trees.iter().enumerate() {
        if i >= layout.tree_centers.len() {
            continue;
        }
        let center_x = layout.tree_centers[i];
        let screen_x = (center_x * (w as f64 - 1.0)) as usize;
        if screen_x >= w { continue; }

        let trunk_height = (h as f64 * 0.6 * state.growth) as usize;
        let leaf_start = h.saturating_sub(trunk_height);

        // Draw trunk
        let trunk_color = tree.color;
        for y in leaf_start..h {
            if y < h && screen_x < w {
                screen[y][screen_x] = '|';
                colors[y][screen_x] = trunk_color;
            }
        }

        // Draw leaves (proportional to commit frequency)
        let leaf_density = (tree.commits.len() as f64).sqrt() as usize;
        for _ in 0..leaf_density.min(5) {
            let leaf_y = leaf_start + fastrand::usize(0..trunk_height.max(1));
            let leaf_x = screen_x + fastrand::i32(-2..=2) as usize;
            if leaf_y < h && leaf_x < w {
                let leaf_char = match state.season {
                    0 => '*', // spring buds
                    1 => '@', // summer full
                    2 => '%', // autumn
                    _ => '.', // winter sparse
                };
                screen[leaf_y][leaf_x] = leaf_char;
                let leaf_color = match state.season {
                    0 => (100, 200, 100), // light green
                    1 => (50, 180, 50),   // dark green
                    2 => (200, 100, 50),  // orange
                    _ => (150, 150, 150), // gray
                };
                colors[leaf_y][leaf_x] = leaf_color;
            }
        }
    }

    // Draw merge nodes as root systems
    for (merge_id, pos) in &layout.merge_positions {
        let mx = (pos.0 * (w as f64 - 1.0)) as usize;
        let my = (pos.1 * (h as f64 - 1.0)) as usize;
        if mx < w && my < h {
            screen[my][mx] = '#';
            colors[my][mx] = (139, 69, 19); // brown
        }
    }

    // Build output string with ANSI color codes
    use termion::color::Fg;
    use termion::color::Bg;
    for y in 0..h {
        for x in 0..w {
            let (r, g, b) = colors[y][x];
            let fg = Fg(color::Rgb(r, g, b));
            output.push_str(&format!("{}{}", fg, screen[y][x]));
        }
        if y < h - 1 {
            output.push('\n');
        }
    }
    // Reset color
    output.push_str(&format!("{}", color::Fg(color::Reset)));
    output
}

/// Run the animation loop for a fixed duration.
pub fn run_animation(forest: &Forest, layout: &LayoutResult, duration_secs: f64) -> Result<(), Box<dyn std::error::Error>> {
    let mut stdout = stdout().into_raw_mode()?;
    let (width, height) = termion::terminal_size()?;
    let mut state = AnimationState::default();
    let start = Instant::now();
    let mut last_frame = Instant::now();

    write!(stdout, "{}", termion::clear::All)?;
    write!(stdout, "{}", termion::cursor::Hide)?;

    loop {
        let elapsed = start.elapsed().as_secs_f64();
        if elapsed > duration_secs {
            break;
        }
        let delta = last_frame.elapsed().as_secs_f64();
        last_frame = Instant::now();

        let frame = render_animated_frame(forest, layout, &mut state, width, height, delta);
        write!(stdout, "{}", termion::cursor::Goto(1, 1))?;
        write!(stdout, "{}", frame)?;
        stdout.flush()?;

        // Cap at ~30fps
        let frame_duration = Duration::from_secs_f64(1.0 / 30.0);
        let remaining = frame_duration.saturating_sub(last_frame.elapsed());
        if remaining > Duration::from_millis(1) {
            thread::sleep(remaining);
        }
    }

    write!(stdout, "{}", termion::cursor::Show)?;
    write!(stdout, "{}", termion::clear::All)?;
    stdout.flush()?;
    Ok(())
}

/// Render a static ASCII frame (no animation) for export or fallback.
pub fn render_static_frame(
    forest: &Forest,
    layout: &LayoutResult,
    width: u16,
    height: u16,
) -> String {
    let mut state = AnimationState {
        growth: 1.0,
        season: 1,
        running: false,
        last_frame: Instant::now(),
    };
    render_animated_frame(forest, layout, &mut state, width, height, 0.0)
}