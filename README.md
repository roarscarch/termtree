# Git Forest

> See your git history grow

A CLI tool that renders your git commit graph as an interactive ASCII-art forest where branches become trees and merges become root systems, revealing the hidden organic structure of your repository.

## Stack
- Language: **rust**
- git2, termion

## Features
- Render the full commit graph as a scrolling ASCII forest with trunks for long-lived branches
- Color trees by author, with leaf density proportional to commit frequency on each branch
- Interactive mode: scroll/zoom with arrow keys and inspect commits by clicking on leaves
- Detect and highlight merge storms (many simultaneous merges) as tangled root systems
- Export the forest as a static SVG for sharing on social media or docs

## Architecture
Uses a custom topological layout algorithm that maps the DAG to a 2D grid with gravitational pull between related commits, then applies a recursive tree-skeletonizer to convert linear segments into organic tree shapes with realistic branch taper.

## Getting Started
```bash
# Coming soon — this project is under active development.
```

*Built fresh every day by an AI-powered automation pipeline.*
