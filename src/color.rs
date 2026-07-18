use crate::Forest;
use std::collections::HashMap;

/// Assign a unique color to each author using golden angle hue.
/// Colors are distributed evenly in HSL space, then converted to (r,g,b) for terminal use.
pub fn assign_author_colors(forest: &Forest) -> HashMap<String, (u8, u8, u8)> {
    let mut authors: Vec<&String> = forest.commit_map.keys().collect();
    authors.sort();
    let mut author_colors = HashMap::new();
    let golden_angle = 137.508; // degrees
    let mut hue = 0.0;
    for author in &authors {
        let h = hue % 360.0;
        let (r, g, b) = hsl_to_rgb(h, 0.7, 0.6);
        author_colors.insert((*author).clone(), (r, g, b));
        hue += golden_angle;
    }
    author_colors
}

/// Convert HSL to RGB, all values in [0,1] for H, S, L; output u8.
fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r1, g1, b1) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    (
        ((r1 + m) * 255.0) as u8,
        ((g1 + m) * 255.0) as u8,
        ((b1 + m) * 255.0) as u8,
    )
}

/// Apply a color tag to a string for terminal output using termion.
pub fn colorize(text: &str, r: u8, g: u8, b: u8) -> String {
    format!(
        "{}{}{}",
        termion::color::Fg(termion::color::Rgb(r, g, b)),
        text,
        termion::color::Fg(termion::color::Reset)
    )
}

/// Return a brightness-adjusted version of the given color for highlighting.
pub fn highlight_color(r: u8, g: u8, b: u8) -> (u8, u8, u8) {
    let factor = 1.3;
    (
        (r as f64 * factor).min(255.0) as u8,
        (g as f64 * factor).min(255.0) as u8,
        (b as f64 * factor).min(255.0) as u8,
    )
}

/// Return a dimmed version of the given color for background or inactive elements.
pub fn dim_color(r: u8, g: u8, b: u8) -> (u8, u8, u8) {
    let factor = 0.6;
    (
        (r as f64 * factor) as u8,
        (g as f64 * factor) as u8,
        (b as f64 * factor) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hsl_to_rgb_red() {
        let (r, g, b) = hsl_to_rgb(0.0, 1.0, 0.5);
        assert_eq!(r, 255);
        assert_eq!(g, 0);
        assert_eq!(b, 0);
    }

    #[test]
    fn test_hsl_to_rgb_green() {
        let (r, g, b) = hsl_to_rgb(120.0, 1.0, 0.5);
        assert_eq!(r, 0);
        assert_eq!(g, 255);
        assert_eq!(b, 0);
    }

    #[test]
    fn test_hsl_to_rgb_blue() {
        let (r, g, b) = hsl_to_rgb(240.0, 1.0, 0.5);
        assert_eq!(r, 0);
        assert_eq!(g, 0);
        assert_eq!(b, 255);
    }

    #[test]
    fn test_assign_author_colors_empty_forest() {
        let forest = Forest {
            trees: Vec::new(),
            commit_map: HashMap::new(),
            root: String::new(),
            merge_points: Vec::new(),
        };
        let colors = assign_author_colors(&forest);
        assert!(colors.is_empty());
    }

    #[test]
    fn test_assign_author_colors_unique() {
        let mut forest = Forest {
            trees: Vec::new(),
            commit_map: HashMap::new(),
            root: String::new(),
            merge_points: Vec::new(),
        };
        forest.commit_map.insert("author1".to_string(), crate::CommitNode {
            id: "abc".to_string(),
            author: "author1".to_string(),
            message: "msg".to_string(),
            timestamp: 0,
            parent_ids: Vec::new(),
            children_ids: Vec::new(),
            branch_id: 0,
            x: 0.0,
            y: 0.0,
        });
        forest.commit_map.insert("author2".to_string(), crate::CommitNode {
            id: "def".to_string(),
            author: "author2".to_string(),
            message: "msg2".to_string(),
            timestamp: 1,
            parent_ids: Vec::new(),
            children_ids: Vec::new(),
            branch_id: 1,
            x: 0.0,
            y: 0.0,
        });
        let colors = assign_author_colors(&forest);
        assert_eq!(colors.len(), 2);
        // Ensure colors are different
        assert_ne!(colors.get("author1"), colors.get("author2"));
    }

    #[test]
    fn test_colorize() {
        let result = colorize("hello", 255, 0, 0);
        assert!(result.contains("hello"));
        assert!(result.contains("\x1b[38;2;255;0;0m"));
    }

    #[test]
    fn test_highlight_color() {
        let (r, g, b) = highlight_color(100, 100, 100);
        assert!(r > 100);
        assert!(g > 100);
        assert!(b > 100);
    }

    #[test]
    fn test_dim_color() {
        let (r, g, b) = dim_color(200, 200, 200);
        assert!(r < 200);
        assert!(g < 200);
        assert!(b < 200);
    }
}