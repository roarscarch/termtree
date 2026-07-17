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
    state: &AnimationState,
    author_colors: &std::collections::HashMap<String, (u8, u8, u8)>,
) -> String {
    let mut output = String::new();
    let mut lines: Vec<String> = Vec::new();
    let height = 40;
    let width = 120;

    for y in (0..height).rev() {
        let mut line = String::new();
        for x in 0..width {
            let mut found = false;
            for (i, tree) in forest.trees.iter().enumerate() {
                let trunk_x = (tree.pos_x * (width as f64)) as usize;
                let trunk_y = (tree.pos_y * (height as f64)) as usize;
                if x == trunk_x && y <= trunk_y {
                    let visible_height = (tree.height * state.growth) as usize;
                    if y >= trunk_y - visible_height && y <= trunk_y {
                        // Draw trunk segment
                        let brightness = 40 + (80.0 * (1.0 - (trunk_y - y) as f64 / tree.height)) as u8;
                        let r = brightness;
                        let g = brightness.saturating_sub(20);
                        let b = brightness.saturating_sub(40);
                        line.push_str(&format!(
                            "{}{}",
                            color::Fg(color::Rgb(r, g, b)),
                            "||"
                        ));
                        found = true;
                        break;
                    }
                }
                // Draw leaves for tree
                if let Some(ref leaves) = tree.leaves {
                    for leaf in leaves {
                        let lx = (leaf.x * (width as f64)) as usize;
                        let ly = (leaf.y * (height as f64)) as usize;
                        if x == lx && y == ly {
                            // Leaf color based on season
                            let (r, g, b) = match state.season {
                                0 => (100, 200, 100), // spring green
                                1 => (50, 180, 50),   // summer green
                                2 => (200, 150, 50),  // autumn orange
                                _ => (150, 150, 150), // winter gray
                            };
                            line.push_str(&format!(
                                "{}{}",
                                color::Fg(color::Rgb(r, g, b)),
                                "@"
                            ));
                            found = true;
                            break;
                        }
                    }
                }
            }
            if !found {
                line.push(' ');
            }
        }
        lines.push(line);
    }

    for line in lines.iter().rev() {
        output.push_str(line);
        output.push_str("\r\n");
    }

    output
}

/// Update animation state: advance growth and season.
pub fn update_animation_state(state: &mut AnimationState, dt: f64) {
    if state.running {
        // Growth: complete in 5 seconds
        state.growth += dt * 0.2;
        if state.growth > 1.0 {
            state.growth = 1.0;
            state.running = false;
        }
        // Season: cycle every 30 seconds
        state.season = ((state.sway_time / 30.0) as u8) % 4;
        state.sway_time += dt;
    }
}

/// Run the growth animation for a given forest, printing frames to stdout.
pub fn run_growth_animation(
    forest: &Forest,
    author_colors: &std::collections::HashMap<String, (u8, u8, u8)>,
) {
    let mut state = AnimationState {
        running: true,
        growth: 0.0,
        ..Default::default()
    };

    let stdout = stdout();
    let mut stdout = stdout.lock();
    let mut last_time = Instant::now();

    // Clear screen
    write!(stdout, "{}", termion::clear::All).unwrap();
    write!(stdout, "{}", termion::cursor::Goto(1, 1)).unwrap();
    stdout.flush().unwrap();

    while state.running {
        let now = Instant::now();
        let dt = now.duration_since(last_time).as_secs_f64();
        last_time = now;

        update_animation_state(&mut state, dt);

        let frame = render_animated_frame(forest, &state, author_colors);

        write!(stdout, "{}", termion::cursor::Goto(1, 1)).unwrap();
        write!(stdout, "{}", frame).unwrap();
        stdout.flush().unwrap();

        thread::sleep(Duration::from_millis(50));
    }

    // Final frame
    let frame = render_animated_frame(forest, &state, author_colors);
    write!(stdout, "{}", termion::cursor::Goto(1, 1)).unwrap();
    write!(stdout, "{}", frame).unwrap();
    stdout.flush().unwrap();
}
