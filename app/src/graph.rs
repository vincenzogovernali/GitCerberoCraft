//! Commit graph layout: gives each commit a column ("lane") and lists the line
//! segments to draw between one row and the next.

use crate::git::CommitRow;
use git2::Oid;
use std::collections::HashSet;

#[derive(Clone, Debug, Default)]
pub struct Segment {
    /// Column at the upper row.
    pub from: usize,
    /// Column at the lower row.
    pub to: usize,
    pub color: usize,
}

#[derive(Clone, Debug, Default)]
pub struct RowLayout {
    pub col: usize,
    pub color: usize,
    pub is_merge: bool,
    /// Segments from this row's centre to the next row's centre.
    pub down: Vec<Segment>,
}

#[derive(Clone, Debug, Default)]
pub struct Layout {
    pub rows: Vec<RowLayout>,
    pub max_cols: usize,
}

fn alloc(lanes: &mut Vec<Option<(Oid, usize)>>) -> usize {
    if let Some(i) = lanes.iter().position(|l| l.is_none()) {
        i
    } else {
        lanes.push(None);
        lanes.len() - 1
    }
}

pub fn layout(commits: &[CommitRow]) -> Layout {
    let known: HashSet<Oid> = commits.iter().map(|c| c.id).collect();
    let mut lanes: Vec<Option<(Oid, usize)>> = Vec::new();
    let mut next_color = 0usize;
    let mut rows: Vec<RowLayout> = Vec::with_capacity(commits.len());
    let mut max_cols = 1;

    for (r, c) in commits.iter().enumerate() {
        let found = lanes.iter().position(|l| matches!(l, Some((o, _)) if *o == c.id));
        let (col, color) = match found {
            Some(i) => (i, lanes[i].unwrap().1),
            None => {
                let i = alloc(&mut lanes);
                next_color += 1;
                (i, next_color - 1)
            }
        };
        for l in lanes.iter_mut() {
            if matches!(l, Some((o, _)) if *o == c.id) {
                *l = None;
            }
        }
        let parents: Vec<Oid> = c.parents.iter().copied().filter(|p| known.contains(p)).collect();
        let mut from_node: Vec<usize> = Vec::new();
        for (pi, p) in parents.iter().enumerate() {
            if let Some(existing) = lanes.iter().position(|l| matches!(l, Some((o, _)) if o == p)) {
                from_node.push(existing);
            } else if pi == 0 {
                lanes[col] = Some((*p, color));
                from_node.push(col);
            } else {
                let k = alloc(&mut lanes);
                lanes[k] = Some((*p, next_color));
                next_color += 1;
                from_node.push(k);
            }
        }
        while matches!(lanes.last(), Some(None)) {
            lanes.pop();
        }
        max_cols = max_cols.max(lanes.len()).max(col + 1);

        // Where the next commit will sit, so converging lines land on it.
        let next = commits.get(r + 1).map(|n| n.id);
        let next_col = next.and_then(|n| lanes.iter().position(|l| matches!(l, Some((o, _)) if *o == n)));
        let mut down = Vec::new();
        for (k, l) in lanes.iter().enumerate() {
            if let Some((o, colr)) = l {
                let from = if from_node.contains(&k) { col } else { k };
                let to = match (next, next_col) {
                    (Some(n), Some(nc)) if *o == n => nc,
                    _ => k,
                };
                down.push(Segment { from, to, color: *colr });
                // A lane that already existed and also receives a parent line from the node.
                if from_node.contains(&k) && from != k && found.is_some() {
                    // nothing extra: the line from the node replaces the straight one
                }
            }
        }
        // Lanes reused by a parent that was already expected keep their straight line too.
        for &k in &from_node {
            let straight_exists = down.iter().any(|s| s.from == k && s.to == k);
            if k != col && !straight_exists {
                if let Some(Some((o, colr))) = lanes.get(k) {
                    let to = match (next, next_col) {
                        (Some(n), Some(nc)) if *o == n => nc,
                        _ => k,
                    };
                    down.push(Segment { from: k, to, color: *colr });
                }
            }
        }
        rows.push(RowLayout { col, color, is_merge: c.parents.len() > 1, down });
    }
    Layout { rows, max_cols }
}

/// Colours of the lanes, in order (bright on dark, similar to the reference).
pub const LANE_COLORS: [u32; 10] = [
    0x15A0BF, 0x0669F7, 0x8E00C2, 0xC517B6, 0xD90171, 0xCD0101, 0xF25D2E, 0xF2CA33, 0x7BD938, 0x2ECE9D,
];
