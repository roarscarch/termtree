use crate::{Forest, CommitNode, MergeNode};
use std::collections::HashMap;
use termion::color;

/// Format commit details for display in inspect panel.
pub fn format_commit_details(
    forest: &Forest,
    commit: &CommitNode,
    author_colors: &HashMap<String, (u8, u8, u8)>,
) -> String {
    let mut details = String::new();

    // Header
    details.push_str(&format!(
        "{}Commit Details{}\r\n",
        color::Fg(color::Rgb(255, 255, 255)),
        color::Fg(color::Reset)
    ));
    details.push_str(&"─".repeat(40));
    details.push_str("\r\n");

    // Commit ID (short)
    let short_id = if commit.id.len() > 7 {
        &commit.id[..7]
    } else {
        &commit.id
    };
    details.push_str(&format!(
        "{}ID:{} {}\r\n",
        color::Fg(color::Rgb(100, 200, 255)),
        color::Fg(color::Reset),
        short_id
    ));

    // Author
    let author_color = author_colors.get(&commit.author).copied().unwrap_or((200, 200, 200));
    details.push_str(&format!(
        "{}Author:{} {}{}{}\r\n",
        color::Fg(color::Rgb(100, 200, 255)),
        color::Fg(color::Rgb(author_color.0, author_color.1, author_color.2)),
        commit.author,
        color::Fg(color::Reset),
    ));

    // Timestamp
    details.push_str(&format!(
        "{}Date:{} {}\r\n",
        color::Fg(color::Rgb(100, 200, 255)),
        color::Fg(color::Reset),
        commit.time
    ));

    // Message (first line)
    let first_line = commit.message.lines().next().unwrap_or(&commit.message);
    details.push_str(&format!(
        "{}Message:{} {}\r\n",
        color::Fg(color::Rgb(100, 200, 255)),
        color::Fg(color::Reset),
        first_line
    ));

    // Number of parents
    details.push_str(&format!(
        "{}Parents:{} {}\r\n",
        color::Fg(color::Rgb(100, 200, 255)),
        color::Fg(color::Reset),
        commit.parents.len()
    ));

    details.push_str(&"─".repeat(40));
    details.push_str("\r\n");

    details
}

/// Format merge node details for display in inspect panel.
pub fn format_merge_details(
    forest: &Forest,
    merge: &MergeNode,
    author_colors: &HashMap<String, (u8, u8, u8)>,
) -> String {
    let mut details = String::new();

    details.push_str(&format!(
        "{}Merge Details{}\r\n",
        color::Fg(color::Rgb(255, 200, 100)),
        color::Fg(color::Reset)
    ));
    details.push_str(&"─".repeat(40));
    details.push_str("\r\n");

    details.push_str(&format!(
        "{}Merge ID:{} {}\r\n",
        color::Fg(color::Rgb(255, 200, 100)),
        color::Fg(color::Reset),
        merge.id
    ));

    details.push_str(&format!(
        "{}Branches merged:{} {}\r\n",
        color::Fg(color::Rgb(255, 200, 100)),
        color::Fg(color::Reset),
        merge.branches.len()
    ));

    for branch in &merge.branches {
        let author_color = author_colors.get(&branch.author).copied().unwrap_or((200, 200, 200));
        details.push_str(&format!(
            "  - {} (author: {}{}{})\r\n",
            branch.id,
            color::Fg(color::Rgb(author_color.0, author_color.1, author_color.2)),
            branch.author,
            color::Fg(color::Reset)
        ));
    }

    details.push_str(&"─".repeat(40));
    details.push_str("\r\n");

    details
}

/// Format a full inspection panel string for a given commit or merge.
pub fn format_inspection_panel(
    forest: &Forest,
    selected_commit: Option<&CommitNode>,
    selected_merge: Option<&MergeNode>,
    author_colors: &HashMap<String, (u8, u8, u8)>,
) -> String {
    let mut panel = String::new();

    if let Some(commit) = selected_commit {
        panel.push_str(&format_commit_details(forest, commit, author_colors));
    } else if let Some(merge) = selected_merge {
        panel.push_str(&format_merge_details(forest, merge, author_colors));
    } else {
        panel.push_str(&format!(
            "{}No commit selected. Use arrow keys to navigate and click on a leaf to inspect.{}\r\n",
            color::Fg(color::Rgb(150, 150, 150)),
            color::Fg(color::Reset)
        ));
    }

    panel.push_str(&format!(
        "{}Press 'i' to toggle this panel, 'q' to quit.{}\r\n",
        color::Fg(color::Rgb(100, 100, 100)),
        color::Fg(color::Reset)
    ));

    panel
}
