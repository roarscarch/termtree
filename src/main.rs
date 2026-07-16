use std::io::{self, Write, stdout, stdin};
use std::time::{Duration, Instant};
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::cursor;
use termion::clear;
use termion::color;

mod types;
mod scan;
mod color;
mod layout;
mod render;
mod interact;
mod animate;
mod merge_storm;
mod svg;

use types::{Forest, Tree, CommitNode, MergeNode};
use scan::scan_repository;
use color::assign_author_colors;
use layout::layout_forest;
use render::render_forest;
use interact::{InteractiveView, find_commit_at_position};
use animate::{AnimationState, render_animated_frame};
use merge_storm::detect_merge_storms;
use svg::export_svg;

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let repo_path = if args.len() > 1 { &args[1] } else { "." };

    println!("Scanning repository: {}", repo_path);
    let forest = scan_repository(repo_path).map_err(|e| {
        io::Error::new(io::ErrorKind::Other, format!("Failed to scan repo: {}", e))
    })?;

    if forest.trees.is_empty() {
        eprintln!("No commits found in repository.");
        std::process::exit(1);
    }

    println!("Found {} trees (branches) with {} commits total.", forest.trees.len(), forest.commit_map.len());

    // Assign colors to authors
    let author_colors = assign_author_colors(&forest);
    println!("Assigned colors to {} authors.", author_colors.len());

    // Detect merge storms
    let storms = detect_merge_storms(&forest);
    if storms.is_empty() {
        println!("No merge storms detected.");
    } else {
        println!("Detected {} merge storm(s).", storms.len());
    }

    // Layout the forest
    let (width, height) = termion::terminal_size().unwrap_or((80, 24));
    let layout = layout_forest(&forest, width as f64, height as f64);
    println!("Layout computed: {} positions, {} merge positions.", layout.positions.len(), layout.merge_positions.len());

    // Check for SVG export flag
    if args.iter().any(|a| a == "--svg") || args.iter().any(|a| a == "-s") {
        let svg_path = if args.len() > 2 {
            &args[2]
        } else {
            "forest.svg"
        };
        export_svg(&forest, &layout, &author_colors, &storms, svg_path).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("SVG export failed: {}", e))
        })?;
        println!("Exported forest to {}", svg_path);
        return Ok(());
    }

    // Check for animation flag
    let animate_flag = args.iter().any(|a| a == "--animate") || args.iter().any(|a| a == "-a");

    if animate_flag {
        // Animated mode
        let mut state = AnimationState::default();
        state.running = true;
        println!("Starting animated forest view. Press 'q' to quit.");
        let stdout = stdout();
        let mut stdout = stdout.lock();
        write!(stdout, "{}", cursor::Hide)?;
        let mut stdin = stdin();
        // We need non-blocking input; we'll use a polling approach
        let mut last_update = Instant::now();
        let frame_duration = Duration::from_millis(50);
        let mut terminal_size = termion::terminal_size().unwrap_or((80, 24));

        loop {
            let now = Instant::now();
            let delta = (now - last_update).as_secs_f64();
            last_update = now;

            // Check for keypress (non-blocking)
            if let Some(Ok(key)) = stdin.lock().keys().next() {
                match key {
                    Key::Char('q') | Key::Char('Q') => break,
                    Key::Char('r') => state.growth = 0.0,
                    Key::Char('s') => state.season = (state.season + 1) % 4,
                    _ => {}
                }
            }

            // Update animation state
            state.growth = (state.growth + delta * 0.3).min(1.0);
            if state.growth >= 1.0 {
                state.growth = 0.0;
                state.season = (state.season + 1) % 4;
            }

            // Get current terminal size
            if let Ok((w, h)) = termion::terminal_size() {
                terminal_size = (w, h);
            }

            // Render frame
            let frame = render_animated_frame(&forest, &layout, &mut state, terminal_size.0, terminal_size.1, delta);

            // Write frame
            write!(stdout, "{}{}{}", cursor::Goto(1, 1), clear::All, frame)?;
            stdout.flush()?;

            // Sleep for frame duration
            std::thread::sleep(frame_duration);
        }

        write!(stdout, "{}", cursor::Show)?;
        stdout.flush()?;
        println!("Animated view closed.");
    } else {
        // Interactive mode
        let mut view = InteractiveView::default();
        let stdout = stdout();
        let mut stdout = stdout.into_raw_mode()?;
        write!(stdout, "{}{}", clear::All, cursor::Hide)?;
        stdout.flush()?;

        let stdin = stdin();
        let mut keys = stdin.keys();

        loop {
            // Re-render forest
            let rendered = render_forest(&forest, &layout, &author_colors, &storms, &view);
            write!(stdout, "{}", cursor::Goto(1, 1))?;
            write!(stdout, "{}", rendered)?;
            stdout.flush()?;

            // Handle input
            if let Some(Ok(key)) = keys.next() {
                match key {
                    Key::Char('q') | Key::Char('Q') => break,
                    Key::Left => view.offset_x -= 0.1 / view.zoom,
                    Key::Right => view.offset_x += 0.1 / view.zoom,
                    Key::Up => view.offset_y -= 0.1 / view.zoom,
                    Key::Down => view.offset_y += 0.1 / view.zoom,
                    Key::Char('+') | Key::Char('=') => view.zoom = (view.zoom * 1.2).min(10.0),
                    Key::Char('-') => view.zoom = (view.zoom / 1.2).max(0.1),
                    Key::Char('i') => view.info_visible = !view.info_visible,
                    Key::Char(' ') => {
                        // Inspect commit at center of screen (or near center)
                        let screen_x = width / 2;
                        let screen_y = height / 2;
                        if let Some(commit) = find_commit_at_position(&forest, screen_x, screen_y, &view) {
                            view.selected_commit = Some(commit);
                        }
                    }
                    _ => {}
                }
            }
        }

        write!(stdout, "{}", cursor::Show)?;
        stdout.suspend_raw_mode()?;
        println!("Interactive view closed.");
    }

    Ok(())
}
