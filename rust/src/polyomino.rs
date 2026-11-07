//! Generate and name free polyominoes (shapes made of edge-joined squares,
//! counting rotations and mirror images as the same shape).
use crate::{Cell, normalize, orientations};
use std::collections::BTreeSet;

/// A shape in canonical form: the smallest of its 8 orientations.
pub fn canonical(cells: &[Cell]) -> Vec<Cell> {
    orientations(cells, true).into_iter().min().unwrap()
}

/// Every free polyomino with `n` squares, in a stable order.
/// Sizes 4, 5 and 6 give 5, 12 and 35 shapes.
pub fn free_polyominoes(n: usize) -> Vec<Vec<Cell>> {
    let mut shapes: BTreeSet<Vec<Cell>> = BTreeSet::from([vec![(0, 0)]]);
    for _ in 1..n {
        let mut grown = BTreeSet::new();
        for shape in &shapes {
            // Shift by one so neighbours above/left stay non-negative.
            let shifted: Vec<Cell> = shape.iter().map(|&(r, c)| (r + 1, c + 1)).collect();
            for &(r, c) in &shifted {
                for (nr, nc) in [(r - 1, c), (r + 1, c), (r, c - 1), (r, c + 1)] {
                    if !shifted.contains(&(nr, nc)) {
                        let mut next = shifted.clone();
                        next.push((nr, nc));
                        grown.insert(canonical(&normalize(&next)));
                    }
                }
            }
        }
        shapes = grown;
    }
    shapes.into_iter().collect()
}

fn parse(art: &str) -> Vec<Cell> {
    let mut cells = Vec::new();
    for (r, line) in art.split('/').enumerate() {
        for (c, ch) in line.chars().enumerate() {
            if ch == '#' {
                cells.push((r, c));
            }
        }
    }
    cells
}

/// Conventional letter names for tetrominoes and pentominoes.
const NAMED: [(&str, &str); 17] = [
    ("I4", "####"),
    ("O4", "##/##"),
    ("T4", "###/.#"),
    ("S4", ".##/##"),
    ("L4", "###/#"),
    ("F", ".##/##./.#"),
    ("I", "#####"),
    ("L", "####/#"),
    ("N", "##../.###"),
    ("P", "##/##/#"),
    ("T", "###/.#./.#"),
    ("U", "#.#/###"),
    ("V", "#../#../###"),
    ("W", "#../##./.##"),
    ("X", ".#./###/.#"),
    ("Y", "####/.#"),
    ("Z", "##./.#./.##"),
];

/// Name a shape: a letter for tetrominoes and pentominoes ("I4".."L4", "F".."Z"),
/// or "H01".."H35" for hexominoes (in [`free_polyominoes`] order).
pub fn name(cells: &[Cell]) -> String {
    let canon = canonical(cells);
    for (name, art) in NAMED {
        if canonical(&parse(art)) == canon {
            return name.to_string();
        }
    }
    if cells.len() == 6 {
        if let Some(i) = free_polyominoes(6).iter().position(|s| *s == canon) {
            return format!("H{:02}", i + 1);
        }
    }
    format!("{}-omino", cells.len())
}

/// Draw a shape with `#` and `.`, rows separated by `/`.
pub fn art(cells: &[Cell]) -> String {
    let cells = normalize(cells);
    let h = cells.iter().map(|c| c.0).max().unwrap_or(0) + 1;
    let w = cells.iter().map(|c| c.1).max().unwrap_or(0) + 1;
    (0..h)
        .map(|r| (0..w).map(|c| if cells.contains(&(r, c)) { '#' } else { '.' }).collect::<String>())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_and_names() {
        assert_eq!(free_polyominoes(4).len(), 5);
        assert_eq!(free_polyominoes(5).len(), 12);
        assert_eq!(free_polyominoes(6).len(), 35);
        let mut names: Vec<String> = free_polyominoes(5).iter().map(|s| name(s)).collect();
        names.sort();
        assert_eq!(names.join(""), "FILNPTUVWXYZ");
        assert_eq!(name(&parse("##/##/##")), name(&parse("###/###")));
    }
}
