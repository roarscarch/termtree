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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assign_author_colors_empty() {
        let forest = Forest {
            trees: vec![],
            commit_map: HashMap::new(),
            merges: vec![],
        };
        let colors = assign_author_colors(&forest);
        assert!(colors.is_empty());
    }

    #[test]
    fn test_assign_author_colors_distinct() {
        use std::collections::HashSet;
        let mut commit_map = HashMap::new();
        commit_map.insert("alice".to_string(), crate::CommitNode {
            id: "abc".to_string(),
            author: "alice".to_string(),
            time: 100,
            message: "msg".to_string(),
            parents: vec![],
        });
        commit_map.insert("bob".to_string(), crate::CommitNode {
            id: "def".to_string(),
            author: "bob".to_string(),
            time: 200,
            message: "msg2".to_string(),
            parents: vec![],
        });
        let forest = Forest {
            trees: vec![],
            commit_map,
            merges: vec![],
        };
        let colors = assign_author_colors(&forest);
        assert_eq!(colors.len(), 2);
        // Colors should be different
        let mut unique: HashSet<(u8,u8,u8)> = HashSet::new();
        for c in colors.values() {
            unique.insert(*c);
        }
        assert_eq!(unique.len(), 2);
    }
}