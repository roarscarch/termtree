# Git Forest

ASCII-art visualization of git history. Branches are drawn as tree limbs, merges as root systems. Works in any terminal.

## Why trees?
I wanted a way to "see" branch health at a glance. Long straight branches = stale, bushy with merges = active. The tree metaphor actually maps pretty well to git topology.

## Usage
```
cargo build --release
./target/release/git-forest --repo /path/to/repo
```

## Controls
- Arrow keys: scroll/zoom
- Click a branch: show commit details
- `q`: quit
- Export to SVG with `--export`
